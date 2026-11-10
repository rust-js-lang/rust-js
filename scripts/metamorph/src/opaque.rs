//! `opaque` (#1204): each iterator the case makes, `v.iter()`,
//! `it.map(f)`, given on as an `impl Iterator`, an opaque type, as a
//! function that returns one gives it: `metamorph_opaque::at_0(v.iter())`,
//! of `fn at_0<I: Iterator + DoubleEndedIterator + ..>(it: I) -> impl
//! Iterator<Item = I::Item> + DoubleEndedIterator + .. { it }`. What it
//! promises besides `Iterator` is what the iterator is, of
//! `DoubleEndedIterator`, `ExactSizeIterator`, `FusedIterator`, `Clone`
//! and `Debug`: rustc's errors take each it isn't off, and a place whose
//! use needs more, a `Peekable`'s `peek`, a type written out, is left as
//! it was (`narrow.rs`). The lowering's rules decide on what an iterator
//! is, and an opaque type is one it has to reveal (#1142).

use quote::{ToTokens, format_ident, quote};
use syn::Expr;
use syn::visit_mut::{self, VisitMut};

use crate::narrow;

/// The methods whose result is an iterator, of std's types and traits.
pub(crate) const MAKERS: &[&str] = &[
    "iter",
    "iter_mut",
    "into_iter",
    "map",
    "filter",
    "filter_map",
    "enumerate",
    "rev",
    "skip",
    "take",
    "chain",
    "zip",
    "flat_map",
    "flatten",
    "take_while",
    "skip_while",
    "map_while",
    "step_by",
    "cloned",
    "copied",
    "inspect",
    "scan",
    "peekable",
    "fuse",
    "chars",
    "char_indices",
    "bytes",
    "lines",
    "split",
    "split_whitespace",
    "keys",
    "values",
    "values_mut",
    "into_keys",
    "into_values",
    "drain",
    "windows",
    "chunks",
    "chunks_exact",
    "rchunks",
    "rsplit",
    "splitn",
    "split_terminator",
    "matches",
    "cycle",
];

/// What an opaque type may promise besides `Iterator`.
const PROMISES: &[(&str, &str)] = &[
    ("DoubleEndedIterator", "::std::iter::DoubleEndedIterator"),
    ("ExactSizeIterator", "::std::iter::ExactSizeIterator"),
    ("FusedIterator", "::std::iter::FusedIterator"),
    ("Clone", "::std::clone::Clone"),
    ("Debug", "::std::fmt::Debug"),
];

const MODULE: &str = "metamorph_opaque";

/// Each place's promises, by `PROMISES`' order, or `None` where it's left
/// as it was.
struct Places {
    promised: Vec<Option<Vec<bool>>>,
    budget: usize,
}

/// Each iterator the case makes, wrapped in its place's call, of those
/// `places` keeps: numbered in the order they're met.
struct Wrap<'p> {
    places: &'p Places,
    count: usize,
    depth_const: usize,
}

