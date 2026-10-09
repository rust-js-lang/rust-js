//@ run-fail: RefCell already mutably borrowed
// A `RefCell`'s `clone` borrows it, so it panics while a `borrow_mut()`
// lives, as std's does (ADR 0328).

use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(vec![1]);
    let writing = cell.borrow_mut();
    let copy = cell.clone();
    println!("{:?} {}", copy, writing.len());
}
