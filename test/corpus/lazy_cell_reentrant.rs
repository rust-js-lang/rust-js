//@ run-fail: LazyCell instance has previously been poisoned
// A `LazyCell` whose init reads it itself panics, as std's does (ADR 0318).

use std::cell::{Cell, LazyCell};

thread_local! {
    static SELF: Cell<Option<&'static LazyCell<u32>>> = const { Cell::new(None) };
}

fn init() -> u32 {
    SELF.get().map_or(0, |me| **me + 1)
}

fn main() {
    let lazy: &'static LazyCell<u32> = Box::leak(Box::new(LazyCell::new(init as fn() -> u32)));
    SELF.set(Some(lazy));
    println!("{}", **lazy);
}
