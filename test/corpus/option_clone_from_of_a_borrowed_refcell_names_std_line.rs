//@ run-fail: RefCell already mutably borrowed
// An `Option`'s `clone_from` of a `RefCell` mutably borrowed, both `Some`,
// panics where std's writes the value it keeps by the cell's `clone_from`,
// `option.rs:2288:42`, its `clonedFromAt` row; of a `None` it's given,
// where its `*to = from.clone()` clones it, `option.rs:2280:31` (#2627).
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let source = Some(RefCell::new(1u8));
    let mut kept = Some(RefCell::new(5u8));
    kept.clone_from(&source);
    println!("{kept:?}");
    let _held = source.as_ref().unwrap().borrow_mut();
    let mut none: Option<RefCell<u8>> = None;
    let cloned = panic::catch_unwind(AssertUnwindSafe(|| none.clone_from(&source)));
    println!("{} {none:?}", cloned.is_err());
    kept.clone_from(&source);
    println!("{kept:?}");
}
