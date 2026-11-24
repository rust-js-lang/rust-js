//! A generic function given an array of std's filters of a double-ended
//! source, which it steps from their back where its consumer can't see: what
//! the consumer gives it is made knowing its back where it's made, as each
//! filter its signature names is (#1024; #1979's `library-given-backs-unmade`).
use std::iter::Filter;

pub fn backs<I, P>(all: [Filter<I, P>; 2]) -> Vec<Option<u32>>
where
    I: DoubleEndedIterator<Item = u32>,
    P: FnMut(&u32) -> bool,
{
    all.into_iter().map(|mut it| it.next_back()).collect()
}
