//! rust-js's wasm-bindgen: wasm-bindgen's API, as crates use it, for an
//! app's dependencies only (ADR 1112). An app's own code binds JS with the
//! `js` and `webapi` crates.
//!
//! `#[wasm_bindgen]` of an `extern "C"` block makes each function a call of
//! the JS it names, as `wasm-bindgen-macro`'s table converts it; a
//! [`JsValue`] is any JS value, what a `catch` function's call threw.

pub use wasm_bindgen_macro::wasm_bindgen;

/// Any JS value, `undefined` too: what JS threw at a `catch` function. The
/// value itself, as its one field is (ADR 0584); a clone the same value, as
/// wasm-bindgen's clone holds the same JS value.
#[repr(transparent)]
#[derive(Clone)]
pub struct JsValue {
    /// The value, `None` where it's `undefined`: what `#[wasm_bindgen]`'s
    /// functions make, not to be named.
    #[doc(hidden)]
    pub __value: Option<&'static js::Unknown>,
}

/// `JsValue(..)`, of what wasm-bindgen's JS shows of the value, its
/// `debugString` (`debug_string`). Upstream's native branch has none to
/// show, a `JsValue` being JS's alone (ADR 1112).
impl core::fmt::Debug for JsValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "JsValue({})", debug_string(self.__value))
    }
}

/// wasm-bindgen's own `debugString(val)`, cli-support 0.2.130's, case by
/// case, of the js crate's bindings: a number's, a `bool`'s and
/// `undefined`'s `String(val)`, text's `JSON.stringify`, a `bigint`'s with
/// `n`, a symbol's and a function's of their names, `null`, an array's of
/// its items, an `Error`'s name, message and stack, and another object's
/// class, `Object(..)` of its JSON.
fn debug_string(value: Option<&'static js::Unknown>) -> String {
    match js::classify(value) {
        js::Kind::Number(_) | js::Kind::Bool(_) | js::Kind::Undefined => js::string(value),
        js::Kind::String(text) => js::json::stringify(text),
        js::Kind::BigInt(_) => format!("{}n", js::string(value)),
        js::Kind::Symbol(symbol) => match symbol.description() {
            Some(description) => format!("Symbol({description})"),
            None => "Symbol".to_string(),
        },
        js::Kind::Function(function) => match js::function::name(function) {
            name if name.is_empty() => "Function".to_string(),
            name => format!("Function({name})"),
        },
        js::Kind::Null => "null".to_string(),
        js::Kind::Array(items) => {
            let shown: Vec<String> = items.iter().map(|&item| debug_string(item)).collect();
            format!("[{}]", shown.join(", "))
        }
        js::Kind::Object(_) => object_string(value),
    }
}

/// `debugString` of an object: an `Error`'s `name: message\nstack`, or of
/// `Object.prototype.toString`'s `[object Class]`, `Object(..)` of an
/// `Object`'s JSON, or its class's name, or what it said where it isn't
/// of that form. What JS throws, a getter's, is thrown on, as upstream's.
fn object_string(value: Option<&'static js::Unknown>) -> String {
    // SAFETY: an object, which `instanceof Error` asks of.
    let error: &js::JsError = unsafe { js::cast(value) };
    if error.is_error() {
        let stack = error.stack().unwrap_or_else(|| "undefined".to_string());
        return format!("{}: {}\n{}", error.name(), error.message(), stack);
    }
    let tag = match js::object::to_string(value) {
        Ok(tag) => tag,
        Err(thrown) => js::throw(thrown.as_unknown()),
    };
    match class_name(&tag) {
        None => tag,
        Some("Object") => match js::json::stringify(&value) {
            Ok(json) => format!("Object({})", json.unwrap_or_else(|| "undefined".to_string())),
            Err(_) => "Object".to_string(),
        },
        Some(class) => class.to_string(),
    }
}

/// `/\[object ([^\]]+)\]/.exec(tag)?.[1]`: the first `[object X]`'s `X`.
fn class_name(tag: &str) -> Option<&str> {
    let mut rest = tag;
    while let Some(at) = rest.find("[object ") {
        let after = &rest[at + "[object ".len()..];
        match after.find(']') {
            Some(end) if end > 0 => return Some(&after[..end]),
            _ => rest = &rest[at + 1..],
        }
    }
    None
}

