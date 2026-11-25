//@ run-fail: RefCell already mutably borrowed
// A `take()` of a `repeat` stepped from its back, of a `RefCell` mutably
// borrowed, panics where `Take<Repeat<T>>`'s `next_back` clones it, at
// `Repeat::next`'s line, which std's code of it inlines (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let cell = RefCell::new(1u8);
    // Its guard leaked, so the cell is moved in mutably borrowed.
    std::mem::forget(cell.borrow_mut());
    let mut items = std::iter::repeat(cell).take(3);
    println!("made");
    println!("{:?}", items.next_back());
}
