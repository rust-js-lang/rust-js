//! JS's weak references (ADR 0283): `WeakMap`, `WeakSet`, `WeakRef` and
//! `FinalizationRegistry`, which don't keep what they hold alive.

// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

use super::{Defined, JsObject};
use core::marker::PhantomData;

/// What JS holds weakly: an object, or a symbol not the registry's, never a
/// string or a number, which a weak collection throws on. A struct is an
/// object, so one is a key as its `impl` says, `unsafe impl WeakKey for State {}`.
/// TypeScript's `WeakKey`, a `dyn WeakKey` (ADR 0677).
#[cfg_attr(rust_js, rust_js::types = "WeakKey")]
pub unsafe trait WeakKey {}

// What `js::Kind` holds of an array and a function, each an object (ADR 0677).
unsafe impl<T> WeakKey for [T] {}
unsafe impl WeakKey for dyn Fn() -> Option<&'static super::Unknown> + '_ {}

/// [`WeakMap`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakMap):
/// a value of each key it holds, the key held weakly.
#[cfg_attr(rust_js, rust_js::types = "WeakMap<K, V>")]
pub struct WeakMap<K: WeakKey + ?Sized, V: Defined>(PhantomData<JsObject>, PhantomData<(*const K, V)>);

impl<K: WeakKey + ?Sized, V: Defined> WeakMap<K, V> {
    /// [`map.get(key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakMap/get):
    /// its value of `key`, `None` of a key it doesn't have.
    #[cfg_attr(rust_js, rust_js::link_name = "get")]
    pub fn get(&self, key: &K) -> Option<V> {
        unreachable!()
    }

    /// [`map.set(key, value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakMap/set):
    /// `value` its value of `key`.
    #[cfg_attr(rust_js, rust_js::link_name = "set")]
    pub fn set(&self, key: &K, value: V) {
        unreachable!()
    }

    /// [`map.has(key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakMap/has):
    /// whether it has a value of `key`.
    #[cfg_attr(rust_js, rust_js::link_name = "has")]
    pub fn has(&self, key: &K) -> bool {
        unreachable!()
    }

    /// [`map.delete(key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakMap/delete):
    /// `key` and its value gone; whether it had them.
    #[cfg_attr(rust_js, rust_js::link_name = "delete")]
    pub fn delete(&self, key: &K) -> bool {
        unreachable!()
    }
}

pub mod weak_map {
    use super::*;

    /// [`new WeakMap()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakMap/WeakMap):
    /// one of nothing.
    #[cfg_attr(rust_js, rust_js::link_name = "new WeakMap")]
    pub fn new<K: WeakKey + ?Sized, V: Defined>() -> &'static WeakMap<K, V> {
        unreachable!()
    }
}

/// [`Map`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map)
/// of what a [`WeakMap`] keys, an object by itself, held strongly: what a
/// program keeps where the host has no `WeakMap`, as React's
/// `typeof WeakMap === 'function' ? WeakMap : Map` does. TypeScript's
/// `Map<K, V>`, which is a `WeakMap<K, V>` too, so it derefs to one: its
/// `get`, `set`, `has` and `delete` are a `WeakMap`'s (ADR 0699). A map of
/// what Rust hashes, text or a number, is a `HashMap`, a `Map` in JS.
#[cfg_attr(rust_js, rust_js::types = "Map<K, V>")]
pub struct Map<K: WeakKey + ?Sized, V: Defined>(PhantomData<JsObject>, PhantomData<(*const K, V)>);

/// A `Map` is a `WeakMap` of what it holds, as TypeScript's `Map<K, V>` is
/// one: the map itself.
impl<K: WeakKey + ?Sized, V: Defined> core::ops::Deref for Map<K, V> {
    type Target = WeakMap<K, V>;

    fn deref(&self) -> &WeakMap<K, V> {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const WeakMap<K, V>) }
    }
}

impl<K: WeakKey + ?Sized, V: Defined> Map<K, V> {
    /// [`map.size`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/size):
    /// how many keys it has.
    #[cfg_attr(rust_js, rust_js::link_name = "get size")]
    pub fn size(&self) -> u32 {
        unreachable!()
    }

    /// [`map.clear()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/clear):
    /// each key and its value gone.
    #[cfg_attr(rust_js, rust_js::link_name = "clear")]
    pub fn clear(&self) {
        unreachable!()
    }
}

