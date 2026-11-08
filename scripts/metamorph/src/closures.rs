//! `closure-fn`: a closure that captures nothing made a function item
//! nested in its function, `|x| x * 2` → `fn metamorph_closure_0(x: &i64)
//! -> i64 { x * 2 }`, its types rustc's (its MIR's signature of the
//! closure); and a free function the case passes as a value made a
//! closure that calls it, `map(double)` → `map(|a0| double(a0))`.

use proc_macro2::{TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote};
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::{Block, Expr, FnArg, Item, Pat, Stmt};

use crate::Bindings;

/// What a case says that makes a function's identity, or its type's name,
/// part of what it prints: a function and a closure aren't alike there.
const IDENTITY: &[&str] = &["fn_addr_eq", "ptr::eq", "type_name", "as fn"];

pub fn closure_fn(ast: &mut syn::File, file: &str, source: &str, budget: &mut usize) -> bool {
    if IDENTITY.iter().any(|said| source.contains(said)) {
        return false;
    }
    let made = closures_made_fns(ast, file, budget);
    let given = fns_given_as_closures(ast, budget);
    made || given
}

/// A closure's signature, as rustc's MIR prints it: its parameters' types
/// and what it returns.
#[derive(Debug, PartialEq)]
struct Signature {
    inputs: Vec<String>,
    output: String,
}

/// Each closure's signature in rustc's MIR of `file`, by where it begins,
/// its line and its column (1-based). Empty where rustc can't say.
fn signatures(file: &str) -> BTreeMap<(usize, usize), Signature> {
    let edition = std::env::var("METAMORPH_EDITION").unwrap_or_else(|_| String::from("2024"));
    let out = Command::new("rustc")
        .args([
            &format!("--edition={edition}"),
            "--crate-type=lib",
            "-Zunpretty=mir",
            "-Ztrim-diagnostic-paths=false",
            "-Awarnings",
            file,
        ])
        .env("RUSTC_BOOTSTRAP", "1")
        .output();
    match out {
        Ok(out) if out.status.success() => parse_mir(&String::from_utf8_lossy(&out.stdout)),
        _ => BTreeMap::new(),
    }
}

/// `text`'s top-level parts, split at each `,` outside brackets.
fn split_top(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut part = String::new();
    for c in text.chars() {
        match c {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(std::mem::take(&mut part).trim().to_string());
                continue;
            }
            _ => {}
        }
        part.push(c);
    }
    if !part.trim().is_empty() {
        parts.push(part.trim().to_string());
    }
    parts
}

/// Each closure's MIR header, `fn main::{closure#0}(_1: &{closure@f.rs:9:36:
/// 9:39}, _2: &i64) -> i64 {`: its environment, the first parameter, says
/// where it is.
fn parse_mir(mir: &str) -> BTreeMap<(usize, usize), Signature> {
    let mut found = BTreeMap::new();
    for line in mir.lines() {
        let Some(rest) = line.strip_prefix("fn ") else {
            continue;
        };
        let Some(open) = rest.find("(_1: ") else { continue };
        if !rest[..open].ends_with('}') || !rest[..open].contains("{closure#") {
            continue;
        }
        let Some(body) = rest.strip_suffix(" {") else { continue };
        let Some(arrow) = body.rfind(") -> ") else { continue };
        let params = &body[open + 1..arrow];
        let output = body[arrow + 5..].to_string();
        let parts = split_top(params);
        let Some(env) = parts.first() else { continue };
        let Some(at) = env.find("{closure@") else { continue };
        let place: Vec<&str> = env[at..].split(':').collect();
        let (Some(line), Some(column)) = (
            place.get(1).and_then(|l| l.parse().ok()),
            place.get(2).and_then(|c| c.parse().ok()),
        ) else {
            continue;
        };
        let inputs = parts[1..]
            .iter()
            .map(|p| p.split_once(": ").map(|(_, t)| t.to_string()).unwrap_or_default())
            .collect();
        found.insert((line, column), Signature { inputs, output });
    }
    found
}

/// Whether `ty`, a type MIR printed, can be written in a function's
/// signature: no closure's, opaque or inferred type, and none of the
/// enclosing function's type parameters or `Self`.
fn writable(ty: &str, generics: &BTreeSet<String>) -> bool {
    if ty.is_empty() || ty.contains('{') || ty.contains("impl ") || ty.contains("'?") || ty.contains("dyn ") {
        return false;
    }
    let words = ty.split(|c: char| !(c.is_alphanumeric() || c == '_'));
    !words.into_iter().any(|w| w == "Self" || generics.contains(w))
}

