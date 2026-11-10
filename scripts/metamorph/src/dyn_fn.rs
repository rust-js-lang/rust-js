//! `dyn-fn` (#1204): each closure called through a `Box<dyn Fn>`, as code
//! that keeps callbacks of many kinds in one place holds them: `|x| x + 1`
//! → `(metamorph_dyn::at_0(Box::new(|x: &i64| x + 1)) as Box<dyn Fn(&i64)
//! -> i64 + '_>)`, its types and how it's called, `Fn`, `FnMut` or
//! `FnOnce`, rustc's (its MIR's signature of the closure, `closures.rs`).
//! A place whose use needs the closure's own type, a `Clone` or a `Copy`
//! one, one a `fn` pointer is made of, is left as it was: rustc's errors
//! say which (`narrow.rs`).

use quote::{ToTokens, format_ident, quote};
use std::collections::{BTreeMap, BTreeSet};
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, Pat};

use crate::closures::{Signature, compares_identity, signatures, writable};
use crate::narrow;

const MODULE: &str = "metamorph_dyn";

struct Wrap<'s> {
    signatures: &'s BTreeMap<(usize, usize), Signature>,
    /// The places left as they were.
    off: &'s BTreeSet<usize>,
    budget: usize,
    count: usize,
    made: usize,
}

impl Wrap<'_> {
    /// The closure annotated with its parameters' types, and the `dyn` it's
    /// called through, or `None` where rustc's types can't be written.
    fn boxed(&self, closure: &syn::ExprClosure) -> Option<(syn::ExprClosure, syn::Type)> {
        if closure.asyncness.is_some() || closure.constness.is_some() || closure.lifetimes.is_some() {
            return None;
        }
        let start = closure.span().start();
        let sig = self.signatures.get(&(start.line, start.column + 1))?;
        let none = BTreeSet::new();
        if sig.inputs.len() != closure.inputs.len()
            || !sig.inputs.iter().all(|t| writable(t, &none))
            || !writable(&sig.output, &none)
        {
            return None;
        }
        let mut annotated = closure.clone();
        let mut inputs = Vec::new();
        for (pat, ty) in annotated.inputs.iter_mut().zip(&sig.inputs) {
            let ty: syn::Type = syn::parse_str(ty).ok()?;
            if !matches!(pat, Pat::Type(_)) {
                *pat = Pat::Type(syn::PatType {
                    attrs: Vec::new(),
                    pat: Box::new(pat.clone()),
                    colon_token: Default::default(),
                    ty: Box::new(ty.clone()),
                });
            }
            inputs.push(ty);
        }
        let kind = format_ident!("{}", sig.kind);
        let output = match sig.output.as_str() {
            "()" => quote!(),
            out => {
                let ty: syn::Type = syn::parse_str(out).ok()?;
                quote!(-> #ty)
            }
        };
        let dyn_ty: syn::Type = syn::parse2(quote!(::std::boxed::Box<dyn #kind(#(#inputs),*) #output + '_>)).ok()?;
        Some((annotated, dyn_ty))
    }
}

impl VisitMut for Wrap<'_> {
    fn visit_expr_mut(&mut self, e: &mut Expr) {
        visit_mut::visit_expr_mut(self, e);
        let Expr::Closure(closure) = e else { return };
        let Some((annotated, dyn_ty)) = self.boxed(closure) else {
            return;
        };
        let k = self.count;
        self.count += 1;
        if k >= self.budget || self.off.contains(&k) {
            return;
        }
        self.made += 1;
        let module = format_ident!("{MODULE}");
        let at = format_ident!("at_{k}");
        *e = syn::parse_quote!((#module::#at(::std::boxed::Box::new(#annotated)) as #dyn_ty));
    }

    fn visit_item_mod_mut(&mut self, m: &mut syn::ItemMod) {
        visit_mut::visit_item_mod_mut(self, m);
        if let Some((_, items)) = &mut m.content {
            let module = format_ident!("{MODULE}");
            items.insert(0, syn::parse_quote!(#[allow(unused_imports)] use super::#module;));
        }
    }
}

fn made(
    ast: &syn::File,
    signatures: &BTreeMap<(usize, usize), Signature>,
    off: &BTreeSet<usize>,
    budget: usize,
) -> (syn::File, usize) {
    let mut ast = ast.clone();
    let mut wrap = Wrap {
        signatures,
        off,
        budget,
        count: 0,
        made: 0,
    };
    wrap.visit_file_mut(&mut ast);
    let fns = (0..wrap.count).filter(|k| !off.contains(k) && *k < budget).map(|k| {
        let at = format_ident!("at_{k}");
        quote!(pub fn #at<T>(it: T) -> T { it })
    });
    let module = format_ident!("{MODULE}");
    let fns: Vec<_> = fns.collect();
    ast.items.push(syn::parse_quote!(
        #[allow(dead_code)]
        mod #module {
            #(#fns)*
        }
    ));
    (ast, wrap.made)
}

pub fn dyn_fn(ast: &mut syn::File, file: &str, source: &str, budget: &mut usize) -> bool {
    // Not a case with a binding, `rust_js::link_name`: a boxed closure it
    // gives JS, which Rust never drops (ADR 0873, the guide's decided
    // difference 12), its native stub drops.
    if compares_identity(source) || !source.contains('|') || source.contains("rust_js::link_name") {
        return false;
    }
    let signatures = signatures(file);
    let mut off = BTreeSet::new();
    for _ in 0..24 {
        let (program, count) = made(ast, &signatures, &off, *budget);
        if count == 0 {
            return false;
        }
        let text = program.to_token_stream().to_string();
        let Some(errors) = narrow::diagnostics(&text) else {
            *ast = program;
            return true;
        };
        let sites = narrow::sites(&text, MODULE);
        let before = off.len();
        for error in &errors {
            off.extend(narrow::places_of(&text, &sites, error));
        }
        if off.len() == before {
            return false;
        }
    }
    false
}
