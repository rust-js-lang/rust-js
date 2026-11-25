//@ run-fail: RefCell already mutably borrowed
// A clone of a `repeat` of a `RefCell` mutably borrowed panics where
// `Repeat`'s derived `Clone` clones it, never a step's line (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(1u8);
    // Its guard leaked, so the cell is moved in mutably borrowed.
    std::mem::forget(cell.borrow_mut());
    let items = std::iter::repeat(cell);
    println!("made");
    let again = items.clone();
    drop(again);
}
