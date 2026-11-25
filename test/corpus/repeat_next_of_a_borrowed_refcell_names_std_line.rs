//@ run-fail: RefCell already mutably borrowed
// `repeat`'s `next` of a `RefCell` mutably borrowed panics where its
// `next` clones it, `repeat.rs`'s, never its caller's line (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(1u8);
    // Its guard leaked, so the cell is moved in mutably borrowed.
    std::mem::forget(cell.borrow_mut());
    let mut items = std::iter::repeat(cell);
    println!("made");
    println!("{:?}", items.next());
}
