//@ run-fail: RefCell already mutably borrowed
// A `HashMap`'s clone of a value's `RefCell` mutably borrowed panics where
// std's clone clones it, by rustc's `Clone::clone` shim, `clone.rs:236:5`,
// read from std's MIR (`clonedAt`, #2509).
use std::cell::RefCell;
use std::collections::HashMap;

fn main() {
    let map: HashMap<u8, RefCell<u8>> = [(1, RefCell::new(1))].into_iter().collect();
    let _held = map[&1].borrow_mut();
    println!("{}", map.len());
    println!("{}", map.clone().len());
}
