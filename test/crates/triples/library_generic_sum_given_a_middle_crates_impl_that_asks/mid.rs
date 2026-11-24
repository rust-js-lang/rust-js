//@ uses:
//! Another library's `Sum` that asks the hint of what it's given, which
//! the first library's compile can't see; `dep`'s sibling, which it doesn't
//! use.
use std::iter::Sum;

pub struct Room(pub usize, pub Option<usize>);

impl Sum<u32> for Room {
    fn sum<I: Iterator<Item = u32>>(items: I) -> Self {
        let (low, high) = items.size_hint();
        Room(low, high)
    }
}
