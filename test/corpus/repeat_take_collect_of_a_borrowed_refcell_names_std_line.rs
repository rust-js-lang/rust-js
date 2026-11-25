//@ run-fail: RefCell already mutably borrowed
// A `take()` of a `repeat` of a `RefCell` mutably borrowed, collected,
// panics where the step that runs clones it, `next`'s line, though
// `take` may ask several steps of it (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(1u8);
    // Its guard leaked, so the cell is moved in mutably borrowed.
    std::mem::forget(cell.borrow_mut());
    println!("made");
    let items: Vec<RefCell<u8>> = std::iter::repeat(cell).take(3).collect();
    println!("{}", items.len());
}
