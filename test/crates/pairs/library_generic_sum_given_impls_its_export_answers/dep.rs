//! A library's generic `sum()` of an iterator it makes, into an impl its
//! callers choose, and its own `Sum` that asks the hint of what it's given:
//! its compile, seeing no caller, makes that iterator knowing its hint for
//! its own impl, which a consumer may give it (#2512).
use std::iter::Sum;

pub struct Tally(pub usize, pub Option<usize>);

impl Sum<u32> for Tally {
    fn sum<I: Iterator<Item = u32>>(items: I) -> Self {
        let (low, high) = items.size_hint();
        Tally(low, high)
    }
}

pub fn total_for_anyone<S: Sum<u32>>(v: Vec<u32>) -> S {
    v.into_iter().skip(1).sum()
}
