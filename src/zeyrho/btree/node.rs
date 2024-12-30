use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Node<K: Ord + std::fmt::Debug, V: std::fmt::Debug> {
    Leaf {
        key_vals: Vec<(Rc<K>, V)>,
        // next: Option<Rc<RefCell<Node<K, V>>>>,
        // prev: Option<Rc<RefCell<Node<K, V>>>>,
    },
    Link {
        // TODO: Should these be Vec<Option<>>? It makes it a lot easier to know if we need to insert something new.
        separators: Vec<Rc<K>>, // a link has DEGREE - 1 separators
        children: Vec<Rc<RefCell<Node<K, V>>>>, // and DEGREE children
    },
}

impl<K: Debug + Ord, V: Debug> Node<K, V> {
    fn fmt_depth(&self, f: &mut Formatter<'_>, depth: usize) -> std::fmt::Result {
        match self {
            Node::Leaf { key_vals, .. } => {
                f.write_str(&" ".repeat(depth))?;
                f.write_str(&format!("key vals: {:?}\n", key_vals))
            }
            Node::Link { separators, children, .. } => {
                f.write_str(&" ".repeat(depth))?;
                f.write_str(&format!("separators: {:?}\n", separators))?;
                let _ = children.iter().for_each(|child| {
                    let _ = f.write_str(&" ".repeat(depth));
                    let _ = (*child).borrow().fmt_depth(f, depth + 1);
                });
                Ok(())
            }
        }
    }
}
impl<K: Debug + Ord, V: Debug> Display for Node<K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_depth(f, 1)
    }
}

impl<K: Ord + std::fmt::Debug, V: std::fmt::Debug> Node<K, V> {
    pub(super) fn new_leaf() -> Self {
        Node::Leaf {
            key_vals: Vec::new(),
            // next: None,
            // prev: None,
        }
    }

    pub(super) fn new_link() -> Self {
        Node::Link {
            separators: Vec::new(),
            children: Vec::new(),
        }
    }

    pub(super) fn new_leaf_with_kv(key: Rc<K>, value: V) -> Self {
        let mut vec = Vec::new();
        vec.push((key, value));

        Node::Leaf {
            key_vals: vec,
            // next: None,
            // prev: None,
        }
    }

    pub(super) fn split_link_node(&mut self) -> Self {
        if let Node::Link { separators, children, ..} = self {
            let mid = separators.len() / 2;
            let new_node = Rc::new(RefCell::new(Node::new_link()));
            if let Node::Link { separators: new_separators, children: new_children } = &mut *new_node.borrow_mut() {
                *new_separators = separators.split_off(mid);
                *new_children = children.split_off(mid + 1);
            }

            todo!()
        } else {
            panic!("trying to split link node on child node")
        }

    }

    pub(super) fn split_leaf_node(&mut self, link_to_self: &Rc<RefCell<Self>>) -> Self {
        if let Node::Leaf {key_vals/*, next, prev*/} = self {
            let mid = key_vals.len() / 2;
            println!("splitting leaf node: {:?}", key_vals);
            let new_node = Rc::new(RefCell::new(Node::new_leaf()));

            let mut new_keys_padded = key_vals.split_off(mid);

            let new_node_separator = new_keys_padded.last().unwrap().0.clone();
            if let Node::Leaf { key_vals: new_keys, /*next: new_next, prev: new_prev*/ } = &mut *new_node.borrow_mut() {
                *new_keys = new_keys_padded;
                // *new_next = next.take();
                // *new_prev = Some(Rc::clone(link_to_self));
            }

            //*next = Some(Rc::clone(&new_node));

            println!("new node after split: {:?}", new_node);
            println!("self after split: {:?}", key_vals);
            let new_link = Node::Link {
                separators: vec![key_vals.last().unwrap().0.clone(), new_node_separator],
                children: vec![Rc::clone(link_to_self), Rc::clone(&new_node)],
            };


            println!("new link: {:?}", new_link);
            new_link
        } else {
            panic!("trying to split leaf node on link node");
        }
    }
}
//
// impl<K: Ord + Debug, V: Debug> Drop for Node<K, V>  {
//     fn drop(&mut self) {
//         todo!()
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    fn create_leaf_with_kvs(vec: Vec<(i32, &str)>) -> Rc<RefCell<Node<i32, &str>>> {
        Rc::new(RefCell::new((Node::Leaf {
            key_vals: vec.iter().map(|(k, v)| (Rc::new(*k), *v)).collect()
        })))
    }

    #[test]
    fn test_split_leaf() {
        let initial_node = create_leaf_with_kvs(vec!((1, "1"), (2, "2"), (3, "3"), (4, "4")));

        let mut node_ref = initial_node.borrow_mut();
        let new_node = node_ref.split_leaf_node(&initial_node);

        if let Node::Link {separators, ..} = new_node {
            let collected_seps: Vec<&i32> = separators.iter().map(|i| i.as_ref()).collect();
            assert_eq!(collected_seps, vec![&2,&4])
        }
    }
}