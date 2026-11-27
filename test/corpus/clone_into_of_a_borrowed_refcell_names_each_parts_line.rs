//@ run-fail: RefCell already mutably borrowed
// A slice's `clone_into` of `RefCell`s, one mutably borrowed, writes what
// the target keeps by each cell's `clone_from`, `slice.rs:838:16`, and
// pushes a clone of the rest, `function.rs:166:5`; a `RefCell`'s own,
// std's blanket `ToOwned`'s, `borrow.rs:82:16` (#2627).
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let source = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = source[1].borrow_mut();
    let mut shorter = vec![RefCell::new(5u8)];
    let pushed = panic::catch_unwind(AssertUnwindSafe(|| source[..].clone_into(&mut shorter)));
    println!("{} {}", pushed.is_err(), shorter.len());
    let mut target = RefCell::new(0u8);
    let into = panic::catch_unwind(AssertUnwindSafe(|| source[1].clone_into(&mut target)));
    println!("{} {}", into.is_err(), target.borrow());
    let mut same = vec![RefCell::new(5u8), RefCell::new(6)];
    source[..].clone_into(&mut same);
}
