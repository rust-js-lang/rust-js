//! A library's generic `sum()` and `collect()` of an iterator it makes,
//! into an impl its callers choose: its own compile sees no impl that asks
//! the hint, so its JS makes the iterator not knowing it, JS's own
//! `take(1)` and `drop(1)` (#2518).
use std::iter::Sum;

pub fn total_for_anyone<S: Sum<u32>>(v: Vec<u32>) -> S {
    v.into_iter().take(1).sum()
}

pub fn gathered_for_anyone<B: FromIterator<u32>>(v: &[u32]) -> B {
    v.iter().copied().skip(1).collect()
}

pub fn first_two<S: Sum<u32>>(v: &[u32]) -> S {
    v.iter().copied().take(2).sum()
}
