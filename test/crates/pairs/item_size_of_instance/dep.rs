//! A generic type over a trait's associated type, as smallvec's
//! `SmallVec<A: Array>` is of `A::Item`, whose code asks
//! `size_of::<A::Item>()` (#476).
use std::mem::size_of;

pub trait Array {
    type Item;

    fn size() -> usize;
}

impl<T, const N: usize> Array for [T; N] {
    type Item = T;

    fn size() -> usize {
        N
    }
}

pub struct Small<A: Array> {
    items: Vec<A::Item>,
}

impl<A: Array> Small<A> {
    pub fn new() -> Self {
        Small { items: Vec::new() }
    }

    /// As smallvec's: unbounded of a zero-sized item.
    fn inline_capacity() -> usize {
        if size_of::<A::Item>() > 0 { A::size() } else { usize::MAX }
    }

    pub fn push(&mut self, item: A::Item) {
        if self.items.len() == Self::inline_capacity() {
            println!("spilling");
        }
        self.items.push(item);
    }

    pub fn spilled(&self) -> bool {
        self.items.len() > Self::inline_capacity()
    }

    /// Drops what's past `len`, the drop a consumer's item gives.
    pub fn truncate(&mut self, len: usize) -> bool {
        self.items.truncate(len);
        self.spilled()
    }
}

impl<A: Array> Default for Small<A> {
    fn default() -> Self {
        Self::new()
    }
}
