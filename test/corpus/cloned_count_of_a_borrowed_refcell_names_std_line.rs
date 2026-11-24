//@ run-fail: RefCell already mutably borrowed
// A `cloned()`'s `count()` of a `RefCell` mutably borrowed panics where
// std's `Cloned::fold` clones it, through `FnMut::call_mut`'s shim,
// `function.rs:166:5`, never the caller's line: the line every step its
// value is asked reaches its item's clone at (`clonedAt`, #2509).
use std::cell::RefCell;

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    println!("{}", items.len());
    println!("{}", items.iter().cloned().count());
}
