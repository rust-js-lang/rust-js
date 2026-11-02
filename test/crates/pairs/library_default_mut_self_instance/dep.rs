//! A trait's provided method that takes `&mut self` and calls another of
//! `Self`'s, as arrayvec's `ArrayVecImpl::try_push` calls its `len` and
//! `as_mut_ptr`, called by an inherent method of a generic type (#472).
trait Bounded {
    type Item;

    fn items(&mut self) -> &mut Vec<Self::Item>;

    fn try_push(&mut self, item: Self::Item) -> Result<(), Self::Item> {
        if self.items().len() < 2 {
            self.items().push(item);
            Ok(())
        } else {
            Err(item)
        }
    }
}

pub struct Stack<T> {
    items: Vec<T>,
}

impl<T> Bounded for Stack<T> {
    type Item = T;

    fn items(&mut self) -> &mut Vec<T> {
        &mut self.items
    }
}

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack { items: Vec::new() }
    }

    pub fn try_push(&mut self, item: T) -> Result<(), T> {
        Bounded::try_push(self, item)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self::new()
    }
}

fn add_one(n: &mut u32) {
    *n += 1;
}

/// A `&mut u32`, a box, given on to a private function, `add_one(copy _1)`.
pub fn bump<T>(n: &mut u32, value: T) {
    add_one(n);
    add_one(n);
    drop(value);
}
