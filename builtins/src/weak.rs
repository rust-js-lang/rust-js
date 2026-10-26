//! JS's weak references (ADR 0283): `WeakMap`, `WeakSet`, `WeakRef` and
//! `FinalizationRegistry`, which don't keep what they hold alive.

// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

use super::JsObject;
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
/// a value of each key it holds, the key held weakly, any value, as a
/// [`Map`]'s is (ADR 0884).
#[cfg_attr(rust_js, rust_js::types = "WeakMap<K, V>")]
pub struct WeakMap<K: WeakKey + ?Sized, V>(PhantomData<JsObject>, PhantomData<(*const K, V)>);

impl<K: WeakKey + ?Sized, V> WeakMap<K, V> {
    /// [`map.get(key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakMap/get):
    /// its value of `key`, `None` of a key it doesn't have; of an `Option`,
    /// `Some(None)` of a key whose value is `None`, as a [`Map`]'s.
    #[cfg_attr(rust_js, rust_js::link_name = "get")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn get(&self, key: &K) -> Option<V> {
        unreachable!()
    }

    /// [`map.set(key, value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakMap/set):
    /// `value` its value of `key`.
    #[cfg_attr(rust_js, rust_js::link_name = "set")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn set(&self, key: &K, value: V) {
        unreachable!()
    }

    /// [`map.has(key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakMap/has):
    /// whether it has a value of `key`.
    #[cfg_attr(rust_js, rust_js::link_name = "has")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn has(&self, key: &K) -> bool {
        unreachable!()
    }

    /// [`map.delete(key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakMap/delete):
    /// `key` and its value gone; whether it had them.
    #[cfg_attr(rust_js, rust_js::link_name = "delete")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn delete(&self, key: &K) -> bool {
        unreachable!()
    }
}

pub mod weak_map {
    use super::*;

    /// [`new WeakMap()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakMap/WeakMap):
    /// one of nothing.
    #[cfg_attr(rust_js, rust_js::link_name = "new WeakMap")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn new<K: WeakKey + ?Sized, V>() -> &'static WeakMap<K, V> {
        unreachable!()
    }
}

/// [`Map`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map)
/// of a value of each key, held strongly, any JS value, as TypeScript's
/// `Map<K, V>` takes any `K` (ADR 0849): a key is the one JS's
/// `SameValueZero` says it is, an object by itself, text or a number by its
/// value, `undefined` and `null` each, as React's cache keys a primitive
/// argument, an `Option<&Unknown>`. One of what a [`WeakMap`] keys is a
/// `WeakMap` too, as TypeScript's is, so it derefs to one: what a program
/// keeps where the host has no `WeakMap`, as React's `typeof WeakMap ===
/// 'function' ? WeakMap : Map` does (ADR 0699). Its value is any value too,
/// as TypeScript's `Map<K, V>` takes any `V` and a [`Dict`](super::Dict)'s
/// is (ADR 0884): an `Option`'s `None` is `undefined`, as it is everywhere,
/// and JS's `null` is `Kind::Null`, upstream's `new Map([[frozenObject,
/// null]])` of `map::new(Some(&[(frozen_object, Kind::Null)]))`. A map a
/// program keeps of what Rust hashes is a `HashMap`, a `Map` in JS too.
#[cfg_attr(rust_js, rust_js::types = "Map<K, V>")]
pub struct Map<K: ?Sized, V>(PhantomData<JsObject>, PhantomData<(*const K, V)>);

/// A `Map` of what a `WeakMap` keys is a `WeakMap` of what it holds, as
/// TypeScript's `Map<K, V>` is one: the map itself.
impl<K: WeakKey + ?Sized, V> core::ops::Deref for Map<K, V> {
    type Target = WeakMap<K, V>;

    fn deref(&self) -> &WeakMap<K, V> {
        // Never runs: rust-js compiles this `Deref` to the object itself.
        unsafe { &*(self as *const Self as *const WeakMap<K, V>) }
    }
}

