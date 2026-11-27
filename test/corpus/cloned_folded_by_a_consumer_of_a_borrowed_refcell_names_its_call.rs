//@ run-fail: RefCell already mutably borrowed
//@ located-otherwise: #1458 a `cloned()` asked a step the step catalog doesn't run names its call's line
// A `cloned()` consumed by a method std writes over its `fold`, of a
// `RefCell` mutably borrowed: native Rust names `Cloned::fold`'s
// `FnMut::call_mut` shim, `function.rs:166:5`, and of `try_for_each` its
// `try_fold` closure's `cloned.rs:29:32`; `for_each`, `partition`,
// `min_by_key`, `max_by_key`, `reduce`, `try_for_each` and a `map()`'s
// `sum()` are steps std's catalog doesn't run, so rust-js names the call's
// line until std's `Cloned` is compiled (#1458, #2627).
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let items = vec![RefCell::new(1u8), RefCell::new(2)];
    let _held = items[1].borrow_mut();
    let caught = |what: &str, ran: Result<(), Box<dyn std::any::Any + Send>>| println!("{what} {}", ran.is_err());
    caught("for_each", panic::catch_unwind(AssertUnwindSafe(|| items.iter().cloned().for_each(drop))));
    caught("partition", panic::catch_unwind(AssertUnwindSafe(|| {
        let (_, _): (Vec<RefCell<u8>>, Vec<RefCell<u8>>) = items.iter().cloned().partition(|_| true);
    })));
    caught("min_by_key", panic::catch_unwind(AssertUnwindSafe(|| drop(items.iter().cloned().min_by_key(|_| 0)))));
    caught("max_by_key", panic::catch_unwind(AssertUnwindSafe(|| drop(items.iter().cloned().max_by_key(|_| 0)))));
    caught("reduce", panic::catch_unwind(AssertUnwindSafe(|| drop(items.iter().cloned().reduce(|a, _| a)))));
    caught("try_for_each", panic::catch_unwind(AssertUnwindSafe(|| {
        let _: Result<(), ()> = items.iter().cloned().try_for_each(|_| Ok(()));
    })));
    println!("{}", items.iter().cloned().map(RefCell::into_inner).sum::<u8>());
}
