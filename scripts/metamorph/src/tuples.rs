//! `tuple-struct`: a struct of named fields made a tuple struct, its fields
//! by their places, `struct P { x: i32, y: i32 }` → `struct P(i32, i32);`:
//! `P { x: 1, y: 2 }` → `P(1, 2)`, `p.y` → `p.1`, `P { x, .. }` → `P(x,
//! _)`. A literal whose fields are out of their order is made its values
//! first, in its order, `{ let metamorph_y = ..; let metamorph_x = ..; P(
//! metamorph_x, metamorph_y) }`. Only a struct whose field names no other
//! type of the case has, of no attribute of rust-js's or serde's, and none
//! of whose literals takes the rest of another, `..base`; and none of a
//! case with a binding, whose structs JS may make or read by their keys.

use proc_macro2::{Group, Ident, Literal, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote};
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, Fields, Item, Member, Pat, Token};

/// Every named field of every struct and enum variant the case has, by
/// name, with the struct or variant it's of.
#[derive(Default)]
struct Named(Vec<(String, String)>);

impl<'a> Visit<'a> for Named {
    fn visit_item_struct(&mut self, s: &'a syn::ItemStruct) {
        for field in &s.fields {
            if let Some(name) = &field.ident {
                self.0.push((name.to_string(), s.ident.to_string()));
            }
        }
        visit::visit_item_struct(self, s);
    }
    fn visit_variant(&mut self, v: &'a syn::Variant) {
        for field in &v.fields {
            if let Some(name) = &field.ident {
                self.0.push((name.to_string(), format!("variant {}", v.ident)));
            }
        }
        visit::visit_variant(self, v);
    }
}

fn of_rust_js_or_serde(attrs: &[syn::Attribute]) -> bool {
    (attrs.iter()).any(|attr| {
        let said = attr.to_token_stream().to_string();
        said.contains("rust_js") || said.contains("serde")
    })
}

pub fn tuple_struct(ast: &mut syn::File, budget: &mut usize) -> bool {
    // A binding's struct is what JS gives or takes, an object of its named
    // keys: by places, it's another JS value, no rewrite of the same meaning.
    if ast.to_token_stream().to_string().contains("link_name") {
        return false;
    }
    let mut named = Named::default();
    named.visit_file(ast);
    let candidates: Vec<(Ident, Vec<Ident>)> = (ast.items.iter())
        .filter_map(|item| match item {
            Item::Struct(s) => match &s.fields {
                Fields::Named(fields)
                    if !fields.named.is_empty()
                        && !of_rust_js_or_serde(&s.attrs)
                        && fields.named.iter().all(|f| f.attrs.is_empty()) =>
                {
                    let names: Vec<Ident> = fields.named.iter().map(|f| f.ident.clone().expect("named")).collect();
                    let own = s.ident.to_string();
                    let alone = names
                        .iter()
                        .all(|n| named.0.iter().all(|(field, of)| field != &n.to_string() || of == &own));
                    alone.then(|| (s.ident.clone(), names))
                }
                _ => None,
            },
            _ => None,
        })
        .collect();
    let mut changed = false;
    for (ty, names) in candidates {
        if *budget == 0 {
            break;
        }
        let mut made = ast.clone();
        let mut places = Places {
            ty: &ty,
            names: &names,
            impls: Vec::new(),
            problem: false,
            fresh: 0,
        };
        places.visit_file_mut(&mut made);
        if places.problem {
            continue;
        }
        for item in &mut made.items {
            if let Item::Struct(s) = item
                && s.ident == ty
                && let Fields::Named(fields) = &s.fields
            {
                let mut unnamed: syn::FieldsUnnamed = syn::parse_quote!(());
                for field in &fields.named {
                    let mut field = field.clone();
                    field.ident = None;
                    field.colon_token = None;
                    unnamed.unnamed.push(field);
                }
                s.fields = Fields::Unnamed(unnamed);
                s.semi_token = Some(Default::default());
            }
        }
        *ast = made;
        *budget -= 1;
        changed = true;
    }
    changed
}

/// Each use of `ty`'s fields by name made one by place.
struct Places<'t> {
    ty: &'t Ident,
    names: &'t [Ident],
    /// Whether each impl it's in is `ty`'s, whose `Self` it is.
    impls: Vec<bool>,
    problem: bool,
    fresh: usize,
}

