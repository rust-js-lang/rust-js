//! A library that keeps no `Vec`'s capacity, whose function reads its
//! private state: a consumer that reads a capacity compiles that function
//! itself, which can't reach the library's state, so it's refused, saying
//! why it's compiled here (#1180's (5), ADR 0914).
use std::sync::atomic::{AtomicU32, Ordering};

static MADE: AtomicU32 = AtomicU32::new(0);

pub fn made() -> Vec<u32> {
    let n = MADE.fetch_add(1, Ordering::Relaxed);
    let mut v = Vec::new();
    for i in 0..5 {
        v.push(i + n);
    }
    v
}
