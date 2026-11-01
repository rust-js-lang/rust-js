//! A crate that holds a borrow of a cell while its library borrows another
//! of the same type it's given: as the library checks the cells it's given,
//! it isn't refused (ADR 0863, amended).
use std::cell::RefCell;

pub fn main() {
    let mine = RefCell::new(vec![1u32]);
    let other = RefCell::new(vec![2u32]);
    let held = mine.borrow();
    dep::push(&other);
    dep::around(|n| print!("{n} "));
    println!("{} {}", held.len(), dep::len(&other));
}
