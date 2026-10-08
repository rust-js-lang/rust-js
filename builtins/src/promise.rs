//! JS's [`Promise`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise)'s
//! methods and statics (ADR 0283), as ReScript's Stdlib and TypeScript's lib
//! have them; `.await` is the way to wait for one (ADR 0029). A rejection's
//! reason is a `JsError`, as `settle` has it.

// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

use super::{JsError, Promise};

impl<T> Promise<T> {
    /// [`promise.then(f)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/then):
    /// the promise `f` makes of what it fulfils, as ReScript's `then`.
    #[cfg_attr(rust_js, rust_js::link_name = "then")]
    pub fn then<U>(self, f: impl FnOnce(T) -> Promise<U> + 'static) -> Promise<U> {
        unreachable!()
    }

    /// [`promise.catch(f)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/catch):
    /// of a rejection, the promise `f` makes of its reason.
    #[cfg_attr(rust_js, rust_js::link_name = "catch")]
    pub fn catch(self, f: impl FnOnce(&'static JsError) -> Promise<T> + 'static) -> Promise<T> {
        unreachable!()
    }

    /// [`promise.finally(f)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/finally):
    /// `f` once it's settled, either way; it, as it settled.
    #[cfg_attr(rust_js, rust_js::link_name = "finally")]
    pub fn finally(self, f: impl FnOnce() + 'static) -> Promise<T> {
        unreachable!()
    }
}

/// [What `allSettled` gives](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/allSettled#return_value)
/// of each promise, told apart by its `status`, TypeScript's
/// `PromiseSettledResult` and ReScript's `settledResult` (ADR 0284).
#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum PromiseSettledResult<T> {
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Fulfilled { value: T },
    #[cfg_attr(rust_js, rust_js::name = "rejected")]
    Rejected { reason: &'static JsError },
}

/// [What `withResolvers` gives](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/withResolvers):
/// a promise and the functions that settle it.
pub struct PromiseWithResolvers<T: 'static> {
    pub promise: Promise<T>,
    pub resolve: &'static dyn Fn(T),
    pub reject: &'static dyn Fn(&JsError),
}

/// [`new Promise(executor)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/Promise):
/// a promise `executor` settles, given the functions that fulfil it and reject it.
#[cfg_attr(rust_js, rust_js::link_name = "new Promise")]
pub fn new<T: 'static>(executor: impl FnOnce(&'static dyn Fn(T), &'static dyn Fn(&JsError)) + 'static) -> Promise<T> {
    unreachable!()
}

/// [`Promise.resolve(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/resolve):
/// a promise fulfilled with `value`.
#[cfg_attr(rust_js, rust_js::link_name = "Promise.resolve")]
pub fn resolve<T>(value: T) -> Promise<T> {
    unreachable!()
}

/// [`Promise.reject(reason)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/reject):
/// a promise rejected with `reason`.
#[cfg_attr(rust_js, rust_js::link_name = "Promise.reject")]
pub fn reject<T>(reason: &JsError) -> Promise<T> {
    unreachable!()
}

/// [`Promise.withResolvers()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/withResolvers):
/// a promise, and the functions that settle it.
#[cfg_attr(rust_js, rust_js::link_name = "Promise.withResolvers")]
pub fn with_resolvers<T: 'static>() -> PromiseWithResolvers<T> {
    unreachable!()
}

/// [`Promise.all(promises)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/all):
/// what each fulfils, in order, or the first rejection.
#[cfg_attr(rust_js, rust_js::link_name = "Promise.all")]
pub fn all<T>(promises: Vec<Promise<T>>) -> Promise<Vec<T>> {
    unreachable!()
}

/// `Promise.all([a, b])`: what each fulfils, a tuple, as ReScript's `all2`.
#[cfg_attr(rust_js, rust_js::link_name = "Promise.all")]
pub fn all2<A, B>(promises: (Promise<A>, Promise<B>)) -> Promise<(A, B)> {
    unreachable!()
}

/// `Promise.all([a, b, c])`, as ReScript's `all3`.
#[cfg_attr(rust_js, rust_js::link_name = "Promise.all")]
pub fn all3<A, B, C>(promises: (Promise<A>, Promise<B>, Promise<C>)) -> Promise<(A, B, C)> {
    unreachable!()
}

/// `Promise.all([a, b, c, d])`, as ReScript's `all4`.
#[cfg_attr(rust_js, rust_js::link_name = "Promise.all")]
pub fn all4<A, B, C, D>(promises: (Promise<A>, Promise<B>, Promise<C>, Promise<D>)) -> Promise<(A, B, C, D)> {
    unreachable!()
}

/// [`Promise.allSettled(promises)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/allSettled):
/// how each settled, in order, once each has.
#[cfg_attr(rust_js, rust_js::link_name = "Promise.allSettled")]
pub fn all_settled<T>(promises: Vec<Promise<T>>) -> Promise<Vec<PromiseSettledResult<T>>> {
    unreachable!()
}

/// [`Promise.race(promises)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/race):
/// as the first to settle settles.
#[cfg_attr(rust_js, rust_js::link_name = "Promise.race")]
pub fn race<T>(promises: Vec<Promise<T>>) -> Promise<T> {
    unreachable!()
}

/// [`Promise.any(promises)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/any):
/// the first fulfilment, or of none an `AggregateError` of each rejection.
#[cfg_attr(rust_js, rust_js::link_name = "Promise.any")]
pub fn any<T>(promises: Vec<Promise<T>>) -> Promise<T> {
    unreachable!()
}
