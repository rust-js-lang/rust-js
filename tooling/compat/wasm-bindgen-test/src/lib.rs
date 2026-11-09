//! rust-js's wasm-bindgen-test, for an app's dependencies only (ADR 1112):
//! `#[wasm_bindgen_test]` is a `#[test]`, which `rust-js test` runs as JS,
//! in the host its runner is, Node's or a browser's.

pub use wasm_bindgen_test_macro::wasm_bindgen_test;

/// `wasm_bindgen_test_configure!(run_in_browser)`: nothing. Where a test
/// runs is `rust-js test`'s runner's, which the crate's configuration
/// names, never a test's own (ADR 1112).
#[macro_export]
macro_rules! wasm_bindgen_test_configure {
    ($($any:tt)*) => {};
}
