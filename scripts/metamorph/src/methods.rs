//! `method-fn`: an inherent method's body moved into a free function its
//! receiver is given to, `fn area(&self) -> u32 { self.w * self.h }` →
//! `fn area(&self) -> u32 { metamorph_rect_area(self) }` and `fn
//! metamorph_rect_area(this: &Rect) -> u32 { this.w * this.h }`. And
//! `fn-method`, back: a free function whose first parameter is a reference
//! to one of the case's types made a method of it the function calls,
//! `fn area(r: &Rect) -> u32 { r.w * r.h }` → `fn area(r: &Rect) -> u32 {
//! r.metamorph_area() }` and `impl Rect { fn metamorph_area(&self) -> u32 {
//! self.w * self.h } }`.

use proc_macro2::{Group, Ident, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote};
use syn::visit::Visit;
use syn::{FnArg, ImplItem, Item, Pat, Type};

use crate::Bindings;

/// Whether `attrs` say anything of rust-js's: a binding's, a class's
/// getter's, which a function apart wouldn't be.
fn of_rust_js(attrs: &[syn::Attribute]) -> bool {
    (attrs.iter()).any(|attr| attr.to_token_stream().to_string().contains("rust_js"))
}

/// `stream` with each `ident` that names the local `from` made `to`'s
/// tokens: not a field, `a.from`, a path's segment, `m::from` or
/// `from::m`, a field named in a literal, `S { from: 1 }`, nor a macro.
fn renamed(stream: TokenStream, from: &str, to: &TokenStream) -> TokenStream {
    let trees: Vec<TokenTree> = stream.into_iter().collect();
    let mut out = Vec::new();
    for (i, tree) in trees.iter().enumerate() {
        let punct = |at: Option<&TokenTree>, c: char| matches!(at, Some(TokenTree::Punct(p)) if p.as_char() == c);
        let before = i.checked_sub(1).and_then(|i| trees.get(i));
        let after = trees.get(i + 1);
        match tree {
            TokenTree::Ident(id) if id == from => {
                if punct(before, '.') || punct(before, ':') || punct(after, ':') || punct(after, '!') {
                    out.push(tree.clone());
                } else {
                    out.extend(to.clone());
                }
            }
            TokenTree::Group(g) => {
                let mut group = Group::new(g.delimiter(), renamed(g.stream(), from, to));
                group.set_span(g.span());
                out.push(TokenTree::Group(group));
            }
            _ => out.push(tree.clone()),
        }
    }
    out.into_iter().collect()
}

/// `stream` with each `Self` made `ty`.
fn of_type(stream: TokenStream, ty: &Ident) -> TokenStream {
    stream
        .into_iter()
        .map(|tree| match tree {
            TokenTree::Ident(id) if id == "Self" => TokenTree::Ident(ty.clone()),
            TokenTree::Group(g) => {
                let mut group = Group::new(g.delimiter(), of_type(g.stream(), ty));
                group.set_span(g.span());
                TokenTree::Group(group)
            }
            other => other,
        })
        .collect()
}

/// A plain function's: not `const`, `async`, `unsafe` nor `extern`, of no
/// C-variadic.
fn plain(sig: &syn::Signature) -> bool {
    sig.constness.is_none()
        && sig.asyncness.is_none()
        && sig.unsafety.is_none()
        && sig.abi.is_none()
        && sig.variadic.is_none()
}

/// The names each of `inputs` but the first binds, each by itself.
fn given(inputs: impl Iterator<Item = FnArg>) -> Option<Vec<Ident>> {
    inputs
        .map(|input| match input {
            FnArg::Typed(t) => match *t.pat {
                Pat::Ident(p) if p.by_ref.is_none() && p.subpat.is_none() => Some(p.ident),
                _ => None,
            },
            FnArg::Receiver(_) => None,
        })
        .collect()
}

