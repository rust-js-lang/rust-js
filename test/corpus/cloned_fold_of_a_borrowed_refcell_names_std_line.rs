//@ run-fail: RefCell already mutably borrowed
// A `cloned()`'s `fold` of a `RefCell` mutably borrowed panics where std's
// `Cloned::fold` clones it, through `FnMut::call_mut`'s shim,
// `function.rs:166:5` (`clonedAt`, #2509).
use std::cell::RefCell;

fn main() {
    let items = [RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    let total = items.iter().cloned().fold(0, |sum, cell| sum + *cell.borrow());
    println!("{total}");
}
