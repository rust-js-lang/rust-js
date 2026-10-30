//@ compile-fail: a `cycle()` of what's partly filtered
// `collect()` into a `Vec` of a `cycle()` of an array's items chained with
// what's filtered: its hint has no end where the array has items, as its
// lower bound counts them, and a lower bound of 0 where only the filter
// gives any, which only what it holds as it runs says (ADR 0940, amended).
use std::panic::{self, AssertUnwindSafe};

fn main() {
    let items = [1u8, 2];
    let cycled = items
        .iter()
        .copied()
        .chain(items.iter().copied().filter(|&i| i > 1))
        .cycle();
    let collected = panic::catch_unwind(AssertUnwindSafe(move || cycled.collect::<Vec<u8>>()));
    println!("{}", collected.is_err());
}
