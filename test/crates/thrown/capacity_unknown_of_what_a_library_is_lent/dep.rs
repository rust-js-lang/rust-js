//! A library that keeps no `Vec`'s capacity, whose function reads its
//! private state, a thread-local whose cell it lends whole, which its
//! module keeps as its `{ value }` and exports by no internal name (#2017):
//! a consumer that reads a capacity calls the library's own, and each `Vec`
//! it lends it has no known capacity after, which `capacity()` refuses at
//! run time, naming #1180 (#1180's (5)).
use std::cell::Cell;

thread_local! {
    static BASE: Cell<u32> = const { Cell::new(3) };
}

fn base_of(base: &Cell<u32>) -> u32 {
    base.get()
}

pub fn fill(v: &mut Vec<u32>) {
    for i in 0..5 {
        v.push(i + BASE.with(base_of));
    }
}
