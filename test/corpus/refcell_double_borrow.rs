//@ run-fail: RefCell already borrowed
// A `borrow_mut()` while a `borrow()` lives panics, as std's does (ADR 0328).

use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(vec![1]);
    let reading = cell.borrow();
    cell.borrow_mut().push(2);
    println!("{}", reading.len());
}
