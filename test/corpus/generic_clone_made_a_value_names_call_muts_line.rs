//@ run-fail: RefCell already mutably borrowed
// Generic code's `T::clone` made a value, `map(T::clone)`, of a `RefCell`
// mutably borrowed, panics at the shim `Iterator::map`'s `F: FnMut` calls
// it through, `FnMut::call_mut`'s, `function.rs:166:5`, never where it's
// made a value: its impl tracks its caller where `Clone::clone` doesn't
// (#2522).
use std::cell::RefCell;

fn copies<T: Clone>(items: &[T]) -> usize {
    items.iter().map(T::clone).count()
}

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    println!("{}", items.len());
    println!("{}", copies(&items));
}
