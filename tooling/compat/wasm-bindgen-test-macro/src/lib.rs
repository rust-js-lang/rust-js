//! rust-js's `#[wasm_bindgen_test]`, for an app's dependencies only (ADR
//! 1112): the function a `#[test]`, which `rust-js test` runs as JS. An
//! `async` one, which `rust-js test` doesn't run yet, or one of options,
//! `unsupported = test`, is a compile error saying so, never a test left out.

use proc_macro::TokenStream;
use quote::quote;
use syn::ItemFn;

/// `#[wasm_bindgen_test]`: see the crate's documentation.
#[proc_macro_attribute]
pub fn wasm_bindgen_test(attr: TokenStream, item: TokenStream) -> TokenStream {
    let function = match syn::parse::<ItemFn>(item) {
        Ok(function) => function,
        Err(error) => return error.to_compile_error().into(),
    };
    let attr = proc_macro2::TokenStream::from(attr);
    if !attr.is_empty() {
        let message = "rust-js's wasm-bindgen-test doesn't take a test's options yet: `#[wasm_bindgen_test]` is a `#[test]` (ADR 1112)";
        return syn::Error::new_spanned(attr, message).to_compile_error().into();
    }
    if let Some(asyncness) = function.sig.asyncness {
        let message =
            "rust-js's wasm-bindgen-test doesn't run an `async` test yet, as `rust-js test` runs none (ADR 1112)";
        return syn::Error::new_spanned(asyncness, message).to_compile_error().into();
    }
    quote! {
        #[test]
        #function
    }
    .into()
}
