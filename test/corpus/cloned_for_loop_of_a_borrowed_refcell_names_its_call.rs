//@ run-fail: RefCell already mutably borrowed
//@ located-otherwise: #1458 a `cloned()` whose steps aren't one line names its call's line
// A `for` loop over a `cloned()` of a `RefCell` mutably borrowed: native
// Rust names `Cloned::next`'s `function.rs:250:5`; rust-js names the
// call's line until std's `Cloned` is compiled (#1458).
use std::cell::RefCell;

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    for cell in items.iter().cloned() {
        println!("{}", cell.into_inner());
    }
}
