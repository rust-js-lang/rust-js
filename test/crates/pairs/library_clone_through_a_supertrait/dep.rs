//! A library's impl whose dictionary holds its supertrait's, `Clone` of
//! `trait Dup: Clone`, and one whose default clones `Self`: its consumer,
//! which changes a `Point`, compiles its own of each, as its library's JS
//! clones what its own crate changes, none of `Point` (#1927).
use std::fmt::Debug;

/// What can be duplicated, by its clone.
pub trait Dup: Clone {}

/// What can make another of itself.
pub trait Twice {
    /// Another of it, a clone.
    fn again(&self) -> Self
    where
        Self: Clone,
    {
        self.clone()
    }
}

/// A point its consumer changes.
#[derive(Clone, Copy, Debug)]
pub struct Point {
    pub x: u32,
}

impl Dup for Point {}

impl Twice for Point {}

/// A clone of `value`, changed by `change`, beside `value`.
pub fn duped<T: Dup + Debug>(value: &T, change: impl Fn(&mut T)) -> String {
    let mut copy = value.clone();
    change(&mut copy);
    format!("{value:?} {copy:?}")
}
