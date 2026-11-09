//! A failing program's reductions (#1088), C-Reduce style: each one makes
//! the program smaller or simpler, whatever it then means, and
//! `scripts/reduce.ts` keeps one where rustc still accepts the program and
//! it still fails as it did.
//!
//!   metamorph reduce <case.rs>        # how many reductions there are
//!   metamorph reduce <case.rs> <n>    # the program with the n-th made; exit 2 past the last
//!
//! In the order they're tried, largest first: an item removed, `main` kept,
//! a module's, an impl's or a trait's too; a statement; a match's arm; an
//! enum's variant, with each arm naming it; a struct's field, with each
//! literal and pattern of the struct naming it; an `if` made its branch,
//! either one; a format macro's argument, `println!`'s, with its `{}`;
//! a free function called once inlined where it's called, a
//! `let` of each argument, and removed; a number made `0` and a string
//! `""`; a `let`'s type left to inference.

use proc_macro2::TokenTree;
use quote::ToTokens;
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, Item, Lit, Pat, Stmt};

/// The program `ast` with reduction `target` made, or how many there are.
pub fn reduce(ast: &mut syn::File, target: Option<usize>) -> Reduced {
    let mut at = At {
        seen: 0,
        target,
        done: false,
    };
    items(&mut ast.items, &mut at);
    for pass in [
        Pass::Stmts,
        Pass::Arms,
        Pass::Ifs,
        Pass::Formats,
        Pass::Literals,
        Pass::LetTypes,
    ] {
        if at.done {
            break;
        }
        Walk { at: &mut at, pass }.visit_file_mut(ast);
    }
    if !at.done {
        variants(ast, &mut at);
    }
    if !at.done {
        fields(ast, &mut at);
    }
    if !at.done {
        inlined(ast, &mut at);
    }
    match at.done {
        true => Reduced::Made,
        false => Reduced::Count(at.seen),
    }
}

pub enum Reduced {
    Made,
    Count(usize),
}

/// Which reduction is being counted, and the one to make.
struct At {
    seen: usize,
    target: Option<usize>,
    done: bool,
}

impl At {
    /// Whether the reduction just counted is the one to make.
    fn hit(&mut self) -> bool {
        if self.done {
            return false;
        }
        let hit = self.target == Some(self.seen);
        self.seen += 1;
        self.done = hit;
        hit
    }
}

fn is_main(item: &Item) -> bool {
    matches!(item, Item::Fn(f) if f.sig.ident == "main")
}

