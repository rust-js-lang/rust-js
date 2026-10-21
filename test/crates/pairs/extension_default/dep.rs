// A crate's extension of `Iterator` whose default gives `self` on to a
// function that holds it, as itertools' `intersperse` does: a consumer
// calls the library's default, given `Self`'s `Iterator` dictionary apart,
// which its own trait's dictionary has no entry of (ADR 0548).
pub struct Every<I> {
    iter: I,
    step: usize,
}

impl<I: Iterator> Iterator for Every<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let item = self.iter.next()?;
        for _ in 1..self.step {
            self.iter.next();
        }
        Some(item)
    }
}

pub fn every<I: Iterator>(iter: I, step: usize) -> Every<I> {
    Every { iter, step }
}

// Given as it is, as it's given back: stepped by its `Iterator` dictionary.
pub fn skip_one<I: Iterator>(mut iter: I) -> I {
    iter.next();
    iter
}

pub trait Ext: Iterator {
    fn skipped(self) -> Self
    where
        Self: Sized,
    {
        skip_one(self)
    }

    fn every(self, step: usize) -> Every<Self>
    where
        Self: Sized,
    {
        every(self, step)
    }
}

impl<T: Iterator + ?Sized> Ext for T {}
