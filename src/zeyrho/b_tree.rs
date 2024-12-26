use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
use tonic::codegen::tokio_stream::StreamExt;

const DEGREE: usize = 3;

#[derive(Debug, Clone)]
pub enum Node<K: Ord + std::fmt::Debug, V: std::fmt::Debug> {
    Leaf {
        key_vals: Vec<(Rc<K>, V)>,
        next: Option<Rc<RefCell<Node<K, V>>>>,
        prev: Option<Rc<RefCell<Node<K, V>>>>,
    },
    Link {
        separators: Vec<Rc<K>>,
        children: Vec<Rc<RefCell<Node<K, V>>>>,
    },
}
//
// impl<K: Ord + Debug, V: Debug> Drop for Node<K, V>  {
//     fn drop(&mut self) {
//         todo!()
//     }
// }

#[derive(Debug)]
pub struct BPlusTree<K: Ord + std::fmt::Debug, V: std::fmt::Debug> {
    pub root: Option<Rc<RefCell<Node<K, V>>>>,
}

//
// impl<K: Ord + Debug, V: Debug> Drop for BPlusTree<K, V>  {
//     fn drop(&mut self) {
//         todo!()
//     }
// }

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
        vec.push((key, value));

        Node::Leaf {
            key_vals: vec,
            next: None,
            prev: None,
        }
    }

    fn split_leaf_node(&mut self, link_to_self: &Rc<RefCell<Self>>) -> Self {
        if let Node::Leaf {key_vals, next, prev} = self {
           let mid = key_vals.len() / 2;
            println!("{:?}", key_vals);
            let new_node = Rc::new(RefCell::new(Node::new_leaf()));

            let mut new_keys_padded = key_vals.split_off(mid);

            let new_node_separator = new_keys_padded.last().unwrap().0.clone();
            if let Node::Leaf { key_vals: new_keys, next: new_next, prev: new_prev } = &mut *new_node.borrow_mut() {
                *new_keys = new_keys_padded;
                *new_next = next.take();
                *new_prev = Some(Rc::clone(link_to_self));
            }

            *next = Some(Rc::clone(&new_node));

            Node::Link {
                separators: vec![key_vals.last().unwrap().0.clone(), new_node_separator],
                children: vec![Rc::clone(link_to_self), Rc::clone(&new_node)],
            }
        } else {
            panic!("should never be called");
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
                separators.push(new_separator);
                children.push(Rc::clone(self.root.as_ref().unwrap()));
                children.push(new_child);
            }
            self.root = Some(new_root);
        }
    }

    fn insert_internal(&mut self, node: Rc<RefCell<Node<K, V>>>, key: Rc<K>, value: V) -> Option<(Rc<K>, Rc<RefCell<Node<K, V>>>)> {
        let mut node_ref = node.borrow_mut();
        match &mut *node_ref {
            Node::Leaf { key_vals, next, .. } => {
                let pos = key_vals.iter().position(|(k, _)| {
                        k.as_ref() > key.as_ref()
                }).unwrap_or(key_vals.len());

                let pk = key.clone();
                key_vals.insert(pos, (key, value));

                if key_vals.len() <= DEGREE {
                    println!("no need to split on inset of {:?}, size is: {}, kvs are: {:?}", pk, key_vals.len(), key_vals);
                    return None;
                }

                let new_link_node = node_ref.split_leaf_node(&node);

                if let Node::Link {separators, .. } = &new_link_node {
                    return Some((Rc::clone(separators.last().unwrap()), Rc::new(RefCell::new(new_link_node))));
                };

                return None
            }
            Node::Link { separators, children } => {
                let pos = separators.iter().position(|k| {
                   k.as_ref() > key.as_ref()
                }).unwrap_or(separators.len() - 1);
                let child = children[pos].clone();

                if let Some((new_separator, new_child)) = self.insert_internal(child, key, value) {
                    separators.insert(pos, new_separator);
                    children.insert(pos + 1, new_child);

                    if separators.len() < DEGREE {
                        return None;
                    }

                    let mid = separators.len() / 2;
                    let new_node = Rc::new(RefCell::new(Node::new_link()));
                    if let Node::Link { separators: new_separators, children: new_children } = &mut *new_node.borrow_mut() {
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
            key_vals.iter().for_each(|(x, _)| {
               assert_eq!(x.as_ref(), &i);
               i += 1;
            })
        } else {
            assert_eq!(true, false);
        }
    }

    #[test]
    fn test_root_link_node() {
        let mut tree = create_tree();

        for i in 0..4 {
            tree.insert(i, i.to_string());
        }

        let root = tree.root.as_ref().unwrap().borrow();
        println!("{:?}", root);
        if let Node::Link { separators, .. } = &*root {
            println!("{:?}", separators);
        } else {
            assert_eq!(true, false);
        }

    }
}
