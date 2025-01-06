use crate::zeyrho::btree::{DEGREE, MAX_KVS_IN_LEAF, SEPARATORS_MAX_SIZE};
use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::rc::{Rc, Weak};

#[derive(Debug)]
pub enum Node<K: Ord + Debug, V: Debug> {
    /*
    TODO: It would be great to use prefix compression on leaf nodes' Keys. The problem is I'd need to make sure K is iterable and summable?
    https://lobste.rs/s/za4cxl/b_trees_more_than_i_thought_i_d_want_know#c_d23bla
    Essentially this would mean that for storing values 100, 101, 102 we would only keep 0, 1, 2 in the key_vals list then prepend the prefix if needed
     */
    Leaf {
        internal_leaf: InternalLeaf<K, V>
    },
    /*
    TODO: It would also be nice not to use a value between two child nodes' Keys as a separator, instead of a value directly.
    i.e. if we're inserting 0, 10, 20, 30, 40, 50 then rather than having the separator be 30, we could make it 25, or even 3* (this is suffix compression)
     */
    Link {
        // TODO: Should these be Vec<Option<>>? It makes it a lot easier to know if we need to insert something new.
        internal_link: InternalLink<K, V>
    },
}

#[derive(Debug)]
pub(super) struct InternalLink<K: Ord + Debug, V: Debug> {
    pub(super) separators: Vec<Rc<K>>, // a link has DEGREE - 1 separators
    pub(super) children: Vec<Rc<RefCell<Node<K, V>>>>, // and DEGREE children
}

#[derive(Debug)]
pub(super) struct InternalLeaf<K: Ord + Debug, V: Debug> {
    // TODO: Should these be Vec<Option<>>? It makes it a lot easier to know if we need to insert something new.
    key_vals: Vec<(Rc<K>, V)>,
    next: Option<Weak<RefCell<Node<K, V>>>>,
    prev: Option<Weak<RefCell<Node<K, V>>>>,
}

impl<K: Debug + Ord, V: Debug> InternalLink<K, V> {
    pub(super) fn split_borrowed_link_node(&mut self) -> (Rc<K>, Rc<RefCell<Node<K, V>>>) {
        let mid = self.separators.len() / 2;

        let new_right_link = Node::<K, V>::new_link();
        let new_right_children = self.children.split_off(mid + 1);

        let new_right_separators = self.separators.split_off(mid + 1);
        let parent_separators = self.separators.split_off(mid);

        let bubbling_separator = parent_separators.first().unwrap().clone();

        if let Node::Link {
            internal_link: InternalLink {
                separators: right_separators,
                children: right_children,
            }
        } = &mut *new_right_link.borrow_mut()
        {
            *right_separators = new_right_separators;
            *right_children = new_right_children;
        };

        (bubbling_separator, new_right_link)
    }

    fn collect_link_separators(&self) -> Vec<&K> {
            let collected_separators: Vec<&K> = self.separators
                .iter()
                .map(|k: &Rc<K>| k.as_ref())
                .collect();

            collected_separators
    }
}

impl<K: Debug + Ord, V: Debug> InternalLeaf<K, V> {

    pub(super) fn len(&self) -> usize {
        self.key_vals.len()
    }

    pub(super) fn iter(&self) -> impl Iterator<Item=&(Rc<K>, V)>{
        self.key_vals.iter()
    }

    pub(super) fn split_borrowed_leaf_node(
        &mut self,
        rc_self: &Rc<RefCell<Node<K, V>>>,
    ) -> (Rc<K>, Rc<RefCell<Node<K, V>>>) {
            let mid = self.key_vals.len() / 2;

            let split_point = self.key_vals[mid].0.clone();
            let new_keys_padded = self.key_vals.split_off(mid);

            let new_right_node = Rc::new(RefCell::new(Node::Leaf {
                internal_leaf: InternalLeaf {

                key_vals: new_keys_padded,
                next: self.next.take().map(|maybe_weak| maybe_weak.clone()),
                prev: Some(Rc::downgrade(rc_self)),
                }
            }));

            self.next = Some(Rc::downgrade(&new_right_node));

            (split_point, new_right_node)
    }

    fn collect_leaf_kvs(&self) -> Vec<&K> {
            let collected_keys: Vec<&K> = self.key_vals
.iter()
                .map(|(k, _): &(Rc<K>, V)| k.as_ref())
                .collect();

            collected_keys
    }
}


