//! The metamorphic net's rewrites (#939): a corpus case rewritten in a way
//! that keeps its meaning wherever rustc still compiles it, so native Rust's
//! run of the rewrite is its own oracle. A rewrite rustc rejects is no
//! variant: the harness leaves it out.
//!
//!   metamorph <rewrite> <case.rs> [<limit>]   # the rewrite on stdout; exit 2 where it changes nothing
//!
//! - `outline`: `main`'s body moved into a function `main` calls.
//! - `boxed`: each `let x = e;` whose `x` is bound once in its function made
//!   `let x = Box::new(e);`, each use of `x` read through it, `(*x)`.
//! - `dyn-ref`: each parameter `x: &T` or `x: &mut T` of a free function
//!   taken as `&dyn MetamorphGet<T>`, `&mut dyn MetamorphGetMut<T>`, and
//!   read back as `&T` first: what a call gives coerces, so calls are as
//!   they were.
//! - `impl-ref`: the same, taken as `&impl MetamorphGet<T>`, generic code.
//! - `impl-owned`: each parameter `x: T` of a free function taken as
//!   `impl MetamorphIs<T>`, and read back as `T` first.
//!
//! `<limit>`: only so many of the places a rewrite finds, in order.

use proc_macro2::{Group, Ident, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use std::collections::BTreeMap;
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::{Block, Expr, FnArg, Item, Pat, Stmt, Type};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (rewrite, file, limit) = match &args[1..] {
        [rewrite, file] => (rewrite.as_str(), file, usize::MAX),
        [rewrite, file, limit] => (rewrite.as_str(), file, limit.parse().expect("a limit")),
        _ => {
            eprintln!("metamorph <outline|boxed|dyn-ref|impl-ref|impl-owned> <case.rs> [<limit>]");
            std::process::exit(64);
        }
    };
    let source = std::fs::read_to_string(file).expect("the case");
    let mut ast = syn::parse_file(&source).expect("the case parses");
    let mut budget = limit;
    let changed = match rewrite {
        "outline" => outline(&mut ast),
        "boxed" => boxed(&mut ast, &mut budget),
        "dyn-ref" => refs(&mut ast, Refs::Dyn, &mut budget),
        "impl-ref" => refs(&mut ast, Refs::Impl, &mut budget),
        "impl-owned" => owned(&mut ast, &mut budget),
        _ => {
            eprintln!("no rewrite `{rewrite}`");
            std::process::exit(64);
        }
    };
    if !changed {
        std::process::exit(2);
    }
    print!("{}", ast.to_token_stream());
}

