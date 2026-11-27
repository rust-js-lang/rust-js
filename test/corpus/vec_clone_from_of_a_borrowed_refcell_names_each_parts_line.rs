//@ run-fail: RefCell already mutably borrowed
// A `Vec`'s `clone_from` of `RefCell`s, one mutably borrowed, writes what it
// keeps by each cell's `clone_from`, panicking at `slice.rs:838:16`, its
// `clonedFromAt` row, and pushes a clone of the rest, at `function.rs:166:5`,
// its `clonedAt` row; what it wrote before it panicked stays written, as a
// cell's clone may panic (#2627).
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

fn values(cells: &[RefCell<u8>]) -> Vec<u8> {
    cells.iter().map(|cell| *cell.borrow()).collect()
}

fn main() {
    let source = vec![RefCell::new(1u8), RefCell::new(2)];
    let mut longer = vec![RefCell::new(5u8), RefCell::new(6), RefCell::new(7)];
    longer.clone_from(&source);
    println!("{:?}", values(&longer));
    let _held = source[1].borrow_mut();
    let mut shorter = vec![RefCell::new(5u8)];
    let pushed = panic::catch_unwind(AssertUnwindSafe(|| shorter.clone_from(&source)));
    println!("{} {}", pushed.is_err(), shorter.len());
    let mut same = vec![RefCell::new(5u8), RefCell::new(6)];
    let kept = panic::catch_unwind(AssertUnwindSafe(|| same.clone_from(&source)));
    println!("{} {:?}", kept.is_err(), same.iter().map(|cell| cell.try_borrow().map(|v| *v).ok()).collect::<Vec<_>>());
    same.clone_from(&source);
}
