//@ run-fail: RefCell already mutably borrowed
// A `Vec`'s `extend_from_slice` of a `RefCell` mutably borrowed panics
// where std's clones it, its `fold` through `FnMut::call_mut`'s shim,
// `function.rs:166:5` (`clonedAt`, #2509).
use std::cell::RefCell;

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    let mut copies = Vec::new();
    copies.extend_from_slice(&items);
    println!("{}", copies.len());
}
