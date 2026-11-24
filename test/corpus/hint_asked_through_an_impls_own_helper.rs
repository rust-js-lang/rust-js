//@ library-differs
// Generic code's `sum()` and `collect()` of an iterator it makes, into the
// crate's own impl, which gives what it's given on to its own function that
// asks `size_hint()`: by reference, by value, through `IntoIterator`, and
// two helpers down. Std's `Skip` and `StepBy` answer theirs, so what generic
// code makes is made knowing its hint, as where the impl asks itself
// (#2521). Each adapter is one no other call here makes, so none is marked
// for another's sake.
use std::iter::Sum;

fn by_ref<I: Iterator>(it: &I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn by_value<I: Iterator>(it: I) -> (usize, Option<usize>) {
    it.size_hint()
}

fn into_iter<I: IntoIterator<Item = u32>>(items: I) -> (usize, Option<usize>) {
    items.into_iter().size_hint()
}

fn passed_on<I: Iterator>(it: I) -> (usize, Option<usize>) {
    by_value(it)
}

struct ByRef(usize, Option<usize>);

impl Sum<u32> for ByRef {
    fn sum<I: Iterator<Item = u32>>(items: I) -> Self {
        let (low, high) = by_ref(&items);
        ByRef(low, high)
    }
}

struct ByValue(usize, Option<usize>);

impl FromIterator<u32> for ByValue {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let (low, high) = by_value(items.into_iter());
        ByValue(low, high)
    }
}

struct Through(usize, Option<usize>);

impl FromIterator<u32> for Through {
    fn from_iter<I: IntoIterator<Item = u32>>(items: I) -> Self {
        let (low, high) = into_iter(items);
        Through(low, high)
    }
}

struct TwoDown(usize, Option<usize>);

impl Sum<u32> for TwoDown {
    fn sum<I: Iterator<Item = u32>>(items: I) -> Self {
        let (low, high) = passed_on(items);
        TwoDown(low, high)
    }
}

fn summed<S: Sum<u32>>(v: &[u32]) -> S {
    v.iter().copied().skip(1).sum()
}

fn gathered<B: FromIterator<u32>>(v: Vec<u32>) -> B {
    v.into_iter().step_by(2).collect()
}

fn stepped_sum<S: Sum<u32>>(v: Vec<u32>) -> S {
    v.into_iter().skip(2).sum()
}

fn gathered_skipping<B: FromIterator<u32>>(v: &[u32]) -> B {
    v.iter().copied().step_by(3).collect()
}

fn main() {
    let ByRef(low, high) = summed(&[1, 2, 3]);
    println!("by ref {low} {high:?}");
    let ByValue(low, high) = gathered(vec![1, 2, 3, 4, 5]);
    println!("by value {low} {high:?}");
    let Through(low, high) = gathered_skipping(&[1, 2, 3, 4, 5, 6, 7]);
    println!("through into_iter {low} {high:?}");
    let TwoDown(low, high) = stepped_sum(vec![1, 2, 3, 4]);
    println!("two down {low} {high:?}");
}
