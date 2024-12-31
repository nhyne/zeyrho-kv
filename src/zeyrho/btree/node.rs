use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use crate::zeyrho::btree::{CHILDREN_MAX_SIZE, SEPARATORS_MAX_SIZE};

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

    // splitting a link node with separators 1, 2, 3, should result in a new link node with a single separator of 2 and child link nodes of 1, 3
    pub(super) fn split_link_node(self_rc: Rc<RefCell<Self>>) -> (Rc<RefCell<Self>>, Rc<RefCell<Self>> , Rc<RefCell<Self>>) {
        if let Node::Link { separators, children, ..} = &mut *self_rc.borrow_mut() {
            let mid = separators.len() / 2;
            println!("separators are: {:?}, mid is: {}", separators, mid);

            let parent_link_node = Rc::new(RefCell::new(Node::<K, V>::new_link()));
            let new_left_link = Rc::new(RefCell::new(Node::<K, V>::new_link()));
            let new_left_children = children.split_off(mid);

            let new_self_separators = separators.split_off(mid + 1);
            let parent_separators = separators.split_off(mid);

            if SEPARATORS_MAX_SIZE != 2 {
                panic!("only configured for separators max of 2")
            }

            if let Node::Link { separators: left_separators, children: left_children} = &mut *new_left_link.borrow_mut() {
                *left_separators = separators.clone();
                *left_children = new_left_children;
            };
            if let Node::Link { separators: new_separators, children: new_children } = &mut *parent_link_node.borrow_mut() {
                *new_separators = parent_separators;
                *new_children = vec![new_left_link.clone(), self_rc.clone()];
            };

            *separators = new_self_separators;

            (new_left_link, parent_link_node, self_rc.clone())
        } else {
            panic!("trying to split link node on child node")
        }

    }

    pub(super) fn split_leaf_node(link_to_self: &Rc<RefCell<Self>>) -> (Rc<RefCell<Self>>, Rc<K>, Rc<RefCell<Self>>){
        println!("borrowing original node as mut");
        if let Node::Leaf {key_vals/*, next, prev*/} = &mut *link_to_self.borrow_mut() {
            let mid = key_vals.len() / 2;

            let split_point = key_vals[mid].0.clone();
            let new_keys_padded = key_vals.split_off(mid);

            let new_left_node = Rc::new(RefCell::new(Node::Leaf {
                key_vals: new_keys_padded
            }));

            (new_left_node, split_point, link_to_self.clone())
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
    use std::ops::Deref;
    use super::*;

    fn create_leaf_with_kvs(items: Vec<i32>) -> Rc<RefCell<Node<i32, String>>> {
        Rc::new(RefCell::new((Node::Leaf {
            key_vals: items.iter().map(|k| (Rc::new(*k), k.to_string())).collect()
        })))
    }


    #[test]
    fn test_split_leaf() {
        let initial_node = create_leaf_with_kvs(vec!(1, 2, 3, 4));

        let (left, split, right )= Node::split_leaf_node(&initial_node);

        if let Node::Leaf {key_vals, ..} = left.borrow().deref() {
            let collected_seps: Vec<&i32> = key_vals.iter().map(|(k, _): &(Rc<i32>, String)| k.as_ref()).collect();
            assert_eq!(collected_seps, vec![&1,&2])
        };

        assert_eq!(split.as_ref(), &3);

        if let Node::Leaf {key_vals, ..} = right.borrow().deref() {
            let collected_seps: Vec<&i32> = key_vals.iter().map(|(k, v): &(Rc<i32>, String)| k.as_ref()).collect();
            assert_eq!(collected_seps, vec![&3,&4])
        };
    }

    #[test]
    fn test_split_link() {
        let first = create_leaf_with_kvs(vec!(1));
        let second = create_leaf_with_kvs(vec!(2));
        let third = create_leaf_with_kvs(vec!(3));
        let fourth = create_leaf_with_kvs(vec!(4, 5));

        let link_node = Node::Link {
            separators: vec!(Rc::new(2), Rc::new(3), Rc::new(4)),
            children: vec!(first, second, third, fourth)
        };

        let (left, parent, right) = Node::split_link_node(Rc::new(RefCell::new(link_node)));

        if let Node::Link {separators, ..} = parent.borrow().deref() {
            let collected_seps : Vec<&i32> = separators.iter().map(|k: &Rc<i32>| k.as_ref()).collect();
            assert_eq!(vec![&3], collected_seps);
        };

        if let Node::Link {separators, ..} = left.borrow().deref() {
            let collected_seps : Vec<&i32> = separators.iter().map(|k: &Rc<i32>| k.as_ref()).collect();
            assert_eq!(vec![&2], collected_seps);
        };
        if let Node::Link {separators, ..} = right.borrow().deref() {
            let collected_seps : Vec<&i32> = separators.iter().map(|k: &Rc<i32>| k.as_ref()).collect();
            assert_eq!(vec![&4], collected_seps);
        };

    }
}