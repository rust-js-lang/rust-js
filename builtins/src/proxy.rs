//! JS's [`Proxy`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Proxy):
//! an object whose operations a handler's traps make (ADR 0283), as
//! TypeScript's `lib.es2015.proxy.d.ts` has it. A proxy is a JS value of
//! any shape, an [`Unknown`], as its traps may give anything: one of a
//! Rust type would be that type's in name only.

// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

use core::marker::PhantomData;

use super::{JsObject, PropertyDescriptor, Unknown};

/// [`new Proxy(target, handler)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Proxy/Proxy):
/// `target`, each operation `handler` has a trap for its trap's.
#[cfg_attr(rust_js, rust_js::link_name = "new Proxy")]
pub fn new(target: &Unknown, handler: &ProxyHandler) -> &'static Unknown {
    unreachable!()
}

/// [`Proxy.revocable(target, handler)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Proxy/revocable):
/// a proxy, and the function that ends it.
#[cfg_attr(rust_js, rust_js::link_name = "Proxy.revocable")]
pub fn revocable(target: &Unknown, handler: &ProxyHandler) -> &'static RevocableProxy {
    unreachable!()
}

/// What `revocable` gives: the proxy, and `revoke`, after which each of
/// its operations throws a `TypeError`.
pub struct RevocableProxy(PhantomData<JsObject>);

impl RevocableProxy {
    #[cfg_attr(rust_js, rust_js::link_name = "get proxy")]
    pub fn proxy(&self) -> &'static Unknown {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "revoke")]
    pub fn revoke(&self) {
        unreachable!()
    }
}

/// [A proxy's traps](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Proxy/Proxy/handler),
/// TypeScript's `ProxyHandler`: a dictionary, as a `PropertyDescriptor` is
/// (ADR 0666), of a closure JS keeps for each trap, `ProxyHandler { get:
/// Some(Box::new(|target, key, receiver| ..)), ..Default::default() }`; an
/// operation without one is its target's. Each takes the target, and a
/// key is a string or a symbol.
#[derive(Default)]
#[allow(clippy::type_complexity)]
pub struct ProxyHandler {
    /// `apply(target, this, args)`: a call of the proxy.
    pub apply: Option<Box<dyn Fn(&Unknown, Option<&Unknown>, &[Option<&Unknown>]) -> Option<&'static Unknown>>>,
    /// `construct(target, args, newTarget)`: `new` of the proxy, an object.
    pub construct: Option<Box<dyn Fn(&Unknown, &[Option<&Unknown>], &Unknown) -> &'static Unknown>>,
    pub define_property: Option<Box<dyn Fn(&Unknown, &Unknown, &PropertyDescriptor<'_>) -> bool>>,
    pub delete_property: Option<Box<dyn Fn(&Unknown, &Unknown) -> bool>>,
    /// `get(target, key, receiver)`: `proxy[key]`.
    pub get: Option<Box<dyn Fn(&Unknown, &Unknown, &Unknown) -> Option<&'static Unknown>>>,
    pub get_own_property_descriptor: Option<Box<dyn Fn(&Unknown, &Unknown) -> Option<PropertyDescriptor<'static>>>>,
    /// `getPrototypeOf(target)`: an object; a trap's `None` would be
    /// `undefined`, which JS throws a `TypeError` of, where `null` isn't.
    pub get_prototype_of: Option<Box<dyn Fn(&Unknown) -> &'static Unknown>>,
    /// `has(target, key)`: `key in proxy`.
    pub has: Option<Box<dyn Fn(&Unknown, &Unknown) -> bool>>,
    pub is_extensible: Option<Box<dyn Fn(&Unknown) -> bool>>,
    pub own_keys: Option<Box<dyn Fn(&Unknown) -> Vec<&'static Unknown>>>,
    pub prevent_extensions: Option<Box<dyn Fn(&Unknown) -> bool>>,
    /// `set(target, key, value, receiver)`: `proxy[key] = value`, whether
    /// it was.
    pub set: Option<Box<dyn Fn(&Unknown, &Unknown, Option<&Unknown>, &Unknown) -> bool>>,
    /// `setPrototypeOf(target, prototype)`, `None` of `null`.
    pub set_prototype_of: Option<Box<dyn Fn(&Unknown, Option<&Unknown>) -> bool>>,
}
