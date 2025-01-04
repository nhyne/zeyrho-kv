use crate::zeyrho::btree::tree::BPlusTree;
use crate::zeyrho::btree::{DEGREE, MAX_KVS_IN_LEAF, SEPARATORS_MAX_SIZE};
use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::rc::{Rc, Weak};

#[derive(Debug, Clone)]
pub enum Node<K: Ord + Debug, V: Debug> {
    Leaf {
        key_vals: Vec<(Rc<K>, V)>,
        next: Option<Weak<RefCell<Self>>>,
        prev: Option<Weak<RefCell<Self>>>,
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
            Node::Link {
                separators,
                children,
                ..
            } => {
                f.write_str(&" ".repeat(depth))?;
                f.write_str(&format!("separators: {:?}\n", separators))?;
                children.iter().for_each(|child| {
                    let _ = f.write_str(&" ".repeat(depth));
                    let _ = (*child).borrow().fmt_depth(f, depth + 1);
                });
                Ok(())
            }
        }
    }
}

impl<K: Debug + Ord, V: Debug> Drop for Node<K, V> {
    fn drop(&mut self) {
        // need to set prev's <next> to our next and next's <prev> to our prev
        match self {
            Node::Leaf { prev, next, .. } => match (prev, next) {
                (Some(p), Some(n)) => match (p.upgrade(), n.upgrade()) {
                    (Some(p_upgrade), Some(n_upgrade)) => {
                        let mut p_ref = p_upgrade.borrow_mut();
                        let mut n_ref = n_upgrade.borrow_mut();

                        match (&mut *p_ref, &mut *n_ref) {
                            (Node::Leaf { next: p_next, .. }, Node::Leaf { prev: n_prev, .. }) => {
                                *p_next = Some(n.clone());
                                *n_prev = Some(p.clone());
                            }
                            (_, _) => {
                                // this isn't a great place to panic, but I'm not sure what other option we have here.
                                // we could have a background process checking for orphaned leaves or even fix them when we scan -- probably the latter
                                panic!("could not borrow both next and prev leafs")
                            }
                        }
                    }
                    _ => {
                        // also not great to do this :(
                        // but this is all learning and toy code
                        println!("not able to upgrade weak link for: {:?}", self);
                        panic!("failed to upgrade")
                    }
                },
                (Some(p), None) => {
                    // this is the case where we are the far right leaf
                    if let Some(p_upgrade) = p.upgrade() {
                        let mut p_ref = p_upgrade.borrow_mut();

                        if let Node::Leaf { next: p_next, .. } = &mut *p_ref {
                            *p_next = None;
                        }
                    }
                }
                (None, Some(n)) => {
                    // this is the case where we are the far left leaf
                    if let Some(n_upgrade) = n.upgrade() {
                        let mut n_ref = n_upgrade.borrow_mut();

                        if let Node::Leaf { prev: n_prev, .. } = &mut *n_ref {
                            *n_prev = None;
                        }
                    }
                }
                (None, None) => {
                    // this is the case where we're the only node, do nothing
                }
            },
            Node::Link { .. } => { /* we don't do anything special for link nodes right now*/ }
        }
    }
}

impl<K: Debug + Ord, V: Debug> Display for Node<K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_depth(f, 1)
    }
}

