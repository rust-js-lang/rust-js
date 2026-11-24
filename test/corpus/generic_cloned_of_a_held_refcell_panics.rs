//@ run-fail: RefCell already mutably borrowed
//@ located-otherwise: #2522 generic code's `cloned()` clones a `RefCell` at its call's line
// Generic code's `cloned()` of a `RefCell` whose guard its caller holds:
// std's `Cloned` clones it, borrowing it, so it panics, and the guard is
// kept for it to see. A type parameter that may be a cell, through std's
// code, borrows what its callers' instances say (#2529).
use std::cell::RefCell;

fn copies<T: Clone>(items: &[T]) -> usize {
    items.iter().cloned().count()
}

fn made<T: Clone>(items: &[T]) -> usize {
    items.iter().map(T::clone).count()
}

fn main() {
    let numbers = vec![1u8, 2];
    println!("{} {}", copies(&numbers), made(&numbers));
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let held = items[0].borrow();
    println!("{} {}", copies(&items), made(&items));
    drop(held);
    let _held = items[1].borrow_mut();
    println!("{}", copies(&items));
}
