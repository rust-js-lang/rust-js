//! `loud` (#1203): a case made to say what std does with its values, as the
//! consumer net's `Loud` iterator does. Each struct and enum of the case's
//! own prints its drops, a `Drop` added where it has none; each that's
//! `Clone` its clones, a derived `Clone` written out; each closure its
//! calls, and each `for` loop its steps. Each is an event, numbered by one
//! counter, `~loud <n> <what>`, so what the JS drops, clones or calls, and
//! in what order, is a line native Rust's run of the rewrite is the oracle
//! of. `METAMORPH_PANIC_AT=<n>` makes the n-th event panic, where nothing
//! is unwinding yet, so the rest of the run is what an unwind does: the
//! harness picks the middle event of each kind (`loud-drop`, `loud-clone`,
//! `loud-call`, `loud-step`) from `loud`'s native run.
//!
//! A `Drop` rustc rejects, a type moved out of or one a `const fn` drops,
//! is taken off again: the rewriter asks rustc (`--emit=metadata`) and
//! takes each type an error names off, then all of them, then the clones.

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use std::collections::BTreeSet;
use std::process::Command;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::{Attribute, Block, Data, Fields, Item, Stmt, Token};

/// How many events are printed: past it they're counted, so a panic point
/// is still one, but a case's loop of a million calls prints no more.
const PRINTED: usize = 2000;

/// What's made loud, and what rustc took off.
struct Loud {
    /// Types given a `Drop` of the rewrite's own.
    drops: BTreeSet<String>,
    /// Whether a derived `Clone` is written out.
    clones: bool,
    /// Types a `Drop` is no longer added to.
    quiet: BTreeSet<String>,
    budget: usize,
    /// How many places say an event.
    events: usize,
}

/// What the case already says of each of its types, by name, in any scope.
#[derive(Default)]
struct Known {
    drop: BTreeSet<String>,
    copy: BTreeSet<String>,
}

impl<'a> Visit<'a> for Known {
    fn visit_item_impl(&mut self, imp: &'a syn::ItemImpl) {
        if let (Some((_, path, _)), syn::Type::Path(ty)) = (&imp.trait_, &*imp.self_ty)
            && let (Some(t), Some(on)) = (path.segments.last(), ty.path.segments.last())
        {
            match t.ident.to_string().as_str() {
                "Drop" => _ = self.drop.insert(on.ident.to_string()),
                "Copy" => _ = self.copy.insert(on.ident.to_string()),
                _ => {}
            }
        }
        visit::visit_item_impl(self, imp);
    }
}

/// Whether `attrs` derive `name`.
fn derives(attrs: &[Attribute], name: &str) -> bool {
    attrs.iter().any(|attr| {
        derived(attr)
            .iter()
            .any(|p| p.segments.last().is_some_and(|s| s.ident == name))
    })
}

fn derived(attr: &Attribute) -> Vec<syn::Path> {
    if !attr.path().is_ident("derive") {
        return Vec::new();
    }
    attr.parse_args_with(Punctuated::<syn::Path, Token![,]>::parse_terminated)
        .map(|paths| paths.into_iter().collect())
        .unwrap_or_default()
}

/// `attrs` without `Clone` in their derives, a derive of nothing left out.
fn underive_clone(attrs: &mut Vec<Attribute>) {
    attrs.retain_mut(|attr| {
        let paths = derived(attr);
        if paths.is_empty() {
            return true;
        }
        let kept: Vec<&syn::Path> = paths
            .iter()
            .filter(|p| !p.segments.last().is_some_and(|s| s.ident == "Clone"))
            .collect();
        if kept.is_empty() {
            return false;
        }
        *attr = syn::parse_quote!(#[derive(#(#kept),*)]);
        true
    });
}

/// A struct's or an enum's parts this needs.
struct Shape<'a> {
    ident: &'a syn::Ident,
    attrs: &'a [Attribute],
    generics: &'a syn::Generics,
    data: Data,
}

