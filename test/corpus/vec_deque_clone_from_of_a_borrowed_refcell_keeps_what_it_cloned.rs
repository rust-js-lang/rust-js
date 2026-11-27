//@ run-fail: RefCell already mutably borrowed
// A `VecDeque`'s `clone_from` of `RefCell`s, one mutably borrowed, clears
// itself, then pushes a clone of each: what it pushed before the cell's
// clone panicked stays, as a cell's clone may panic, never `*w = v.clone()`
// that leaves it as it was (#2627).
use std::cell::RefCell;
use std::collections::VecDeque;
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let source: VecDeque<RefCell<u8>> = [RefCell::new(1u8), RefCell::new(2)].into_iter().collect();
    let _held = source[1].borrow_mut();
    let mut into: VecDeque<RefCell<u8>> = [RefCell::new(5u8), RefCell::new(6), RefCell::new(7)].into_iter().collect();
    let pushed = panic::catch_unwind(AssertUnwindSafe(|| into.clone_from(&source)));
    println!("{} {:?}", pushed.is_err(), into.iter().map(|cell| *cell.borrow()).collect::<Vec<_>>());
    into.clone_from(&source);
}
