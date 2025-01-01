mod zeyrho;

use crate::zeyrho::btree::tree::BPlusTree;
use std::ops::Deref;

fn main() {
    let mut tree = BPlusTree::new();
    for i in 0..20000000 {
        tree.insert(i, i.to_string());
    }

    // println!("tree head: {:?}", tree.root);
}
