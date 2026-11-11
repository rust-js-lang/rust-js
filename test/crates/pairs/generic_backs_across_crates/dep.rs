//! A library's generic code that steps what it's given from its back,
//! compiled by the app that gives it its iterators (ADR 0914): `rev()`,
//! `next_back()`, `nth_back()`, `rfold`, and adapters it makes of them.
//! Decision coverage found what's made knowing its back given only in a
//! crate's own bodies, never in another's (#1376's second pass).
pub fn last_two<I: DoubleEndedIterator>(it: I) -> Vec<I::Item> {
    it.rev().take(2).collect()
}

pub fn ends<I: DoubleEndedIterator>(mut it: I) -> (Option<I::Item>, Option<I::Item>) {
    let first = it.next();
    (first, it.next_back())
}

pub fn back_digits<I: DoubleEndedIterator<Item = u32>>(it: I) -> u32 {
    it.rfold(0, |acc, x| acc * 10 + x)
}

pub fn nth_from_back<I: DoubleEndedIterator>(mut it: I, n: usize) -> Option<I::Item> {
    it.nth_back(n)
}

pub fn mapped_back<I: DoubleEndedIterator<Item = i32>>(it: I) -> Vec<i32> {
    it.map(|x| x * 2).filter(|x| x % 3 != 0).rev().collect()
}

pub fn pairs_back<I: DoubleEndedIterator + ExactSizeIterator>(it: I) -> Vec<(usize, I::Item)> {
    it.enumerate().rev().collect()
}
