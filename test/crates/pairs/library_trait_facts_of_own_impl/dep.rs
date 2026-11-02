//! A library's generic type over its trait's associated type, whose
//! `Default` asks `size_of::<A::Item>()`, as smallvec's `SmallVec<A:
//! Array>` asks it (#476): what its `Array` dictionaries carry, `ItemSize`,
//! a consumer's of its own impls too, which the library's code reads.
use std::mem::size_of;

pub trait Array {
    type Item;

    fn size() -> usize;
}

pub struct Small<A: Array> {
    items: Vec<A::Item>,
    unbounded: bool,
}

impl<A: Array> Small<A> {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// As smallvec's: unbounded, of a zero-sized item.
    pub fn unbounded(&self) -> bool {
        self.unbounded
    }
}

impl<A: Array> Default for Small<A> {
    fn default() -> Self {
        Small {
            items: Vec::new(),
            unbounded: size_of::<A::Item>() == 0,
        }
    }
}
