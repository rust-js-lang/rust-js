//! `default-copy`: a trait's default method written out in each of the
//! case's impls of the trait that takes it, `fn describe(&self) -> String
//! { .. }` copied into `impl Shape for Circle`: the same body, `Self` the
//! impl's type there as it is where the default runs for it.

use quote::ToTokens;
use syn::{ImplItem, Item, TraitItem};

pub fn default_copy(ast: &mut syn::File, budget: &mut usize) -> bool {
    // Each trait of no generics of its own, its defaults by name.
    let defaults: Vec<(syn::Ident, Vec<syn::TraitItemFn>)> = (ast.items.iter())
        .filter_map(|item| match item {
            Item::Trait(t) if t.generics.params.is_empty() && t.unsafety.is_none() => Some((
                t.ident.clone(),
                (t.items.iter())
                    .filter_map(|item| match item {
                        TraitItem::Fn(f) if f.default.is_some() => Some(f.clone()),
                        _ => None,
                    })
                    .collect(),
            )),
            _ => None,
        })
        .filter(|(_, fns): &(_, Vec<_>)| !fns.is_empty())
        .collect();
    let mut changed = false;
    for item in &mut ast.items {
        let Item::Impl(imp) = item else { continue };
        let Some((None, path, _)) = &imp.trait_ else { continue };
        let Some(name) = path.get_ident() else { continue };
        let Some((_, fns)) = defaults.iter().find(|(t, _)| t == name) else {
            continue;
        };
        let written: Vec<String> = (imp.items.iter())
            .filter_map(|item| match item {
                ImplItem::Fn(f) => Some(f.sig.ident.to_string()),
                _ => None,
            })
            .collect();
        for f in fns {
            if *budget == 0 || written.contains(&f.sig.ident.to_string()) {
                continue;
            }
            let (attrs, sig, block) = (&f.attrs, &f.sig, f.default.as_ref().expect("a default"));
            let copied = quote::quote!(#(#attrs)* #sig #block);
            imp.items
                .push(syn::parse2(copied.into_token_stream()).expect("a method"));
            *budget -= 1;
            changed = true;
        }
    }
    changed
}
