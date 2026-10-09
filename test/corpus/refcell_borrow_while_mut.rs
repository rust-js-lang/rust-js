//@ run-fail: RefCell already mutably borrowed
// A `borrow()` while a `borrow_mut()` lives, a temporary its statement holds,
// panics, as std's does (ADR 0328).

use std::cell::RefCell;

fn total(cell: &RefCell<Vec<i32>>) -> i32 {
    cell.borrow().iter().sum()
}

fn main() {
    let cell = RefCell::new(vec![1, 2]);
    cell.borrow_mut().push(total(&cell));
    println!("{:?}", cell.borrow());
}
