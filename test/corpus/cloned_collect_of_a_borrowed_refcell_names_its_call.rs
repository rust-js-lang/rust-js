//@ run-fail: RefCell already mutably borrowed
//@ located-otherwise: #2509 a `cloned()` whose steps aren't one line names its call's line
// A `cloned()` collected, of a `RefCell` mutably borrowed: native Rust
// names `function.rs:166:5` of a slice's, whose `collect` folds, and
// `250:5` of a `filter()`'s, which steps `next`; rust-js names the
// call's line until std's `Cloned` and `collect` are compiled (#1458, #2509).
use std::cell::RefCell;

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    let copies: Vec<RefCell<u8>> = items.iter().cloned().collect();
    println!("{}", copies.len());
}