/// `fn main() { body }` as `fn main() { metamorph_main() }` and
/// `fn metamorph_main() { body }`.
fn outline(ast: &mut syn::File) -> bool {
    let Some(at) = ast.items.iter().position(|item| {
        matches!(item, Item::Fn(f) if f.sig.ident == "main" && f.sig.inputs.is_empty() && matches!(f.sig.output, syn::ReturnType::Default))
    }) else {
        return false;
    };
    let Item::Fn(main) = &mut ast.items[at] else {
        unreachable!()
    };
    let body = std::mem::replace(&mut *main.block, syn::parse_quote!({ metamorph_main() }));
    ast.items.push(syn::parse_quote!(fn metamorph_main() #body));
    true
}

/// Each name bound in what's visited, how many times: a `let`'s, a
/// parameter's, a closure's, a pattern's.
#[derive(Default)]
struct Bindings(BTreeMap<String, usize>);

impl<'a> Visit<'a> for Bindings {
    fn visit_pat_ident(&mut self, p: &'a syn::PatIdent) {
        *self.0.entry(p.ident.to_string()).or_default() += 1;
        visit::visit_pat_ident(self, p);
    }
    // A function inside is a scope of its own.
    fn visit_item(&mut self, _: &'a Item) {}
}

/// Each function's body in `ast`, a free one's or a method's, with its
/// signature's inputs.
fn each_function(ast: &mut syn::File, f: &mut dyn FnMut(&syn::Signature, &mut Block)) {
    for item in &mut ast.items {
        match item {
            Item::Fn(item) => f(&item.sig, &mut item.block),
            Item::Impl(imp) => {
                for item in &mut imp.items {
                    if let syn::ImplItem::Fn(item) = item {
                        f(&item.sig, &mut item.block);
                    }
                }
            }
            _ => {}
        }
    }
}

fn boxed(ast: &mut syn::File, budget: &mut usize) -> bool {
    let mut changed = false;
    each_function(ast, &mut |sig, block| {
        let mut bound = Bindings::default();
        for input in &sig.inputs {
            bound.visit_fn_arg(input);
        }
        bound.visit_block(block);
        let once = |name: &Ident| bound.0.get(&name.to_string()) == Some(&1);
        let mut names = Vec::new();
        Lets {
            once: &once,
            names: &mut names,
            budget,
        }
        .visit_block_mut(block);
        for name in names {
            Through { name: &name }.visit_block_mut(block);
            changed = true;
        }
    });
    changed
}

/// Each `let x = e;` of a name bound once made `let x = Box::new(e);`.
struct Lets<'n, 'o> {
    once: &'o dyn Fn(&Ident) -> bool,
    names: &'n mut Vec<Ident>,
    budget: &'n mut usize,
}

impl VisitMut for Lets<'_, '_> {
    fn visit_local_mut(&mut self, local: &mut syn::Local) {
        visit_mut::visit_local_mut(self, local);
        let Some(init) = &mut local.init else { return };
        if init.diverge.is_some() || *self.budget == 0 {
            return;
        }
        let (ident, ty) = match &mut local.pat {
            Pat::Ident(p) if p.by_ref.is_none() && p.subpat.is_none() => (p.ident.clone(), None),
            Pat::Type(t) => match &*t.pat {
                Pat::Ident(p) if p.by_ref.is_none() && p.subpat.is_none() => (p.ident.clone(), Some(&mut t.ty)),
                _ => return,
            },
            _ => return,
        };
        if !(self.once)(&ident) {
            return;
        }
        let value = &init.expr;
        init.expr = Box::new(syn::parse_quote!(Box::new(#value)));
        if let Some(ty) = ty {
            let inner = &**ty;
            *ty = Box::new(syn::parse_quote!(Box<#inner>));
        }
        *self.budget -= 1;
        self.names.push(ident);
    }
    fn visit_item_mut(&mut self, _: &mut Item) {}
}

/// Each use of `name` read through its box, `(*name)`: a path's, a struct
/// literal's shorthand field's, and a macro's tokens'.
struct Through<'n> {
    name: &'n Ident,
}

impl Through<'_> {
    fn is_it(&self, e: &Expr) -> bool {
        matches!(e, Expr::Path(p) if p.qself.is_none() && p.path.is_ident(self.name))
    }

    fn tokens(&self, stream: TokenStream) -> TokenStream {
        let trees: Vec<TokenTree> = stream.into_iter().collect();
        let mut out = Vec::new();
        for (i, tree) in trees.iter().enumerate() {
            let punct = |at: Option<&TokenTree>, c: char| matches!(at, Some(TokenTree::Punct(p)) if p.as_char() == c);
            match tree {
                TokenTree::Ident(id) if id == self.name => {
                    let before = i.checked_sub(1).and_then(|i| trees.get(i));
                    let after = trees.get(i + 1);
                    // A field, `a.x`; a path's segment, `m::x` or `x::y`; a
                    // field named in a literal, `S { x: 1 }`; a macro, `x!`.
                    if punct(before, '.') || punct(before, ':') || punct(after, ':') || punct(after, '!') {
                        out.push(tree.clone());
                    } else {
                        let name = self.name;
                        out.extend(quote!((*#name)));
                    }
                }
                TokenTree::Group(g) => {
                    let mut group = Group::new(g.delimiter(), self.tokens(g.stream()));
                    group.set_span(g.span());
                    out.push(TokenTree::Group(group));
                }
                _ => out.push(tree.clone()),
            }
        }
        out.into_iter().collect()
    }
}

impl VisitMut for Through<'_> {
    fn visit_expr_mut(&mut self, e: &mut Expr) {
        if self.is_it(e) {
            let name = self.name;
            *e = syn::parse_quote!((*#name));
            return;
        }
        visit_mut::visit_expr_mut(self, e);
    }
    fn visit_field_value_mut(&mut self, field: &mut syn::FieldValue) {
        if field.colon_token.is_none() && matches!(&field.member, syn::Member::Named(m) if m == self.name) {
            field.colon_token = Some(Default::default());
        }
        visit_mut::visit_field_value_mut(self, field);
    }
    fn visit_local_mut(&mut self, local: &mut syn::Local) {
        // Its own `let`'s value, `Box::new(e)`, read as it was.
        if let Some(init) = &mut local.init {
            self.visit_expr_mut(&mut init.expr);
            if let Some((_, diverge)) = &mut init.diverge {
                self.visit_expr_mut(diverge);
            }
        }
    }
    fn visit_macro_mut(&mut self, m: &mut syn::Macro) {
        m.tokens = self.tokens(std::mem::take(&mut m.tokens));
    }
    fn visit_item_mut(&mut self, _: &mut Item) {}
}

#[derive(Clone, Copy, PartialEq)]
enum Refs {
    Dyn,
    Impl,
}

/// What a reference parameter's referent may be read back through: a
/// sized type of no `Self`, not a trait object's, a slice's or `str`.
fn plain_referent(ty: &Type) -> bool {
    let mentions_self = ty
        .to_token_stream()
        .into_iter()
        .any(|t| matches!(t, TokenTree::Ident(i) if i == "Self"));
    !mentions_self
        && match ty {
            Type::Path(p) => !p.path.is_ident("str"),
            Type::Tuple(t) => !t.elems.is_empty(),
            Type::Array(_) => true,
            _ => false,
        }
}

fn refs(ast: &mut syn::File, how: Refs, budget: &mut usize) -> bool {
    let mut changed = false;
    for item in &mut ast.items {
        let Item::Fn(f) = item else { continue };
        if f.sig.ident == "main" || f.sig.constness.is_some() {
            continue;
        }
        let mut reads = Vec::new();
        for input in &mut f.sig.inputs {
            let FnArg::Typed(arg) = input else { continue };
            let Pat::Ident(p) = &*arg.pat else { continue };
            if p.by_ref.is_some() || p.subpat.is_some() || *budget == 0 {
                continue;
            }
            let Type::Reference(r) = &*arg.ty else { continue };
            if !plain_referent(&r.elem) {
                continue;
            }
            let (name, mutable, elem, lifetime) = (p.ident.clone(), r.mutability.is_some(), &r.elem, &r.lifetime);
            let (by, through) = match mutable {
                false => (quote!(MetamorphGet<#elem>), quote!(MetamorphGet::metamorph_get)),
                true => (
                    quote!(MetamorphGetMut<#elem>),
                    quote!(MetamorphGetMut::metamorph_get_mut),
                ),
            };
            let referent = match how {
                Refs::Dyn => quote!(dyn #by),
                Refs::Impl => quote!(impl #by),
            };
            let held = match mutable {
                false => quote!(& #lifetime #elem),
                true => quote!(& #lifetime mut #elem),
            };
            let given = p.mutability.map(|_| quote!(mut));
            reads.push(quote!(let #given #name: #held = #through(#name);));
            arg.ty = Box::new(match mutable {
                false => syn::parse_quote!(& #lifetime #referent),
                true => syn::parse_quote!(& #lifetime mut #referent),
            });
            if let Pat::Ident(p) = &mut *arg.pat {
                p.mutability = None;
            }
            *budget -= 1;
        }
        if reads.is_empty() {
            continue;
        }
        let stmts: Vec<Stmt> = reads
            .into_iter()
            .map(|read| syn::parse2(read).expect("a `let`"))
            .collect();
        f.block.stmts.splice(0..0, stmts);
        changed = true;
    }
    if changed {
        ast.items.push(syn::parse_quote!(
            trait MetamorphGet<T: ?Sized> {
                fn metamorph_get(&self) -> &T;
            }
        ));
        ast.items.push(syn::parse_quote!(
            impl<T: ?Sized> MetamorphGet<T> for T {
                fn metamorph_get(&self) -> &T {
                    self
                }
            }
        ));
        ast.items.push(syn::parse_quote!(
            trait MetamorphGetMut<T: ?Sized> {
                fn metamorph_get_mut(&mut self) -> &mut T;
            }
        ));
        ast.items.push(syn::parse_quote!(
            impl<T: ?Sized> MetamorphGetMut<T> for T {
                fn metamorph_get_mut(&mut self) -> &mut T {
                    self
                }
            }
        ));
    }
    changed
}

fn owned(ast: &mut syn::File, budget: &mut usize) -> bool {
    let mut changed = false;
    for item in &mut ast.items {
        let Item::Fn(f) = item else { continue };
        if f.sig.ident == "main" || f.sig.constness.is_some() {
            continue;
        }
        let mut reads = Vec::new();
        for input in &mut f.sig.inputs {
            let FnArg::Typed(arg) = input else { continue };
            let Pat::Ident(p) = &*arg.pat else { continue };
            if p.by_ref.is_some() || p.subpat.is_some() || *budget == 0 {
                continue;
            }
            let ty = &arg.ty;
            let plain = matches!(&**ty, Type::Path(_) | Type::Tuple(_) | Type::Array(_)) && plain_referent(ty);
            if !plain {
                continue;
            }
            let name = p.ident.clone();
            let given = p.mutability.map(|_| quote!(mut));
            reads.push(quote!(let #given #name: #ty = MetamorphIs::metamorph_is(#name);));
            arg.ty = Box::new(syn::parse_quote!(impl MetamorphIs<#ty>));
            if let Pat::Ident(p) = &mut *arg.pat {
                p.mutability = None;
            }
            *budget -= 1;
        }
        if reads.is_empty() {
            continue;
        }
        let stmts: Vec<Stmt> = reads
            .into_iter()
            .map(|read| syn::parse2(read).expect("a `let`"))
            .collect();
        f.block.stmts.splice(0..0, stmts);
        changed = true;
    }
    if changed {
        ast.items.push(syn::parse_quote!(
            trait MetamorphIs<T> {
                fn metamorph_is(self) -> T;
            }
        ));
        ast.items.push(syn::parse_quote!(
            impl<T> MetamorphIs<T> for T {
                fn metamorph_is(self) -> T {
                    self
                }
            }
        ));
    }
    changed
}
