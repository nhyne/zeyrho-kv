
mod zeyrho;

use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use zeyrho::b_tree::*;

fn main() {
    let mut tree = BPlusTree::new();
    for i in 0..6 {
        tree.insert(i, i.to_string());
    }

    if let BPlusTree{root} = tree {
        if let Some(node) = root {
            match node.as_ref().borrow().deref() {
                Node::Leaf { key_vals , ..} => {
                    println!("key_vals: {:?}", key_vals)
                }
                Node::Link { separators, children } => {
                    println!("separators: {:?}", separators);

                    for i in children {
                        println!("child: {:?}", i)
                    }

                }
            }
        }
    }

}