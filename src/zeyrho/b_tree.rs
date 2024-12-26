use std::cell::RefCell;
use std::rc::Rc;

const DEGREE: usize = 3;

#[derive(Debug, Clone)]
enum Node<K: Ord + std::fmt::Debug, V: std::fmt::Debug> {
    Leaf {
        key_vals: Vec<Option<(Rc<K>, V)>>,
        next: Option<Rc<RefCell<Node<K, V>>>>,
        prev: Option<Rc<RefCell<Node<K, V>>>>,
    },
    Link {
        separators: Vec<Option<Rc<K>>>,
        children: Vec<Option<Rc<RefCell<Node<K, V>>>>>,
    },
}

#[derive(Debug)]
struct BPlusTree<K: Ord + std::fmt::Debug, V: std::fmt::Debug> {
    root: Option<Rc<RefCell<Node<K, V>>>>,
}

impl<K: Ord + std::fmt::Debug, V: std::fmt::Debug> Node<K, V> {
    fn new_leaf() -> Self {
        Node::Leaf {
            key_vals: Vec::new(),
            next: None,
            prev: None,
        }
    }

    fn new_link() -> Self {
        Node::Link {
            separators: Vec::new(),
            children: Vec::new(),
        }
    }

    fn new_leaf_with_kv(key: Rc<K>, value: V) -> Self {
        let mut vec = Vec::new();
        vec.push(Some((key, value)));

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
            root: None,
        }
    }

    pub fn insert(&mut self, key: K, value: V) {

        if self.root.is_none() {
            self.root = Some(Rc::new(RefCell::new(Node::new_leaf_with_kv(Rc::new(key), value))));
            return
        }

        if let Some((new_separator, new_child)) = self.insert_internal(self.root.as_ref().unwrap().clone(), Rc::new(key), value) {
            let new_root = Rc::new(RefCell::new(Node::new_link()));
            if let Node::Link { separators, children } = &mut *new_root.borrow_mut() {
                separators.push(Some(new_separator));
                children.push(Some(Rc::clone(self.root.as_ref().unwrap())));
                children.push(Some(new_child));
            }
            self.root = Some(new_root);
        }
    }

    fn insert_internal(&mut self, node: Rc<RefCell<Node<K, V>>>, key: Rc<K>, value: V) -> Option<(Rc<K>, Rc<RefCell<Node<K, V>>>)> {
        let mut node_ref = node.borrow_mut();
        match &mut *node_ref {
            Node::Leaf { key_vals, next, .. } => {

                // if the leaf is already full then we need to make a new one and split
                let pos = key_vals.iter().position(|maybe_filled_key| {
                    maybe_filled_key.as_ref().map(|(k, _)| {
                        k.as_ref() > key.as_ref()
                    }).unwrap_or(false)
                }).unwrap_or(key_vals.len());

                key_vals.insert(pos, Some((key, value)));

                if key_vals.len() <= DEGREE {
                    return None;
                }

                let mid = key_vals.len() / 2; // this is ALWAYS going to be 1
                println!("{:?}", key_vals);
                let new_node = Rc::new(RefCell::new(Node::new_leaf()));

                let mut new_keys_padded = key_vals.split_off(mid);
                new_keys_padded.push(None);

                if let Node::Leaf { key_vals: new_keys, next: new_next, prev: new_prev } = &mut *new_node.borrow_mut() {
                    *new_keys = new_keys_padded;
                    *new_next = next.take();
                    *new_prev = Some(Rc::clone(&node));
                }

                // push twice b/c we need to fill the vec
                key_vals.push(None); key_vals.push(None);

                *next = Some(Rc::clone(&new_node));

                Some((key_vals[mid].take().unwrap().0.clone(), new_node))
            }
            Node::Link { separators, children } => {
                let pos = separators.iter().position(|maybe_k| {
                   maybe_k.as_ref().map(|k| k.as_ref() > key.as_ref()).unwrap_or(false)
                }).unwrap_or(separators.len());

                if let Some(child) = children[pos].take() {
                    if let Some((new_separator, new_child)) = self.insert_internal(child, key, value) {
                        separators.insert(pos, Some(new_separator));
                        children.insert(pos + 1, Some(new_child));

                        if separators.len() < DEGREE {
                            return None;
                        }

                        let mid = separators.len() / 2;
                        let new_node = Rc::new(RefCell::new(Node::new_link()));
                        if let Node::Link { separators: new_separators, children: new_children } = &mut *new_node.borrow_mut() {
                            *new_separators = separators.split_off(mid + 1);
                            *new_children = children.split_off(mid + 1);
                        }

                        return Some((separators.pop().unwrap().unwrap(), new_node));
                    }
                }

                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    fn create_tree() -> BPlusTree<i32, String> {
        BPlusTree::new()
    }

    #[test]
    fn test_single_leaf_node() {
        let mut tree = create_tree();

        for i in 0..3 {
            tree.insert(i, i.to_string());
        }

        let root = tree.root.as_ref().unwrap().borrow();
        println!("{:?}", root);

        if let Node::Leaf { key_vals, .. } = &*root {
            println!("{:?}", key_vals);
            assert_eq!(key_vals.len(), 3);
            let mut i = 0;
            key_vals.iter().for_each(|x| {
                match x {
                    None => {},
                    Some((y, _)) => {
                        assert_eq!(y.as_ref(), &i);
                        i += 1;
                    }
                }
            })
        }
    }
}
