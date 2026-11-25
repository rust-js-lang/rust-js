//@ run-fail: RefCell already mutably borrowed
// `repeat_n`'s `nth` of a `RefCell` mutably borrowed panics where
// `RepeatN`'s step clones it (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(1u8);
    // Its guard leaked, so the cell is moved in mutably borrowed.
    std::mem::forget(cell.borrow_mut());
    let mut items = std::iter::repeat_n(cell, 4);
    println!("made");
    println!("{:?}", items.nth(1));
}
