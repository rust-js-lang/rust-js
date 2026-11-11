//! A library's extension trait every iterator has, by a blanket impl, as
//! itertools' `impl<T: Iterator + ?Sized> Itertools for T` is: one default
//! steps what it's given, `counted`'s `self.count()`, another doesn't,
//! `first_of`'s, which only hands it on (#1409).
pub trait Tools: Iterator {
    /// How many it has, stepping it.
    fn counted(self) -> usize
    where
        Self: Sized,
    {
        self.count()
    }

    /// What `pick` makes of it, handing it on unstepped.
    fn first_of<R>(self, pick: impl FnOnce(Self) -> R) -> R
    where
        Self: Sized,
    {
        pick(self)
    }
}

impl<T: Iterator + ?Sized> Tools for T {}
