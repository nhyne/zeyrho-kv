use crate::zeyrho::btree::node::Node;
use crate::zeyrho::btree::{CHILDREN_MAX_SIZE, DEGREE, MAX_KVS_IN_LEAF, SEPARATORS_MAX_SIZE};
use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
/*
TODO:
    We have some problems with the Rc pointers to neighbors. I'm not sure if these should really be owning references, probably need to be weak ownership and during the
    drop of a Node we update pointers. The problem with this is that it's going to get _really_ complicated. How about for now we just drop the `next` and `previous` pointers.
 */

#[derive(Debug)]
pub struct BPlusTree<K: Ord + Debug, V: Debug> {
    pub root: Option<Rc<RefCell<Node<K, V>>>>,
}

impl<K: Debug + Ord, V: Debug> Display for BPlusTree<K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("root\n")?;

        match &self.root {
            None => f.write_str("None"),
            Some(node) => f.write_str(&format!("{}\n", *node.borrow())),
        }
    }
}

impl<K: Ord + Debug, V: Debug> Default for BPlusTree<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord + Debug, V: Debug> BPlusTree<K, V> {
    pub fn new() -> Self {
        BPlusTree { root: None }
    }

    pub fn insert(&mut self, key: K, value: V) {
        if self.root.is_none() {
            self.root = Some(Node::new_leaf_with_kv(Rc::new(key), value));
            return;
        }

        if let Some((new_separator, new_node)) =
            self.insert_internal(&self.root.as_ref().unwrap().clone(), Rc::new(key), value)
        {
            let new_root = Rc::new(RefCell::new(Node::Link {
                separators: vec![new_separator],
                children: vec![self.root.take().unwrap(), new_node],
            }));

            self.root = Some(new_root)
        }
    }

    // the left Option is the new separator and the right is the new right node. We don't need to do anything with the left node b/c the parent is already pointing to it
    fn insert_internal(
        &mut self,
        node: &Rc<RefCell<Node<K, V>>>,
        inserted_key: Rc<K>,
        inserted_value: V,
    ) -> Option<(Rc<K>, Rc<RefCell<Node<K, V>>>)> {
        let mut node_ref = node.borrow_mut();
        match &mut *node_ref {
            Node::Leaf { key_vals, .. } => {
                let pos = key_vals
                    .iter()
                    .position(|(k, _)| k.as_ref() > inserted_key.as_ref())
                    .unwrap_or(key_vals.len());

                key_vals.insert(pos, (inserted_key, inserted_value));

                if key_vals.len() <= MAX_KVS_IN_LEAF {
                    return None;
                }

                let (split, new_right) = (*node_ref).split_borrowed_leaf_node();

                Some((split, new_right))
            }
            Node::Link {
                separators,
                children,
            } => {
                let mut child_to_update = separators
                    .iter()
                    .position(|k| k.as_ref() > inserted_key.as_ref());

                // if we're inserting the biggest and the child location is empty then create new leaf and return current link
                if child_to_update.is_none() {
                    if separators.len() == SEPARATORS_MAX_SIZE {
                        // here we must insert into the right most subtree
                        if children.get(DEGREE - 1).is_none() {
                            // no child is here, we need to make a new one
                            let new_leaf = Node::new_leaf_with_kv(inserted_key, inserted_value);
                            children.push(new_leaf);
                            return None;
                        }
                    }
                    child_to_update = Some(children.len() - 1);
                }

                let child = children[child_to_update.unwrap()].clone();

                if let Some((new_separator, new_node)) =
                    self.insert_internal(&child, inserted_key, inserted_value)
                {
                    Node::insert_separator_and_child_into_link(
                        separators,
                        children,
                        new_separator,
                        new_node,
                    );
                    if separators.len() <= SEPARATORS_MAX_SIZE {
                        return None;
                    }

                    let (new_sep, new_right) = (*node_ref).split_borrowed_link_node();

                    return Some((new_sep, new_right));
                }

                None
            }
        }
    }
}

