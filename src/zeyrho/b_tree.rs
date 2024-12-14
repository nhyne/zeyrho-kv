use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const DEGREE: usize = 3;

#[derive(Debug, Clone)]
enum Node<K: Ord + std::fmt::Debug, V: std::fmt::Debug> {
    Leaf {
        key_vals: Vec<(Rc<K>, V)>,
        next: Option<Rc<RefCell<Node<K, V>>>>,
        prev: Option<Rc<RefCell<Node<K, V>>>>,
    },
    NonLeaf {
        separators: Vec<Rc<K>>,
        children: Vec<Rc<RefCell<Node<K, V>>>>,
    },
}

#[derive(Debug)]
struct BPlusTree<K: Ord + std::fmt::Debug, V: std::fmt::Debug> {
    root: Rc<RefCell<Node<K, V>>>,
}

impl<K: Ord + std::fmt::Debug, V: std::fmt::Debug> Node<K, V> {
    fn new_leaf() -> Self {
        Node::Leaf {
            key_vals: Vec::new(),
            next: None,
            prev: None,
        }
    }

    fn new_non_leaf() -> Self {
        Node::NonLeaf {
            separators: Vec::new(),
            children: Vec::new(),
        }
    }

    fn new_non_leaf_with_kv(key: K, value: V) -> Self {
        let mut vec = Vec::new();
        vec.push((Rc::new(key), value));

        Node::Leaf {
            key_vals: vec,
            next: None,
            prev: None,
        }
    }
}

impl<K: Ord + std::fmt::Debug, V: std::fmt::Debug> BPlusTree<K, V> {
    pub fn new() -> Self {
        BPlusTree {
            root: Rc::new(RefCell::new(Node::Leaf {
                key_vals: Vec::new(),
                prev: None,
                next: None,
            }))
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        if let Some((new_separator, new_child)) = self.insert_internal(self.root.clone(), Rc::new(key), value) {
            let new_root = Rc::new(RefCell::new(Node::new_non_leaf()));
            if let Node::NonLeaf { separators, children } = &mut *new_root.borrow_mut() {
                separators.push(new_separator);
                children.push(Rc::clone(&self.root));
                children.push(new_child);
            }
            self.root = new_root;
        }
    }

    fn insert_internal(&mut self, node: Rc<RefCell<Node<K, V>>>, key: Rc<K>, value: V) -> Option<(Rc<K>, Rc<RefCell<Node<K, V>>>)> {
        let mut node_ref = node.borrow_mut();
        match &mut *node_ref {
            Node::Leaf { key_vals, next, .. } => {
                let pos = key_vals.iter().position(|(k, _)| k.as_ref() > key.as_ref()).unwrap_or(key_vals.len());
                key_vals.insert(pos, (key, value));

                if key_vals.len() < DEGREE {
                    return None;
                }

                let mid = key_vals.len() / 2;
                let new_node = Rc::new(RefCell::new(Node::new_leaf()));
                if let Node::Leaf { key_vals: new_keys, next: new_next, prev: new_prev } = &mut *new_node.borrow_mut() {
                    *new_keys = key_vals.split_off(mid);
                    *new_next = next.take();
                    *new_prev = Some(Rc::clone(&node));
                }

                *next = Some(Rc::clone(&new_node));

                return Some((key_vals[mid].0.clone(), new_node));
            }
            Node::NonLeaf { separators, children } => {
                let pos = separators.iter().position(|k| k.as_ref() > key.as_ref()).unwrap_or(separators.len());
                let child = children[pos].clone();

                if let Some((new_separator, new_child)) = self.insert_internal(child, key, value) {
                    separators.insert(pos, new_separator);
                    children.insert(pos + 1, new_child);

                    if separators.len() < DEGREE {
                        return None;
                    }

                    let mid = separators.len() / 2;
                    let new_node = Rc::new(RefCell::new(Node::new_non_leaf()));
                    if let Node::NonLeaf { separators: new_separators, children: new_children } = &mut *new_node.borrow_mut() {
                        *new_separators = separators.split_off(mid + 1);
                        *new_children = children.split_off(mid + 1);
                    }

                    return Some((separators.pop().unwrap(), new_node));
                }

                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const DEGREE: usize = 3;


    fn create_tree() -> BPlusTree<i32, String> {
        BPlusTree::new()
    }

    #[test]
    fn test_insert_single_node() {
        let mut tree = create_tree();

        // Insert a single key-value pair
        tree.insert(10, "Ten".to_string());

        // Check if the root contains the correct key-value pair
        let root = tree.root.borrow();
        if let Node::Leaf { key_vals, .. } = &*root {
            assert_eq!(key_vals.len(), 1);
        } else {
            panic!("Root should be a leaf node");
        }
    }
}
