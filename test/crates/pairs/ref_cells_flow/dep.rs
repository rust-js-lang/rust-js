//! A library that holds a borrow of a cell of its own while what it's
//! given runs, and borrows a cell its consumer gives it of the same type
//! for a moment (ADR 0863, amended): the one it's given may be one its
//! consumer holds a borrow of while it runs, so it's checked as its type
//! is, and its contract lists no type it borrows unchecked.
use std::cell::RefCell;

pub fn around(f: impl Fn(usize)) {
    let mine = RefCell::new(vec![1u32]);
    let held = mine.borrow_mut();
    f(held.len());
}

pub fn push(cell: &RefCell<Vec<u32>>) {
    cell.borrow_mut().push(1);
}

pub fn len(cell: &RefCell<Vec<u32>>) -> usize {
    cell.borrow().len()
}
