//@ run-fail: RefCell already mutably borrowed
// A slice's `to_vec` of a `RefCell` mutably borrowed panics where std's
// `to_vec` clones each item, `alloc/src/slice.rs`, never its caller's
// line (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let items = [RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    println!("{}", items.len());
    let copied = items.to_vec();
    println!("{}", copied.len());
}
