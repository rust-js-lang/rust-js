//@ run-fail: RefCell already mutably borrowed
// `VecDeque::resize` of a `RefCell` mutably borrowed panics where std's
// `repeat_n` it extends by clones it, `iter/sources/repeat_n.rs`, never
// its caller's line (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(1u8);
    // Its guard leaked, so the cell is moved in mutably borrowed.
    std::mem::forget(cell.borrow_mut());
    let mut items = std::collections::VecDeque::from([RefCell::new(0u8)]);
    println!("{}", items.len());
    items.resize(3, cell);
    println!("{}", items.len());
}
