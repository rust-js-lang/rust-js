//@ run-fail: RefCell already mutably borrowed
// A `LinkedList`'s `clone_from` of `RefCell`s, one mutably borrowed, writes
// what it keeps by each cell's `clone_from`, panicking at
// `linked_list.rs:2227:18`, its `clonedFromAt` row, and appends a clone of
// the rest, at `function.rs:166:5`, its `clonedAt` row (#2627).
use std::cell::RefCell;
use std::collections::LinkedList;
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let source: LinkedList<RefCell<u8>> = [RefCell::new(1u8), RefCell::new(2)].into_iter().collect();
    let _held = source.back().unwrap().borrow_mut();
    let mut shorter: LinkedList<RefCell<u8>> = [RefCell::new(5u8)].into_iter().collect();
    let appended = panic::catch_unwind(AssertUnwindSafe(|| shorter.clone_from(&source)));
    println!("{} {}", appended.is_err(), shorter.len());
    let mut longer: LinkedList<RefCell<u8>> = [RefCell::new(5u8), RefCell::new(6), RefCell::new(7)].into_iter().collect();
    longer.clone_from(&source);
}
