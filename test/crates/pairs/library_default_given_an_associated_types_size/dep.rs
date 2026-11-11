//! A library's default of an extension trait every iterator has, as
//! itertools' `get` is: `IteratorIndex::index`, a sealed trait's, gives a
//! `Skip<Take<I>>` or a `Take<I>` of what it's given (#1401).
use std::iter::{Skip, Take};
use std::ops::{Range, RangeTo};

pub trait IteratorIndex<I: Iterator> {
    type Output: Iterator<Item = I::Item>;
    fn index(self, from: I) -> Self::Output;
}

impl<I: Iterator> IteratorIndex<I> for Range<usize> {
    type Output = Skip<Take<I>>;
    fn index(self, iter: I) -> Self::Output {
        iter.take(self.end).skip(self.start)
    }
}

impl<I: Iterator> IteratorIndex<I> for RangeTo<usize> {
    type Output = Take<I>;
    fn index(self, iter: I) -> Self::Output {
        iter.take(self.end)
    }
}

pub trait Tools: Iterator {
    fn get<R>(self, index: R) -> R::Output
    where
        Self: Sized,
        R: IteratorIndex<Self>,
    {
        index.index(self)
    }

    /// The size of its items, which its impl's dictionary is given.
    fn item_size(&self) -> usize {
        std::mem::size_of::<Self::Item>()
    }
}

impl<T: Iterator> Tools for T {}
