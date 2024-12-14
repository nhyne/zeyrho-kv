use std::cell::RefCell;
use std::rc::Rc;

const DEGREE: usize = 3;

#[derive(Debug, Clone)]
enum Node<K: Ord + std::fmt::Debug, V: std::fmt::Debug> {
    Leaf {
        keys: Vec<Rc<K>>,
        values: Vec<V>,
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
            keys: Vec::new(),
            values: Vec::new(),
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
}

impl<K: Ord + std::fmt::Debug, V: std::fmt::Debug> BPlusTree<K, V> {
    pub fn new() -> Self {
        BPlusTree {
            root: Rc::new(RefCell::new(Node::NonLeaf {
                separators: Vec::new(),
                children: Vec::new(),
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
            Node::Leaf { keys, values, next, .. } => {
                let pos = keys.iter().position(|k| k.as_ref() > key.as_ref()).unwrap_or(keys.len());
                keys.insert(pos, key);
                values.insert(pos, value);

                if keys.len() < DEGREE {
                    return None;
                }

                let mid = keys.len() / 2;
                let new_node = Rc::new(RefCell::new(Node::new_leaf()));
                if let Node::Leaf { keys: new_keys, values: new_values, next: new_next, prev: new_prev } = &mut *new_node.borrow_mut() {
                    *new_keys = keys.split_off(mid);
                    *new_values = values.split_off(mid);
                    *new_next = next.take();
                    *new_prev = Some(Rc::clone(&node));
                }

                *next = Some(Rc::clone(&new_node));

                return Some((keys[mid].clone(), new_node));
            }
            Node::NonLeaf { separators, children } => {
                let pos = separators.iter().position(|k| k.as_ref() > key.as_ref()).unwrap_or(separators.len());
                let child = children[pos].clone();

                if let Some( (new_separator, new_child)) = self.insert_internal(child, key, value) {
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
