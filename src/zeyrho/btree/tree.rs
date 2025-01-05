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
            Node::insert_internal(&self.root.as_ref().unwrap().clone(), Rc::new(key), value)
        {
            let new_root = Node::new_link_with_seps_and_children(vec![new_separator], vec![self.root.take().unwrap(), new_node]);
            self.root = Some(new_root)
        }
    }

    pub fn delete(&mut self, key: K) -> Result<V, ()> {
        if self.root.is_none() {
            Err(())
        } else {
            let internal_deletion = Node::delete_internal(&self.root.as_ref().unwrap().clone(), key);
            match internal_deletion {
                Some(_) => {todo!()}
                None => {todo!()}
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

        if let Node::Leaf { internal_leaf, .. } = &*root {
            assert_eq!(internal_leaf.len(), CHILDREN_MAX_SIZE);
            let mut i = 0;
            internal_leaf.iter().for_each(|(x, _)| {
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
            internal_link,
        } = &*root
        {
            assert_eq!(internal_link.separators.len(), 1);
            assert!(!internal_link.separators.is_empty());
            assert_eq!(internal_link.separators.first().unwrap().as_ref(), &1);
            assert_eq!(internal_link.children.len(), 2);

            let mut separator_index = 0;
            internal_link.children.iter().for_each(|child| {
                if let Node::Leaf { internal_leaf, .. } = &*child.borrow() {
                    internal_leaf
                        .iter()
                        .for_each(|(key, value): &(Rc<i32>, String)| {
                            match internal_link.separators.get(separator_index) {
                                None => {
                                    assert!(internal_link.separators.last().unwrap().as_ref() <= key.as_ref());
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
            internal_link,
        } = &*root
        {
            assert_eq!(internal_link.separators.len(), 1);

            let mut separator_index = 0;
            internal_link.children.iter().for_each(|child| {
                if let Node::Leaf { internal_leaf, .. } = &*child.borrow() {
                    internal_leaf
                        .iter()
                        .for_each(|(key, value): &(Rc<i32>, String)| {
                            assert!(internal_link.separators[separator_index].as_ref() >= key.as_ref());
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
            internal_link,
        } = tree.root.unwrap().borrow().deref()
        {
            assert_eq!(internal_link.separators.len(), 1);

            internal_link.children.iter().for_each(|child| {
                if let Node::Link {
                    internal_link,
                } = &*child.borrow()
                {
                    let collected: Vec<&i32> = internal_link.separators.iter().map(|s| s.as_ref()).collect();
                    assert_eq!(expected_separators[separator_index], collected);
                    separator_index += 1;

                    for child in internal_link.children.iter() {
                        if let Node::Leaf { internal_leaf, .. } = child.borrow().deref() {
                            let collected: Vec<&i32> = internal_leaf
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
            internal_link
        } = tree.root.unwrap().borrow().deref()
        {
            assert_eq!(internal_link.separators.len(), 1);

            internal_link.children.iter().for_each(|child| {
                if let Node::Link {
                    internal_link
                } = &*child.borrow()
                {
                    let collected: Vec<&i32> = internal_link.separators.iter().map(|s| s.as_ref()).collect();
                    assert_eq!(expected_separators[separator_index], collected);
                    separator_index += 1;

                    for child in internal_link.children.iter() {
                        if let Node::Leaf { internal_leaf, .. } = child.borrow().deref() {
                            let collected: Vec<&i32> = internal_leaf
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
