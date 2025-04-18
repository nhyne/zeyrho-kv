use crate::zeyrho::btree::node::{DeletionResult, Node};
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

    pub fn delete(&mut self, key: K) -> Result<(), ()> {
        match &self.root {
            None => Err(()),
            Some(root_rc) => {
                let internal_deletion = Node::delete_internal(&root_rc.clone(), key);
                match internal_deletion {
                    DeletionResult::RemovedFromLeaf { .. } => {
                        // If we removed a key from a leaf, we need to check if the root needs updating
                        let root_ref = root_rc.borrow();
                        if let Node::Link { internal_link } = &*root_ref {
                            if internal_link.separators.is_empty() {
                                // If root has no separators, it should have exactly one child
                                if internal_link.children.len() == 1 {
                                    let new_root = internal_link.children[0].clone();
                                    drop(root_ref); // Drop the borrow before modifying self.root
                                    self.root = Some(new_root);
                                } else {
                                    panic!("Root link node with no separators should have exactly one child");
                                }
                            }
                        }
                        Ok(())
                    }
                    DeletionResult::LeafNeedsBalancing { .. } => {
                        // If we need to balance a leaf, we need to check if the root needs updating
                        let root_ref = root_rc.borrow();
                        if let Node::Link { internal_link } = &*root_ref {
                            if internal_link.separators.is_empty() {
                                // If root has no separators, it should have exactly one child
                                if internal_link.children.len() == 1 {
                                    let new_root = internal_link.children[0].clone();
                                    drop(root_ref); // Drop the borrow before modifying self.root
                                    self.root = Some(new_root);
                                } else {
                                    panic!("Root link node with no separators should have exactly one child");
                                }
                            }
                        }
                        Ok(())
                    }
                    DeletionResult::NoOperation() => Ok(()),
                    DeletionResult::LinkNeedsBubble { link_needs_assistance, .. } => {
                        // If a link needs bubbling at the root level, we need to handle it specially
                        let mut root_ref = root_rc.borrow_mut();
                        match &mut *root_ref {
                            Node::Leaf { .. } => panic!("Cannot bubble link if root is a leaf"),
                            Node::Link { internal_link } => {
                                match internal_link.separators.len() {
                                    1 => {
                                        // If root has only one separator, we need to merge with a child
                                        let pos = internal_link.children.iter()
                                            .position(|c| Rc::ptr_eq(c, &link_needs_assistance))
                                            .unwrap();
                                        
                                        if pos > 0 {
                                            // Merge with left child
                                            let left_child = &internal_link.children[pos - 1];
                                            let mut left_ref = left_child.borrow_mut();
                                            let mut right_ref = link_needs_assistance.borrow_mut();
                                            
                                            match (&mut *left_ref, &mut *right_ref) {
                                                (Node::Leaf { internal_leaf: left_leaf }, Node::Leaf { internal_leaf: right_leaf }) => {
                                                    // Move all elements from right to left
                                                    while !right_leaf.key_vals.is_empty() {
                                                        let kv = right_leaf.key_vals.pop_front().unwrap();
                                                        left_leaf.key_vals.push_back(kv);
                                                    }
                                                    
                                                    // Update next/prev pointers
                                                    left_leaf.next = right_leaf.next.take();
                                                    if let Some(next) = &left_leaf.next {
                                                        if let Some(next_upgrade) = next.upgrade() {
                                                            let mut next_ref = next_upgrade.borrow_mut();
                                                            if let Node::Leaf { internal_leaf } = &mut *next_ref {
                                                                internal_leaf.prev = Some(Rc::downgrade(left_child));
                                                            }
                                                        }
                                                    }
                                                    
                                                    // Remove the right child and its separator
                                                    internal_link.children.remove(pos);
                                                    internal_link.separators.remove(pos - 1);
                                                }
                                                _ => panic!("Expected leaf nodes when merging")
                                            }
                                        } else {
                                            // Merge with right child
                                            let right_child = &internal_link.children[pos + 1];
                                            let mut left_ref = link_needs_assistance.borrow_mut();
                                            let mut right_ref = right_child.borrow_mut();
                                            
                                            match (&mut *left_ref, &mut *right_ref) {
                                                (Node::Leaf { internal_leaf: left_leaf }, Node::Leaf { internal_leaf: right_leaf }) => {
                                                    // Move all elements from left to right
                                                    while !left_leaf.key_vals.is_empty() {
                                                        let kv = left_leaf.key_vals.pop_back().unwrap();
                                                        right_leaf.key_vals.push_front(kv);
                                                    }
                                                    
                                                    // Update next/prev pointers
                                                    right_leaf.prev = left_leaf.prev.take();
                                                    if let Some(prev) = &right_leaf.prev {
                                                        if let Some(prev_upgrade) = prev.upgrade() {
                                                            let mut prev_ref = prev_upgrade.borrow_mut();
                                                            if let Node::Leaf { internal_leaf } = &mut *prev_ref {
                                                                internal_leaf.next = Some(Rc::downgrade(right_child));
                                                            }
                                                        }
                                                    }
                                                    
                                                    // Remove the left child and its separator
                                                    internal_link.children.remove(pos);
                                                    internal_link.separators.remove(pos);
                                                }
                                                _ => panic!("Expected leaf nodes when merging")
                                            }
                                        }
                                        
                                        // If we're left with no separators, the root should be replaced with its only child
                                        if internal_link.separators.is_empty() {
                                            if internal_link.children.len() == 1 {
                                                let new_root = internal_link.children[0].clone();
                                                drop(root_ref); // Drop the borrow before modifying self.root
                                                self.root = Some(new_root);
                                            } else {
                                                panic!("Root link node with no separators should have exactly one child");
                                            }
                                        }
                                    }
                                    _ => {
                                        // If root has more than one separator, we can just remove the child and separator
                                        let pos = internal_link.children.iter()
                                            .position(|c| Rc::ptr_eq(c, &link_needs_assistance))
                                            .unwrap();
                                        internal_link.children.remove(pos);
                                        internal_link.separators.remove(pos);
                                    }
                                }
                                Ok(())
                            }
                        }
                    }
                    DeletionResult::NothingDeleted() => Err(())
                }
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
    use crate::zeyrho::btree::{DEGREE, MAX_KVS_IN_LEAF};
    use std::ops::Deref;

    fn create_tree() -> BPlusTree<i32, String> {
        BPlusTree::new()
    }

    #[test]
    fn test_single_leaf_node() {
        let mut tree = create_tree();

        for i in 0..MAX_KVS_IN_LEAF {
            tree.insert(i as i32, i.to_string());
        }
        let root = tree.root.as_ref().unwrap().borrow();

        if let Node::Leaf { internal_leaf, .. } = &*root {
            assert_eq!(internal_leaf.len(), MAX_KVS_IN_LEAF);
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

    #[test]
    fn test_delete_only_key() {
        let mut tree = create_tree();
        tree.insert(3, 3.to_string());
        _ = tree.delete(3);
        assert!(tree.root.is_none());
    }

    #[test]
    fn test_delete_only_key_in_left_node() {
        let mut tree = create_tree();
        for i in (0..DEGREE) {
            tree.insert(i as i32, i.to_string());
        }
        println!("tree: {}", tree);
        _ = tree.delete(0);
        assert!(tree.root.is_some());
        if let Node::Link {
            internal_link
        } = tree.root.unwrap().borrow().deref()
        {
            assert_eq!(internal_link.separators.len(), 1);
            assert_eq!(internal_link.separators.first().unwrap().deref(), &2);

        }
    }

    #[test]
    fn test_delete_multi_level_tree() {
        let mut tree = create_tree();
        for i in [1,3,4,5,6,7] {
            tree.insert(i as i32, i.to_string());
        }
        println!("tree: {}", tree);
        _ = tree.delete(0);
        assert!(tree.root.is_none());
    }
}
