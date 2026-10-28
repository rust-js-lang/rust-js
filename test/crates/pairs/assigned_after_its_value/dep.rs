// A `RefCell<String>` this library borrows unchecked, and a function of its
// own: React's `shared`, beside React DOM's inputValueTracking (gap rd32).
use std::cell::RefCell;

pub fn set(cell: &RefCell<String>, text: &str) {
    *cell.borrow_mut() = text.to_string();
}

pub fn shout(text: &str) -> String {
    text.to_uppercase()
}