impl<K: Debug + Ord, V: Debug> Node<K, V> {
    fn fmt_depth(&self, f: &mut Formatter<'_>, depth: usize) -> std::fmt::Result {
        match self {
            Node::Leaf { internal_leaf, .. } => {
                f.write_str(&" ".repeat(depth))?;
                f.write_str(&format!("key vals: {:?}\n", internal_leaf.key_vals))
            }
            Node::Link {
                internal_link,
                ..
            } => {
                f.write_str(&" ".repeat(depth))?;
                f.write_str(&format!("separators: {:?}\n", internal_link.separators))?;
                internal_link.children.iter().for_each(|child| {
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
            Node::Leaf { internal_leaf, .. } => match (&internal_leaf.prev, &internal_leaf.next) {
                (Some(p), Some(n)) => match (p.upgrade(), n.upgrade()) {
                    (Some(p_upgrade), Some(n_upgrade)) => {
                        let mut p_ref = p_upgrade.borrow_mut();
                        let mut n_ref = n_upgrade.borrow_mut();

                        match (&mut *p_ref, &mut *n_ref) {
                            (Node::Leaf {internal_leaf: l_internal_leaf}, Node::Leaf {internal_leaf: r_internal_leaf}) => {
                                l_internal_leaf.next = Some(n.clone());
                                r_internal_leaf.prev = Some(p.clone());

                            }

                            (_, _) => {
                                println!("could not get mutable refs to both nodes");
                                panic!("failed to get borrow refs to both nodes")
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

                        if let Node::Leaf {internal_leaf, .. } = &mut *p_ref {
                            internal_leaf.next = None;
                        }
                    }
                }
                (None, Some(n)) => {
                    // this is the case where we are the far left leaf
                    if let Some(n_upgrade) = n.upgrade() {
                        let mut n_ref = n_upgrade.borrow_mut();

                        if let Node::Leaf { internal_leaf } = &mut *n_ref {
                            internal_leaf.prev = None;
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
            internal_link: InternalLink {
                separators: Vec::new(),
                children: Vec::new(),
            }
        }))
    }

    pub(super) fn new_link_with_seps_and_children(separators: Vec<Rc<K>>, children: Vec<Rc<RefCell<Node<K, V>>>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Node::Link {
            internal_link: InternalLink {
            separators,
            children,
            }
        }))
    }

    pub(super) fn new_leaf() -> Self {
        Node::Leaf {
            internal_leaf: InternalLeaf {

            key_vals: Vec::new(),
            next: None,
            prev: None,
            }
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        match self {
            Node::Leaf { internal_leaf , .. } => {
                internal_leaf.key_vals.is_empty()
            }
            Node::Link { internal_link, .. } => {
                internal_link.children.is_empty()
            }
        }
    }

    pub(super) fn new_leaf_with_kv(key: Rc<K>, value: V) -> Rc<RefCell<Self>> {
        let mut vec = Vec::new();
        vec.push((key, value));

        Rc::new(RefCell::new(Node::Leaf {
            internal_leaf: InternalLeaf {

            key_vals: vec,
            next: None,
            prev: None,
            }
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


    // // splitting a link node with separators 1, 2, 3, should result in a new link node with a single separator of 2 and child link nodes of 1, 3
    // pub(super) fn split_link_node(
    //     self_rc: &Rc<RefCell<Self>>,
    // ) -> (Rc<RefCell<Self>>, Rc<K>, Rc<RefCell<Self>>) {
    //     let (sep, new_right) = (*self_rc.borrow_mut()).split_borrowed_link_node();
    //
    //     if let Node::Link { internal_link, .. } = new_right.borrow().deref() {
    //         return (internal_link.children[0].clone(), sep, new_right.clone());
    //     }
    //     panic!("should not fail")
    // }

    pub(super) fn split_leaf_node(
        link_to_self: &Rc<RefCell<Self>>,
    ) -> Option<(Rc<RefCell<Self>>, Rc<K>, Rc<RefCell<Self>>)> {
        match &mut *link_to_self.borrow_mut() {
            Node::Leaf { internal_leaf } => {
                let (split, right) = internal_leaf.split_borrowed_leaf_node(link_to_self);
                Some((link_to_self.clone(), split, right))
            }
            Node::Link { .. } => {
                None
            }
        }

    }

    // the left Option is the new separator and the right is the new right node. We don't need to do anything with the left node b/c the parent is already pointing to it
    pub(super) fn insert_internal(
        node: &Rc<RefCell<Node<K, V>>>,
        inserted_key: Rc<K>,
        inserted_value: V,
    ) -> Option<(Rc<K>, Rc<RefCell<Node<K, V>>>)> {
        let mut node_ref = node.borrow_mut();
        match &mut *node_ref {
            Node::Leaf { internal_leaf, .. } => {
                let pos = internal_leaf.key_vals
                    .iter()
                    .position(|(k, _)| k.as_ref() > inserted_key.as_ref())
                    .unwrap_or(internal_leaf.key_vals.len());

                internal_leaf.key_vals.insert(pos, (inserted_key, inserted_value));

                if internal_leaf.key_vals.len() <= MAX_KVS_IN_LEAF {
                    return None;
                }

                let (split, new_right) = internal_leaf.split_borrowed_leaf_node(node);

                Some((split, new_right))
            }
            Node::Link {
                internal_link,
            } => {
                let mut child_to_update = internal_link.separators
                    .iter()
                    .position(|k| k.as_ref() > inserted_key.as_ref());

                // if we're inserting the biggest and the child location is empty then create new leaf and return current link
                if child_to_update.is_none() {
                    if internal_link.separators.len() == SEPARATORS_MAX_SIZE {
                        // here we must insert into the right most subtree
                        if internal_link.children.get(DEGREE - 1).is_none() {
                            // no child is here, we need to make a new one
                            let new_leaf = Node::new_leaf_with_kv(inserted_key, inserted_value);
                            internal_link.children.push(new_leaf);
                            return None;
                        }
                    }
                    child_to_update = Some(internal_link.children.len() - 1);
                }

                let child = internal_link.children[child_to_update.unwrap()].clone();

                if let Some((new_separator, new_node)) =
                    Node::insert_internal(&child, inserted_key, inserted_value)
                // self.insert_internal(&child, inserted_key, inserted_value)
                {
                    Node::insert_separator_and_child_into_link(
                        &mut internal_link.separators,
                        &mut internal_link.children,
                        new_separator,
                        new_node,
                    );
                    if internal_link.separators.len() <= SEPARATORS_MAX_SIZE {
                        return None;
                    }

                    let (new_sep, new_right) = internal_link.split_borrowed_link_node();

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
    pub(super) fn delete_internal(node: &Rc<RefCell<Node<K, V>>>, deleted_key: K) -> Option<()> {

        // if link then see if any of the values make sense to continue searching -- I think this is always the case?

        // if leaf then iterate through the K/Vs and delete if we get a match

        let mut node_ref = node.borrow_mut();
        match &mut *node_ref {
            Node::Link { internal_link } => {

                // if the K value is a separator then we're going to need to fix that....
                // if we end up deleting a value from a leaf then we need to children's biggest/smallest values
                // should this only happen if the child node has the value? -- Yes

                // find the child to look at
                let child_to_delete_from_pos = internal_link.separators.iter().position(|s| deleted_key < *s.as_ref()).unwrap_or(internal_link.children.len() - 1);

                let child_node = &internal_link.children[child_to_delete_from_pos];
                let deleted_result = Self::delete_internal(child_node, deleted_key);

                match deleted_result {
                    None => {
                        println!("nothing to report, move along");
                        None
                    }
                    Some(_) => {
                        println!("we deleted something and now need to do something about the deletion....");
                        println!("how do we know if a leaf is empty though?");

                        if child_node.borrow().is_empty() {
                            println!("need to handle child node being empty");
                            panic!("child is now empty, must merge");
                            todo!()
                        } else {

                            None
                        }

                    }
                }
            }
            Node::Leaf { internal_leaf, .. } => {

                let original_size = internal_leaf.key_vals.len();
                // let new_key_vals: Vec<(Rc<K>, V)> = key_vals.iter().filter(|(k, _) : &(Rc<K>, V)| *k.as_ref() != deleted_key).collect();
                internal_leaf.key_vals.retain(|(k, _)| *k.as_ref() != deleted_key);
                let new_size = internal_leaf.key_vals.len();

                if original_size != new_size {
                    println!("we removed an element");
                }

                if internal_leaf.key_vals.is_empty() {
                    todo!()
                } else {
                    return Some(())
                }
            }
        }
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
            internal_leaf: InternalLeaf {

            key_vals: items.iter().map(|k| (Rc::new(*k), k.to_string())).collect(),
            next: None,
            prev: None,
            }
        }))
    }

    #[test]
    fn test_split_leaf() {
        let initial_node = create_leaf_with_kvs(vec![1, 2, 3, 4]);

        if let Some((left, split, right)) = Node::split_leaf_node(&initial_node) {


        if let Node::Leaf { internal_leaf, .. } = left.borrow().deref() {
            let collected_key_vals = internal_leaf.collect_leaf_kvs();
            assert_eq!(collected_key_vals, vec![&1, &2])
        };

        assert_eq!(split.as_ref(), &3);

        if let Node::Leaf { internal_leaf, .. } = right.borrow().deref() {
            let collected_key_vals = internal_leaf.collect_leaf_kvs();
            assert_eq!(collected_key_vals, vec![&3, &4])
        };
        }
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

            if let Node::Leaf { internal_leaf, .. } = &mut *first_ref {
                internal_leaf.next = Some(Rc::downgrade(&second));
            }
            if let Node::Leaf { internal_leaf, .. } = &mut *second_ref {
                internal_leaf.prev = Some(Rc::downgrade(&first));
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
            internal_link: InternalLink {

            separators: vec![Rc::new(2), Rc::new(3), Rc::new(4)],
            children: vec![first.clone(), second.clone(), third.clone(), fourth.clone()],
        }
        }));

        let mut link_ref = link_node.borrow_mut();
        if let Node::Link { internal_link, .. } = &mut *link_ref {
            let (new_sep, new_right) = internal_link.split_borrowed_link_node();
            let collected_seps: Vec<&i32> = internal_link.collect_link_separators();
            assert_eq!(vec![&2], collected_seps);

            let expected_children_keys = [vec![&1], vec![&2]];
            let expected_next = [Some(second.clone()), Some(third.clone())];
            let expected_prev = [None, Some(first.clone())];
            for i in 0..internal_link.children.len() {
                let child_ref = internal_link.children[i].borrow();
                if let Node::Leaf {
                    internal_leaf
                } = child_ref.deref()
                {
                    let collected_keys = internal_leaf.collect_leaf_kvs();
                    assert_eq!(expected_children_keys[i], collected_keys);
                    match (&expected_next[i], &internal_leaf.next) {
                        (Some(expected), Some(actual)) => {
                            assert!(Rc::ptr_eq(expected, &actual.upgrade().unwrap()));
                        }
                        (None, None) => {}
                        (_, _) => {
                            println!("got mismatching Some/None for expected next");
                            assert!(false)
                        }
                    }
                    match (&expected_prev[i], &internal_leaf.prev) {
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
            let new_right_ref = new_right.borrow();
            if let Node::Link {
                internal_link
            } = new_right_ref.deref()
            {
                let collected_seps: Vec<&i32> = internal_link.collect_link_separators();
                assert_eq!(vec![&4], collected_seps);
                let expected_children_keys = [vec![&3], vec![&4, &5]];
                let expected_next = [Some(fourth.clone()), None];
                let expected_prev = [Some(second.clone()), Some(third.clone())];
                for i in 0..internal_link.children.len() {

                    let child_ref = internal_link.children[i].borrow();
                    if let Node::Leaf {
                        internal_leaf
                    } = child_ref.deref()
                    {
                        let collected_keys = internal_leaf.collect_leaf_kvs();
                        assert_eq!(expected_children_keys[i], collected_keys);
                        match (&expected_next[i], &internal_leaf.next) {
                            (Some(expected), Some(actual)) => {
                                assert!(Rc::ptr_eq(expected, &actual.upgrade().unwrap()));
                            }
                            (None, None) => {}
                            (_, _) => {
                                println!("got mismatching Some/None for expected next");
                                assert!(false)
                            }
                        }
                        match (&expected_prev[i], &internal_leaf.prev) {
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

        let leaf_ref = leaf.borrow();

        if let Node::Leaf {internal_leaf, .. } = leaf_ref.deref() {
            let collected_keys = internal_leaf.collect_leaf_kvs();

            assert_eq!(vec![&1, &3], collected_keys);
        };
    }

    #[test]
    fn test_delete_internal_from_link_without_merge() {
        let left_leaf = create_leaf_with_kvs(vec!(1, 2, 3));
        let right_leaf = create_leaf_with_kvs(vec!(4, 5, 6));

        assign_prev_next_in_order(vec!(left_leaf.clone(), right_leaf.clone()));

        let link_node = Rc::new(RefCell::new(Node::Link {
            internal_link: InternalLink {

            separators: vec![Rc::new(4)],
            children: vec![left_leaf.clone(), right_leaf.clone()],
            }
        }));


        Node::delete_internal(&link_node, 3);

        let expected_children_kvs = vec![vec![&1, &2], vec![&4, &5, &6]];
        let link_ref = link_node.borrow();
        if let Node::Link {internal_link} = link_ref.deref() {
            assert_eq!(vec![&4], internal_link.collect_link_separators());

            for child_index in 0..(internal_link.children.len()) {
                if let Node::Leaf { internal_leaf } = internal_link.children[child_index].borrow().deref() {
                    assert_eq!(expected_children_kvs[child_index], internal_leaf.collect_leaf_kvs());
                }
            }

        };
    }

    #[test]
    fn test_delete_empty_child_node() {
        let leaf_node = create_leaf_with_kvs(vec![1]);
        assert!(true)
    }

    #[test]
    fn test_sizes_stay_the_same() {
        assert_eq!(size_of::<Node<i32, String>>(), 48);
        assert_eq!(size_of::<InternalLink<i32, String>>(), 48);
        assert_eq!(size_of::<InternalLeaf<i32, String>>(), 40);
    }
}
