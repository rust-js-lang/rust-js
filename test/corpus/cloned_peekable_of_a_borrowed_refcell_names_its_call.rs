//@ run-fail: RefCell already mutably borrowed
//@ located-otherwise: #1458 a `cloned()` held by a `Peekable` is asked steps the step catalog doesn't run
// A `cloned()` held by a `Peekable`, of a `RefCell` mutably borrowed:
// native Rust names `Cloned::next`'s `FnOnce::call_once` shim,
// `function.rs:250:5`, of its `peek()`; a `peek` is no step std's catalog
// runs, so rust-js names the call's line until std's `Cloned` and
// `Peekable` are compiled (#1458, #2627).
use std::cell::RefCell;

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let mut free = items.iter().cloned().peekable();
    println!("{}", free.peek().is_some());
    let _held = items[0].borrow_mut();
    let mut held = items.iter().cloned().peekable();
    println!("{}", held.peek().is_some());
}
