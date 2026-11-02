//! JS's iterator protocol, TypeScript's `Iterator<T>` and
//! `IteratorResult<T>`: what an iterable's `[Symbol.iterator]()` gives,
//! stepped by its `next()` (ADR 0860); and its async one,
//! `AsyncIterator<T>`, whose `next()` gives a promise of each step
//! (ADR 1059).

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

/// [An async iterator](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Iteration_protocols#the_async_iterator_and_async_iterable_protocols),
/// TypeScript's `AsyncIterator<T>`: an object whose `next()` gives a
/// promise of each of its steps, as an async iterable's
/// `[Symbol.asyncIterator]()` makes one, an async generator's too. Only the
/// protocol, as [`Iterator`] is: React's Flight reply client's
/// `iterator.next().then(progress, reject)`.
#[cfg_attr(rust_js, rust_js::types = "AsyncIterator<T>")]
pub struct AsyncIterator<T>(PhantomData<JsObject>, PhantomData<T>);

impl<T> AsyncIterator<T> {
    /// [`iterator.next()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Iteration_protocols#next_2):
    /// a promise of its next step.
    #[cfg_attr(rust_js, rust_js::link_name = "next")]
    pub fn next(&self) -> crate::Promise<IteratorResult<T>> {
        unreachable!()
    }

    /// [`iterator.throw(e)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Iteration_protocols#throwexception):
    /// a promise of the step it takes once `e` is thrown where it waits,
    /// any value, or none, TypeScript's `throw?(e?: any)`. An iterator
    /// needn't have one, as TypeScript's `?` says: `js::is_function((iterator,
    /// AsyncIterator::throw))` asks, as React's Flight reply server's `typeof
    /// iterator.throw === 'function'` does, before `iterator.throw(reason)`
    /// (rd584).
    #[cfg_attr(rust_js, rust_js::link_name = "throw")]
    pub fn throw(&self, e: Option<&dyn crate::IntoUnknown>) -> crate::Promise<IteratorResult<T>> {
        unreachable!()
    }
}

/// [An async iterable](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Iteration_protocols#the_async_iterator_and_async_iterable_protocols),
/// TypeScript's `AsyncIterable<T>`: an object whose `[Symbol.asyncIterator]()`
/// makes an [`AsyncIterator`] of its items, as an async generator, a
/// `ReadableStream` and any `{ [Symbol.asyncIterator]() { .. } }` do. A JS
/// object's type, as [`AsyncIterator`] is, where [`Iterable`] is a trait:
/// Rust's own collections are given where an iterable is, and nothing of
/// Rust's is async iterable.
#[cfg_attr(rust_js, rust_js::types = "AsyncIterable<T>")]
pub struct AsyncIterable<T>(PhantomData<JsObject>, PhantomData<T>);

impl<T> AsyncIterable<T> {
    /// [`iterable[Symbol.asyncIterator]()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/asyncIterator):
    /// an iterator of its items, as `for await` steps it: React's Flight
    /// reply server's `iterable[ASYNC_ITERATOR]()` (rd585).
    #[cfg_attr(rust_js, rust_js::link_name = "[Symbol.asyncIterator]")]
    pub fn async_iterator(&self) -> &'static AsyncIterator<T> {
        unreachable!()
    }
}

/// What an [`Iterator`]'s `next()` gives, an [`AsyncIterator`]'s promise, TypeScript's `IteratorResult<T>`:
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
/// or a `Box<dyn Iterator>` JS steps itself (the user's decision, bw8); a
/// value of any shape, a typed array and a string, as TypeScript's are
/// (ADR 1052).
#[cfg_attr(rust_js, rust_js::types = "Iterable<T>")]
pub trait Iterable<T> {}

impl<T, const N: usize> Iterable<T> for [T; N] {}
impl<T> Iterable<T> for &[T] {}
impl<T> Iterable<T> for Vec<T> {}
impl<T> Iterable<T> for Box<dyn core::iter::Iterator<Item = T> + '_> {}
impl<T: ?Sized> Iterable<&'static T> for crate::Set<T> {}
impl<K: ?Sized, V> Iterable<(&'static K, V)> for crate::Map<K, V> {}
/// A value of any shape, TypeScript's `any`, each item of which is any
/// value, `undefined` too (ADR 1052).
impl Iterable<Option<&'static crate::Unknown>> for crate::Unknown {}
/// A string's characters, each a string of its own, as JS steps it: by
/// code point, `"a😀"` two (ADR 1052).
impl Iterable<String> for &str {}
