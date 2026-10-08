//! JS's [`Symbol`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol):
//! a value unlike any other, a property's key no string can be (ADR 0283).

// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

use super::{Defined, JsObject, PropertyKey};
use core::marker::PhantomData;

/// [`Symbol`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol):
/// `symbol::new("id")` is a new one, `symbol::for_("id")` the registry's.
pub struct Symbol(PhantomData<JsObject>);

// A value, never `undefined`, and a property's key.
unsafe impl Defined for Symbol {}
impl PropertyKey for Symbol {}

impl Symbol {
    /// [`symbol.description`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/description):
    /// what it was made with, `None` of none.
    #[cfg_attr(rust_js, rust_js::link_name = "get description")]
    pub fn description(&self) -> Option<String> {
        unreachable!()
    }

    /// [`symbol.toString()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/toString):
    /// `Symbol(id)`.
    #[cfg_attr(rust_js, rust_js::link_name = "toString")]
    pub fn to_string(&self) -> String {
        unreachable!()
    }

    /// [`symbol.valueOf()`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/valueOf):
    /// itself.
    #[cfg_attr(rust_js, rust_js::link_name = "valueOf")]
    pub fn value_of(&self) -> &'static Symbol {
        unreachable!()
    }
}

unsafe extern "Rust" {
    /// [`Symbol(description)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/Symbol):
    /// a new symbol, unlike every other.
    #[link_name = "Symbol"]
    pub safe fn new(description: &str) -> &'static Symbol;

    /// [`Symbol.for(key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/for):
    /// the registry's symbol of `key`, the same each time.
    #[link_name = "Symbol.for"]
    pub safe fn for_(key: &str) -> &'static Symbol;

    /// [`Symbol.keyFor(symbol)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/keyFor):
    /// the registry's key of `symbol`, `None` of one not the registry's.
    #[link_name = "Symbol.keyFor"]
    pub safe fn key_for(symbol: &Symbol) -> Option<String>;

    /// [`Symbol.asyncIterator`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/asyncIterator).
    #[link_name = "Symbol.asyncIterator"]
    pub safe static async_iterator: &'static Symbol;

    /// [`Symbol.hasInstance`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/hasInstance).
    #[link_name = "Symbol.hasInstance"]
    pub safe static has_instance: &'static Symbol;

    /// [`Symbol.isConcatSpreadable`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/isConcatSpreadable).
    #[link_name = "Symbol.isConcatSpreadable"]
    pub safe static is_concat_spreadable: &'static Symbol;

    /// [`Symbol.iterator`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/iterator).
    #[link_name = "Symbol.iterator"]
    pub safe static iterator: &'static Symbol;

    /// [`Symbol.match`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/match).
    #[link_name = "Symbol.match"]
    pub safe static match_: &'static Symbol;

    /// [`Symbol.matchAll`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/matchAll).
    #[link_name = "Symbol.matchAll"]
    pub safe static match_all: &'static Symbol;

    /// [`Symbol.replace`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/replace).
    #[link_name = "Symbol.replace"]
    pub safe static replace: &'static Symbol;

    /// [`Symbol.search`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/search).
    #[link_name = "Symbol.search"]
    pub safe static search: &'static Symbol;

    /// [`Symbol.species`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/species).
    #[link_name = "Symbol.species"]
    pub safe static species: &'static Symbol;

    /// [`Symbol.split`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/split).
    #[link_name = "Symbol.split"]
    pub safe static split: &'static Symbol;

    /// [`Symbol.toPrimitive`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/toPrimitive).
    #[link_name = "Symbol.toPrimitive"]
    pub safe static to_primitive: &'static Symbol;

    /// [`Symbol.toStringTag`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/toStringTag).
    #[link_name = "Symbol.toStringTag"]
    pub safe static to_string_tag: &'static Symbol;

    /// [`Symbol.unscopables`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/unscopables).
    #[link_name = "Symbol.unscopables"]
    pub safe static unscopables: &'static Symbol;
}