pub mod map {
    use super::*;

    /// [`new Map()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/Map):
    /// one of nothing.
    #[cfg_attr(rust_js, rust_js::link_name = "new Map")]
    pub fn new<K: WeakKey + ?Sized, V: Defined>() -> &'static Map<K, V> {
        unreachable!()
    }
}

/// [`WeakSet`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakSet):
/// the values it holds, each held weakly.
#[cfg_attr(rust_js, rust_js::types = "WeakSet<T>")]
pub struct WeakSet<T: WeakKey + ?Sized>(PhantomData<JsObject>, PhantomData<*const T>);

impl<T: WeakKey + ?Sized> WeakSet<T> {
    /// [`set.add(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakSet/add):
    /// `value` held.
    #[cfg_attr(rust_js, rust_js::link_name = "add")]
    pub fn add(&self, value: &T) {
        unreachable!()
    }

    /// [`set.has(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakSet/has):
    /// whether it holds `value`.
    #[cfg_attr(rust_js, rust_js::link_name = "has")]
    pub fn has(&self, value: &T) -> bool {
        unreachable!()
    }

    /// [`set.delete(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakSet/delete):
    /// `value` gone; whether it held it.
    #[cfg_attr(rust_js, rust_js::link_name = "delete")]
    pub fn delete(&self, value: &T) -> bool {
        unreachable!()
    }
}

pub mod weak_set {
    use super::*;

    /// [`new WeakSet()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakSet/WeakSet):
    /// one of nothing.
    #[cfg_attr(rust_js, rust_js::link_name = "new WeakSet")]
    pub fn new<T: WeakKey + ?Sized>() -> &'static WeakSet<T> {
        unreachable!()
    }
}

/// [`WeakRef`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakRef):
/// what it refers to, while something else keeps it alive.
#[cfg_attr(rust_js, rust_js::types = "WeakRef<T>")]
pub struct WeakRef<T: WeakKey + ?Sized + 'static>(PhantomData<JsObject>, PhantomData<&'static T>);

impl<T: WeakKey + ?Sized + 'static> WeakRef<T> {
    /// [`ref.deref()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakRef/deref):
    /// what it refers to, `None` once it's collected.
    #[cfg_attr(rust_js, rust_js::link_name = "deref")]
    pub fn deref(&self) -> Option<&'static T> {
        unreachable!()
    }
}

pub mod weak_ref {
    use super::*;

    /// [`new WeakRef(target)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakRef/WeakRef):
    /// a weak reference to `target`.
    #[cfg_attr(rust_js, rust_js::link_name = "new WeakRef")]
    pub fn new<T: WeakKey + ?Sized + 'static>(target: &'static T) -> &'static WeakRef<T> {
        unreachable!()
    }
}

/// [`FinalizationRegistry`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/FinalizationRegistry):
/// a callback given each target's held value once the target is collected.
#[cfg_attr(rust_js, rust_js::types = "FinalizationRegistry<T>")]
pub struct FinalizationRegistry<T>(PhantomData<JsObject>, PhantomData<T>);

impl<T> FinalizationRegistry<T> {
    /// [`registry.register(target, held)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/FinalizationRegistry/register):
    /// `held` given to its callback once `target` is collected;
    /// `unregister(token)` undoes it, where there's a `token`.
    #[cfg_attr(rust_js, rust_js::link_name = "register")]
    pub fn register<K: WeakKey + ?Sized>(&self, target: &K, held: T, token: Option<&dyn WeakKey>) {
        unreachable!()
    }

    /// [`registry.unregister(token)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/FinalizationRegistry/unregister):
    /// what `token` registered undone; whether there was any.
    #[cfg_attr(rust_js, rust_js::link_name = "unregister")]
    pub fn unregister<U: WeakKey + ?Sized>(&self, token: &U) -> bool {
        unreachable!()
    }
}

pub mod finalization_registry {
    use super::*;

    /// [`new FinalizationRegistry(cleanup)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/FinalizationRegistry/FinalizationRegistry):
    /// one whose `cleanup` is given each collected target's held value.
    #[cfg_attr(rust_js, rust_js::link_name = "new FinalizationRegistry")]
    pub fn new<T>(cleanup: impl Fn(T) + 'static) -> &'static FinalizationRegistry<T> {
        unreachable!()
    }
}