impl VisitMut for Wrap<'_> {
    fn visit_expr_mut(&mut self, e: &mut Expr) {
        visit_mut::visit_expr_mut(self, e);
        if self.depth_const > 0 {
            return;
        }
        let Expr::MethodCall(call) = e else { return };
        if !MAKERS.contains(&call.method.to_string().as_str()) {
            return;
        }
        let k = self.count;
        self.count += 1;
        if k >= self.places.budget || self.places.promised.get(k).is_some_and(Option::is_none) {
            return;
        }
        let module = format_ident!("{MODULE}");
        let at = format_ident!("at_{k}");
        *e = syn::parse_quote!(#module::#at(#call));
    }

    // What a `const` computes is no call's.
    fn visit_item_const_mut(&mut self, c: &mut syn::ItemConst) {
        self.depth_const += 1;
        visit_mut::visit_item_const_mut(self, c);
        self.depth_const -= 1;
    }

    fn visit_item_static_mut(&mut self, s: &mut syn::ItemStatic) {
        self.depth_const += 1;
        visit_mut::visit_item_static_mut(self, s);
        self.depth_const -= 1;
    }

    fn visit_item_fn_mut(&mut self, f: &mut syn::ItemFn) {
        let constant = f.sig.constness.is_some();
        self.depth_const += usize::from(constant);
        visit_mut::visit_item_fn_mut(self, f);
        self.depth_const -= usize::from(constant);
    }

    fn visit_impl_item_fn_mut(&mut self, f: &mut syn::ImplItemFn) {
        let constant = f.sig.constness.is_some();
        self.depth_const += usize::from(constant);
        visit_mut::visit_impl_item_fn_mut(self, f);
        self.depth_const -= usize::from(constant);
    }

    fn visit_expr_const_mut(&mut self, c: &mut syn::ExprConst) {
        self.depth_const += 1;
        visit_mut::visit_expr_const_mut(self, c);
        self.depth_const -= 1;
    }

    fn visit_item_mod_mut(&mut self, m: &mut syn::ItemMod) {
        visit_mut::visit_item_mod_mut(self, m);
        // The places' module is the crate root's, which a module asks its
        // parent for (the native harness includes the case in a module).
        if let Some((_, items)) = &mut m.content {
            let module = format_ident!("{MODULE}");
            items.insert(0, syn::parse_quote!(#[allow(unused_imports)] use super::#module;));
        }
    }
}

/// `ast` with each place `places` keeps wrapped, and the module of their
/// functions; and how many places it met.
fn made(ast: &syn::File, places: &Places) -> (syn::File, usize) {
    let mut ast = ast.clone();
    let mut wrap = Wrap {
        places,
        count: 0,
        depth_const: 0,
    };
    wrap.visit_file_mut(&mut ast);
    let fns = (0..wrap.count).filter_map(|k| {
        let promised = places.promised.get(k).cloned().unwrap_or_else(|| Some(vec![true; PROMISES.len()]))?;
        if k >= places.budget {
            return None;
        }
        let bounds: Vec<syn::Path> = (PROMISES.iter().zip(&promised))
            .filter(|(_, kept)| **kept)
            .map(|((_, path), _)| syn::parse_str(path).expect("a trait's path"))
            .collect();
        let at = format_ident!("at_{k}");
        Some(quote!(
            pub fn #at<I: ::std::iter::Iterator #(+ #bounds)*>(it: I) -> impl ::std::iter::Iterator<Item = I::Item> #(+ #bounds)* {
                it
            }
        ))
    });
    let module = format_ident!("{MODULE}");
    let fns: Vec<_> = fns.collect();
    ast.items.push(syn::parse_quote!(
        #[allow(dead_code)]
        mod #module {
            #(#fns)*
        }
    ));
    (ast, wrap.count)
}

pub fn opaque(ast: &mut syn::File, budget: &mut usize) -> bool {
    let mut places = Places {
        promised: Vec::new(),
        budget: *budget,
    };
    let (_, count) = made(ast, &places);
    if count == 0 {
        return false;
    }
    places.promised = vec![Some(vec![true; PROMISES.len()]); count];
    for _ in 0..24 {
        let (program, _) = made(ast, &places);
        let text = program.to_token_stream().to_string();
        let Some(errors) = narrow::diagnostics(&text) else {
            *ast = program;
            return places.promised.iter().take(places.budget).any(Option::is_some);
        };
        let sites = narrow::sites(&text, MODULE);
        let mut changed = false;
        for error in &errors {
            for k in narrow::places_of(&text, &sites, error) {
                let Some(Some(promised)) = places.promised.get_mut(k) else {
                    continue;
                };
                // Of a bound the iterator doesn't meet, that promise taken
                // off, whether this round's errors took it off already, as
                // rustc says one unmet bound once of the call and once of
                // its argument; of anything else, the place left as it was.
                let named: Vec<usize> = (PROMISES.iter().enumerate())
                    .filter(|(_, (name, _))| error.message.contains(name))
                    .map(|(i, _)| i)
                    .collect();
                if error.code == "E0277" && !error.message.contains("impl Iterator") && !named.is_empty() {
                    for i in named {
                        promised[i] = false;
                    }
                } else {
                    places.promised[k] = None;
                }
                changed = true;
            }
        }
        if !changed {
            return false;
        }
    }
    false
}
