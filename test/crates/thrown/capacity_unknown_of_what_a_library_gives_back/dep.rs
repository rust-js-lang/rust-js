//! A library that keeps no `Vec`'s capacity, whose function reads its
//! private state through a private function it calls: a thread-local whose
//! cell it lends whole, which its module keeps as its `{ value }`, so it
//! exports it by no internal name (#2017). A consumer that reads a
//! capacity can't compile it itself, as a copy can't reach that state, so
//! it calls the library's own, and each `Vec` it gives back has no known
//! capacity, which `capacity()` refuses at run time, naming #1180 (#1180's
//! (5)).
use std::cell::Cell;

thread_local! {
    static MADE: Cell<u32> = const { Cell::new(0) };
}

fn next_of(made: &Cell<u32>) -> u32 {
    let n = made.get();
    made.set(n + 1);
    n
}

fn next() -> u32 {
    MADE.with(next_of)
}

pub struct Made {
    pub items: Vec<u32>,
}

pub fn made() -> Made {
    let n = next();
    let mut items = Vec::new();
    for i in 0..5 {
        items.push(i + n);
    }
    Made { items }
}
