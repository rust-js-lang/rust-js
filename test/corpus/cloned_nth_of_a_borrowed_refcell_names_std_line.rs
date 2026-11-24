//@ run-fail: RefCell already mutably borrowed
// A `cloned()`'s `nth(1)` of a `RefCell` mutably borrowed panics where
// std's `Cloned` clones it by `next`, through `FnOnce::call_once`'s
// shim, `function.rs:250:5`: std's MIR has a second path, so the native
// oracle says which runs (test/std-cloned-at.rs, #2509).
use std::cell::RefCell;

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    println!("{}", items.len());
    println!("{:?}", items.iter().cloned().nth(1));
}
