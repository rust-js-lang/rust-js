//@ run-fail: RefCell already borrowed
// `swap` borrows each cell mutably, the first while the second, so a cell
// swapped with itself panics, as std's does (ADR 0328).

use std::cell::RefCell;

fn swap(a: &RefCell<i32>, b: &RefCell<i32>) {
    a.swap(b);
}

fn main() {
    let cell = RefCell::new(1);
    let other = RefCell::new(2);
    swap(&cell, &other);
    println!("{} {}", cell.borrow(), other.borrow());
    swap(&cell, &cell);
}
