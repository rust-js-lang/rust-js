//@ run-fail: RefCell already mutably borrowed
// A `BTreeMap`'s clone of a value's `RefCell` mutably borrowed panics where
// std's `clone_subtree` clones a leaf's values, `map.rs:258:56`, read from
// std's MIR (`clonedAt`, #2509).
use std::cell::RefCell;
use std::collections::BTreeMap;

fn main() {
    let map: BTreeMap<u8, RefCell<u8>> = (0..3).map(|i| (i, RefCell::new(i))).collect();
    let _held = map[&1].borrow_mut();
    println!("{}", map.len());
    println!("{}", map.clone().len());
}