impl<K: ?Sized, V> Map<K, V> {
    /// [`map.get(key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/get):
    /// its value of `key`, `None` of a key it doesn't have. Of a value
    /// that may be `undefined` or `null`, an `Option`, `Some(None)` of a key
    /// whose value is `None`, apart from `None` of a key it lacks, as
    /// `map.has(key)` tells, where JS's `get` alone gives `undefined` of
    /// each: `$someGet(map, key)` (ADR 0884).
    #[cfg_attr(rust_js, rust_js::link_name = "get")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn get(&self, key: &K) -> Option<V> {
        unreachable!()
    }

    /// [`map.set(key, value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/set):
    /// `value` its value of `key`.
    #[cfg_attr(rust_js, rust_js::link_name = "set")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn set(&self, key: &K, value: V) {
        unreachable!()
    }

    /// [`map.has(key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/has):
    /// whether it has a value of `key`.
    #[cfg_attr(rust_js, rust_js::link_name = "has")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn has(&self, key: &K) -> bool {
        unreachable!()
    }

    /// [`map.delete(key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/delete):
    /// `key` and its value gone; whether it had them.
    #[cfg_attr(rust_js, rust_js::link_name = "delete")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn delete(&self, key: &K) -> bool {
        unreachable!()
    }

    /// [`map.size`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/size):
    /// how many keys it has.
    #[cfg_attr(rust_js, rust_js::link_name = "get size")]
    pub fn size(&self) -> u32 {
        unreachable!()
    }

    /// [`map.clear()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/clear):
    /// each key and its value gone.
    #[cfg_attr(rust_js, rust_js::link_name = "clear")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn clear(&self) {
        unreachable!()
    }
}

/// What steps a map gives each of its keys, held by JS as long as it holds
/// them, `&'static`, and each value, as [`get`](Map::get) gives it.
impl<K: ?Sized + 'static, V> Map<K, V> {
    /// [`map.forEach(f)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/forEach):
    /// `f` of each value and its key, in the order they were set. JS runs
    /// `f` before `forEach` returns and keeps nothing of it, so it may borrow
    /// what's around it, as a [`Set`]'s `for_each` may (ADR 0875, amended):
    /// `map.for_each(|value, _| names.push(value))`.
    #[cfg_attr(rust_js, rust_js::link_name = "forEach")]
    pub fn for_each(&self, f: impl FnMut(V, &'static K)) {
        unreachable!()
    }

    /// [`map.values()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/values):
    /// its values, in the order their keys were set, a JS iterator (ADR 0140).
    #[cfg_attr(rust_js, rust_js::link_name = "values")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn values(&self) -> Box<dyn Iterator<Item = V>> {
        unreachable!()
    }

    /// [`map.keys()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/keys):
    /// its keys, in the order they were set, a JS iterator.
    #[cfg_attr(rust_js, rust_js::link_name = "keys")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn keys(&self) -> Box<dyn Iterator<Item = &'static K>> {
        unreachable!()
    }

    /// [`map.entries()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/entries):
    /// each key and its value, `[k, v]`, in the order they were set, a JS
    /// iterator, what `for (const [k, v] of map)` steps.
    #[cfg_attr(rust_js, rust_js::link_name = "entries")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn entries(&self) -> Box<dyn Iterator<Item = (&'static K, V)>> {
        unreachable!()
    }
}

pub mod map {
    use super::*;

    /// [`new Map(entries)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Map/Map):
    /// one of each key and its value, the last of a key twice, or of
    /// nothing, `None`, as TypeScript's `new Map(entries?)` (ADR 0875).
    #[cfg_attr(rust_js, rust_js::link_name = "new Map")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn new<K: ?Sized, V>(entries: Option<&[(&K, V)]>) -> &'static Map<K, V> {
        unreachable!()
    }
}

/// [`Set`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Set)
/// of the values it holds, each once, any JS value, as TypeScript's
/// `Set<T>` takes any `T` (ADR 0875): a value is the one JS's
/// `SameValueZero` says it is, an object by itself, text or a number by
/// its value, as a [`Map`]'s key is. What a program keeps of what Rust
/// hashes, its own types by value or by themselves (ADR 0864), is a
/// `HashSet`, a `Set` in JS too; a `js::Set` is JS's, to share with JS or
/// to hold what no `Hash` has, a value of any shape.
#[cfg_attr(rust_js, rust_js::types = "Set<T>")]
pub struct Set<T: ?Sized>(PhantomData<JsObject>, PhantomData<*const T>);

