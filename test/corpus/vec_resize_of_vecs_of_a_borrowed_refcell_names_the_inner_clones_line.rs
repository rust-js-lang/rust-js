//@ run-fail: RefCell already mutably borrowed
// `Vec::resize` of `Vec`s of a `RefCell` mutably borrowed panics where the
// item's own clone, a `Vec`'s, clones the cell, never `resize`'s line of
// the item it clones: a nearer clone places its own (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(1u8);
    // Its guard leaked, so the cell is moved in mutably borrowed.
    std::mem::forget(cell.borrow_mut());
    let mut items = vec![Vec::new()];
    println!("{}", items.len());
    items.resize(3, vec![cell]);
    println!("{}", items.len());
}
