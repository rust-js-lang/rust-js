//! JS's iterator protocol, TypeScript's `Iterator<T>` and
//! `IteratorResult<T>`: what an iterable's `[Symbol.iterator]()` gives,
//! stepped by its `next()` (ADR 0860).

use core::marker::PhantomData;

use crate::JsObject;

/// [An iterator](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Iteration_protocols#the_iterator_protocol),
/// TypeScript's `Iterator<T>`: an object whose `next()` gives each of its
/// steps, as an iterable's `[Symbol.iterator]()` makes one. Only the
/// protocol: one of JS's own is iterable too, but one a program makes,
/// `{ next() { .. } }`, needn't be, so it's stepped by `next()`, not
/// `for..of`.
#[cfg_attr(rust_js, rust_js::types = "Iterator<T>")]
pub struct Iterator<T>(PhantomData<JsObject>, PhantomData<T>);

impl<T> Iterator<T> {
    /// [`iterator.next()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Iteration_protocols#next):
    /// its next step.
    #[cfg_attr(rust_js, rust_js::link_name = "next")]
    pub fn next(&self) -> IteratorResult<T> {
        unreachable!()
    }
}

/// What an [`Iterator`]'s `next()` gives, TypeScript's `IteratorResult<T>`:
/// a step's `value`, or, `done`, the end. `done` is TypeScript's `done?:
/// boolean`, left out of a step that isn't the end, `IteratorYieldResult`'s
/// `done?: false`, which `step.done == Some(true)` reads as JS's protocol
/// does, `if (step.done)`. The end's `value` is its return value, any
/// value, TypeScript's `TReturn`: read a step's only before its end.
#[cfg_attr(rust_js, rust_js::types = "IteratorResult<T>")]
pub struct IteratorResult<T> {
    pub done: Option<bool>,
    pub value: T,
}

/// [An iterable](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Iteration_protocols#the_iterable_protocol)
/// of `T`, TypeScript's `Iterable<T>`: what a JS constructor that takes one,
/// `new Set(values)`, `new Map(entries)`, steps, given as it is: a
/// [`Set`](crate::Set) or a [`Map`](crate::Map) of JS's, an array, a slice, a `Vec`,
/// or a `Box<dyn Iterator>` JS steps itself (the user's decision, bw8).
#[cfg_attr(rust_js, rust_js::types = "Iterable<T>")]
pub trait Iterable<T> {}

impl<T, const N: usize> Iterable<T> for [T; N] {}
impl<T> Iterable<T> for &[T] {}
impl<T> Iterable<T> for Vec<T> {}
impl<T> Iterable<T> for Box<dyn core::iter::Iterator<Item = T> + '_> {}
impl<T: ?Sized> Iterable<&'static T> for crate::Set<T> {}
impl<K: ?Sized, V> Iterable<(&'static K, V)> for crate::Map<K, V> {}
