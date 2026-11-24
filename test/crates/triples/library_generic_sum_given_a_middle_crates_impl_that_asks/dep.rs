//! A library's generic `sum()` of an iterator it makes, into an impl its
//! callers choose, which its compile, seeing none, made not knowing its
//! hint, JS's own `values().drop(1)` (#2518).
use std::iter::Sum;

pub fn total_for_anyone<S: Sum<u32>>(v: Vec<u32>) -> S {
    v.into_iter().skip(1).sum()
}
