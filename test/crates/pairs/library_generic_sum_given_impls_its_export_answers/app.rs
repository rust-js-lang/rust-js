//! The library's generic `sum()` given impls its export answers as Rust
//! does: std's `u32`, the consumer's own that asks nothing of what it's
//! given, and the library's own that asks its hint. Each is the library's
//! export, which the consumer imports (#2518).
use dep::Tally;
use std::iter::Sum;

struct Count(usize);

impl Sum<u32> for Count {
    fn sum<I: Iterator<Item = u32>>(items: I) -> Self {
        Count(items.count())
    }
}

pub fn main() {
    let total: u32 = dep::total_for_anyone(vec![4, 5, 6]);
    println!("std {total}");
    let Count(count) = dep::total_for_anyone(vec![4, 5, 6]);
    println!("count {count}");
    let Tally(low, high) = dep::total_for_anyone(vec![4, 5, 6]);
    println!("tally {low} {high:?}");
}
