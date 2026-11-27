//@ run-fail: RefCell already mutably borrowed
//@ located-otherwise: #1458 a `cloned()` given to `extend` or collected reversed names its call's line
// A `cloned()` given to a `Vec`'s `extend`, or reversed and collected, of a
// `RefCell` mutably borrowed: native Rust names `Cloned::fold`'s
// `FnMut::call_mut` shim, `function.rs:166:5`, as std's `extend` of a
// `TrustedLen` iterator folds it; what generic code's `extend` and
// `collect` ask isn't known, so rust-js names the call's line until std's
// `Cloned` and `collect` are compiled (#1458, #2627).
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    let mut into: Vec<RefCell<u8>> = Vec::new();
    let extended = panic::catch_unwind(AssertUnwindSafe(|| into.extend(items.iter().cloned())));
    println!("{}", extended.is_err());
    let reversed: Vec<RefCell<u8>> = items.iter().cloned().rev().collect();
    println!("{}", reversed.len());
}
