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
/// TypeScript's `ProxyHandler`: [`proxy_handler::new`], then a setter of
/// each, `handler.set_get(Box::new(|target, key, receiver| ..))`; an
/// operation without one is its target's. Each takes the target, and a
/// key is a string or a symbol.
pub struct ProxyHandler(PhantomData<JsObject>);

impl ProxyHandler {
    /// `apply(target, this, args)`: a call of the proxy.
    #[cfg_attr(rust_js, rust_js::link_name = "set apply")]
    pub fn set_apply(&self, trap: Box<dyn Fn(&Unknown, Option<&Unknown>, &[Option<&Unknown>]) -> Option<&'static Unknown>>) {
        unreachable!()
    }

    /// `construct(target, args, newTarget)`: `new` of the proxy, an object.
    #[cfg_attr(rust_js, rust_js::link_name = "set construct")]
    pub fn set_construct(&self, trap: Box<dyn Fn(&Unknown, &[Option<&Unknown>], &Unknown) -> &'static Unknown>) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set defineProperty")]
    pub fn set_define_property(&self, trap: Box<dyn Fn(&Unknown, &Unknown, &PropertyDescriptor) -> bool>) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set deleteProperty")]
    pub fn set_delete_property(&self, trap: Box<dyn Fn(&Unknown, &Unknown) -> bool>) {
        unreachable!()
    }

    /// `get(target, key, receiver)`: `proxy[key]`.
    #[cfg_attr(rust_js, rust_js::link_name = "set get")]
    pub fn set_get(&self, trap: Box<dyn Fn(&Unknown, &Unknown, &Unknown) -> Option<&'static Unknown>>) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set getOwnPropertyDescriptor")]
    pub fn set_get_own_property_descriptor(&self, trap: Box<dyn Fn(&Unknown, &Unknown) -> Option<&'static PropertyDescriptor>>) {
        unreachable!()
    }

    /// `getPrototypeOf(target)`: an object; a trap's `None` would be
    /// `undefined`, which JS throws a `TypeError` of, where `null` isn't.
    #[cfg_attr(rust_js, rust_js::link_name = "set getPrototypeOf")]
    pub fn set_get_prototype_of(&self, trap: Box<dyn Fn(&Unknown) -> &'static Unknown>) {
        unreachable!()
    }

    /// `has(target, key)`: `key in proxy`.
    #[cfg_attr(rust_js, rust_js::link_name = "set has")]
    pub fn set_has(&self, trap: Box<dyn Fn(&Unknown, &Unknown) -> bool>) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set isExtensible")]
    pub fn set_is_extensible(&self, trap: Box<dyn Fn(&Unknown) -> bool>) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set ownKeys")]
    pub fn set_own_keys(&self, trap: Box<dyn Fn(&Unknown) -> Vec<&'static Unknown>>) {
        unreachable!()
    }

    #[cfg_attr(rust_js, rust_js::link_name = "set preventExtensions")]
    pub fn set_prevent_extensions(&self, trap: Box<dyn Fn(&Unknown) -> bool>) {
        unreachable!()
    }

    /// `set(target, key, value, receiver)`: `proxy[key] = value`, whether
    /// it was.
    #[cfg_attr(rust_js, rust_js::link_name = "set set")]
    pub fn set_set(&self, trap: Box<dyn Fn(&Unknown, &Unknown, Option<&Unknown>, &Unknown) -> bool>) {
        unreachable!()
    }

    /// `setPrototypeOf(target, prototype)`, `None` of `null`.
    #[cfg_attr(rust_js, rust_js::link_name = "set setPrototypeOf")]
    pub fn set_set_prototype_of(&self, trap: Box<dyn Fn(&Unknown, Option<&Unknown>) -> bool>) {
        unreachable!()
    }
}

pub mod proxy_handler {
    use super::ProxyHandler;

    /// `{}`: a handler of no traps yet, its setters' to give.
    #[cfg_attr(rust_js, rust_js::link_name = "{}")]
    pub fn new() -> &'static ProxyHandler {
        unreachable!()
    }
}
