//@ library-differs
// An index of an iterator by a range, as itertools' `get` is: its output a
// std adapter of the iterator it's given, `Take<Skip<I>>`, its `I` a
// generic iterator, stepped from both ends where it's given back (#1412).
use std::iter::{Skip, Take};
use std::ops::RangeInclusive;

trait IteratorIndex<I: Iterator> {
    type Output: Iterator<Item = I::Item>;
    fn index(self, from: I) -> Self::Output;
}

impl<I: Iterator> IteratorIndex<I> for RangeInclusive<usize> {
    type Output = Take<Skip<I>>;
    fn index(self, iter: I) -> Self::Output {
        let length = (*self.end() + 1).saturating_sub(*self.start());
        iter.skip(*self.start()).take(length)
    }
}

fn get<I: IntoIterator, R: IteratorIndex<I::IntoIter>>(iter: I, index: R) -> R::Output {
    index.index(iter.into_iter())
}

fn main() {
    let mut it = get(0..5, 1..=3);
    println!("{:?}", it.next());
    println!("{:?}", it.next_back());
    println!("{:?}", it.collect::<Vec<_>>());
}