impl<K: Ord + Debug, V: Debug> Node<K, V> {
    pub(super) fn new_link() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Node::Link {
            separators: Vec::new(),
            children: Vec::new(),
        }))
    }

    pub(super) fn new_link_with_seps_and_children(separators: Vec<Rc<K>>, children: Vec<Rc<RefCell<Node<K, V>>>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Node::Link {
            separators,
            children,
        }))
    }

    pub(super) fn new_leaf() -> Self {
        Node::Leaf {
            key_vals: Vec::new(),
            next: None,
            prev: None,
        }
    }

    pub(super) fn new_leaf_with_kv(key: Rc<K>, value: V) -> Rc<RefCell<Self>> {
        let mut vec = Vec::new();
        vec.push((key, value));

        Rc::new(RefCell::new(Node::Leaf {
            key_vals: vec,
            next: None,
            prev: None,
        }))
    }

    pub(super) fn insert_separator_and_child_into_link(
        separators: &mut Vec<Rc<K>>,
        children: &mut Vec<Rc<RefCell<Node<K, V>>>>,
        new_separator: Rc<K>,
        new_child: Rc<RefCell<Node<K, V>>>,
    ) {
        let sep_pos = separators
            .iter()
            .position(|k| k.as_ref() > new_separator.as_ref());

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

    pub(super) fn split_borrowed_link_node(&mut self) -> (Rc<K>, Rc<RefCell<Self>>) {
        if let Node::Link {
            separators,
            children,
            ..
        } = self
        {
            let mid = separators.len() / 2;

            let new_right_link = Node::<K, V>::new_link();
            let new_right_children = children.split_off(mid + 1);

            let new_right_separators = separators.split_off(mid + 1);
            let parent_separators = separators.split_off(mid);

            let bubbling_separator = parent_separators.first().unwrap().clone();

            if let Node::Link {
                separators: right_separators,
                children: right_children,
            } = &mut *new_right_link.borrow_mut()
            {
                *right_separators = new_right_separators;
                *right_children = new_right_children;
            };

            (bubbling_separator, new_right_link)
        } else {
            panic!("trying to split link node on child node")
        }
    }

    // splitting a link node with separators 1, 2, 3, should result in a new link node with a single separator of 2 and child link nodes of 1, 3
    pub(super) fn split_link_node(
        self_rc: &Rc<RefCell<Self>>,
    ) -> (Rc<RefCell<Self>>, Rc<K>, Rc<RefCell<Self>>) {
        let (sep, new_right) = (*self_rc.borrow_mut()).split_borrowed_link_node();

        if let Node::Link { children, .. } = new_right.borrow().deref() {
            return (children[0].clone(), sep, new_right.clone());
        }
        panic!("should not fail")
    }

    // returns the new separator and the new right node. Self will become the left node
    pub(super) fn split_borrowed_leaf_node(
        &mut self,
        rc_self: &Rc<RefCell<Self>>,
    ) -> (Rc<K>, Rc<RefCell<Self>>) {
        if let Node::Leaf {
            key_vals,
            next,
            prev,
            ..
        } = self
        {
            let mid = key_vals.len() / 2;

            let split_point = key_vals[mid].0.clone();
            let new_keys_padded = key_vals.split_off(mid);

            let new_right_node = Rc::new(RefCell::new(Node::Leaf {
                key_vals: new_keys_padded,
                next: next.take().map(|maybe_weak| maybe_weak.clone()),
                prev: Some(Rc::downgrade(rc_self)),
            }));

            *next = Some(Rc::downgrade(&new_right_node));

            (split_point, new_right_node)
        } else {
            panic!("trying to split leaf node on link")
        }
    }

    pub(super) fn split_leaf_node(
        link_to_self: &Rc<RefCell<Self>>,
    ) -> (Rc<RefCell<Self>>, Rc<K>, Rc<RefCell<Self>>) {
        let (split, right) = (*link_to_self.borrow_mut()).split_borrowed_leaf_node(link_to_self);
        (link_to_self.clone(), split, right)
    }

    // the left Option is the new separator and the right is the new right node. We don't need to do anything with the left node b/c the parent is already pointing to it
    pub(super) fn insert_internal(
        node: &Rc<RefCell<Node<K, V>>>,
        inserted_key: Rc<K>,
        inserted_value: V,
    ) -> Option<(Rc<K>, Rc<RefCell<Node<K, V>>>)> {
        let mut node_ref = node.borrow_mut();
        match &mut *node_ref {
            Node::Leaf { key_vals, .. } => {
                let pos = key_vals
                    .iter()
                    .position(|(k, _)| k.as_ref() > inserted_key.as_ref())
                    .unwrap_or(key_vals.len());

                key_vals.insert(pos, (inserted_key, inserted_value));

                if key_vals.len() <= MAX_KVS_IN_LEAF {
                    return None;
                }

                let (split, new_right) = (*node_ref).split_borrowed_leaf_node(node);

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
                            children.push(new_leaf);
                            return None;
                        }
                    }
                    child_to_update = Some(children.len() - 1);
                }

                let child = children[child_to_update.unwrap()].clone();

                if let Some((new_separator, new_node)) =
                    Node::insert_internal(&child, inserted_key, inserted_value)
                // self.insert_internal(&child, inserted_key, inserted_value)
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


    /* possible cases for this:
        1. we delete a whole leaf node and need to delete the child from the parent link and do some propagation up...
        2. we delete the current K value, which is an active separator in the parent, but retain the node
        3. we delete the K, and it's not used anywhere
        4. nothing was deleted

        given these three options, what should we return? What does the parent Link node need to know about?
        1. That the K and the Node was deleted
        2. Just that the K was deleted
        3. Nothing
        4. Nothing? -- the user expects a different response though... so this is a different return then #3

        Option<(Option<Self>, K)>
        Result<(), (Option<Self>, K)>

        Result<(), Option<(Option<Self>, K)>>

        This is pretty gross, should I just wrap this in a deletion type?
     */
    pub(super) fn delete_internal(node: &Rc<RefCell<Node<K, V>>>, deleted_key: K) -> Result<(K, V, Option<()>), ()> {

        // if link then see if any of the values make sense to continue searching -- I think this is always the case?

        // if leaf then iterate through the K/Vs and delete if we get a match

        let mut node_ref = node.borrow_mut();
        match &mut *node_ref {
            Node::Link { .. } => {

                // if the K value is a separator then we're going to need to fix that....
                // should this only happen if the child node has the value? -- Yes
                todo!()
            }
            Node::Leaf { key_vals, .. } => {

                let original_size = key_vals.len();
                // let new_key_vals: Vec<(Rc<K>, V)> = key_vals.iter().filter(|(k, _) : &(Rc<K>, V)| *k.as_ref() != deleted_key).collect();
                key_vals.retain(|(k, _)| *k.as_ref() != deleted_key);
                let new_size = key_vals.len();

                if original_size != new_size {
                    println!("we removed an element");
                }

                if key_vals.is_empty() {
                    // we need to delete this node
                }
            }
        }

        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Deref;

    fn create_leaf_with_kvs(
        items: Vec<i32>,
    ) -> Rc<RefCell<Node<i32, String>>> {
        Rc::new(RefCell::new(Node::Leaf {
            key_vals: items.iter().map(|k| (Rc::new(*k), k.to_string())).collect(),
            next: None,
            prev: None,
        }))
    }

    #[test]
    fn test_split_leaf() {
        let initial_node = create_leaf_with_kvs(vec![1, 2, 3, 4]);

        let (left, split, right) = Node::split_leaf_node(&initial_node);

        if let Node::Leaf { key_vals, .. } = left.borrow().deref() {
            let collected_seps: Vec<&i32> = key_vals
                .iter()
                .map(|(k, _): &(Rc<i32>, String)| k.as_ref())
                .collect();
            assert_eq!(collected_seps, vec![&1, &2])
        };

        assert_eq!(split.as_ref(), &3);

        if let Node::Leaf { key_vals, .. } = right.borrow().deref() {
            let collected_seps: Vec<&i32> = key_vals
                .iter()
                .map(|(k, _): &(Rc<i32>, String)| k.as_ref())
                .collect();
            assert_eq!(collected_seps, vec![&3, &4])
        };
    }

    fn assign_prev_next_in_order(leaves: Vec<Rc<RefCell<Node<i32, String>>>>) {
        if leaves.len() < 2 {
            return;
        }
        for rc_node_index in 1..leaves.len() {
            let mut first = &leaves[rc_node_index - 1];
            let mut second = &leaves[rc_node_index];
            let mut first_ref = first.borrow_mut();
            let mut second_ref = second.borrow_mut();

            if let Node::Leaf { next, .. } = &mut *first_ref {
                *next = Some(Rc::downgrade(&second));
            }
            if let Node::Leaf { prev, .. } = &mut *second_ref {
                *prev = Some(Rc::downgrade(&first));
            }
        }
    }

    #[test]
    fn test_split_link() {
        let first = create_leaf_with_kvs(vec![1]);
        let second = create_leaf_with_kvs(vec![2]);
        let third = create_leaf_with_kvs(vec![3]);
        let fourth = create_leaf_with_kvs(vec![4, 5]);

        assign_prev_next_in_order(vec![
            first.clone(),
            second.clone(),
            third.clone(),
            fourth.clone(),
        ]);

        let link_node = Rc::new(RefCell::new(Node::Link {
            separators: vec![Rc::new(2), Rc::new(3), Rc::new(4)],
            children: vec![first.clone(), second.clone(), third.clone(), fourth.clone()],
        }));

        let mut link_ref = link_node.borrow_mut();
        if let Node::Link { .. } = &mut *link_ref {
            let (new_sep, new_right) = (*link_ref).split_borrowed_link_node();

            if let Node::Link {
                separators,
                children,
                ..
            } = link_ref.deref()
            {
                let collected_seps: Vec<&i32> =
                    separators.iter().map(|k: &Rc<i32>| k.as_ref()).collect();
                assert_eq!(vec![&2], collected_seps);

                let expected_children_keys = [vec![&1], vec![&2]];
                let expected_next = [Some(second.clone()), Some(third.clone())];
                let expected_prev = [None, Some(first.clone())];
                for i in 0..children.len() {
                    if let Node::Leaf {
                        key_vals,
                        next,
                        prev,
                        ..
                    } = children[i].borrow().deref()
                    {
                        let collected_keys: Vec<&i32> = key_vals
                            .iter()
                            .map(|(k, _): &(Rc<i32>, String)| k.as_ref())
                            .collect();
                        assert_eq!(expected_children_keys[i], collected_keys);
                        match (&expected_next[i], next) {
                            (Some(expected), Some(actual)) => {
                                assert!(Rc::ptr_eq(expected, &actual.upgrade().unwrap()));
                            }
                            (None, None) => {}
                            (_, _) => {
                                println!("got mismatching Some/None for expected next");
                                assert!(false)
                            }
                        }
                        match (&expected_prev[i], prev) {
                            (Some(expected), Some(actual)) => {
                                assert!(Rc::ptr_eq(expected, &actual.upgrade().unwrap()));
                            }
                            (None, None) => {}
                            (_, _) => {
                                println!("got mismatching Some/None for expected prev");
                                assert!(false)
                            }
                        }
                    }
                }
            };
            if let Node::Link {
                separators,
                children,
                ..
            } = new_right.borrow().deref()
            {
                let collected_seps: Vec<&i32> =
                    separators.iter().map(|k: &Rc<i32>| k.as_ref()).collect();
                assert_eq!(vec![&4], collected_seps);
                let expected_children_keys = [vec![&3], vec![&4, &5]];
                let expected_next = [Some(fourth.clone()), None];
                let expected_prev = [Some(second.clone()), Some(third.clone())];
                for i in 0..children.len() {
                    if let Node::Leaf {
                        key_vals,
                        next,
                        prev,
                        ..
                    } = children[i].borrow().deref()
                    {
                        let collected_keys: Vec<&i32> = key_vals
                            .iter()
                            .map(|(k, _): &(Rc<i32>, String)| k.as_ref())
                            .collect();
                        assert_eq!(expected_children_keys[i], collected_keys);
                        match (&expected_next[i], next) {
                            (Some(expected), Some(actual)) => {
                                assert!(Rc::ptr_eq(expected, &actual.upgrade().unwrap()));
                            }
                            (None, None) => {}
                            (_, _) => {
                                println!("got mismatching Some/None for expected next");
                                assert!(false)
                            }
                        }
                        match (&expected_prev[i], prev) {
                            (Some(expected), Some(actual)) => {
                                assert!(Rc::ptr_eq(expected, &actual.upgrade().unwrap()));
                            }
                            (None, None) => {}
                            (_, _) => {
                                println!("got mismatching Some/None for expected prev");
                                assert!(false)
                            }
                        }
                    }
                }
            };

            assert_eq!(new_sep.as_ref(), &3);
        }
    }


    #[test]
    fn test_delete_internal_from_leaf() {
        let leaf = create_leaf_with_kvs(vec!(1, 2, 3));

        let deletion = Node::delete_internal(&leaf, 2);

        assert!(deletion.is_some());
    }
}
