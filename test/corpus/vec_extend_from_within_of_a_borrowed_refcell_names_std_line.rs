//@ run-fail: RefCell already mutably borrowed
// `Vec::extend_from_within` of a `RefCell` mutably borrowed panics where
// std clones each item of the range, `vec/mod.rs`, never its caller's
// line; the range's own check is `slice::range`'s (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(1u8);
    // Its guard leaked, so the cell is moved in mutably borrowed.
    std::mem::forget(cell.borrow_mut());
    let mut items = vec![RefCell::new(0u8), cell];
    println!("{}", items.len());
    items.extend_from_within(..);
    println!("{}", items.len());
}
