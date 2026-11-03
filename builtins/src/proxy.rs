//! JS's [`Proxy`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Proxy):
//! an object whose operations a handler's traps make (ADR 0283), as
//! TypeScript's `lib.es2015.proxy.d.ts` has it. A proxy is an object of
//! its target's type, as TypeScript's `T` is; what its traps give is any
//! value, an [`Unknown`].


use core::marker::PhantomData;

use super::{JsObject, PropertyDescriptor, Unknown, WeakKey};

/// [`new Proxy(target, handler)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Proxy/Proxy):
/// `target`, each operation `handler` has a trap for its trap's: an object,
/// of `target`'s type, as TypeScript's `new <T extends object>(target: T,
/// handler: ProxyHandler<T>): T` has it, so a `WeakMap` keys it, as
/// ReactFlightReplyClient's `knownServerObjectReferences.set(reference, id)`
/// does. Its traps are given `target` as a value of any shape, as JS's are.
#[cfg_attr(rust_js, rust_js::link_name = "new Proxy")]
pub fn new<T: WeakKey + ?Sized>(target: &T, handler: &ProxyHandler) -> &'static T {
    unreachable!()
}

/// [`Proxy.revocable(target, handler)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Proxy/revocable):
/// a proxy of `target`'s type, as [`new`]'s, and the function that ends it.
#[cfg_attr(rust_js, rust_js::link_name = "Proxy.revocable")]
pub fn revocable<T: WeakKey + ?Sized>(target: &T, handler: &ProxyHandler) -> &'static RevocableProxy<T> {
    unreachable!()
}

/// What `revocable` gives: the proxy, and `revoke`, after which each of
/// its operations throws a `TypeError`.
pub struct RevocableProxy<T: ?Sized>(PhantomData<JsObject>, PhantomData<*const T>);

impl<T: ?Sized> RevocableProxy<T> {
    #[cfg_attr(rust_js, rust_js::link_name = "get proxy")]
    pub fn proxy(&self) -> &'static T {
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
/// key is a string or a symbol, each a JS value, `&'static`, as what JS
/// gives is, so a trap may give one back, `get`'s `receiver`, React's
/// Flight `case 'Provider': return receiver;`, or keep it: `apply`'s and
/// `construct`'s arguments, the array JS made of them, and
/// `defineProperty`'s descriptor, the object JS made of it, are JS's too,
/// as what JS gives every callback is (ADR 0281, amended, rd663).
#[derive(Default)]
#[allow(clippy::type_complexity)]
pub struct ProxyHandler {
    /// `apply(target, this, args)`: a call of the proxy.
    pub apply: Option<Box<dyn Fn(&'static Unknown, Option<&'static Unknown>, &'static [Option<&'static Unknown>]) -> Option<&'static Unknown>>>,
    /// `construct(target, args, newTarget)`: `new` of the proxy, an object.
    pub construct: Option<Box<dyn Fn(&'static Unknown, &'static [Option<&'static Unknown>], &'static Unknown) -> &'static Unknown>>,
    /// `defineProperty(target, key, descriptor)`: `Object.defineProperty(proxy, key, ..)`, whether it was.
    pub define_property: Option<Box<dyn Fn(&'static Unknown, &'static Unknown, &'static PropertyDescriptor<'static>) -> bool>>,
    pub delete_property: Option<Box<dyn Fn(&'static Unknown, &'static Unknown) -> bool>>,
    /// `get(target, key, receiver)`: `proxy[key]`.
    pub get: Option<Box<dyn Fn(&'static Unknown, &'static Unknown, &'static Unknown) -> Option<&'static Unknown>>>,
    pub get_own_property_descriptor: Option<Box<dyn Fn(&'static Unknown, &'static Unknown) -> Option<PropertyDescriptor<'static>>>>,
    /// `getPrototypeOf(target)`: an object; a trap's `None` would be
    /// `undefined`, which JS throws a `TypeError` of, where `null` isn't.
    pub get_prototype_of: Option<Box<dyn Fn(&'static Unknown) -> &'static Unknown>>,
    /// `has(target, key)`: `key in proxy`.
    pub has: Option<Box<dyn Fn(&'static Unknown, &'static Unknown) -> bool>>,
    pub is_extensible: Option<Box<dyn Fn(&'static Unknown) -> bool>>,
    pub own_keys: Option<Box<dyn Fn(&'static Unknown) -> Vec<&'static Unknown>>>,
    pub prevent_extensions: Option<Box<dyn Fn(&'static Unknown) -> bool>>,
    /// `set(target, key, value, receiver)`: `proxy[key] = value`, whether
    /// it was.
    pub set: Option<Box<dyn Fn(&'static Unknown, &'static Unknown, Option<&'static Unknown>, &'static Unknown) -> bool>>,
    /// `setPrototypeOf(target, prototype)`, `None` of `null`.
    pub set_prototype_of: Option<Box<dyn Fn(&'static Unknown, Option<&'static Unknown>) -> bool>>,
}
