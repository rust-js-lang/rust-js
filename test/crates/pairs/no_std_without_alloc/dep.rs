//! A `#![no_std]` library that loads only `core`, as powerfmt does: it
//! has no `alloc`, so no B-tree, which the facts of what a call makes
//! asked for (#983).
#![no_std]

/// A generic `into()` of `core`'s `From<T>` for `Option<T>`.
pub fn some<T>(value: T) -> Option<T> {
    value.into()
}
