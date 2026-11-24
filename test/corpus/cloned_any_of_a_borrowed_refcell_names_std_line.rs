//@ run-fail: RefCell already mutably borrowed
// A `cloned()`'s `any` of a `RefCell` mutably borrowed panics where std's
// `Cloned::try_fold` clones it, its closure's line, `cloned.rs:29:32`
// (`clonedAt`, #2509).
use std::cell::RefCell;

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    println!("{}", items.len());
    println!("{}", items.iter().cloned().any(|cell| *cell.borrow() > 5));
}
