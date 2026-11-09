//! rust-js's `#[wasm_bindgen]`, for an app's dependencies alone (ADR 1112).
//!
//! Of an `extern "C"` block, each function is a Rust function of its
//! signature that calls a rust-js binding of the JS it names, its
//! `js_namespace` and `js_name`, converting each argument and its result
//! as [`abi`] says, the one table. What it doesn't know, an attribute or a
//! type, is a compile error naming it, never a guess. An item it can't
//! bind, an export, is one too. Each error keeps the item's `#[cfg]`s, so a
//! function a `cfg` leaves out, getrandom's `atomics` one, is left out with
//! it.

mod abi;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as Tokens};
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, ExprArray, ExprLit, ForeignItem, ForeignItemFn, Ident, Item, ItemForeignMod, Lit, Token};

/// `#[wasm_bindgen]`: see the crate's documentation.
#[proc_macro_attribute]
pub fn wasm_bindgen(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = Tokens::from(attr);
    match syn::parse::<Item>(item) {
        Ok(Item::ForeignMod(block)) if attr.is_empty() => imports(block).into(),
        Ok(Item::ForeignMod(block)) => refused(
            &block.attrs,
            &attr,
            "`#[wasm_bindgen(..)]` of an `extern` block, its options".into(),
        )
        .into(),
        Ok(other) => {
            let attrs = item_attrs(&other);
            refused(
                &attrs,
                &other,
                "`#[wasm_bindgen]` of an item that isn't an `extern` block, which would export it to JS".into(),
            )
            .into()
        }
        Err(error) => error.to_compile_error().into(),
    }
}

/// Each function of an `extern "C"` block.
fn imports(block: ItemForeignMod) -> Tokens {
    block
        .items
        .into_iter()
        .map(|item| match item {
            ForeignItem::Fn(function) => {
                import(function).unwrap_or_else(|(attrs, error)| with_cfgs(&attrs, error.to_compile_error()))
            }
            ForeignItem::Type(ty) => refused(&ty.attrs, &ty, "an imported JS type".into()),
            ForeignItem::Static(item) => refused(&item.attrs, &item, "an imported JS value".into()),
            other => refused(&[], &other, "this item of an `extern` block".into()),
        })
        .collect()
}

/// One option of `#[wasm_bindgen(..)]`: `catch`, `js_name = getRandomValues`,
/// `js_namespace = ["globalThis", "crypto"]`.
struct Opt {
    key: Ident,
    value: Option<Expr>,
}

impl Parse for Opt {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key = Ident::parse_any(input)?;
        let value = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Opt { key, value })
    }
}

/// What a function's `#[wasm_bindgen(..)]`s say.
struct Import {
    namespace: Vec<String>,
    name: Option<String>,
    catch: bool,
}

