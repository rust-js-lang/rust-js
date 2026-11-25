//@ run-fail: RefCell already mutably borrowed
// An `Option<&mut RefCell<_>>`'s `cloned()` of one mutably borrowed panics
// where std's `map(|t| t.clone())` clones it, as `Option<&T>`'s does, never
// its caller's line: both impls' `cloned` are the catalog's one path
// (`clonedAt`, #2577).
use std::cell::RefCell;

fn main() {
    let mut cell = RefCell::new(1u8);
    // Its guard leaked, so the cell is mutably borrowed.
    std::mem::forget(cell.borrow_mut());
    let item = Some(&mut cell);
    println!("{}", item.is_some());
    println!("{:?}", item.cloned());
}
