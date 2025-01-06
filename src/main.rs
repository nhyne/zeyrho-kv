mod zeyrho;

use crate::zeyrho::btree::tree::BPlusTree;

fn main() {
    let mut tree = BPlusTree::new();
    for i in 0..20000 {
        tree.insert(i, i.to_string());
    }

    println!("size of tree: {}", size_of::<BPlusTree<i32, String>>());
}