/// Each item of `items`, and of each module, impl and trait among them,
/// removed.
fn items(items: &mut Vec<Item>, at: &mut At) {
    let mut i = 0;
    while i < items.len() {
        if !is_main(&items[i]) && at.hit() {
            items.remove(i);
            return;
        }
        match &mut items[i] {
            Item::Mod(m) => {
                if let Some((_, inner)) = &mut m.content {
                    self::items(inner, at);
                }
            }
            Item::Impl(imp) => {
                let mut j = 0;
                while j < imp.items.len() {
                    if at.hit() {
                        imp.items.remove(j);
                        return;
                    }
                    j += 1;
                }
            }
            Item::Trait(tr) => {
                let mut j = 0;
                while j < tr.items.len() {
                    if at.hit() {
                        tr.items.remove(j);
                        return;
                    }
                    j += 1;
                }
            }
            _ => {}
        }
        if at.done {
            return;
        }
        i += 1;
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Pass {
    Stmts,
    Arms,
    Ifs,
    Formats,
    Literals,
    LetTypes,
}

struct Walk<'a> {
    at: &'a mut At,
    pass: Pass,
}

impl VisitMut for Walk<'_> {
    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        if self.pass == Pass::Stmts {
            let mut i = 0;
            while i < block.stmts.len() {
                if self.at.hit() {
                    block.stmts.remove(i);
                    return;
                }
                i += 1;
            }
        }
        visit_mut::visit_block_mut(self, block);
    }

    fn visit_expr_match_mut(&mut self, m: &mut syn::ExprMatch) {
        if self.pass == Pass::Arms && m.arms.len() > 1 {
            let mut i = 0;
            while i < m.arms.len() {
                if self.at.hit() {
                    m.arms.remove(i);
                    return;
                }
                i += 1;
            }
        }
        visit_mut::visit_expr_match_mut(self, m);
    }

    fn visit_expr_mut(&mut self, e: &mut Expr) {
        if self.at.done {
            return;
        }
        match (self.pass, &*e) {
            // An `if` as either of its branches.
            (Pass::Ifs, Expr::If(i)) => {
                let then = Expr::Block(syn::ExprBlock {
                    attrs: Vec::new(),
                    label: None,
                    block: i.then_branch.clone(),
                });
                let otherwise = i.else_branch.as_ref().map(|(_, e)| (**e).clone());
                if self.at.hit() {
                    *e = then;
                    return;
                }
                if let Some(otherwise) = otherwise
                    && self.at.hit()
                {
                    *e = otherwise;
                    return;
                }
            }
            (Pass::Literals, Expr::Lit(lit)) => {
                let made = match &lit.lit {
                    Lit::Int(i) if i.base10_digits() != "0" => {
                        Some(Lit::Int(syn::LitInt::new(&format!("0{}", i.suffix()), i.span())))
                    }
                    Lit::Str(s) if !s.value().is_empty() => Some(Lit::Str(syn::LitStr::new("", s.span()))),
                    _ => None,
                };
                if let Some(made) = made
                    && self.at.hit()
                {
                    *e = Expr::Lit(syn::ExprLit {
                        attrs: Vec::new(),
                        lit: made,
                    });
                    return;
                }
            }
            _ => {}
        }
        visit_mut::visit_expr_mut(self, e);
    }

    fn visit_local_mut(&mut self, local: &mut syn::Local) {
        if self.pass == Pass::LetTypes
            && local.init.is_some()
            && let Pat::Type(typed) = &local.pat
        {
            let untyped = (*typed.pat).clone();
            if self.at.hit() {
                local.pat = untyped;
                return;
            }
        }
        visit_mut::visit_local_mut(self, local);
    }

    // A format macro's argument, with its place in the format string;
    // nothing else of what a macro's tokens hold.
    fn visit_macro_mut(&mut self, m: &mut syn::Macro) {
        if self.pass != Pass::Formats {
            return;
        }
        let formats = ["println", "print", "eprintln", "eprint", "format", "panic"];
        if !m
            .path
            .segments
            .last()
            .is_some_and(|s| formats.iter().any(|f| s.ident == f))
        {
            return;
        }
        type Args = syn::punctuated::Punctuated<Expr, syn::Token![,]>;
        let Ok(args) = syn::parse::Parser::parse2(Args::parse_terminated, m.tokens.clone()) else {
            return;
        };
        let mut args: Vec<Expr> = args.into_iter().collect();
        let Some(Expr::Lit(syn::ExprLit {
            lit: Lit::Str(format), ..
        })) = args.first()
        else {
            return;
        };
        let (text, span) = (format.value(), format.span());
        let places = placeholders(&text);
        // Each argument a `{}` of its own, in order, none named or numbered.
        if places.len() != args.len() - 1
            || places.iter().any(|&(start, end)| {
                let inner = &text[start + 1..end - 1];
                !(inner.is_empty() || inner.starts_with(':'))
            })
        {
            return;
        }
        for (k, &(start, end)) in places.iter().enumerate() {
            if self.at.hit() {
                let text = format!("{}{}", &text[..start], &text[end..]);
                args.remove(k + 1);
                args[0] = Expr::Lit(syn::ExprLit {
                    attrs: Vec::new(),
                    lit: Lit::Str(syn::LitStr::new(&text, span)),
                });
                let args: Args = args.into_iter().collect();
                m.tokens = args.to_token_stream();
                return;
            }
        }
    }
}

/// Whether `tokens` hold `name`, an identifier, preceded by `::`.
fn names_path(tokens: proc_macro2::TokenStream, name: &str) -> bool {
    let tokens: Vec<TokenTree> = flatten(tokens);
    tokens.windows(2).any(
        |w| matches!((&w[0], &w[1]), (TokenTree::Punct(p), TokenTree::Ident(i)) if p.as_char() == ':' && i == name),
    )
}

fn flatten(tokens: proc_macro2::TokenStream) -> Vec<TokenTree> {
    tokens
        .into_iter()
        .flat_map(|t| match t {
            TokenTree::Group(g) => flatten(g.stream()),
            t => vec![t],
        })
        .collect()
}

/// Each enum's variant removed, of one of more than one, with each match
/// arm whose pattern names it, `E::V`.
fn variants(ast: &mut syn::File, at: &mut At) {
    let mut removed = None;
    for item in &mut ast.items {
        let Item::Enum(e) = item else { continue };
        if e.variants.len() < 2 {
            continue;
        }
        let names: Vec<String> = e.variants.iter().map(|v| v.ident.to_string()).collect();
        for (i, name) in names.into_iter().enumerate() {
            if at.hit() {
                e.variants = (std::mem::take(&mut e.variants).into_iter().enumerate())
                    .filter(|&(j, _)| j != i)
                    .map(|(_, v)| v)
                    .collect();
                removed = Some(name);
                break;
            }
        }
        if removed.is_some() {
            break;
        }
    }
    struct Arms(String);
    impl VisitMut for Arms {
        fn visit_expr_match_mut(&mut self, m: &mut syn::ExprMatch) {
            m.arms.retain(|arm| !names_path(arm.pat.to_token_stream(), &self.0));
            visit_mut::visit_expr_match_mut(self, m);
        }
    }
    if let Some(name) = removed {
        Arms(name).visit_file_mut(ast);
    }
}

