//@ run-fail: RefCell already mutably borrowed
// `==` of `RefCell`s borrows each, so it panics while a `borrow_mut()`
// lives, as std's does (ADR 0328).

use std::cell::RefCell;

fn main() {
    let a = RefCell::new(1);
    let b = RefCell::new(1);
    let writing = a.borrow_mut();
    println!("{}", a == b);
    println!("{}", *writing);
}
