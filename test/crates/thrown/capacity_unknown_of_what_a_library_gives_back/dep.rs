//! A library that keeps no `Vec`'s capacity, whose function reads its
//! private state through a private function it calls: a consumer that reads
//! a capacity can't compile it itself, as a copy can't reach that state, so
//! it calls the library's own, and each `Vec` it gives back has no known
//! capacity, which `capacity()` refuses at run time, naming #1180 (#1180's
//! (5)).
use std::sync::atomic::{AtomicU32, Ordering};

static MADE: AtomicU32 = AtomicU32::new(0);

fn next() -> u32 {
    MADE.fetch_add(1, Ordering::Relaxed)
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
