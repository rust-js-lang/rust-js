//! rust-js's wasm-bindgen: wasm-bindgen's API, as crates use it, for an
//! app's dependencies only (ADR 1112). An app's own code binds JS with the
//! `js` and `webapi` crates.
//!
//! `#[wasm_bindgen]` of an `extern "C"` block makes each function a call of
//! the JS it names, as `wasm-bindgen-macro`'s table converts it; a
//! [`JsValue`] is any JS value, what a `catch` function's call threw.

pub use wasm_bindgen_macro::wasm_bindgen;

/// Any JS value, `undefined` too: what JS threw at a `catch` function. The
/// value itself, as its one field is (ADR 0584).
#[repr(transparent)]
pub struct JsValue {
    /// The value, `None` where it's `undefined`: what `#[wasm_bindgen]`'s
    /// functions make, not to be named.
    #[doc(hidden)]
    pub __value: Option<&'static js::Unknown>,
}

/// What `use wasm_bindgen::prelude::*` brings.
pub mod prelude {
    pub use crate::{JsValue, wasm_bindgen};
}

/// What `#[wasm_bindgen]`'s functions name, not to be named.
#[doc(hidden)]
pub mod __rt {
    pub use js;
}
