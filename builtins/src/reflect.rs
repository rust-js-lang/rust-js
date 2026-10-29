//! JS's [`Reflect`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect),
//! an object's own operations as functions (ADR 0283), as TypeScript's
//! `lib.es2015.reflect.d.ts` has them, of a JS value of any shape: what
//! [`get`](super::get) and [`set`](super::set) are, and the rest. A target
//! that isn't an object, a string say, JS throws a `TypeError` of.

// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

use super::{Dict, IntoUnknown, PropertyKey, Unknown};

/// [`Reflect.apply(f, this, args)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/apply):
/// what `f` gives, called with `this` and `arguments`.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.apply")]
pub fn apply(target: &dyn Fn() -> Option<&'static Unknown>, this_argument: Option<&Unknown>, arguments: &[Option<&Unknown>]) -> Option<&'static Unknown> {
    unreachable!()
}

/// [`Reflect.construct(C, args)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/construct):
/// `new C(...arguments)`, as if `new_target`'s, whose `prototype` it has,
/// where it's given.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.construct")]
pub fn construct(
    target: &dyn Fn() -> Option<&'static Unknown>,
    arguments: &[Option<&Unknown>],
    new_target: Option<&dyn Fn() -> Option<&'static Unknown>>,
) -> &'static Unknown {
    unreachable!()
}

/// [`Reflect.defineProperty(o, key, attributes)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/defineProperty):
/// whether the property is now as `attributes` say.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.defineProperty")]
pub fn define_property<K: PropertyKey + ?Sized>(target: &Unknown, key: &K, attributes: &PropertyDescriptor<'_>) -> bool {
    unreachable!()
}

/// [`Reflect.deleteProperty(o, key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/deleteProperty):
/// `delete o[key]`, whether it's gone.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.deleteProperty")]
pub fn delete_property<K: PropertyKey + ?Sized>(target: &Unknown, key: &K) -> bool {
    unreachable!()
}

/// [`Reflect.get(o, key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/get):
/// `o[key]`, `None` where it's `undefined` or `null`; a getter's `this`
/// `receiver`, where it's given.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.get")]
pub fn get<K: PropertyKey + ?Sized>(target: &Unknown, key: &K, receiver: Option<&Unknown>) -> Option<&'static Unknown> {
    unreachable!()
}

/// [`Reflect.getOwnPropertyDescriptor(o, key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/getOwnPropertyDescriptor):
/// what an own property is, `None` where there's none.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.getOwnPropertyDescriptor")]
#[cfg_attr(rust_js, rust_js::gives_plain_data)]
pub fn get_own_property_descriptor<K: PropertyKey + ?Sized>(target: &Unknown, key: &K) -> Option<PropertyDescriptor<'static>> {
    unreachable!()
}

/// [`Reflect.getPrototypeOf(o)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/getPrototypeOf):
/// its prototype, `None` of `null`.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.getPrototypeOf")]
pub fn get_prototype_of(target: &Unknown) -> Option<&'static Unknown> {
    unreachable!()
}

/// [`Reflect.has(o, key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/has):
/// `key in o`.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.has")]
pub fn has<K: PropertyKey + ?Sized>(target: &Unknown, key: &K) -> bool {
    unreachable!()
}

/// [`Reflect.isExtensible(o)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/isExtensible):
/// whether properties may be added to it.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.isExtensible")]
pub fn is_extensible(target: &Unknown) -> bool {
    unreachable!()
}

/// [`Reflect.ownKeys(o)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/ownKeys):
/// its own properties' keys, strings and symbols, each one `get` takes.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.ownKeys")]
pub fn own_keys(target: &Unknown) -> Vec<&'static Unknown> {
    unreachable!()
}

/// [`Reflect.preventExtensions(o)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/preventExtensions):
/// whether properties may no longer be added to it.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.preventExtensions")]
pub fn prevent_extensions(target: &Unknown) -> bool {
    unreachable!()
}

/// [`Reflect.set(o, key, value)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/set):
/// `o[key] = value`, whether it was; a setter's `this` `receiver`, where
/// it's given.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.set")]
pub fn set<K: PropertyKey + ?Sized, T>(target: &Unknown, key: &K, value: T, receiver: Option<&Unknown>) -> bool {
    unreachable!()
}

/// [`Reflect.setPrototypeOf(o, prototype)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/setPrototypeOf):
/// whether its prototype is now `prototype`, none of `None`, `null`.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.setPrototypeOf")]
#[cfg_attr(rust_js, rust_js::nullable(prototype))]
pub fn set_prototype_of(target: &Unknown, prototype: Option<&Unknown>) -> bool {
    unreachable!()
}

/// [What a property is](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/defineProperty#description),
/// TypeScript's `PropertyDescriptor`: its value and whether it's
/// `writable`, or its getter and setter, and whether it's `enumerable` and
/// `configurable`. A dictionary, as webapi's are (ADR 0666): a struct of
/// `Option`s, each `None` left out, the rest `..Default::default()`, or
/// another descriptor's, `PropertyDescriptor { value: Some(&prev), ..props
/// }`, which is `{ ...props, value: prev }`. What one `define_property` is
/// given leaves out stays as it was; one of both a value and a getter JS
/// throws a `TypeError` of. Its `value` is any value, as a dictionary's
/// member of `any` is (ADR 0654).
#[derive(Clone, Copy, Default)]
pub struct PropertyDescriptor<'a> {
    pub configurable: Option<bool>,
    pub enumerable: Option<bool>,
    pub value: Option<&'a dyn IntoUnknown>,
    pub writable: Option<bool>,
    /// Its getter: what reading the property gives.
    pub get: Option<&'a dyn Fn() -> Option<&'static Unknown>>,
    /// Its setter: given what's written, any value JS gives, of any
    /// lifetime, as its getter's is: a closure of any lifetime is one, and
    /// so is a function `function::with_this` makes, which reads its `this`
    /// (ADR 0666, amended).
    pub set: Option<&'a dyn Fn(Option<&'static Unknown>)>,
}

/// TypeScript's `PropertyDescriptorMap`: a JS object of a
/// [`PropertyDescriptor`] by each property's name, which
/// `Object.defineProperties` and `Object.create` take. A `Dict` of them is
/// one, and a struct of them, as its `impl` vouches, as a `JsonText` one
/// does: `unsafe impl PropertyDescriptorMap for Logs<'_> {}` of `struct
/// Logs<'a> { log: PropertyDescriptor<'a>, .. }`, `{ log: .. }`.
///
/// # Safety
///
/// Each of its fields is a `PropertyDescriptor`, as JS throws a
/// `TypeError` of any other.
pub unsafe trait PropertyDescriptorMap {}

unsafe impl PropertyDescriptorMap for Dict<PropertyDescriptor<'_>> {}