/// An `extern` function, as a Rust function calling its binding; or its
/// `#[cfg]`s and why it can't be one.
fn import(function: ForeignItemFn) -> Result<Tokens, (Vec<Attribute>, syn::Error)> {
    let mut kept = Vec::new();
    let mut options = Import {
        namespace: Vec::new(),
        name: None,
        catch: false,
    };
    for attr in &function.attrs {
        if !attr.path().is_ident("wasm_bindgen") {
            kept.push(attr.clone());
            continue;
        }
        let opts = attr
            .parse_args_with(Punctuated::<Opt, Token![,]>::parse_terminated)
            .map_err(|error| (function.attrs.clone(), error))?;
        for opt in opts {
            let fail = |what: &str| (function.attrs.clone(), syn::Error::new_spanned(&opt.key, what));
            match (opt.key.to_string().as_str(), &opt.value) {
                ("catch", None) => options.catch = true,
                ("js_name", Some(value)) => {
                    options.name = Some(
                        name_of(value)
                            .ok_or_else(|| fail("rust-js's wasm-bindgen: `js_name` is a name or a string"))?,
                    )
                }
                ("js_namespace", Some(value)) => {
                    options.namespace = namespace_of(value).ok_or_else(|| {
                        fail("rust-js's wasm-bindgen: `js_namespace` is a name, a string or an array of strings")
                    })?
                }
                (key, _) => {
                    return Err(fail(&format!(
                        "rust-js's wasm-bindgen doesn't take `{key}` of an imported function yet: it binds a function by `js_namespace`, `js_name` and `catch` (ADR 1112)"
                    )));
                }
            }
        }
    }
    let ForeignItemFn { vis, sig, .. } = &function;
    let error = |error: syn::Error| (function.attrs.clone(), error);
    let js = [
        options.namespace.clone(),
        vec![options.name.clone().unwrap_or_else(|| sig.ident.to_string())],
    ]
    .concat()
    .join(".");
    let mut binding_params = Vec::new();
    let mut before = Vec::new();
    let mut args = Vec::new();
    let mut after = Vec::new();
    for (i, input) in sig.inputs.iter().enumerate() {
        let syn::FnArg::Typed(param) = input else {
            return Err(error(syn::Error::new_spanned(
                input,
                "rust-js's wasm-bindgen: an imported function has no `self`",
            )));
        };
        let syn::Pat::Ident(name) = &*param.pat else {
            return Err(error(syn::Error::new_spanned(
                &param.pat,
                "rust-js's wasm-bindgen: an imported function's parameter is a name",
            )));
        };
        let arg = abi::argument(&name.ident, &param.ty, i).map_err(error)?;
        binding_params.push(arg.binding_param);
        before.extend(arg.before);
        args.push(arg.passed);
        after.extend(arg.after);
    }
    let result = abi::result(&sig.output, options.catch).map_err(error)?;
    let binding_result = result.binding;
    let finish = result.finish;
    let binding = format_ident!("__wbg_{}", sig.ident);
    Ok(quote! {
        #(#kept)*
        #vis #sig {
            #[cfg_attr(rust_js, rust_js::link_name = #js)]
            fn #binding(#(#binding_params),*) #binding_result {
                unreachable!()
            }
            #(#before)*
            let __result = #binding(#(#args),*);
            #(#after)*
            #finish
        }
    })
}

/// A `js_name`: `getRandomValues` or `"getRandomValues"`.
fn name_of(value: &Expr) -> Option<String> {
    match value {
        Expr::Path(path) => path.path.get_ident().map(|ident| ident.unraw().to_string()),
        Expr::Lit(ExprLit {
            lit: Lit::Str(text), ..
        }) => Some(text.value()),
        _ => None,
    }
}

/// A `js_namespace`: `crypto`, `"crypto"` or `["globalThis", "crypto"]`.
fn namespace_of(value: &Expr) -> Option<Vec<String>> {
    match value {
        Expr::Array(ExprArray { elems, .. }) => elems.iter().map(name_of).collect(),
        other => name_of(other).map(|name| vec![name]),
    }
}

/// The `#[cfg]`s of `attrs`, which an error keeps.
fn with_cfgs(attrs: &[Attribute], error: Tokens) -> Tokens {
    let cfgs = attrs.iter().filter(|attr| attr.path().is_ident("cfg"));
    quote! { #(#cfgs)* const _: () = { #error }; }
}

/// A compile error of what rust-js's wasm-bindgen doesn't bind.
fn refused(attrs: &[Attribute], at: &dyn quote::ToTokens, what: String) -> Tokens {
    let message = format!(
        "rust-js's wasm-bindgen doesn't bind {what} yet: it binds what crates use, grown crate by crate (ADR 1112)"
    );
    let span = quote::ToTokens::to_token_stream(at)
        .into_iter()
        .next()
        .map_or_else(Span::call_site, |token| token.span());
    with_cfgs(attrs, syn::Error::new(span, message).to_compile_error())
}

/// An item's attributes, for its `#[cfg]`s.
fn item_attrs(item: &Item) -> Vec<Attribute> {
    match item {
        Item::Fn(item) => item.attrs.clone(),
        Item::Struct(item) => item.attrs.clone(),
        Item::Enum(item) => item.attrs.clone(),
        Item::Impl(item) => item.attrs.clone(),
        Item::Const(item) => item.attrs.clone(),
        Item::Static(item) => item.attrs.clone(),
        _ => Vec::new(),
    }
}
