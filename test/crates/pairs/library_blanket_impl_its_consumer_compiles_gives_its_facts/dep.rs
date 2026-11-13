//! A library's trait whose defaults a where clause bounds, `Self::Item:
//! IntoIterator`, each asking facts of its items' iterator, and its blanket
//! impl, which a consumer that reads a `Vec`'s capacity compiles itself
//! (ADR 0914, amended; #1613).
pub trait Ext: Iterator {
    fn flat(self) -> Vec<<Self::Item as IntoIterator>::Item>
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        self.flat_map(|x| x).collect()
    }

    fn hint(self) -> (usize, Option<usize>)
    where
        Self: Sized,
        Self::Item: IntoIterator,
    {
        self.flat_map(|x| x).size_hint()
    }
}

impl<T: Iterator + ?Sized> Ext for T {}