fn shape(item: &Item) -> Option<Shape<'_>> {
    match item {
        Item::Struct(s) => Some(Shape {
            ident: &s.ident,
            attrs: &s.attrs,
            generics: &s.generics,
            data: Data::Struct(syn::DataStruct {
                struct_token: s.struct_token,
                fields: s.fields.clone(),
                semi_token: s.semi_token,
            }),
        }),
        Item::Enum(e) => Some(Shape {
            ident: &e.ident,
            attrs: &e.attrs,
            generics: &e.generics,
            data: Data::Enum(syn::DataEnum {
                enum_token: e.enum_token,
                brace_token: e.brace_token,
                variants: e.variants.clone(),
            }),
        }),
        _ => None,
    }
}

/// Whether the type is `repr(packed)`, whose fields aren't borrowed.
fn packed(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().is_ident("repr") && a.to_token_stream().to_string().contains("packed"))
}

/// What an event says of `self`: its `Debug` where it derives one and
/// isn't generic, its name otherwise.
fn said(kind: &str, shape: &Shape) -> TokenStream {
    let name = shape.ident.to_string();
    if derives(shape.attrs, "Debug") && shape.generics.params.is_empty() {
        let text = format!("{kind} {{:?}}");
        quote!(metamorph_loud(format_args!(#text, self)))
    } else {
        let text = format!("{kind} {name}");
        quote!(metamorph_loud(format_args!(#text)))
    }
}

/// `Clone::clone` of each field of `shape`, as `derive(Clone)` writes it.
fn clone_body(shape: &Shape) -> TokenStream {
    let clone = quote!(::std::clone::Clone::clone);
    let fields = |fields: &Fields, by: &dyn Fn(usize, &syn::Field) -> TokenStream| -> (TokenStream, TokenStream) {
        match fields {
            Fields::Named(named) => {
                let names: Vec<_> = named.named.iter().map(|f| f.ident.clone().unwrap()).collect();
                let made: Vec<_> = named.named.iter().enumerate().map(|(i, f)| by(i, f)).collect();
                (quote!({ #(#names),* }), quote!({ #(#names: #made),* }))
            }
            Fields::Unnamed(unnamed) => {
                let names: Vec<_> = (0..unnamed.unnamed.len())
                    .map(|i| format_ident!("metamorph_{i}"))
                    .collect();
                let made: Vec<_> = unnamed.unnamed.iter().enumerate().map(|(i, f)| by(i, f)).collect();
                (quote!((#(#names),*)), quote!((#(#made),*)))
            }
            Fields::Unit => (quote!(), quote!()),
        }
    };
    match &shape.data {
        Data::Struct(s) => {
            let (_, made) = fields(&s.fields, &|i, f| match &f.ident {
                Some(name) => quote!(#clone(&self.#name)),
                None => {
                    let i = syn::Index::from(i);
                    quote!(#clone(&self.#i))
                }
            });
            quote!(Self #made)
        }
        Data::Enum(e) => {
            if e.variants.is_empty() {
                return quote!(match *self {});
            }
            let arms = e.variants.iter().map(|v| {
                let variant = &v.ident;
                let (bound, made) = fields(&v.fields, &|i, f| {
                    let name = f.ident.clone().unwrap_or_else(|| format_ident!("metamorph_{i}"));
                    quote!(#clone(#name))
                });
                quote!(Self::#variant #bound => Self::#variant #made,)
            });
            quote!(match self { #(#arms)* })
        }
        Data::Union(_) => unreachable!(),
    }
}

impl Loud {
    /// The impls `shape` is given: its `Drop`, and its `Clone` where it
    /// derived one, which `underive_clone` takes off its derives.
    fn impls(&mut self, item: &mut Item, known: &Known) -> Vec<Item> {
        let Some(shape) = shape(item) else { return Vec::new() };
        let name = shape.ident.to_string();
        let (impl_generics, ty_generics, where_clause) = shape.generics.split_for_impl();
        let ident = shape.ident;
        let mut made = Vec::new();
        let copy = known.copy.contains(&name) || derives(shape.attrs, "Copy");
        if !copy && !known.drop.contains(&name) && !self.quiet.contains(&name) && self.budget > 0 {
            self.budget -= 1;
            self.drops.insert(name.clone());
            self.events += 1;
            let said = said("drop", &shape);
            made.push(syn::parse_quote!(
                impl #impl_generics Drop for #ident #ty_generics #where_clause {
                    fn drop(&mut self) {
                        #said;
                    }
                }
            ));
        }
        let clones = self.clones && derives(shape.attrs, "Clone") && !packed(shape.attrs);
        if clones {
            let mut generics = shape.generics.clone();
            let params: Vec<syn::Ident> = generics.type_params().map(|p| p.ident.clone()).collect();
            let wheres = generics.make_where_clause();
            for p in params {
                wheres.predicates.push(syn::parse_quote!(#p: ::std::clone::Clone));
            }
            let (impl_generics, _, where_clause) = generics.split_for_impl();
            self.events += 1;
            let said = said("clone", &shape);
            let body = clone_body(&shape);
            made.push(syn::parse_quote!(
                impl #impl_generics ::std::clone::Clone for #ident #ty_generics #where_clause {
                    fn clone(&self) -> Self {
                        #said;
                        #body
                    }
                }
            ));
        }
        drop(shape);
        if clones {
            match item {
                Item::Struct(s) => underive_clone(&mut s.attrs),
                Item::Enum(e) => underive_clone(&mut e.attrs),
                _ => {}
            }
        }
        made
    }

    fn items(&mut self, items: &mut Vec<Item>, known: &Known) {
        let mut added = Vec::new();
        for item in items.iter_mut() {
            added.extend(self.impls(item, known));
            if let Item::Mod(m) = item
                && let Some((_, inner)) = &mut m.content
            {
                self.items(inner, known);
            }
        }
        items.extend(added);
    }
}

/// The ticks in what the case writes: each closure's call, each `for`
/// loop's step, and each `Drop` and `Clone` of its own.
struct Ticks<'a> {
    loud: &'a mut Loud,
    known: &'a Known,
    closures: usize,
    steps: usize,
}

impl Ticks<'_> {
    fn tick(&mut self, what: String) -> Stmt {
        self.loud.events += 1;
        syn::parse_quote!(metamorph_loud(format_args!(#what));)
    }
}

impl VisitMut for Ticks<'_> {
    fn visit_expr_closure_mut(&mut self, c: &mut syn::ExprClosure) {
        visit_mut::visit_expr_closure_mut(self, c);
        self.closures += 1;
        let tick = self.tick(format!("call {}", self.closures));
        // A body that isn't a block in parentheses, so a statement's start,
        // `unsafe { .. } as u32`, still reads as the expression it was.
        let body = match &*c.body {
            syn::Expr::Block(_) => c.body.to_token_stream(),
            body => quote!((#body)),
        };
        *c.body = syn::parse_quote!({ #tick #body });
    }

    fn visit_expr_for_loop_mut(&mut self, f: &mut syn::ExprForLoop) {
        visit_mut::visit_expr_for_loop_mut(self, f);
        self.steps += 1;
        let tick = self.tick(format!("step {}", self.steps));
        f.body.stmts.insert(0, tick);
    }

    fn visit_item_mod_mut(&mut self, m: &mut syn::ItemMod) {
        visit_mut::visit_item_mod_mut(self, m);
        // What says an event is the crate root's, which a module asks its
        // parent for: not `crate::`'s, as the native harness includes the
        // case in a module of its own.
        if let Some((_, items)) = &mut m.content {
            items.insert(
                0,
                syn::parse_quote!(
                    use super::metamorph_loud;
                ),
            );
        }
    }

    fn visit_block_mut(&mut self, block: &mut Block) {
        visit_mut::visit_block_mut(self, block);
        // A type a function declares is given its impls beside it.
        let mut added = Vec::new();
        for stmt in &mut block.stmts {
            if let Stmt::Item(item) = stmt {
                added.extend(self.loud.impls(item, self.known));
            }
        }
        let at = block
            .stmts
            .iter()
            .rposition(|s| matches!(s, Stmt::Item(_)))
            .map_or(0, |i| i + 1);
        block.stmts.splice(at..at, added.into_iter().map(Stmt::Item));
    }

    fn visit_item_impl_mut(&mut self, imp: &mut syn::ItemImpl) {
        visit_mut::visit_item_impl_mut(self, imp);
        // The case's own `Drop` and `Clone` say so first.
        let Some((_, path, _)) = &imp.trait_ else { return };
        let Some(t) = path.segments.last() else { return };
        let syn::Type::Path(on) = &*imp.self_ty else { return };
        let Some(on) = on.path.segments.last() else { return };
        let (method, kind) = match t.ident.to_string().as_str() {
            "Drop" => ("drop", "drop"),
            "Clone" => ("clone", "clone"),
            _ => return,
        };
        let tick = self.tick(format!("{kind} {}", on.ident));
        for item in &mut imp.items {
            if let syn::ImplItem::Fn(f) = item
                && f.sig.ident == method
            {
                f.block.stmts.insert(0, tick.clone());
            }
        }
    }
}

/// The counter, the panic point and what says each event.
fn prelude(panic_at: usize) -> Vec<Item> {
    vec![
        syn::parse_quote!(
            static METAMORPH_LOUD: ::std::sync::atomic::AtomicUsize = ::std::sync::atomic::AtomicUsize::new(0);
        ),
        syn::parse_quote!(
            const METAMORPH_PANIC_AT: usize = #panic_at;
        ),
        syn::parse_quote!(
            fn metamorph_loud(what: ::std::fmt::Arguments) {
                let n = METAMORPH_LOUD.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed) + 1;
                if n <= #PRINTED {
                    let said: String = ::std::string::ToString::to_string(&what).chars().take(160).collect();
                    println!("~loud {} {}", n, said);
                }
                if n == METAMORPH_PANIC_AT && !::std::thread::panicking() {
                    panic!("~loud panic at {}", n);
                }
            }
        ),
    ]
}

fn made(ast: &syn::File, loud: &mut Loud, panic_at: usize) -> syn::File {
    let mut ast = ast.clone();
    loud.events = 0;
    let mut known = Known::default();
    known.visit_file(&ast);
    let mut ticks = Ticks {
        loud,
        known: &known,
        closures: 0,
        steps: 0,
    };
    // Each function's closures and loops, and each type a block declares.
    ticks.visit_file_mut(&mut ast);
    let loud = ticks.loud;
    loud.items(&mut ast.items, &known);
    ast.items.extend(prelude(panic_at));
    ast
}

/// The types rustc's errors of `program` name, of `among`, and whether it
/// has any: `None` where rustc accepts it.
fn rejected(program: &str, among: &BTreeSet<String>) -> Option<BTreeSet<String>> {
    let edition = std::env::var("METAMORPH_EDITION").unwrap_or_else(|_| String::from("2024"));
    let dir = std::env::temp_dir().join(format!("metamorph-loud-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a temporary folder");
    let file = dir.join("loud.rs");
    std::fs::write(&file, program).expect("the program written");
    let out = Command::new("rustc")
        .args([
            &format!("--edition={edition}"),
            "--crate-type=bin",
            "--emit=metadata",
            "-Awarnings",
            "--crate-name=loud",
        ])
        .arg("-o")
        .arg(dir.join("loud.rmeta"))
        .arg(&file)
        .output();
    let _ = std::fs::remove_dir_all(&dir);
    let out = out.ok()?;
    if out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stderr);
    // Each name in a quoted part of a message, `Foo<T>`, `&mut Foo`.
    let named: BTreeSet<String> = text
        .split('`')
        .skip(1)
        .step_by(2)
        .flat_map(|quoted| {
            quoted
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .map(String::from)
                .collect::<Vec<_>>()
        })
        .filter(|word| among.contains(word))
        .collect();
    Some(named)
}

pub fn loud(ast: &mut syn::File, budget: &mut usize) -> bool {
    let panic_at: usize = std::env::var("METAMORPH_PANIC_AT")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    let mut loud = Loud {
        drops: BTreeSet::new(),
        clones: true,
        quiet: BTreeSet::new(),
        budget: *budget,
        events: 0,
    };
    let mut program = made(ast, &mut loud, panic_at);
    // rustc's errors take a `Drop` off each type they name, then off all,
    // then the clones.
    for _ in 0..8 {
        let Some(named) = rejected(&program.to_token_stream().to_string(), &loud.drops) else {
            break;
        };
        if !named.is_empty() {
            loud.quiet.extend(named);
        } else if !loud.drops.is_empty() {
            loud.quiet.extend(std::mem::take(&mut loud.drops));
        } else if loud.clones {
            loud.clones = false;
        } else {
            break;
        }
        loud.drops.clear();
        loud.budget = *budget;
        program = made(ast, &mut loud, panic_at);
    }
    *budget = loud.budget;
    *ast = program;
    loud.events > 0
}
