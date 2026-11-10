// A range given to generic code is advanced by its bounds, as std's
// `Range::advance_by` is, where a chain's `advance_by` of it asks it, a
// `skip`'s, and a `rev()`'s, its `advance_back_by` (#1240): so a skip of
// four billion of it ends at once, as natively, not after stepping each.
fn skipped<I: Iterator>(it: I, n: usize) -> Option<I::Item> {
    it.skip(n).next()
}

fn main() {
    println!("{:?}", skipped((0..u32::MAX).chain(std::iter::empty()), 4_000_000_000));
    println!("{:?}", skipped((0..u32::MAX).rev().chain(std::iter::empty()), 4_000_000_000));
    println!("{:?}", skipped((0..10u32).chain(20..30), 15));
    println!("{:?}", skipped((0..10u32).rev().chain((20..30).rev()), 15));
    println!("{:?}", skipped((0..=10u32).chain(std::iter::once(7)), 11));
    println!("{:?}", skipped((0..3u64).chain(5..7), 9));
}