/*
TODO: Tests could have some better helper functions to reduce code duplication
TODO: Would be nice if the tests weren't based on DEGREE = 3
 */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::zeyrho::btree::DEGREE;
    use std::ops::Deref;

    fn create_tree() -> BPlusTree<i32, String> {
        BPlusTree::new()
    }

    #[test]
    fn test_single_leaf_node() {
        let mut tree = create_tree();

        for i in 0..CHILDREN_MAX_SIZE {
            tree.insert(i as i32, i.to_string());
        }
        let root = tree.root.as_ref().unwrap().borrow();

        if let Node::Leaf { key_vals, .. } = &*root {
            assert_eq!(key_vals.len(), CHILDREN_MAX_SIZE);
            let mut i = 0;
            key_vals.iter().for_each(|(x, _)| {
                assert_eq!(x.as_ref(), &i);
                i += 1;
            })
        } else {
            panic!("root is link node when it should be leaf node");
        }
    }

    #[test]
    fn test_root_link_node() {
        let mut tree = create_tree();
        for i in 0..DEGREE {
            tree.insert(i as i32, i.to_string());
        }
        let root = tree.root.as_ref().unwrap().borrow();
        if let Node::Link {
            separators,
            children,
        } = &*root
        {
            assert_eq!(separators.len(), 1);
            assert!(!separators.is_empty());
            assert_eq!(separators.first().unwrap().as_ref(), &1);
            assert_eq!(children.len(), 2);

            let mut separator_index = 0;
            children.iter().for_each(|child| {
                if let Node::Leaf { key_vals, .. } = &*child.borrow() {
                    key_vals
                        .iter()
                        .for_each(|(key, value): &(Rc<i32>, String)| {
                            match separators.get(separator_index) {
                                None => {
                                    assert!(separators.last().unwrap().as_ref() <= key.as_ref());
                                }
                                Some(separator_val) => {
                                    assert!(separator_val.as_ref() > key.as_ref());
                                }
                            }
                            assert_eq!(&key.as_ref().to_string(), value);
                        })
                }
                separator_index += 1;
            })
        } else {
            panic!("root is leaf node when it should be link node");
        }
    }

    #[test]
    fn test_full_root_link_node() {
        let mut tree = create_tree();
        for i in 0..5 {
            tree.insert(i, i.to_string());
        }

        let root = tree.root.as_ref().unwrap().borrow();
        if let Node::Link {
            separators,
            children,
        } = &*root
        {
            assert_eq!(separators.len(), 1);

            let mut separator_index = 0;
            children.iter().for_each(|child| {
                if let Node::Leaf { key_vals, .. } = &*child.borrow() {
                    key_vals
                        .iter()
                        .for_each(|(key, value): &(Rc<i32>, String)| {
                            assert!(separators[separator_index].as_ref() >= key.as_ref());
                            assert_eq!(&key.as_ref().to_string(), value);
                        })
                }
                separator_index += 1;
            });
        } else {
            panic!("root is leaf node when it should be link node");
        }
    }

    #[test]
    fn test_middle_inserts() {
        let mut tree = create_tree();
        for i in vec![0, 12, 2, 10, 4, 8, 6].iter() {
            tree.insert(*i, i.to_string());
        }

        let mut separator_index = 0;
        let expected_separators = [vec![&2], vec![&6, &10]];

        let mut child_index = 0;
        let expected_children = [vec![&0], vec![&2], vec![&4], vec![&6, &8], vec![&10, &12]];

        if let Node::Link {
            separators,
            children,
        } = tree.root.unwrap().borrow().deref()
        {
            assert_eq!(separators.len(), 1);

            children.iter().for_each(|child| {
                if let Node::Link {
                    separators,
                    children,
                    ..
                } = &*child.borrow()
                {
                    let collected: Vec<&i32> = separators.iter().map(|s| s.as_ref()).collect();
                    assert_eq!(expected_separators[separator_index], collected);
                    separator_index += 1;

                    for child in children.iter() {
                        if let Node::Leaf { key_vals, .. } = child.borrow().deref() {
                            let collected: Vec<&i32> = key_vals
                                .iter()
                                .map(|(k, _): &(Rc<i32>, String)| k.as_ref())
                                .collect();
                            assert_eq!(expected_children[child_index], collected);
                        }
                        child_index += 1;
                    }
                }
            });
        } else {
            panic!("root is leaf node when it should be link node");
        }
    }

    #[test]
    fn test_insert_smaller_keys() {
        let mut tree = create_tree();
        for i in (0..DEGREE * DEGREE).rev() {
            tree.insert(i as i32, i.to_string());
        }

        let mut separator_index = 0;
        let expected_separators = [vec![&1, &3], vec![&7]];

        let mut child_index = 0;
        let expected_children = [
            vec![&0],
            vec![&1, &2],
            vec![&3, &4],
            vec![&5, &6],
            vec![&7, &8],
        ];

        if let Node::Link {
            separators,
            children,
        } = tree.root.unwrap().borrow().deref()
        {
            assert_eq!(separators.len(), 1);

            children.iter().for_each(|child| {
                if let Node::Link {
                    separators,
                    children,
                    ..
                } = &*child.borrow()
                {
                    let collected: Vec<&i32> = separators.iter().map(|s| s.as_ref()).collect();
                    assert_eq!(expected_separators[separator_index], collected);
                    separator_index += 1;

                    for child in children.iter() {
                        if let Node::Leaf { key_vals, .. } = child.borrow().deref() {
                            let collected: Vec<&i32> = key_vals
                                .iter()
                                .map(|(k, _): &(Rc<i32>, String)| k.as_ref())
                                .collect();
                            assert_eq!(expected_children[child_index], collected);
                        }
                        child_index += 1;
                    }
                }
            });
        } else {
            panic!("root is leaf node when it should be link node");
        }
    }
}
