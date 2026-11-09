//! What wasm-bindgen's ABI passes to an imported JS function, and gives
//! back, as rust-js's values: the one table (ADR 1112).
//!
//! Each argument is passed as JS sees it under wasm-bindgen: a number, a
//! `bool` or text as it is; a slice of bytes as a `Uint8Array` of them,
//! made once, `new Uint8Array(len)` or `Uint8Array.from(bytes)`, and of a
//! `&mut` one, what JS wrote copied back once, after the call. wasm-bindgen
//! gives JS a view of wasm's memory; a copy is the same to every Rust
//! program, as what JS does with the view after the call, the borrow over,
//! Rust leaves undefined (ADR 0262 case N). A `catch` function's
//! `Result<T, JsValue>` is the call in a `try`, what JS threw its `Err`.
//! Any other type is a compile error naming it.

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Ident, ReturnType, Type};

/// An argument: its binding's parameter, what's done before the call, what
/// the binding is given, and what's done after.
pub struct Argument {
    pub binding_param: TokenStream,
    pub before: Option<TokenStream>,
    pub passed: TokenStream,
    pub after: Option<TokenStream>,
}

/// A result: the binding's, and what the function gives of `__result`.
pub struct Outcome {
    pub binding: TokenStream,
    pub finish: TokenStream,
}

const PASSED: [&str; 10] = ["u8", "u16", "u32", "i8", "i16", "i32", "f32", "f64", "bool", "usize"];

fn unsupported(ty: &dyn ToTokens, at: &str) -> syn::Error {
    let shown = ty.to_token_stream().to_string().replace(" ", "");
    syn::Error::new_spanned(
        ty,
        format!(
            "rust-js's wasm-bindgen doesn't pass `{shown}` {at} yet: it passes numbers, `bool`, `&str`, `&[u8]`, `&mut [u8]` and `&mut [MaybeUninit<u8>]` (ADR 1112)"
        ),
    )
}

/// The last segment of a path type, `MaybeUninit` of `core::mem::MaybeUninit<u8>`.
fn last(ty: &Type) -> Option<&syn::PathSegment> {
    match ty {
        Type::Path(path) if path.qself.is_none() => path.path.segments.last(),
        _ => None,
    }
}

fn is_u8(ty: &Type) -> bool {
    last(ty).is_some_and(|segment| segment.ident == "u8" && segment.arguments.is_empty())
}

/// `MaybeUninit<u8>`.
fn is_uninit_u8(ty: &Type) -> bool {
    let Some(segment) = last(ty) else { return false };
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return false;
    };
    segment.ident == "MaybeUninit"
        && matches!(args.args.first(), Some(syn::GenericArgument::Type(item)) if is_u8(item))
        && args.args.len() == 1
}

/// The argument `name`, the `index`th, of type `ty`.
pub fn argument(name: &Ident, ty: &Type, index: usize) -> syn::Result<Argument> {
    let view = format_ident!("__view{index}");
    let rt = quote!(::wasm_bindgen::__rt::js);
    if let Type::Reference(reference) = ty
        && let Type::Slice(slice) = &*reference.elem
    {
        let item = &*slice.elem;
        let binding_param = quote!(#view: &#rt::Uint8Array);
        return match (reference.mutability.is_some(), is_u8(item), is_uninit_u8(item)) {
            (false, true, _) => Ok(Argument {
                binding_param,
                before: Some(quote!(let #view = #rt::uint8_array::from(#name);)),
                passed: quote!(#view),
                after: None,
            }),
            (true, true, _) => Ok(Argument {
                binding_param,
                before: Some(quote!(let #view = #rt::uint8_array::from(&*#name);)),
                passed: quote!(#view),
                after: Some(quote! {
                    for (__i, __slot) in #name.iter_mut().enumerate() {
                        *__slot = #view.get(__i as f64).unwrap_or(0.0) as u8;
                    }
                }),
            }),
            // Bytes not yet written, which JS is to fill: none read.
            (true, false, true) => Ok(Argument {
                binding_param,
                before: Some(quote!(let #view = #rt::uint8_array::new(#name.len() as f64);)),
                passed: quote!(#view),
                after: Some(quote! {
                    for (__i, __slot) in #name.iter_mut().enumerate() {
                        *__slot = ::core::mem::MaybeUninit::new(#view.get(__i as f64).unwrap_or(0.0) as u8);
                    }
                }),
            }),
            _ => Err(unsupported(ty, "to JS")),
        };
    }
    let passed = match ty {
        Type::Reference(reference)
            if reference.mutability.is_none() && last(&reference.elem).is_some_and(|s| s.ident == "str") =>
        {
            true
        }
        other => last(other).is_some_and(|segment| {
            segment.arguments.is_empty() && PASSED.contains(&segment.ident.to_string().as_str())
        }),
    };
    if !passed {
        return Err(unsupported(ty, "to JS"));
    }
    Ok(Argument {
        binding_param: quote!(#name: #ty),
        before: None,
        passed: quote!(#name),
        after: None,
    })
}

/// What a function gives, of `output`, a `catch` one's `Result<T, JsValue>`.
pub fn result(output: &ReturnType, catch: bool) -> syn::Result<Outcome> {
    let ty = match output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => Some(&**ty),
    };
    if !catch {
        return match ty {
            None => Ok(Outcome {
                binding: quote!(),
                finish: quote!(__result),
            }),
            Some(ty) => {
                given(ty)?;
                Ok(Outcome {
                    binding: quote!(-> #ty),
                    finish: quote!(__result),
                })
            }
        };
    }
    // `Result<T, JsValue>`, its `T` given as it is.
    let ok = ty
        .and_then(last)
        .filter(|segment| segment.ident == "Result")
        .and_then(|segment| match &segment.arguments {
            syn::PathArguments::AngleBracketed(args) if args.args.len() == 2 => match (&args.args[0], &args.args[1]) {
                (syn::GenericArgument::Type(ok), syn::GenericArgument::Type(err))
                    if last(err).is_some_and(|s| s.ident == "JsValue") =>
                {
                    Some(ok)
                }
                _ => None,
            },
            _ => None,
        });
    let Some(ok) = ok else {
        return Err(syn::Error::new_spanned(
            output,
            "rust-js's wasm-bindgen: a `catch` function gives `Result<T, JsValue>`",
        ));
    };
    given(ok)?;
    Ok(Outcome {
        binding: quote!(-> ::core::result::Result<#ok, &'static ::wasm_bindgen::__rt::js::JsError>),
        finish: quote!(__result.map_err(|__thrown| ::wasm_bindgen::JsValue {
            __value: __thrown.as_unknown()
        })),
    })
}

/// A type JS gives that's passed as it is: `()`, a number or a `bool`.
fn given(ty: &Type) -> syn::Result<()> {
    match ty {
        Type::Tuple(tuple) if tuple.elems.is_empty() => Ok(()),
        other
            if last(other).is_some_and(|segment| {
                segment.arguments.is_empty() && PASSED.contains(&segment.ident.to_string().as_str())
            }) =>
        {
            Ok(())
        }
        other => Err(unsupported(other, "from JS")),
    }
}