impl<T: ?Sized> Set<T> {
    /// [`set.add(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Set/add):
    /// `value` held, once.
    #[cfg_attr(rust_js, rust_js::link_name = "add")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn add(&self, value: &T) {
        unreachable!()
    }

    /// [`set.has(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Set/has):
    /// whether it holds `value`.
    #[cfg_attr(rust_js, rust_js::link_name = "has")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn has(&self, value: &T) -> bool {
        unreachable!()
    }

    /// [`set.delete(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Set/delete):
    /// `value` gone; whether it held it.
    #[cfg_attr(rust_js, rust_js::link_name = "delete")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn delete(&self, value: &T) -> bool {
        unreachable!()
    }

    /// [`set.size`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Set/size):
    /// how many values it holds.
    #[cfg_attr(rust_js, rust_js::link_name = "get size")]
    pub fn size(&self) -> u32 {
        unreachable!()
    }

    /// [`set.clear()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Set/clear):
    /// each value gone.
    #[cfg_attr(rust_js, rust_js::link_name = "clear")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn clear(&self) {
        unreachable!()
    }
}

/// What steps a set gives each of its values, held by JS as long as it
/// holds them, `&'static`.
impl<T: ?Sized + 'static> Set<T> {
    /// [`set.forEach(f)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Set/forEach):
    /// `f` of each value it holds, in the order they were added. JS runs `f`
    /// before `forEach` returns and keeps nothing of it, so it may borrow
    /// what's around it, `set.for_each(|v| list.push(v))` (ADR 0875, amended).
    #[cfg_attr(rust_js, rust_js::link_name = "forEach")]
    pub fn for_each(&self, f: impl FnMut(&'static T)) {
        unreachable!()
    }

    /// [`set.values()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Set/values):
    /// the values it holds, in the order they were added, a JS iterator
    /// (ADR 0140), what `for (const v of set)` steps.
    #[cfg_attr(rust_js, rust_js::link_name = "values")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn values(&self) -> Box<dyn Iterator<Item = &'static T>> {
        unreachable!()
    }

    /// [`set.keys()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Set/keys):
    /// its values, as `values()` gives them: a set's keys are its values.
    #[cfg_attr(rust_js, rust_js::link_name = "keys")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn keys(&self) -> Box<dyn Iterator<Item = &'static T>> {
        unreachable!()
    }

    /// [`set.entries()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Set/entries):
    /// each value it holds twice, `[v, v]`, as a map's entry is its key and
    /// its value, a JS iterator.
    #[cfg_attr(rust_js, rust_js::link_name = "entries")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn entries(&self) -> Box<dyn Iterator<Item = (&'static T, &'static T)>> {
        unreachable!()
    }
}

pub mod set {
    use super::*;

    /// [`new Set(values)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Set/Set):
    /// one of each of `values`, once, or of nothing, `None`, as
    /// TypeScript's `new Set(values?)`.
    #[cfg_attr(rust_js, rust_js::link_name = "new Set")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn new<T: ?Sized>(values: Option<&[&T]>) -> &'static Set<T> {
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
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn add(&self, value: &T) {
        unreachable!()
    }

    /// [`set.has(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakSet/has):
    /// whether it holds `value`.
    #[cfg_attr(rust_js, rust_js::link_name = "has")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn has(&self, value: &T) -> bool {
        unreachable!()
    }

    /// [`set.delete(value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakSet/delete):
    /// `value` gone; whether it held it.
    #[cfg_attr(rust_js, rust_js::link_name = "delete")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
    pub fn delete(&self, value: &T) -> bool {
        unreachable!()
    }
}

pub mod weak_set {
    use super::*;

    /// [`new WeakSet()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/WeakSet/WeakSet):
    /// one of nothing.
    #[cfg_attr(rust_js, rust_js::link_name = "new WeakSet")]
    #[cfg_attr(rust_js, rust_js::runs_nothing)]
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
    /// one whose `cleanup` is given each collected target's held value: a
    /// callback JS calls again, a `Box<dyn Fn(T)>` (ADR 0260), so whether
    /// the host has one is `is_defined(finalization_registry::new::<T>)`,
    /// all its types named (ADR 0597).
    #[cfg_attr(rust_js, rust_js::link_name = "new FinalizationRegistry")]
    pub fn new<T>(cleanup: Box<dyn Fn(T)>) -> &'static FinalizationRegistry<T> {
        unreachable!()
    }
}
