//! `library`: the case's items but `main` moved into a library crate,
//! `metamorph_lib`, each made public, and the program its `main`, which
//! uses the library's items, `use metamorph_lib::*;`. Printed as the
//! library, then `MAIN`'s line, then the program.

use proc_macro2::{Ident, Span, TokenTree};
use quote::ToTokens;
use syn::{Item, Visibility};

/// The line between the library and the program.
pub const MAIN: &str = "\n// metamorph: main\n";

pub fn library(ast: &syn::File, budget: &mut usize) -> Option<String> {
    let mut lib = syn::File {
        shebang: None,
        attrs: ast.attrs.clone(),
        items: Vec::new(),
    };
    let mut program = syn::File {
        shebang: None,
        attrs: ast.attrs.clone(),
        items: Vec::new(),
    };
    let edition = std::env::var("METAMORPH_EDITION").unwrap_or_else(|_| String::from("2024"));
    program.items.push(if edition == "2015" {
        syn::parse_quote!(
            extern crate metamorph_lib;
        )
    } else {
        syn::parse_quote!(
            use metamorph_lib::*;
        )
    });
    if edition == "2015" {
        program.items.push(syn::parse_quote!(
            use self::metamorph_lib::*;
        ));
    }
    let mut moved = false;
    for item in &ast.items {
        match item {
            // What each crate reads by its own `use`s.
            Item::Use(_) => {
                lib.items.push(item.clone());
                program.items.push(item.clone());
            }
            Item::Fn(f) if f.sig.ident == "main" => program.items.push(item.clone()),
            _ if *budget > 0 => {
                let mut item = item.clone();
                public(&mut item);
                lib.items.push(item);
                *budget -= 1;
                moved = true;
            }
            _ => program.items.push(item.clone()),
        }
    }
    moved.then(|| format!("{}{MAIN}{}", lib.to_token_stream(), program.to_token_stream()))
}

/// The visibility of what another crate reads.
fn pub_() -> Visibility {
    syn::parse_quote!(pub)
}

/// `item` and what it holds made public: its fields, an inherent impl's
/// items, a module's items, and a `macro_rules!` exported.
fn public(item: &mut Item) {
    match item {
        Item::Const(i) => i.vis = pub_(),
        Item::Static(i) => i.vis = pub_(),
        Item::Fn(i) => i.vis = pub_(),
        Item::Type(i) => i.vis = pub_(),
        Item::Trait(i) => i.vis = pub_(),
        Item::Union(i) => {
            i.vis = pub_();
            for field in &mut i.fields.named {
                field.vis = pub_();
            }
        }
        Item::Struct(i) => {
            i.vis = pub_();
            for field in i.fields.iter_mut() {
                field.vis = pub_();
            }
        }
        Item::Enum(i) => i.vis = pub_(),
        Item::Mod(i) => {
            i.vis = pub_();
            if let Some((_, items)) = &mut i.content {
                for item in items {
                    public(item);
                }
            }
        }
        Item::Impl(i) if i.trait_.is_none() => {
            for item in &mut i.items {
                match item {
                    syn::ImplItem::Fn(f) => f.vis = pub_(),
                    syn::ImplItem::Const(c) => c.vis = pub_(),
                    syn::ImplItem::Type(t) => t.vis = pub_(),
                    _ => {}
                }
            }
        }
        Item::Macro(m) if m.mac.path.is_ident("macro_rules") => {
            if !m.attrs.iter().any(|a| a.path().is_ident("macro_export")) {
                m.attrs.push(syn::parse_quote!(#[macro_export]));
            }
        }
        // Each of its `static`s public.
        Item::Macro(m) if m.mac.path.is_ident("thread_local") => {
            let trees: Vec<TokenTree> = std::mem::take(&mut m.mac.tokens).into_iter().collect();
            let mut out = Vec::new();
            for (i, tree) in trees.iter().enumerate() {
                let before = i.checked_sub(1).and_then(|i| trees.get(i));
                let public = matches!(before, Some(TokenTree::Ident(p)) if p == "pub")
                    || matches!(before, Some(TokenTree::Group(g)) if g.delimiter() == proc_macro2::Delimiter::Parenthesis);
                if matches!(tree, TokenTree::Ident(s) if s == "static") && !public {
                    out.push(TokenTree::Ident(Ident::new("pub", Span::call_site())));
                }
                out.push(tree.clone());
            }
            m.mac.tokens = out.into_iter().collect();
        }
        Item::ForeignMod(f) => {
            for item in &mut f.items {
                match item {
                    syn::ForeignItem::Fn(i) => i.vis = pub_(),
                    syn::ForeignItem::Static(i) => i.vis = pub_(),
                    syn::ForeignItem::Type(i) => i.vis = pub_(),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}