pub fn method_fn(ast: &mut syn::File, budget: &mut usize) -> bool {
    let mut made = Vec::new();
    for item in &mut ast.items {
        let Item::Impl(imp) = item else { continue };
        if imp.trait_.is_some() || !imp.generics.params.is_empty() || imp.unsafety.is_some() {
            continue;
        }
        let Type::Path(ty) = &*imp.self_ty else { continue };
        let Some(ty) = ty.path.get_ident().cloned() else {
            continue;
        };
        for item in &mut imp.items {
            let ImplItem::Fn(f) = item else { continue };
            if *budget == 0 || !plain(&f.sig) || of_rust_js(&f.attrs) {
                continue;
            }
            let Some(FnArg::Receiver(receiver)) = f.sig.inputs.first() else {
                continue;
            };
            if receiver.colon_token.is_some() {
                continue;
            }
            let Some(rest) = given(f.sig.inputs.iter().skip(1).cloned()) else {
                continue;
            };
            let consts = f.sig.generics.const_params().count();
            if consts > 0 {
                continue;
            }
            let this = match (&receiver.reference, receiver.mutability) {
                (Some((_, lifetime)), None) => quote!(this: & #lifetime #ty),
                (Some((_, lifetime)), Some(_)) => quote!(this: & #lifetime mut #ty),
                (None, None) => quote!(this: #ty),
                (None, Some(_)) => quote!(mut this: #ty),
            };
            let name = format_ident!("metamorph_{}_{}", ty.to_string().to_lowercase(), f.sig.ident);
            let others = f.sig.inputs.iter().skip(1);
            let (generics, output, clause) = (&f.sig.generics, &f.sig.output, &f.sig.generics.where_clause);
            let body = renamed(f.block.to_token_stream(), "self", &quote!(this));
            let signature = of_type(quote!(fn #name #generics (#this, #(#others),*) #output #clause), &ty);
            let body = of_type(body, &ty);
            made.push(syn::parse2::<Item>(quote!(#signature #body)).expect("a function"));
            f.block = syn::parse_quote!({ #name(self, #(#rest),*) });
            *budget -= 1;
        }
    }
    let changed = !made.is_empty();
    ast.items.extend(made);
    changed
}

pub fn fn_method(ast: &mut syn::File, budget: &mut usize) -> bool {
    // The case's own types of no generics, each a method may be of.
    let types: Vec<Ident> = (ast.items.iter())
        .filter_map(|item| match item {
            Item::Struct(s) if s.generics.params.is_empty() => Some(s.ident.clone()),
            Item::Enum(e) if e.generics.params.is_empty() => Some(e.ident.clone()),
            _ => None,
        })
        .collect();
    let mut made = Vec::new();
    for item in &mut ast.items {
        let Item::Fn(f) = item else { continue };
        if *budget == 0 || f.sig.ident == "main" || !plain(&f.sig) || of_rust_js(&f.attrs) {
            continue;
        }
        let Some(FnArg::Typed(first)) = f.sig.inputs.first() else {
            continue;
        };
        let Pat::Ident(p) = &*first.pat else { continue };
        if p.by_ref.is_some() || p.subpat.is_some() {
            continue;
        }
        let Type::Reference(r) = &*first.ty else { continue };
        let Type::Path(of) = &*r.elem else { continue };
        let Some(ty) = of.path.get_ident().filter(|ty| types.contains(ty)).cloned() else {
            continue;
        };
        let x = p.ident.clone();
        // Bound once, as the parameter: `self` is the only name it has.
        let mut bound = Bindings::default();
        for input in &f.sig.inputs {
            bound.visit_fn_arg(input);
        }
        bound.visit_block(&f.block);
        if bound.0.get(&x.to_string()) != Some(&1) || f.block.to_token_stream().to_string().contains("self") {
            continue;
        }
        let Some(rest) = given(f.sig.inputs.iter().skip(1).cloned()) else {
            continue;
        };
        let receiver = match (&r.lifetime, r.mutability) {
            (lifetime, None) => quote!(& #lifetime self),
            (lifetime, Some(_)) => quote!(& #lifetime mut self),
        };
        let name = format_ident!("metamorph_{}", f.sig.ident);
        let others = f.sig.inputs.iter().skip(1);
        let (generics, output, clause) = (&f.sig.generics, &f.sig.output, &f.sig.generics.where_clause);
        let body = renamed(f.block.to_token_stream(), &x.to_string(), &quote!(self));
        made.push(syn::parse_quote!(
            impl #ty {
                fn #name #generics (#receiver, #(#others),*) #output #clause #body
            }
        ));
        f.block = syn::parse_quote!({ #x.#name(#(#rest),*) });
        *budget -= 1;
    }
    let changed = !made.is_empty();
    ast.items.extend(made);
    changed
}