/// The names used in a closure's body: each path of one segment, each
/// ident in a macro's tokens, and each name a format string's text names.
#[derive(Default)]
struct Used {
    names: BTreeSet<String>,
    texts: Vec<String>,
    this: bool,
}

impl Used {
    fn tokens(&mut self, stream: TokenStream) {
        for tree in stream {
            match tree {
                TokenTree::Ident(i) => {
                    if i == "self" {
                        self.this = true;
                    }
                    self.names.insert(i.to_string());
                }
                TokenTree::Literal(l) => self.texts.push(l.to_string()),
                TokenTree::Group(g) => self.tokens(g.stream()),
                TokenTree::Punct(_) => {}
            }
        }
    }
}

impl<'a> Visit<'a> for Used {
    fn visit_expr_path(&mut self, p: &'a syn::ExprPath) {
        if let Some(ident) = p.path.get_ident() {
            self.names.insert(ident.to_string());
        }
        if p.path
            .segments
            .first()
            .is_some_and(|s| s.ident == "self" || s.ident == "Self")
        {
            self.this = true;
        }
        visit::visit_expr_path(self, p);
    }
    fn visit_macro(&mut self, m: &'a syn::Macro) {
        self.tokens(m.tokens.clone());
    }
    fn visit_item(&mut self, _: &'a Item) {}
}

/// Whether `closure` takes nothing from its function: no name it uses is
/// one its function binds and it doesn't, `self` neither, a format
/// string's `{name}` neither. A name it binds itself is its own where it's
/// used: a use of the one outside before a `let` of the name inside is a
/// capture this misses, which the function can't name, so rustc rejects it.
fn captures_nothing(closure: &syn::ExprClosure, around: &Bindings) -> bool {
    let mut inside = Bindings::default();
    inside.visit_expr_closure(closure);
    let outside: BTreeSet<&String> = (around.0.keys()).filter(|name| !inside.0.contains_key(*name)).collect();
    let mut used = Used::default();
    used.visit_expr(&closure.body);
    !used.this
        && !used.names.iter().any(|n| outside.contains(n))
        && !(used.texts.iter()).any(|text| outside.iter().any(|n| text.contains(&format!("{{{n}"))))
}

fn closures_made_fns(ast: &mut syn::File, file: &str, budget: &mut usize) -> bool {
    if *budget == 0 {
        return false;
    }
    let mut any = Any(false);
    any.visit_file(ast);
    if !any.0 {
        return false;
    }
    let signatures = signatures(file);
    let mut count = 0;
    let mut changed = false;
    let mut each = |generics: BTreeSet<String>, sig: &syn::Signature, block: &mut Block| {
        let mut around = Bindings::default();
        for input in &sig.inputs {
            around.visit_fn_arg(input);
        }
        around.visit_block(block);
        let mut made = Made {
            signatures: &signatures,
            generics: &generics,
            around: &around,
            count: &mut count,
            budget,
            fns: Vec::new(),
        };
        made.visit_block_mut(block);
        if !made.fns.is_empty() {
            changed = true;
            let fns: Vec<Stmt> = made.fns.into_iter().map(|f| Stmt::Item(Item::Fn(f))).collect();
            block.stmts.splice(0..0, fns);
        }
    };
    for item in &mut ast.items {
        match item {
            Item::Fn(f) => each(type_params(&f.sig.generics), &f.sig, &mut f.block),
            Item::Impl(imp) => {
                let outer = type_params(&imp.generics);
                for item in &mut imp.items {
                    if let syn::ImplItem::Fn(f) = item {
                        let mut generics = type_params(&f.sig.generics);
                        generics.extend(outer.iter().cloned());
                        each(generics, &f.sig, &mut f.block);
                    }
                }
            }
            _ => {}
        }
    }
    changed
}

fn type_params(generics: &syn::Generics) -> BTreeSet<String> {
    generics.type_params().map(|p| p.ident.to_string()).collect()
}

/// Whether a file has a closure.
struct Any(bool);

impl<'a> Visit<'a> for Any {
    fn visit_expr_closure(&mut self, _: &'a syn::ExprClosure) {
        self.0 = true;
    }
}

/// Each closure that captures nothing, and whose types rustc names, made a
/// function: its parameters' patterns given their types.
struct Made<'s> {
    signatures: &'s BTreeMap<(usize, usize), Signature>,
    generics: &'s BTreeSet<String>,
    around: &'s Bindings,
    count: &'s mut usize,
    budget: &'s mut usize,
    fns: Vec<syn::ItemFn>,
}

