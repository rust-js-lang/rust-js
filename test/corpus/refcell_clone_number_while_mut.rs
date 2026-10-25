//@ run-fail: RefCell already mutably borrowed
// A `RefCell` of a number, a cell of its own, borrows itself to clone, so
// its clone panics while a `borrow_mut()` lives, as std's does (ADR 0328,
// #279).

use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(5u32);
    let mut writing = cell.borrow_mut();
    *writing += 1;
    let copy = cell.clone();
    println!("{:?} {}", copy, *writing);
}
