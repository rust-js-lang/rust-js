//! A library's trait whose method's own `where` asks more of `Self` than
//! the trait does, as itertools' `Itertools` methods do: whether its JS
//! steps `Self` by `DoubleEndedIterator`, which says whether a caller's
//! type parameter given as `Self` is given that dictionary, no contract
//! says of a trait's method, so a consumer compiles the impl itself (ADR
//! 0914, amended).
pub trait Ends: Iterator {
    /// Its last `n` dropped, from its back, collected.
    fn all_but_last(self, n: usize) -> Vec<Self::Item>
    where
        Self: DoubleEndedIterator + Sized,
    {
        let mut kept: Vec<Self::Item> = self.rev().skip(n).collect();
        kept.reverse();
        kept
    }

    /// Its first and last, stepped from each end.
    fn first_and_last(mut self) -> Option<(Self::Item, Self::Item)>
    where
        Self: DoubleEndedIterator + Sized,
    {
        let first = self.next()?;
        let last = self.next_back()?;
        Some((first, last))
    }
}

impl<T: Iterator + ?Sized> Ends for T {}
