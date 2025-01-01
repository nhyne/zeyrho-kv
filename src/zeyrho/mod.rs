use std::collections::HashMap;
use std::rc::Rc;
use crate::zeyrho::TreeNode::{LeafNode, NonLeafNode};

#[rustfmt::skip]
pub mod kv_store;
pub mod queue;
pub mod btree;


trait BTree<K: Ord, V> {
    fn insert(&mut self, key: K, value: V);
    fn get(&self, key: &K) -> Option<V>;
    fn delete(&mut self, key: &K) -> bool;
    fn scan(&self, query: String) -> [V];
}


enum TreeNode<K: Ord, V> {
    NonLeafNode{ separators: Vec<Rc<K>>,  child_nodes: Vec<TreeNode<K, V>>},
    LeafNode{map: HashMap<Rc<K>, V>},
}

pub struct InMemoryBTree<K: Ord, V> {
    root: TreeNode<K, V>,
}


pub fn put_together_list() {

    let only_key = "adam".to_string();
    let only_value = 32;

    let pointer = Rc::new(only_key);
    let mut node_map = HashMap::new();
    _ = node_map.insert(Rc::clone(&pointer), only_value);

    let leaf_node = LeafNode {
        map: node_map,
    };

    let root_node = NonLeafNode {
        separators: vec![Rc::clone(&pointer)],
        child_nodes: vec![leaf_node]
    };

}