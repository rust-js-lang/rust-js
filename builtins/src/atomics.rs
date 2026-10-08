//! JS's [`Atomics`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics):
//! operations on an integer typed array's elements that no other thread
//! sees half done (ADR 0283), as TypeScript's `lib.es2017.sharedmemory.d.ts`
//! has them; ReScript has none. Each takes the array and an element's index,
//! and gives the element as it was.

// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

use super::{BigInt64Array, BigUint64Array, Int8Array, Int16Array, Int32Array, Promise, Uint8Array, Uint16Array, Uint32Array};

/// A typed array of integers, as TypeScript's `Atomics` takes, its
/// `Element` the Rust number each is: not a `Uint8ClampedArray`, nor one of
/// floats, which JS throws a `TypeError` of.
///
/// # Safety
///
/// It must be one of JS's integer typed arrays, of `Element`s.
pub unsafe trait AtomicArray {
    type Element;
}

unsafe impl AtomicArray for Int8Array {
    type Element = i8;
}
unsafe impl AtomicArray for Uint8Array {
    type Element = u8;
}
unsafe impl AtomicArray for Int16Array {
    type Element = i16;
}
unsafe impl AtomicArray for Uint16Array {
    type Element = u16;
}
unsafe impl AtomicArray for Int32Array {
    type Element = i32;
}
unsafe impl AtomicArray for Uint32Array {
    type Element = u32;
}
unsafe impl AtomicArray for BigInt64Array {
    type Element = i64;
}
unsafe impl AtomicArray for BigUint64Array {
    type Element = u64;
}

/// One a thread may wait on an element of: an `Int32Array` or a
/// `BigInt64Array`, of a `SharedArrayBuffer`.
///
/// # Safety
///
/// It must be one of these.
pub unsafe trait WaitableArray: AtomicArray {}

unsafe impl WaitableArray for Int32Array {}
unsafe impl WaitableArray for BigInt64Array {}

/// How a wait ended.
pub enum WaitResult {
    /// Woken by `notify`.
    #[cfg_attr(rust_js, rust_js::name = "ok")]
    Ok,
    /// The element wasn't the value waited on.
    #[cfg_attr(rust_js, rust_js::name = "not-equal")]
    NotEqual,
    #[cfg_attr(rust_js, rust_js::name = "timed-out")]
    TimedOut,
}

/// What `wait_async` gives: whether it's waiting, and how it ended or the
/// promise of it.
pub struct WaitAsyncResult {
    pub r#async: bool,
    pub value: WaitAsyncValue,
}

/// How a `wait_async` ended, a string, or the promise of it, as its
/// `async` says.
#[cfg_attr(rust_js, rust_js::untagged)]
pub enum WaitAsyncValue {
    Waiting(Promise<WaitResult>),
    #[cfg_attr(rust_js, rust_js::otherwise)]
    Ended(WaitResult),
}

/// [`Atomics.add(a, i, x)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics/add):
/// `a[i] += x`.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.add")]
pub fn add<A: AtomicArray + ?Sized>(array: &A, index: u32, value: A::Element) -> A::Element {
    unreachable!()
}

/// [`Atomics.and(a, i, x)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics/and):
/// `a[i] &= x`.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.and")]
pub fn and<A: AtomicArray + ?Sized>(array: &A, index: u32, value: A::Element) -> A::Element {
    unreachable!()
}

/// [`Atomics.compareExchange(a, i, expected, x)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics/compareExchange):
/// `a[i] = x` where it's `expected`.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.compareExchange")]
pub fn compare_exchange<A: AtomicArray + ?Sized>(array: &A, index: u32, expected: A::Element, replacement: A::Element) -> A::Element {
    unreachable!()
}

/// [`Atomics.exchange(a, i, x)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics/exchange):
/// `a[i] = x`.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.exchange")]
pub fn exchange<A: AtomicArray + ?Sized>(array: &A, index: u32, value: A::Element) -> A::Element {
    unreachable!()
}

/// [`Atomics.isLockFree(size)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics/isLockFree):
/// whether elements of `size` bytes are worked on without a lock.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.isLockFree")]
pub fn is_lock_free(size: u32) -> bool {
    unreachable!()
}

/// [`Atomics.load(a, i)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics/load):
/// `a[i]`.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.load")]
pub fn load<A: AtomicArray + ?Sized>(array: &A, index: u32) -> A::Element {
    unreachable!()
}

/// [`Atomics.notify(a, i, count)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics/notify):
/// wakes up to `count` of the threads waiting on `a[i]`; how many it woke.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.notify")]
pub fn notify<A: WaitableArray + ?Sized>(array: &A, index: u32, count: u32) -> u32 {
    unreachable!()
}

/// `Atomics.notify(a, i)`: wakes every thread waiting on `a[i]`.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.notify")]
pub fn notify_all<A: WaitableArray + ?Sized>(array: &A, index: u32) -> u32 {
    unreachable!()
}

/// [`Atomics.or(a, i, x)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics/or):
/// `a[i] |= x`.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.or")]
pub fn or<A: AtomicArray + ?Sized>(array: &A, index: u32, value: A::Element) -> A::Element {
    unreachable!()
}

/// [`Atomics.store(a, i, x)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics/store):
/// `a[i] = x`; `x`.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.store")]
pub fn store<A: AtomicArray + ?Sized>(array: &A, index: u32, value: A::Element) -> A::Element {
    unreachable!()
}

/// [`Atomics.sub(a, i, x)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics/sub):
/// `a[i] -= x`.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.sub")]
pub fn sub<A: AtomicArray + ?Sized>(array: &A, index: u32, value: A::Element) -> A::Element {
    unreachable!()
}

/// [`Atomics.wait(a, i, x)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics/wait):
/// sleeps while `a[i]` is `x`, until notified. A browser's main thread
/// may not, which JS throws a `TypeError` of.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.wait")]
pub fn wait<A: WaitableArray + ?Sized>(array: &A, index: u32, value: A::Element) -> WaitResult {
    unreachable!()
}

/// `Atomics.wait(a, i, x, timeout)`: sleeps at most `timeout`
/// milliseconds.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.wait")]
pub fn wait_with_timeout<A: WaitableArray + ?Sized>(array: &A, index: u32, value: A::Element, timeout: f64) -> WaitResult {
    unreachable!()
}

/// [`Atomics.waitAsync(a, i, x)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics/waitAsync):
/// `wait`'s without sleeping, a promise of how it ends where it waits.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.waitAsync")]
pub fn wait_async<A: WaitableArray + ?Sized>(array: &A, index: u32, value: A::Element) -> WaitAsyncResult {
    unreachable!()
}

/// `Atomics.waitAsync(a, i, x, timeout)`: waits at most `timeout`
/// milliseconds.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.waitAsync")]
pub fn wait_async_with_timeout<A: WaitableArray + ?Sized>(array: &A, index: u32, value: A::Element, timeout: f64) -> WaitAsyncResult {
    unreachable!()
}

/// [`Atomics.xor(a, i, x)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Atomics/xor):
/// `a[i] ^= x`.
#[cfg_attr(rust_js, rust_js::link_name = "Atomics.xor")]
pub fn xor<A: AtomicArray + ?Sized>(array: &A, index: u32, value: A::Element) -> A::Element {
    unreachable!()
}
