use crate::zeyrho::btree::node::Node;
use crate::zeyrho::btree::{CHILDREN_MAX_SIZE, DEGREE, MAX_KVS_IN_LEAF, SEPARATORS_MAX_SIZE};
use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;
/*
TODO:
    We have some problems with the Rc pointers to neighbors. I'm not sure if these should really be owning references, probably need to be weak ownership and during the
    drop of a Node we update pointers. The problem with this is that it's going to get _really_ complicated. How about for now we just drop the `next` and `previous` pointers.

    Let's start with a basic BST without any pointers. It'll be easier and then after we can try doing the pointers to next and previous.

 */

#[derive(Debug)]
pub struct BPlusTree<K: Ord + std::fmt::Debug, V: std::fmt::Debug> {
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

    // fn depth_fmt(&self, node: Rc<RefCell<Node<K, V>>>, f: &mut Formatter<'_>, depth: i32) -> std::fmt::Result {
    //     match &*node.borrow() {
    //         Node::Leaf { key_vals, .. } => {
    //             f.write_str(&format!("key vals: {:?}", key_vals))?
    //         }
    //         Node::Link { separators, children, .. } => {
    //             f.write_str(&format!("separators: {:?}", separators))?;
    //             children.iter().for_each(|child| {
    //
    //             })
    //
    //         }
    //     }
    //     todo!()
    // }
}

//
// impl<K: Ord + Debug, V: Debug> Drop for BPlusTree<K, V>  {
//     fn drop(&mut self) {
//         todo!()
//     }
// }

impl<K: Ord + std::fmt::Debug, V: std::fmt::Debug> Default for BPlusTree<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord + std::fmt::Debug, V: std::fmt::Debug> BPlusTree<K, V> {
    pub fn new() -> Self {
        BPlusTree { root: None }
    }

    pub fn insert(&mut self, key: K, value: V) {
        if self.root.is_none() {
            self.root = Some(Rc::new(RefCell::new(Node::new_leaf_with_kv(
                Rc::new(key),
                value,
            ))));
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

    // TODO: The bubbling up is not correct right now. Inserting 0-6 is fine, but on insert of 7 we end up with a root Link node of just [7], with 3 children, which makes no sense.

    // the left Option is the new separator and the right is the new right node. We don't need to do anything with the left node b/c the parent is already pointing to it
    fn insert_internal(
        &mut self,
        node: &Rc<RefCell<Node<K, V>>>,
        inserted_key: Rc<K>,
        inserted_value: V,
    ) -> Option<(Rc<K>, Rc<RefCell<Node<K, V>>>)> {
        let mut node_ref = node.borrow_mut();
        match &mut *node_ref {
            Node::Leaf {
                key_vals, /* next, */
                ..
            } => {
                let pos = key_vals
                    .iter()
                    .position(|(k, _)| k.as_ref() > inserted_key.as_ref())
                    .unwrap_or(key_vals.len());

                key_vals.insert(pos, (inserted_key, inserted_value));

                if key_vals.len() <= MAX_KVS_IN_LEAF {
                    return None;
                }

                // the problem with inserting 7 comes after this line
                // the link node generation is working properly
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
                            children.push(Rc::new(RefCell::new(new_leaf)));
                            return None;
                        }
                    }
                    child_to_update = Some(children.len() - 1);
                }

                let child = children[child_to_update.unwrap()].clone();

                // Here somewhere we have a problem bubbling up the 7
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
    fn test_insert_smaller_keys() {
        let mut tree = create_tree();
        for i in (0..DEGREE * DEGREE).rev() {
            tree.insert(i as i32, i.to_string());
        }

        // with DEGREE = 3 tree should look like:
        /*
        root: 5
            left link: 1 , 3
                left leaf: 0
                mid leaf: 1, 2
                right leaf: 3, 4
            right link: 7
                left leaf: 5, 6
                right leaf: 8
         */

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
