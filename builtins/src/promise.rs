//! JS's [`Promise`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise)'s
//! methods and statics (ADR 0283), as ReScript's Stdlib and TypeScript's lib
//! have them; `.await` is the way to wait for one (ADR 0029). A rejection's
//! reason a handler reads is a `JsError`, as `settle` has it; what rejects
//! one takes any value, as TypeScript's `reason?: any` (ADR 0782).

// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

use super::{IntoUnknown, JsError, Promise, Unknown};

impl<T> Promise<T> {
    /// TypeScript's `Promise<any>` of it: the promise itself, what it
    /// fulfils with any value, so promises of different values make one
    /// list, `Promise.all([document.fonts.ready, image])` (rd211).
    #[cfg_attr(rust_js, rust_js::link_name = "this")]
    pub fn into_unknown(self) -> Promise<Option<&'static Unknown>> {
        unreachable!()
    }

    /// [`promise.then(f)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/then):
    /// what `f` gives of what it fulfils, a value or a promise of one
    /// ([`Fulfils`]), TypeScript's `TResult | PromiseLike<TResult>`, as
    /// `catch`'s is: where rustc can't tell which, the promise it makes says,
    /// `let p: Promise<String> = ..` (ADR 0782, amended).
    #[cfg_attr(rust_js, rust_js::link_name = "then")]
    pub fn then<U, F: Fulfils<U>>(self, f: impl FnOnce(T) -> F + 'static) -> Promise<U> {
        unreachable!()
    }

    /// [`promise.then(on_fulfilled, on_rejected)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/then):
    /// TypeScript's two handlers, what `on_fulfilled` gives of what it
    /// fulfils, or `on_rejected` of a rejection's reason, each a value or a
    /// promise of one ([`Fulfils`]), as `catch`'s is (ADR 0782).
    #[cfg_attr(rust_js, rust_js::link_name = "then")]
    pub fn then_with<U, F: Fulfils<U>, R: Fulfils<U>>(
        self,
        on_fulfilled: impl FnOnce(T) -> F + 'static,
        on_rejected: impl FnOnce(&'static JsError) -> R + 'static,
    ) -> Promise<U> {
        unreachable!()
    }

    /// [`promise.catch(f)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/catch):
    /// of a rejection, what `f` gives of its reason, a value or a promise
    /// of one ([`Fulfils`]), TypeScript's `TResult | PromiseLike<TResult>`:
    /// `promise.catch(|_| {})` is `promise.catch(() => {})` (ADR 0698).
    #[cfg_attr(rust_js, rust_js::link_name = "catch")]
    pub fn catch<R: Fulfils<T>>(self, f: impl FnOnce(&'static JsError) -> R + 'static) -> Promise<T> {
        unreachable!()
    }

    /// [`promise.finally(f)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/finally):
    /// `f` once it's settled, either way; it, as it settled.
    #[cfg_attr(rust_js, rust_js::link_name = "finally")]
    pub fn finally(self, f: impl FnOnce() + 'static) -> Promise<T> {
        unreachable!()
    }
}

/// What a handler gives that fulfils a promise of `T`, TypeScript's
/// `TResult | PromiseLike<TResult>`: a `T`, or a promise of one, which JS
/// waits for; of an `Option`'s, nothing too, `()`, which is `undefined`,
/// its `None` (ADR 0698).
#[diagnostic::on_unimplemented(
    message = "`{Self}` doesn't fulfil a promise of `{T}`",
    note = "a handler gives a `{T}`, or a `Promise<{T}>`; of an `Option`'s, nothing"
)]
pub trait Fulfils<T> {}

impl<T> Fulfils<T> for T {}
impl<T> Fulfils<T> for Promise<T> {}
impl<T> Fulfils<Option<T>> for () {}

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
/// a promise and the functions that settle it, `reject` of any value, as
/// a JS function's `any` is (ADR 0653).
pub struct PromiseWithResolvers<T: 'static> {
    pub promise: Promise<T>,
    pub resolve: &'static dyn Fn(T),
    pub reject: &'static dyn Fn(Option<&Unknown>),
}

/// [`new Promise(executor)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/Promise):
/// a promise `executor` settles, given the functions that fulfil it and reject it,
/// `reject` of any value, TypeScript's `reason?: any`, as a JS function's
/// `any` is, `reject(Some(js::unknown(error)))` of a `&JsError` (ADR 0653).
/// JS runs it before `new Promise` returns, so it may borrow what's around
/// it, and set it, `resolve_debug_value = Some(resolve)` (ADR 0782).
#[cfg_attr(rust_js, rust_js::link_name = "new Promise")]
pub fn new<T: 'static>(executor: impl FnOnce(&'static dyn Fn(T), &'static dyn Fn(Option<&Unknown>))) -> Promise<T> {
    unreachable!()
}

/// [`Promise.resolve(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/resolve):
/// a promise fulfilled with `value`.
#[cfg_attr(rust_js, rust_js::link_name = "Promise.resolve")]
pub fn resolve<T>(value: T) -> Promise<T> {
    unreachable!()
}

/// [`Promise.reject(reason)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Promise/reject):
/// a promise rejected with `reason`, any value, TypeScript's `reason?:
/// any`, left out or not: `reject(Some(&error))`.
#[cfg_attr(rust_js, rust_js::link_name = "Promise.reject")]
pub fn reject<T>(reason: Option<&dyn IntoUnknown>) -> Promise<T> {
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
