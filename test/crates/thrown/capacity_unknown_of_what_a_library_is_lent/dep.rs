//! A library that keeps no `Vec`'s capacity, whose function reads its
//! private state: a consumer that reads a capacity calls the library's own,
//! and each `Vec` it lends it has no known capacity after, which
//! `capacity()` refuses at run time, naming #1180 (#1180's (5)).
use std::sync::atomic::{AtomicU32, Ordering};

static BASE: AtomicU32 = AtomicU32::new(3);

pub fn fill(v: &mut Vec<u32>) {
    for i in 0..5 {
        v.push(i + BASE.load(Ordering::Relaxed));
    }
}
