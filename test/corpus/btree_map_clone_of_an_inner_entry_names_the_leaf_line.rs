//@ run-fail: RefCell already mutably borrowed
//@ located-otherwise: difference 65, a `BTreeMap`'s internal-node entry names the leaf's line
// A `BTreeMap` of forty entries, its eleventh in an internal node of std's
// B-tree, cloned with that entry's `RefCell` mutably borrowed: native Rust
// names `clone_subtree`'s internal nodes' line, `map.rs:278`; rust-js, whose
// map keeps no tree, names the leaf's, `map.rs:258` (the guide's
// difference 65, #2509).
use std::cell::RefCell;
use std::collections::BTreeMap;

fn main() {
    let map: BTreeMap<u8, RefCell<u8>> = (0..40).map(|i| (i, RefCell::new(i))).collect();
    let _held = map[&11].borrow_mut();
    println!("{}", map.len());
    println!("{}", map.clone().len());
}
