use crate::zeyrho::btree::SEPARATORS_MAX_SIZE;
use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
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

    pub(super) fn insert_separator_and_child_into_link(separators: &mut Vec<Rc<K>>, children: &mut Vec<Rc<RefCell<Node<K, V>>>>, new_separator: Rc<K>, new_child: Rc<RefCell<Node<K, V>>>) {
        let sep_pos = separators.iter().position(|k| {
            k.as_ref() > new_separator.as_ref()
        });

        match sep_pos {
            None => {
                separators.push(new_separator);
                children.push(new_child);
            }
            Some(pos) => {
                separators.insert(pos, new_separator);
                children.insert(pos + 1, new_child);
            }
        }
    }

    pub(super) fn split_borrowed_link_node(&mut self, link_to_self: &Rc<RefCell<Node<K, V>>>) -> (Rc<K>, Rc<RefCell<Self>>) {
        if let Node::Link { separators, children, ..} = self {
            let mid = separators.len() / 2;

            let new_right_link = Rc::new(RefCell::new(Node::<K, V>::new_link()));
            let new_right_children = children.split_off(mid + 1);

            let new_right_separators = separators.split_off(mid + 1);
            let parent_separators = separators.split_off(mid);

            let bubbling_separator = parent_separators.first().unwrap().clone();

            if let Node::Link { separators: right_separators, children: right_children} = &mut *new_right_link.borrow_mut() {
                *right_separators = new_right_separators;
                *right_children = new_right_children;
            };

            (bubbling_separator, new_right_link)
        } else {
            panic!("trying to split link node on child node")
        }


    }

    // splitting a link node with separators 1, 2, 3, should result in a new link node with a single separator of 2 and child link nodes of 1, 3
    pub(super) fn split_link_node(self_rc: &Rc<RefCell<Self>>) -> (Rc<RefCell<Self>>, Rc<K> , Rc<RefCell<Self>>) {
        let (sep, new_right) = (*self_rc.borrow_mut()).split_borrowed_link_node(self_rc);

        if let Node::Link {children, ..} = new_right.borrow().deref() {
            return (children[0].clone(), sep, new_right.clone())
        }
        panic!("should not fail")
    }

    // returns the new separator and the new right node. Self will become the left node
    pub(super) fn split_borrowed_leaf_node(self: &mut Self) -> (Rc<K>, Rc<RefCell<Self>>) {
        if let Node::Leaf {key_vals, ..} = self {
            let mid = key_vals.len() / 2;

            let split_point = key_vals[mid].0.clone();
            let new_keys_padded = key_vals.split_off(mid);

            let new_right_node = Rc::new(RefCell::new(Node::Leaf {
                key_vals: new_keys_padded
            }));

            (split_point, new_right_node)
        } else {
            panic!("trying to split leaf node on link")
        }

    }

    pub(super) fn split_leaf_node(link_to_self: &Rc<RefCell<Self>>) -> (Rc<RefCell<Self>>, Rc<K>, Rc<RefCell<Self>>){
        let (split, right) = (*link_to_self.borrow_mut()).split_borrowed_leaf_node();
        (link_to_self.clone(), split, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Deref;

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

        let link_node = Rc::new(RefCell::new(Node::Link {
            separators: vec!(Rc::new(2), Rc::new(3), Rc::new(4)),
            children: vec!(first, second, third, fourth)
        }));

        let mut link_ref = link_node.borrow_mut();
        if let Node::Link { separators, children, ..} = &mut *link_ref {

            let (new_sep, new_right) = (&mut *link_ref).split_borrowed_link_node(&link_node);

            if let Node::Link {separators, children, ..} = link_ref.deref() {
                let collected_seps : Vec<&i32> = separators.iter().map(|k: &Rc<i32>| k.as_ref()).collect();
                assert_eq!(vec![&2], collected_seps);

                let expected_children_keys = vec!(vec![&1], vec![&2]);
                for i in 0..children.len() {
                    if let Node::Leaf {key_vals, ..} = children[i].borrow().deref() {
                        let collected_keys: Vec<&i32> = key_vals.iter().map(|(k, v): &(Rc<i32>, String)| k.as_ref()).collect();
                        assert_eq!(expected_children_keys[i], collected_keys);
                    }
                }
            };
            if let Node::Link {separators, children, ..} = new_right.borrow().deref() {
                let collected_seps : Vec<&i32> = separators.iter().map(|k: &Rc<i32>| k.as_ref()).collect();
                assert_eq!(vec![&4], collected_seps);
                let expected_children_keys = vec!(vec![&3], vec![&4, &5]);
                for i in 0..children.len() {
                    if let Node::Leaf {key_vals, ..} = children[i].borrow().deref() {
                        let collected_keys: Vec<&i32> = key_vals.iter().map(|(k, v): &(Rc<i32>, String)| k.as_ref()).collect();
                        assert_eq!(expected_children_keys[i], collected_keys);
                    }
                }
            };

            assert_eq!(new_sep.as_ref(), &3);
        }

    }
}