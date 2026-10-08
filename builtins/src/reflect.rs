//! JS's [`Reflect`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect),
//! an object's own operations as functions (ADR 0283), as TypeScript's
//! `lib.es2015.reflect.d.ts` has them, of a JS value of any shape: what
//! [`get`](super::get) and [`set`](super::set) are, and the rest. A target
//! that isn't an object, a string say, JS throws a `TypeError` of.

// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

use core::marker::PhantomData;

use super::{JsObject, PropertyKey, Unknown};

/// [`Reflect.apply(f, this, args)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/apply):
/// what `f` gives, called with `this` and `arguments`.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.apply")]
pub fn apply(target: &dyn Fn(), this_argument: Option<&Unknown>, arguments: &[Option<&Unknown>]) -> Option<&'static Unknown> {
    unreachable!()
}

/// [`Reflect.construct(C, args)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/construct):
/// `new C(...arguments)`.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.construct")]
pub fn construct(target: &dyn Fn(), arguments: &[Option<&Unknown>]) -> &'static Unknown {
    unreachable!()
}

/// `Reflect.construct(C, args, newTarget)`: `new C(...arguments)` as if
/// `new_target`'s, whose `prototype` it has.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.construct")]
pub fn construct_with_new_target(target: &dyn Fn(), arguments: &[Option<&Unknown>], new_target: &dyn Fn()) -> &'static Unknown {
    unreachable!()
}

/// [`Reflect.defineProperty(o, key, attributes)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/defineProperty):
/// whether the property is now as `attributes` say.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.defineProperty")]
pub fn define_property<K: PropertyKey + ?Sized>(target: &Unknown, key: &K, attributes: &PropertyDescriptor) -> bool {
    unreachable!()
}

/// [`Reflect.deleteProperty(o, key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/deleteProperty):
/// `delete o[key]`, whether it's gone.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.deleteProperty")]
pub fn delete_property<K: PropertyKey + ?Sized>(target: &Unknown, key: &K) -> bool {
    unreachable!()
}

/// [`Reflect.get(o, key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/get):
/// `o[key]`, `None` where it's `undefined` or `null`.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.get")]
pub fn get<K: PropertyKey + ?Sized>(target: &Unknown, key: &K) -> Option<&'static Unknown> {
    unreachable!()
}

/// `Reflect.get(o, key, receiver)`: `o[key]`, a getter's `this` `receiver`.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.get")]
pub fn get_with_receiver<K: PropertyKey + ?Sized>(target: &Unknown, key: &K, receiver: &Unknown) -> Option<&'static Unknown> {
    unreachable!()
}

/// [`Reflect.getOwnPropertyDescriptor(o, key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/getOwnPropertyDescriptor):
/// what an own property is, `None` where there's none.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.getOwnPropertyDescriptor")]
pub fn get_own_property_descriptor<K: PropertyKey + ?Sized>(target: &Unknown, key: &K) -> Option<&'static PropertyDescriptor> {
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
/// `o[key] = value`, whether it was.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.set")]
pub fn set<K: PropertyKey + ?Sized, T>(target: &Unknown, key: &K, value: T) -> bool {
    unreachable!()
}

/// `Reflect.set(o, key, value, receiver)`: `o[key] = value`, a setter's
/// `this` `receiver`.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.set")]
pub fn set_with_receiver<K: PropertyKey + ?Sized, T>(target: &Unknown, key: &K, value: T, receiver: &Unknown) -> bool {
    unreachable!()
}

/// [`Reflect.setPrototypeOf(o, prototype)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Reflect/setPrototypeOf):
/// whether its prototype is now `prototype`, none of `None`, `null`.
#[cfg_attr(rust_js, rust_js::link_name = "Reflect.setPrototypeOf")]
#[cfg_attr(rust_js, rust_js::nullable(prototype))]
pub fn set_prototype_of(target: &Unknown, prototype: Option<&Unknown>) -> bool {
    unreachable!()
}

/// [What a property is](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Object/defineProperty#description):
/// its value and whether it's `writable`, or its getter and setter, and
/// whether it's `enumerable` and `configurable`, TypeScript's
/// `PropertyDescriptor`. One `define_property` takes has what it's given,
/// [`property_descriptor::new`] and its setters, `d.set_writable(false)`:
/// what it isn't given stays as it is, and one of both a value and a
/// getter JS throws a `TypeError` of.
pub struct PropertyDescriptor(PhantomData<JsObject>);

impl PropertyDescriptor {
    #[cfg_attr(rust_js, rust_js::link_name = "get value")]
    pub fn value(&self) -> Option<&'static Unknown> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set value")]
    pub fn set_value(&self, value: Option<&Unknown>) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get writable")]
    pub fn writable(&self) -> Option<bool> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set writable")]
    pub fn set_writable(&self, writable: bool) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get enumerable")]
    pub fn enumerable(&self) -> Option<bool> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set enumerable")]
    pub fn set_enumerable(&self, enumerable: bool) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "get configurable")]
    pub fn configurable(&self) -> Option<bool> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set configurable")]
    pub fn set_configurable(&self, configurable: bool) {
        unreachable!()
    }

    /// Its getter, `get`.
    #[cfg_attr(rust_js, rust_js::link_name = "get get")]
    pub fn get(&self) -> Option<&'static dyn Fn() -> Option<&'static Unknown>> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set get")]
    pub fn set_get(&self, getter: Box<dyn Fn() -> Option<&'static Unknown>>) {
        unreachable!()
    }

    /// Its setter, `set`.
    #[cfg_attr(rust_js, rust_js::link_name = "get set")]
    pub fn set(&self) -> Option<&'static dyn Fn(Option<&Unknown>)> {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set set")]
    pub fn set_set(&self, setter: Box<dyn Fn(Option<&Unknown>)>) {
        unreachable!()
    }
}

pub mod property_descriptor {
    use super::PropertyDescriptor;

    /// `{}`: a descriptor of nothing yet, its setters' to give.
    #[cfg_attr(rust_js, rust_js::link_name = "{}")]
    pub fn new() -> &'static PropertyDescriptor {
        unreachable!()
    }
}
