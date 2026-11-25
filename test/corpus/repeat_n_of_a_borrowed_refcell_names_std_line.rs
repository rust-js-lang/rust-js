//@ run-fail: RefCell already mutably borrowed
// `iter::repeat_n` of a `RefCell` mutably borrowed, collected, panics
// where its `next` clones it, `iter/sources/repeat_n.rs`, never its
// caller's line (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(1u8);
    // Its guard leaked, so the cell is moved in mutably borrowed.
    std::mem::forget(cell.borrow_mut());
    println!("made");
    let items: Vec<RefCell<u8>> = std::iter::repeat_n(cell, 3).collect();
    println!("{}", items.len());
}