impl JsValue {
    /// Whether it's `undefined`.
    pub fn is_undefined(&self) -> bool {
        matches!(js::classify(self.__value), js::Kind::Undefined)
    }

    /// The number `n` as a JS value: itself.
    #[cfg_attr(rust_js, rust_js::link_name = "this")]
    pub fn from_f64(this: f64) -> JsValue {
        unreachable!()
    }
}

impl AsRef<JsValue> for JsValue {
    fn as_ref(&self) -> &JsValue {
        self
    }
}

/// A JS value an `extern` block's `static` names, `static MODULE:
/// NodeModule`: read where it's used, as wasm-bindgen's reads it the first
/// time and keeps it, of a value that stays the same (#1404).
pub struct JsStatic<T: 'static> {
    #[doc(hidden)]
    pub __get: fn() -> T,
}

impl<T> core::ops::Deref for JsStatic<T> {
    type Target = T;

    fn deref(&self) -> &T {
        Box::leak(Box::new((self.__get)()))
    }
}

/// `Option`'s and `Result`'s `unwrap` and `expect`, of wasm-bindgen's names:
/// its native branch, upstream's of every target but wasm's, a Rust panic
/// of native Rust's message, caught by `catch_unwind` and running drops as
/// any panic does. Upstream's wasm branch throws to JS instead; native Rust
/// is rust-js's reference (ADR 1112).
pub trait UnwrapThrowExt<T>: Sized {
    /// `self.unwrap()`, of wasm-bindgen's message.
    #[track_caller]
    fn unwrap_throw(self) -> T {
        self.expect_throw("called `unwrap_throw()`")
    }

    /// `self.expect(message)`.
    #[track_caller]
    fn expect_throw(self, message: &str) -> T;
}

impl<T> UnwrapThrowExt<T> for Option<T> {
    fn unwrap_throw(self) -> T {
        self.expect("called `Option::unwrap_throw()` on a `None` value")
    }

    fn expect_throw(self, message: &str) -> T {
        self.expect(message)
    }
}

impl<T, E: core::fmt::Debug> UnwrapThrowExt<T> for Result<T, E> {
    fn unwrap_throw(self) -> T {
        self.expect("called `Result::unwrap_throw()` on an `Err` value")
    }

    fn expect_throw(self, message: &str) -> T {
        self.expect(message)
    }
}

/// What `use wasm_bindgen::prelude::*` brings.
pub mod prelude {
    pub use crate::{JsValue, UnwrapThrowExt, wasm_bindgen};
}

/// What `#[wasm_bindgen]`'s functions name, not to be named.
#[doc(hidden)]
pub mod __rt {
    pub use js;

    /// `#[wasm_bindgen(js_name = module)] static`, CommonJS's `module`, which
    /// an ES module has none of, served its `require` of Node's
    /// `createRequire(import.meta.url)`, as getrandom 0.1's `MODULE.require(
    /// "crypto")` asks: `{ require }`, read where it's used, after the
    /// browser's `self.crypto` it looks for first, so a browser's bundle
    /// imports nothing of Node's (the user's decision on #1400, #1404).
    pub fn node_module() -> Option<&'static js::Unknown> {
        #[cfg_attr(rust_js, rust_js::link_name = "process.getBuiltinModule")]
        fn builtin(name: &str) -> &'static js::Unknown {
            unreachable!()
        }
        #[cfg_attr(rust_js, rust_js::link_name = "get import.meta.url")]
        fn url() -> &'static str {
            unreachable!()
        }
        #[cfg_attr(rust_js, rust_js::link_name = "createRequire")]
        fn create_require(this: &'static js::Unknown, url: &str) -> &'static js::Unknown {
            unreachable!()
        }
        #[cfg_attr(rust_js, rust_js::link_name = "{require}")]
        fn holder(require: &'static js::Unknown) -> &'static js::Unknown {
            unreachable!()
        }
        Some(holder(create_require(builtin("module"), url())))
    }
}
