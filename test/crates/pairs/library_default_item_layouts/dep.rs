//! A trait extending `Iterator` whose defaults ask the layout of
//! `Self::Item`, compiled over `Self` for each consumer impl that doesn't
//! write them: what they read is the consumer's dictionary's (#990).

pub trait Tools: Iterator {
    fn collect_vec(self) -> Vec<Self::Item>
    where
        Self: Sized,
    {
        self.collect()
    }

    fn item_layout(&self) -> (usize, usize) {
        (
            std::mem::size_of::<Self::Item>(),
            std::mem::align_of::<Self::Item>(),
        )
    }
}

impl<T: Iterator> Tools for std::iter::Fuse<T> {}

/// A default that asks its items' layout only by its collect.
pub trait Gather: Iterator {
    fn gather(self) -> Vec<Self::Item>
    where
        Self: Sized,
    {
        self.collect()
    }
}

/// A default that asks its items' size alone, of no collect.
pub trait Measure: Iterator {
    fn item_size(&self) -> usize {
        std::mem::size_of::<Self::Item>()
    }
}
