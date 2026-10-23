//! JS's [`Symbol`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol):
//! a value unlike any other, a property's key no string can be (ADR 0283);
//! and the registry's, [`SymbolFor`], a `const` a `match` names (ADR 0634).

// A binding's parameters are its JS function's: its body never runs.
#![allow(unused_variables)]

use super::{Defined, JsObject, PropertyKey, Unknown};
use core::marker::PhantomData;

/// [`Symbol`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol):
/// any symbol, `symbol::new("id")` a new one; the registry's is a
/// [`SymbolFor`].
#[cfg_attr(rust_js, rust_js::types = "symbol")]
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
    pub safe static r#match: &'static Symbol;

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

/// [`Symbol(description)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/Symbol):
/// a new symbol, unlike every other.
#[cfg_attr(rust_js, rust_js::link_name = "Symbol")]
pub fn new(description: &str) -> &'static Symbol {
    unreachable!()
}

/// Gone (ADR 0634): the registry's symbol of `key` is a value,
/// `js::SymbolFor(key)`, which a `const` holds and a `match` names.
pub fn r#for<K: Gone>(key: K) -> &'static Symbol {
    unreachable!()
}

/// What [`r#for`] took, which nothing is: it's gone (ADR 0634).
#[diagnostic::on_unimplemented(
    message = "`js::symbol::r#for` is gone: the registry's symbol of a key is `js::SymbolFor(key)` (ADR 0634)",
    label = "write `js::SymbolFor(key)`"
)]
pub trait Gone {}

/// [`Symbol.keyFor(symbol)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/keyFor):
/// the registry's key of `symbol`, `None` of one not the registry's.
#[cfg_attr(rust_js, rust_js::link_name = "Symbol.keyFor")]
pub fn key_for(symbol: &Symbol) -> Option<String> {
    unreachable!()
}

/// [`Symbol.for(key)`](https://developer.mozilla.org/docs/Web/JavaScript/Reference/Global_Objects/Symbol/for):
/// the registry's symbol of `key`, the same wherever it's made, so two are
/// equal where their keys are, as JS's `===` finds them (ADR 0634). JS's
/// is the symbol: `pub const REACT_MEMO_TYPE: js::SymbolFor =
/// js::SymbolFor("react.memo");` is `export const REACT_MEMO_TYPE =
/// Symbol.for("react.memo");`, and a `match` of them a `switch`. Its key,
/// `.0`, is `Symbol.keyFor(symbol)`.
#[cfg_attr(rust_js, rust_js::types = "symbol")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SymbolFor<'a>(pub &'a str);

// A value, never `undefined`, and a property's key.
unsafe impl Defined for SymbolFor<'_> {}
impl PropertyKey for SymbolFor<'_> {}

impl SymbolFor<'static> {
    /// `value` as the registry's symbol, `None` of any other value, a
    /// symbol of its own or none: matched against the registry's, `match
    /// js::SymbolFor::from_value(ty) { Some(REACT_MEMO_TYPE) => .., _ => ..
    /// }`, the value itself, `switch (type) { case REACT_MEMO_TYPE: .. }`,
    /// as no other value is one of them; else
    /// `$symbolFor(value)`, the value where it's the registry's, `undefined`
    /// where not (ADR 0634).
    pub fn from_value(value: Option<&Unknown>) -> Option<SymbolFor<'static>> {
        unreachable!()
    }
}
