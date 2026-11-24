//@ run-fail: RefCell already mutably borrowed
// A `VecDeque`'s clone of an item's `RefCell` mutably borrowed panics where
// std's runs clone it, its extend's `fold` through `FnMut::call_mut`'s
// shim, `function.rs:166:5`: std's MIR has a second path, a `try_fold`'s,
// so the native oracle says which runs (test/std-cloned-at.rs, #2509).
use std::cell::RefCell;
use std::collections::VecDeque;

fn main() {
    let items: VecDeque<RefCell<u8>> = VecDeque::from([RefCell::new(1), RefCell::new(2)]);
    let _held = items[1].borrow_mut();
    println!("{}", items.len());
    println!("{}", items.clone().len());
}
