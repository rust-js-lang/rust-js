//@ run-fail: RefCell already mutably borrowed
//@ located-otherwise: #1458 a `cloned()` zipped is stepped by std's `TrustedRandomAccess` step, which the step catalog doesn't run
// A `cloned()` zipped, of a `RefCell` mutably borrowed: native Rust names
// `Cloned::__iterator_get_unchecked`'s `cloned.rs:70:55`, std's `Zip`
// stepping a source of `TrustedRandomAccess` by it, a step std's catalog
// doesn't run, so rust-js names `next`'s `function.rs:250:5` of a
// `count()` and the call's line of a `collect()`, until std's `Cloned` and
// `Zip` are compiled (#1458, #2627).
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    println!("{}", items.iter().cloned().zip(0..2).count());
    let _held = items[1].borrow_mut();
    let counted = panic::catch_unwind(AssertUnwindSafe(|| items.iter().cloned().zip(0..2).count()));
    println!("{}", counted.is_err());
    let pairs: Vec<(RefCell<u8>, i32)> = items.iter().cloned().zip(0..2).collect();
    println!("{}", pairs.len());
}
