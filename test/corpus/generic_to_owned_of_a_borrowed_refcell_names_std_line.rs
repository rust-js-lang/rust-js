//@ run-fail: RefCell already mutably borrowed
// Generic code's `to_owned()` of a slice of `T`, given `RefCell`s, one
// mutably borrowed, panics where std's `to_vec` clones it,
// `slice.rs:432:38`, its dictionary's clone given the row of the function
// rustc resolves the call to (`clonedAt`, #2509).
use std::cell::RefCell;

fn owned<T: Clone>(items: &[T]) -> Vec<T> {
    items.to_owned()
}

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    println!("{:?}", owned(&items));
    println!("{:?}", owned(&[1u8, 2]));
    let _held = items[1].borrow_mut();
    println!("{}", owned(&items).len());
}
