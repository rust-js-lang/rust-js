//@ run-fail: RefCell already borrowed
// A thread-local's storage holds what its initializer gives, which may be
// another's cell (ADR 0863, amended): a borrow of the first's held while
// the second's is borrowed mutably is of one cell, checked, and panics, as
// std's does.

use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static FIRST: Rc<RefCell<Vec<i32>>> = Rc::new(RefCell::new(vec![1]));
    static SECOND: Rc<RefCell<Vec<i32>>> = FIRST.with(Rc::clone);
}

fn main() {
    let first = FIRST.with(Rc::clone);
    let second_len = || SECOND.with(|second| second.borrow_mut().len());
    println!("{}", second_len());
    let held = first.borrow();
    println!("{}", second_len());
    println!("{}", held.len());
}
