//! A library's thread-local of an array, its module's variable, beside a
//! generic impl for `RefMut<'_, T>`, which makes a `RefCell` of an object a
//! `{ value }` the library unboxes into its variable.

use std::cell::{RefCell, RefMut};

thread_local! {
    pub static LOG: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
}

pub trait Observe {
    fn observe(&self) -> usize;
}

impl<T: ?Sized + Observe> Observe for RefMut<'_, T> {
    fn observe(&self) -> usize {
        (**self).observe()
    }
}

impl Observe for Vec<u32> {
    fn observe(&self) -> usize {
        self.len()
    }
}

pub fn logged() -> usize {
    LOG.with_borrow(|log| log.len())
}