/// Each struct's named field removed, with each literal and pattern of
/// the struct, by its name, naming it.
fn fields(ast: &mut syn::File, at: &mut At) {
    let mut removed = None;
    for item in &mut ast.items {
        let Item::Struct(s) = item else { continue };
        let syn::Fields::Named(named) = &mut s.fields else {
            continue;
        };
        let names: Vec<String> = (named.named.iter())
            .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()))
            .collect();
        for (i, name) in names.into_iter().enumerate() {
            if at.hit() {
                named.named = (std::mem::take(&mut named.named).into_iter().enumerate())
                    .filter(|&(j, _)| j != i)
                    .map(|(_, f)| f)
                    .collect();
                removed = Some((s.ident.to_string(), name));
                break;
            }
        }
        if removed.is_some() {
            break;
        }
    }
    struct Uses(String, String);
    impl Uses {
        fn of(&self, path: &syn::Path) -> bool {
            path.segments
                .last()
                .is_some_and(|s| s.ident == self.0 || (s.ident == "Self"))
        }
    }
    impl VisitMut for Uses {
        fn visit_expr_struct_mut(&mut self, e: &mut syn::ExprStruct) {
            if self.of(&e.path) {
                let field = self.1.clone();
                e.fields = (std::mem::take(&mut e.fields).into_iter())
                    .filter(|f| !matches!(&f.member, syn::Member::Named(n) if *n == field))
                    .collect();
            }
            visit_mut::visit_expr_struct_mut(self, e);
        }
        fn visit_pat_struct_mut(&mut self, p: &mut syn::PatStruct) {
            if self.of(&p.path) {
                let field = self.1.clone();
                p.fields = (std::mem::take(&mut p.fields).into_iter())
                    .filter(|f| !matches!(&f.member, syn::Member::Named(n) if *n == field))
                    .collect();
                if p.rest.is_none() {
                    p.rest = Some(syn::PatRest {
                        attrs: Vec::new(),
                        dot2_token: Default::default(),
                    });
                }
            }
            visit_mut::visit_pat_struct_mut(self, p);
        }
    }
    if let Some((ty, field)) = removed {
        Uses(ty, field).visit_file_mut(ast);
    }
}

/// Each free function called once, of plain parameters, no generics and
/// no `return` or `?`, inlined where it's called, each argument a `let` of
/// its parameter, and removed.
fn inlined(ast: &mut syn::File, at: &mut At) {
    struct Calls<'a>(&'a str, usize);
    impl<'ast> Visit<'ast> for Calls<'_> {
        fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
            if matches!(&*c.func, Expr::Path(p) if p.path.is_ident(self.0)) {
                self.1 += 1;
            }
            visit::visit_expr_call(self, c);
        }
        fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
            // Named but not called, given as a value: not inlined.
            if p.path.is_ident(self.0) {
                self.1 += 2;
            }
        }
    }
    let inlinable = |f: &syn::ItemFn| {
        f.sig.generics.params.is_empty()
            && f.sig.ident != "main"
            && f.sig.asyncness.is_none()
            && (f.sig.inputs.iter()).all(|a| matches!(a, syn::FnArg::Typed(t) if matches!(&*t.pat, Pat::Ident(_))))
            && !flatten(f.block.to_token_stream()).iter().any(|t| match t {
                TokenTree::Ident(i) => i == "return",
                TokenTree::Punct(p) => p.as_char() == '?',
                _ => false,
            })
    };
    let candidates: Vec<usize> = (ast.items.iter().enumerate())
        .filter_map(|(i, item)| match item {
            Item::Fn(f) if inlinable(f) => {
                let mut calls = Calls(&f.sig.ident.to_string(), 0);
                calls.visit_file(ast);
                (calls.1 == 1).then_some(i)
            }
            _ => None,
        })
        .collect();
    for i in candidates {
        if !at.hit() {
            continue;
        }
        let Item::Fn(f) = ast.items.remove(i) else {
            unreachable!()
        };
        struct Inline(syn::ItemFn);
        impl VisitMut for Inline {
            fn visit_expr_mut(&mut self, e: &mut Expr) {
                if let Expr::Call(c) = e
                    && matches!(&*c.func, Expr::Path(p) if p.path.is_ident(&self.0.sig.ident))
                {
                    let mut stmts: Vec<Stmt> = Vec::new();
                    for (param, arg) in self.0.sig.inputs.iter().zip(c.args.iter()) {
                        let syn::FnArg::Typed(t) = param else { continue };
                        let (pat, ty) = (&t.pat, &t.ty);
                        stmts.push(syn::parse_quote!(let #pat: #ty = #arg;));
                    }
                    stmts.extend(self.0.block.stmts.iter().cloned());
                    *e = Expr::Block(syn::ExprBlock {
                        attrs: Vec::new(),
                        label: None,
                        block: syn::Block {
                            brace_token: Default::default(),
                            stmts,
                        },
                    });
                    return;
                }
                visit_mut::visit_expr_mut(self, e);
            }
        }
        Inline(f).visit_file_mut(ast);
        return;
    }
}

/// The places of a format string's `{..}`s, each from its `{` to past its
/// `}`; not an escaped `{{` or `}}`.
fn placeholders(text: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut places = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' if bytes.get(i + 1) == Some(&b'{') => i += 2,
            b'}' if bytes.get(i + 1) == Some(&b'}') => i += 2,
            b'{' => match text[i..].find('}') {
                Some(end) => {
                    places.push((i, i + end + 1));
                    i += end + 1;
                }
                None => return Vec::new(),
            },
            _ => i += 1,
        }
    }
    places
}