impl Made<'_> {
    fn function(&mut self, closure: &syn::ExprClosure) -> Option<syn::ItemFn> {
        if *self.budget == 0
            || closure.asyncness.is_some()
            || closure.constness.is_some()
            || closure.lifetimes.is_some()
        {
            return None;
        }
        let start = closure.span().start();
        let sig = self.signatures.get(&(start.line, start.column + 1))?;
        if sig.inputs.len() != closure.inputs.len()
            || !sig.inputs.iter().all(|t| writable(t, self.generics))
            || !writable(&sig.output, self.generics)
            || !captures_nothing(closure, self.around)
        {
            return None;
        }
        let mut inputs = Vec::new();
        for (pat, ty) in closure.inputs.iter().zip(&sig.inputs) {
            let pat = match pat {
                Pat::Type(t) => (*t.pat).clone(),
                p => p.clone(),
            };
            let ty: syn::Type = syn::parse_str(ty).ok()?;
            inputs.push(quote!(#pat: #ty));
        }
        let output = match sig.output.as_str() {
            "()" => None,
            out => {
                let ty: syn::Type = syn::parse_str(out).ok()?;
                Some(quote!(-> #ty))
            }
        };
        let body = match &*closure.body {
            Expr::Block(b) if b.label.is_none() && b.attrs.is_empty() => b.block.to_token_stream(),
            e => quote!({ #e }),
        };
        let name = format_ident!("metamorph_closure_{}", *self.count);
        *self.count += 1;
        *self.budget -= 1;
        syn::parse2(quote!(fn #name(#(#inputs),*) #output #body)).ok()
    }
}

impl VisitMut for Made<'_> {
    fn visit_expr_mut(&mut self, e: &mut Expr) {
        if let Expr::Closure(closure) = e
            && let Some(f) = self.function(closure)
        {
            let name = &f.sig.ident;
            *e = syn::parse_quote!(#name);
            self.fns.push(f);
            return;
        }
        visit_mut::visit_expr_mut(self, e);
    }
    // A closure given to std's macros of expressions, `println!("{:?}",
    // v.map(|o| o.flatten()))`.
    fn visit_macro_mut(&mut self, m: &mut syn::Macro) {
        const EXPRESSIONS: &[&str] = &[
            "print",
            "println",
            "eprint",
            "eprintln",
            "format",
            "write",
            "writeln",
            "panic",
            "assert",
            "assert_eq",
            "assert_ne",
            "debug_assert",
            "debug_assert_eq",
            "debug_assert_ne",
            "vec",
            "dbg",
        ];
        if !EXPRESSIONS.iter().any(|name| m.path.is_ident(name)) {
            return;
        }
        let parser = syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated;
        let Ok(mut args) = syn::parse::Parser::parse2(parser, m.tokens.clone()) else {
            return;
        };
        for arg in args.iter_mut() {
            self.visit_expr_mut(arg);
        }
        m.tokens = args.to_token_stream();
    }
    fn visit_item_mut(&mut self, _: &mut Item) {}
}

/// Each free function of the case, by name, with how many parameters it
/// takes: a path of its name that isn't called is the function as a value.
fn fns_given_as_closures(ast: &mut syn::File, budget: &mut usize) -> bool {
    let mut locals = BTreeSet::new();
    struct Locals<'l>(&'l mut BTreeSet<String>);
    impl<'a> Visit<'a> for Locals<'_> {
        fn visit_pat_ident(&mut self, p: &'a syn::PatIdent) {
            self.0.insert(p.ident.to_string());
            visit::visit_pat_ident(self, p);
        }
    }
    Locals(&mut locals).visit_file(ast);
    let fns: BTreeMap<String, usize> = (ast.items.iter())
        .filter_map(|item| match item {
            Item::Fn(f)
                if f.sig.ident != "main"
                    && !locals.contains(&f.sig.ident.to_string())
                    && !f.sig.inputs.iter().any(|i| matches!(i, FnArg::Receiver(_)))
                    && f.sig.variadic.is_none() =>
            {
                Some((f.sig.ident.to_string(), f.sig.inputs.len()))
            }
            _ => None,
        })
        .collect();
    if fns.is_empty() || *budget == 0 {
        return false;
    }
    let mut values = Values {
        fns: &fns,
        budget,
        changed: false,
    };
    values.visit_file_mut(ast);
    values.changed
}

struct Values<'f> {
    fns: &'f BTreeMap<String, usize>,
    budget: &'f mut usize,
    changed: bool,
}

impl Values<'_> {
    /// `path` called by a closure of as many parameters as it takes.
    fn closure(&mut self, path: TokenStream, arity: usize) -> TokenStream {
        *self.budget -= 1;
        self.changed = true;
        let args: Vec<_> = (0..arity).map(|i| format_ident!("a{i}")).collect();
        quote!((|#(#args),*| #path(#(#args),*)))
    }

    fn named(&self, p: &syn::ExprPath) -> Option<usize> {
        let ident = &p.path.segments.last()?.ident;
        let local = p.path.segments.len() == 1
            || (p.path.segments.len() == 2
                && ["crate", "self"].contains(&p.path.segments[0].ident.to_string().as_str()));
        (p.qself.is_none() && local).then(|| self.fns.get(&ident.to_string()).copied())?
    }

    fn tokens(&mut self, stream: TokenStream) -> TokenStream {
        let trees: Vec<TokenTree> = stream.into_iter().collect();
        let mut out = Vec::new();
        for (i, tree) in trees.iter().enumerate() {
            let punct = |at: Option<&TokenTree>, c: char| matches!(at, Some(TokenTree::Punct(p)) if p.as_char() == c);
            match tree {
                TokenTree::Ident(id) if self.fns.contains_key(&id.to_string()) && *self.budget > 0 => {
                    let before = i.checked_sub(1).and_then(|i| trees.get(i));
                    let after = trees.get(i + 1);
                    let keyword = matches!(before, Some(TokenTree::Ident(k)) if k == "fn");
                    let called = matches!(after, Some(TokenTree::Group(g)) if g.delimiter() == proc_macro2::Delimiter::Parenthesis);
                    if keyword
                        || called
                        || punct(before, '.')
                        || punct(before, ':')
                        || punct(after, ':')
                        || punct(after, '!')
                    {
                        out.push(tree.clone());
                    } else {
                        let arity = self.fns[&id.to_string()];
                        out.extend(self.closure(id.to_token_stream(), arity));
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

impl VisitMut for Values<'_> {
    fn visit_expr_mut(&mut self, e: &mut Expr) {
        match e {
            // A call's own function is called, not given.
            Expr::Call(call) => {
                if let Expr::Path(_) = &*call.func {
                    for arg in &mut call.args {
                        self.visit_expr_mut(arg);
                    }
                    return;
                }
            }
            Expr::Path(p) if *self.budget > 0 => {
                if let Some(arity) = self.named(p) {
                    let path = p.to_token_stream();
                    *e = syn::parse2(self.closure(path, arity)).expect("a closure");
                    return;
                }
            }
            _ => {}
        }
        visit_mut::visit_expr_mut(self, e);
    }
    fn visit_macro_mut(&mut self, m: &mut syn::Macro) {
        m.tokens = self.tokens(std::mem::take(&mut m.tokens));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closures_signature_is_read_from_its_mir_header() {
        let mir = "fn main() -> () {\n\
                   fn main::{closure#0}(_1: &mut {closure@rv.rs:9:36: 9:39}, _2: &i64) -> i64 {\n\
                   fn main::{closure#2}(_1: &{closure@rv.rs:11:13: 11:45}, _2: m::P, _3: &Q<'_>) -> (m::P, usize) {\n\
                   fn m::P(_1: i64) -> m::P {\n";
        let found = parse_mir(mir);
        assert_eq!(found.len(), 2);
        assert_eq!(
            found[&(9, 36)],
            Signature {
                inputs: vec!["&i64".into()],
                output: "i64".into()
            }
        );
        assert_eq!(
            found[&(11, 13)],
            Signature {
                inputs: vec!["m::P".into(), "&Q<'_>".into()],
                output: "(m::P, usize)".into()
            }
        );
    }

    #[test]
    fn a_type_of_a_closure_or_a_type_parameter_is_not_written() {
        let generics: BTreeSet<String> = ["T".to_string()].into();
        assert!(writable("std::vec::Vec<i64>", &generics));
        assert!(!writable("{closure@a.rs:1:1: 1:2}", &generics));
        assert!(!writable("&T", &generics));
        assert!(!writable("Self", &generics));
    }
}
