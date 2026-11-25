//@ run-fail: RefCell already mutably borrowed
// A slice's `fill` of a `RefCell` mutably borrowed panics where std's
// `spec_fill` clones it into each item but the last, `clone_from`, in
// `slice/specialize.rs`, never its caller's line (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(1u8);
    // Its guard leaked, so the cell is moved in mutably borrowed.
    std::mem::forget(cell.borrow_mut());
    let mut items = [RefCell::new(0u8), RefCell::new(0)];
    println!("{}", items.len());
    items.fill(cell);
    println!("{:?}", items);
}
