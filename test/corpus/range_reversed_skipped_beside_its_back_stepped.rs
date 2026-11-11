//@ instances-keep-cost: it skips four billion numbers at once, which a `map` or a `filter` steps one by one
// A reversed range skipped, `(0..u64::MAX).rev().skip(4_000_000_000)`, beside a
// function that steps a reversed range of the same type from its back,
// which makes that type's `rev()` one made knowing its back, `$rev` of the
// range: its `skip` asks the `rev()`'s `nth`, std's `Rev::nth`, the
// range's `nth_back`, which moves its end, as std's does, so it doesn't
// step each of the 4e9 numbers it skips, 15 s of each, where Rust's is
// instant (#950). A `rev()` given to generic code that steps it from its
// back is such a `$rev` too, its `nth` and its `skip` the range's
// `nth_back` (#1109). And far from either end: a range made knowing its
// back given to generic code's `nth_back`, the range's own `nth_back`, and
// the `nth_back` of its `rev()`, the range's `nth`, each moving a bound,
// not stepping each number (#1109).
fn second_from_its_front(range: std::ops::Range<u64>) -> Option<u64> {
    range.rev().nth_back(1)
}

fn back_then_far<I: DoubleEndedIterator<Item = u64>>(mut it: I) -> (Option<u64>, Option<u64>) {
    (it.next_back(), it.nth(4_000_000_000))
}

fn back_then_skipped<I: DoubleEndedIterator<Item = u64>>(mut it: I) -> (Option<u64>, Option<u64>) {
    (it.next_back(), it.skip(4_000_000_000).next())
}

fn main() {
    println!("{:?}", (0..u64::MAX).rev().skip(4_000_000_000).next());
    println!("{:?}", (0..u32::MAX).rev().skip(4_000_000_000).next());
    println!("{:?}", second_from_its_front(3..9));
    println!("{:?}", far_from_its_front(0..u64::MAX));
    println!("{:?}", back_then_far((0..u64::MAX).rev()));
    println!("{:?}", back_then_skipped((0..u64::MAX).rev()));
    println!("{:?}", front_then_far_from_its_back(0..u64::MAX));
}

fn far_from_its_front(range: std::ops::Range<u64>) -> Option<u64> {
    range.rev().nth_back(4_000_000_000)
}

fn front_then_far_from_its_back<I: DoubleEndedIterator<Item = u64>>(mut it: I) -> (Option<u64>, Option<u64>) {
    (it.next(), it.nth_back(4_000_000_000))
}
