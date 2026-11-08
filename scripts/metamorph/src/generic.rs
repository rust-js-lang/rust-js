//! `generic`: a free function's concrete parameter type made a type
//! parameter, `fn f(x: u32)` → `fn f<MetamorphT: Copy + Add<Output =
//! MetamorphT> + ..>(x: MetamorphT)`, bound by what the type has that a
//! body may ask, and each use of the function given that type,
//! `f::<u32>(..)`, so it's the same function, now generic code.

use proc_macro2::{Ident, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use std::collections::{BTreeMap, BTreeSet};
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, FnArg, GenericArgument, Item, PathArguments, ReturnType, Type};

use crate::bound_to_js;

const UNSIGNED: &[&str] = &["u8", "u16", "u32", "u64", "u128", "usize"];
const SIGNED: &[&str] = &["i8", "i16", "i32", "i64", "i128", "isize"];
const FLOATS: &[&str] = &["f32", "f64"];

/// The bounds a type parameter standing for `ty` is given: each trait of
/// std's the type has that a body may ask of it, and each of the case's
/// own impls of it, `None` where it's no type this rewrites.
fn bounds(ty: &str, own: &BTreeMap<String, Vec<TokenStream>>) -> Option<Vec<TokenStream>> {
    let t = quote!(MetamorphT);
    let common = || {
        vec![
            quote!(::std::clone::Clone),
            quote!(::std::fmt::Debug),
            quote!(::std::fmt::Display),
            quote!(::std::cmp::PartialEq),
            quote!(::std::cmp::PartialOrd),
            quote!(::std::default::Default),
        ]
    };
    let total = || {
        vec![
            quote!(::std::cmp::Eq),
            quote!(::std::cmp::Ord),
            quote!(::std::hash::Hash),
        ]
    };
    let arithmetic = || {
        let mut b = Vec::new();
        for op in ["Add", "Sub", "Mul", "Div", "Rem"] {
            let op = Ident::new(op, proc_macro2::Span::call_site());
            let assign = Ident::new(&format!("{op}Assign"), proc_macro2::Span::call_site());
            b.push(quote!(::std::ops::#op<Output = #t>));
            b.push(quote!(::std::ops::#assign));
        }
        b
    };
    let bits = || {
        vec![
            quote!(::std::ops::BitAnd<Output = #t>),
            quote!(::std::ops::BitOr<Output = #t>),
            quote!(::std::ops::BitXor<Output = #t>),
            quote!(::std::ops::Not<Output = #t>),
        ]
    };
    let neg = quote!(::std::ops::Neg<Output = #t>);
    let copy = quote!(::std::marker::Copy);
    Some(match ty {
        _ if UNSIGNED.contains(&ty) => [vec![copy], common(), total(), arithmetic(), bits()].concat(),
        _ if SIGNED.contains(&ty) => [vec![copy], common(), total(), arithmetic(), bits(), vec![neg]].concat(),
        _ if FLOATS.contains(&ty) => [vec![copy], common(), arithmetic(), vec![neg]].concat(),
        "bool" => [vec![copy], common(), total(), bits()].concat(),
        "char" => [vec![copy], common(), total()].concat(),
        "String" => [common(), total()].concat(),
        _ => own.get(ty)?.clone(),
    })
}

/// The case's own types this rewrites, each with its bounds: a struct or
/// an enum of no generics, bound by what it derives and by each impl of a
/// trait of the case's or std's for it of no associated type.
fn own_types(ast: &syn::File) -> BTreeMap<String, Vec<TokenStream>> {
    let mut own = BTreeMap::new();
    for item in &ast.items {
        let (ident, generics, attrs) = match item {
            Item::Struct(s) => (&s.ident, &s.generics, &s.attrs),
            Item::Enum(e) => (&e.ident, &e.generics, &e.attrs),
            _ => continue,
        };
        if !generics.params.is_empty() {
            continue;
        }
        let mut b = Vec::new();
        for attr in attrs.iter().filter(|a| a.path().is_ident("derive")) {
            let _ = attr.parse_nested_meta(|meta| {
                let path = &meta.path;
                let name = path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
                let full = match name.as_str() {
                    "Clone" => quote!(::std::clone::Clone),
                    "Copy" => quote!(::std::marker::Copy),
                    "Debug" => quote!(::std::fmt::Debug),
                    "PartialEq" => quote!(::std::cmp::PartialEq),
                    "Eq" => quote!(::std::cmp::Eq),
                    "PartialOrd" => quote!(::std::cmp::PartialOrd),
                    "Ord" => quote!(::std::cmp::Ord),
                    "Hash" => quote!(::std::hash::Hash),
                    "Default" => quote!(::std::default::Default),
                    _ => return Ok(()),
                };
                b.push(full);
                Ok(())
            });
        }
        own.insert(ident.to_string(), b);
    }
    for item in &ast.items {
        let Item::Impl(imp) = item else { continue };
        let Some((None, trait_path, _)) = &imp.trait_ else {
            continue;
        };
        let Type::Path(self_ty) = &*imp.self_ty else {
            continue;
        };
        let Some(name) = self_ty.path.get_ident().map(|i| i.to_string()) else {
            continue;
        };
        let mentions = |tokens: TokenStream| {
            tokens
                .into_iter()
                .any(|t| matches!(&t, TokenTree::Ident(i) if *i == "Self" || *i == name))
        };
        let assoc = imp.items.iter().any(|i| !matches!(i, syn::ImplItem::Fn(_)));
        let last = trait_path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if !imp.generics.params.is_empty() || assoc || mentions(trait_path.to_token_stream()) || last == "Drop" {
            continue;
        }
        if let Some(b) = own.get_mut(&name) {
            b.push(trait_path.to_token_stream());
        }
    }
    own
}

/// The first type `ty` names, in order, that `bounds` knows: a path of one
/// segment, `u32`, `String` or the case's own `Point`.
fn first_known(ty: &Type, known: &dyn Fn(&str) -> bool) -> Option<String> {
    struct First<'k> {
        known: &'k dyn Fn(&str) -> bool,
        found: Option<String>,
    }
    impl<'a> Visit<'a> for First<'_> {
        fn visit_type_path(&mut self, p: &'a syn::TypePath) {
            if self.found.is_some() {
                return;
            }
            if p.qself.is_none()
                && let Some(ident) = p.path.get_ident()
                && (self.known)(&ident.to_string())
            {
                self.found = Some(ident.to_string());
                return;
            }
            visit::visit_type_path(self, p);
        }
        // `impl Fn(u32)`: no turbofish is given a function of one.
        fn visit_type_impl_trait(&mut self, _: &'a syn::TypeImplTrait) {}
    }
    let mut first = First { known, found: None };
    first.visit_type(ty);
    first.found
}

/// Each `ty` in a type made `MetamorphT`.
struct Replace<'n> {
    ty: &'n str,
}

impl VisitMut for Replace<'_> {
    fn visit_type_mut(&mut self, t: &mut Type) {
        if let Type::Path(p) = t
            && p.qself.is_none()
            && p.path.is_ident(self.ty)
        {
            *t = syn::parse_quote!(MetamorphT);
            return;
        }
        visit_mut::visit_type_mut(self, t);
    }
}

/// A function made generic: how many type parameters it had, and the type
/// its callers give it.
struct Made {
    params: usize,
    ty: String,
}

/// Each name bound as a local anywhere in the file: a function of one of
/// those names isn't rewritten, as a path of its name may be the local.
#[derive(Default)]
struct Locals(BTreeSet<String>);

impl<'a> Visit<'a> for Locals {
    fn visit_pat_ident(&mut self, p: &'a syn::PatIdent) {
        self.0.insert(p.ident.to_string());
        visit::visit_pat_ident(self, p);
    }
}

pub fn generic(ast: &mut syn::File, budget: &mut usize) -> bool {
    let own = own_types(ast);
    let mut locals = Locals::default();
    locals.visit_file(ast);
    let known = |name: &str| bounds(name, &own).is_some();
    let mut made = BTreeMap::new();
    for item in &mut ast.items {
        let Item::Fn(f) = item else { continue };
        let sig = &f.sig;
        let impl_param = sig.inputs.iter().any(|i| {
            matches!(i, FnArg::Typed(t) if t.ty.to_token_stream().into_iter().any(|t| matches!(t, TokenTree::Ident(i) if i == "impl")))
        });
        let const_param = sig
            .generics
            .params
            .iter()
            .any(|p| matches!(p, syn::GenericParam::Const(_)));
        if *budget == 0
            || sig.ident == "main"
            || sig.constness.is_some()
            || sig.abi.is_some()
            || bound_to_js(f)
            || impl_param
            || const_param
            || locals.0.contains(&sig.ident.to_string())
        {
            continue;
        }
        let Some(ty) = sig.inputs.iter().find_map(|i| match i {
            FnArg::Typed(t) => first_known(&t.ty, &known),
            FnArg::Receiver(_) => None,
        }) else {
            continue;
        };
        let bounds = bounds(&ty, &own).expect("a known type");
        let params = sig.generics.type_params().count();
        let sig = &mut f.sig;
        let mut replace = Replace { ty: &ty };
        for input in &mut sig.inputs {
            if let FnArg::Typed(t) = input {
                replace.visit_type_mut(&mut t.ty);
            }
        }
        if let ReturnType::Type(_, t) = &mut sig.output {
            replace.visit_type_mut(t);
        }
        let param: syn::GenericParam = if bounds.is_empty() {
            syn::parse_quote!(MetamorphT)
        } else {
            syn::parse_quote!(MetamorphT: #(#bounds)+*)
        };
        sig.generics.params.push(param);
        made.insert(sig.ident.to_string(), Made { params, ty });
        *budget -= 1;
    }
    if made.is_empty() {
        return false;
    }
    // Each use given its type: in the function itself, its own parameter.
    for item in &mut ast.items {
        let own_name = match item {
            Item::Fn(f) if made.contains_key(&f.sig.ident.to_string()) => Some(f.sig.ident.to_string()),
            _ => None,
        };
        Given {
            made: &made,
            inside: own_name,
        }
        .visit_item_mut(item);
    }
    true
}

/// Each use of a function made generic given the type it was written of.
struct Given<'m> {
    made: &'m BTreeMap<String, Made>,
    /// The function whose body this is, whose own uses give its parameter.
    inside: Option<String>,
}

impl Given<'_> {
    /// `name`'s turbofish's argument: its parameter inside itself, else the
    /// type it was written of.
    fn argument(&self, name: &str, made: &Made) -> Type {
        if self.inside.as_deref() == Some(name) {
            syn::parse_quote!(MetamorphT)
        } else {
            let ty = Ident::new(&made.ty, proc_macro2::Span::call_site());
            syn::parse_quote!(#ty)
        }
    }

    fn tokens(&self, stream: TokenStream) -> TokenStream {
        let trees: Vec<TokenTree> = stream.into_iter().collect();
        let mut out = Vec::new();
        for (i, tree) in trees.iter().enumerate() {
            let punct = |at: Option<&TokenTree>, c: char| matches!(at, Some(TokenTree::Punct(p)) if p.as_char() == c);
            match tree {
                TokenTree::Ident(id) if self.made.contains_key(&id.to_string()) => {
                    let before = i.checked_sub(1).and_then(|i| trees.get(i));
                    let after = trees.get(i + 1);
                    let keyword = matches!(before, Some(TokenTree::Ident(k)) if k == "fn");
                    out.push(tree.clone());
                    if !(keyword || punct(before, '.') || punct(before, ':') || punct(after, ':') || punct(after, '!'))
                    {
                        let made = &self.made[&id.to_string()];
                        let ty = self.argument(&id.to_string(), made);
                        let inferred = (0..made.params).map(|_| quote!(_));
                        out.extend(quote!(::<#(#inferred,)* #ty>));
                    }
                }
                TokenTree::Group(g) => {
                    let mut group = proc_macro2::Group::new(g.delimiter(), self.tokens(g.stream()));
                    group.set_span(g.span());
                    out.push(TokenTree::Group(group));
                }
                _ => out.push(tree.clone()),
            }
        }
        out.into_iter().collect()
    }
}

impl VisitMut for Given<'_> {
    fn visit_expr_mut(&mut self, e: &mut Expr) {
        if let Expr::Path(p) = e
            && p.qself.is_none()
        {
            let segments = &p.path.segments;
            let local = segments.len() == 1
                || (segments.len() == 2 && ["crate", "self"].contains(&segments[0].ident.to_string().as_str()));
            let name = segments.last().expect("a segment").ident.to_string();
            if local && let Some(made) = self.made.get(&name) {
                let ty = self.argument(&name, made);
                let last = p.path.segments.last_mut().expect("a segment");
                match &mut last.arguments {
                    PathArguments::AngleBracketed(args) => args.args.push(GenericArgument::Type(ty)),
                    _ => {
                        let inferred = (0..made.params).map(|_| quote!(_));
                        last.arguments = PathArguments::AngleBracketed(syn::parse_quote!(::<#(#inferred,)* #ty>));
                    }
                }
                return;
            }
        }
        visit_mut::visit_expr_mut(self, e);
    }
    fn visit_macro_mut(&mut self, m: &mut syn::Macro) {
        m.tokens = self.tokens(std::mem::take(&mut m.tokens));
    }
}