impl Places<'_> {
    fn is_it(&self, path: &syn::Path) -> bool {
        let last = path.segments.last().map(|s| &s.ident);
        last == Some(self.ty) || (path.is_ident("Self") && self.impls.last() == Some(&true))
    }

    fn place(&self, name: &Ident) -> Option<usize> {
        self.names.iter().position(|n| n == name)
    }

    /// A macro's tokens: as expressions where they're a list of them, else
    /// each `.field` by its place, refused where a literal or a pattern of
    /// `ty` is among them.
    fn tokens(&mut self, stream: TokenStream) -> TokenStream {
        if let Ok(exprs) = syn::parse::Parser::parse2(Punctuated::<Expr, Token![,]>::parse_terminated, stream.clone()) {
            let mut exprs = exprs;
            for e in exprs.iter_mut() {
                self.visit_expr_mut(e);
            }
            return exprs.into_token_stream();
        }
        self.fields_in(stream)
    }

    fn fields_in(&mut self, stream: TokenStream) -> TokenStream {
        let trees: Vec<TokenTree> = stream.into_iter().collect();
        let mut out = Vec::new();
        for (i, tree) in trees.iter().enumerate() {
            let before = i.checked_sub(1).and_then(|i| trees.get(i));
            let after = trees.get(i + 1);
            match tree {
                TokenTree::Ident(id) if matches!(before, Some(TokenTree::Punct(p)) if p.as_char() == '.') => {
                    let called = matches!(after, Some(TokenTree::Group(g)) if g.delimiter() == proc_macro2::Delimiter::Parenthesis);
                    match self.place(id) {
                        Some(at) if !called => out.push(TokenTree::Literal(Literal::usize_unsuffixed(at))),
                        _ => out.push(tree.clone()),
                    }
                }
                TokenTree::Ident(id)
                    if (id == self.ty || id == "Self")
                        && matches!(after, Some(TokenTree::Group(g)) if g.delimiter() == proc_macro2::Delimiter::Brace) =>
                {
                    self.problem = true;
                    out.push(tree.clone());
                }
                TokenTree::Group(g) => {
                    let mut group = Group::new(g.delimiter(), self.fields_in(g.stream()));
                    group.set_span(g.span());
                    out.push(TokenTree::Group(group));
                }
                _ => out.push(tree.clone()),
            }
        }
        out.into_iter().collect()
    }
}

impl VisitMut for Places<'_> {
    fn visit_item_impl_mut(&mut self, imp: &mut syn::ItemImpl) {
        let ours = matches!(&*imp.self_ty, syn::Type::Path(p) if p.path.is_ident(self.ty));
        self.impls.push(ours);
        visit_mut::visit_item_impl_mut(self, imp);
        self.impls.pop();
    }

    fn visit_expr_mut(&mut self, e: &mut Expr) {
        visit_mut::visit_expr_mut(self, e);
        match e {
            Expr::Field(f) => {
                if let Member::Named(name) = &f.member
                    && let Some(at) = self.place(name)
                {
                    f.member = Member::Unnamed(syn::Index::from(at));
                }
            }
            Expr::Struct(s) if self.is_it(&s.path) => {
                if s.rest.is_some() || s.dot2_token.is_some() || s.fields.len() != self.names.len() {
                    self.problem = true;
                    return;
                }
                let mut given: Vec<(usize, Expr)> = Vec::new();
                for field in &s.fields {
                    let Member::Named(name) = &field.member else {
                        self.problem = true;
                        return;
                    };
                    let Some(at) = self.place(name) else {
                        self.problem = true;
                        return;
                    };
                    given.push((at, field.expr.clone()));
                }
                let path = &s.path;
                let in_order = given.iter().enumerate().all(|(i, (at, _))| i == *at);
                if in_order {
                    let values = given.iter().map(|(_, e)| e);
                    *e = syn::parse_quote!(#path(#(#values),*));
                } else {
                    let mut lets = Vec::new();
                    let mut held = vec![None; self.names.len()];
                    for (at, value) in &given {
                        let name = format_ident!("metamorph_{}_{}", self.names[*at], self.fresh);
                        lets.push(quote!(let #name = #value;));
                        held[*at] = Some(name);
                    }
                    self.fresh += 1;
                    let held = held.into_iter().map(|h| h.expect("each field"));
                    *e = syn::parse_quote!({ #(#lets)* #path(#(#held),*) });
                }
            }
            _ => {}
        }
    }

    fn visit_pat_mut(&mut self, p: &mut Pat) {
        visit_mut::visit_pat_mut(self, p);
        let Pat::Struct(s) = p else { return };
        if !self.is_it(&s.path) {
            return;
        }
        let mut places: Vec<Pat> = vec![syn::parse_quote!(_); self.names.len()];
        for field in &s.fields {
            let Member::Named(name) = &field.member else {
                self.problem = true;
                return;
            };
            let Some(at) = self.place(name) else {
                self.problem = true;
                return;
            };
            places[at] = (*field.pat).clone();
        }
        if s.rest.is_none() && s.fields.len() != self.names.len() {
            self.problem = true;
            return;
        }
        let path = &s.path;
        *p = syn::parse_quote!(#path(#(#places),*));
    }

    fn visit_macro_mut(&mut self, m: &mut syn::Macro) {
        let tokens = std::mem::take(&mut m.tokens);
        m.tokens = self.tokens(tokens);
    }
}
