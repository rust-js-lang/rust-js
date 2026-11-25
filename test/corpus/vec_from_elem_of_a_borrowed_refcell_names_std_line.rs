//@ run-fail: RefCell already mutably borrowed
// `vec![x; n]` of a `RefCell` mutably borrowed panics where std's
// `from_elem` clones it, `vec/mod.rs`'s `extend_with`, never its
// caller's line (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(1u8);
    // Its guard leaked, so the cell is moved in mutably borrowed.
    std::mem::forget(cell.borrow_mut());
    println!("made");
    let items = vec![cell; 3];
    println!("{}", items.len());
}
