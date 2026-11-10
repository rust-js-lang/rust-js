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
//! - `option`, `rc-refcell` and `field`: the same, `Some(e)`,
//!   `Rc::new(RefCell::new(e))` and `MetamorphField(e)`, read
//!   `(*x.as_ref().unwrap())`, `(*x.borrow())` and `(x.0)` (`wrap.rs`).
//! - `dyn-ref`: each parameter `x: &T` or `x: &mut T` of a free function
//!   taken as `&dyn MetamorphGet<T>`, `&mut dyn MetamorphGetMut<T>`, and
//!   read back as `&T` first: what a call gives coerces, so calls are as
//!   they were.
//! - `impl-ref`: the same, taken as `&impl MetamorphGet<T>`, generic code.
//! - `impl-owned`: each parameter `x: T` of a free function taken as
//!   `impl MetamorphIs<T>`, and read back as `T` first.
//! - `generic`: a free function's first concrete parameter type made a type
//!   parameter, called with that type (`generic.rs`).
//! - `closure-fn`: a closure that captures nothing made a nested `fn`, and
//!   a free function given as a value a closure that calls it
//!   (`closures.rs`).
//! - `library`: every item but `main` moved into a library crate the
//!   program uses, printed library first (`library.rs`).
//! - `array` and `tuple`: each `let x = e;` as `option`'s, held in an array
//!   of one, `[e]`, read `(x[0])`, and a tuple of one, `(e,)`, read
//!   `(x.0)` (`wrap.rs`).
//! - `for-each`: a `for` loop whose body leaves it only at its end made
//!   `(e).into_iter().for_each(|x| body)`; `for-loop`, a `for_each` of a
//!   closure made a `for` loop (`loops.rs`).
//! - `if-let`: a `match` of two arms, the second `_`, made an `if let`, and
//!   a `let` of one whose first arm gives what it binds a `let else`
//!   (`matches.rs`).
//! - `method-fn`: an inherent method's body moved into a free function
//!   given its receiver; `fn-method`, a free function of a reference to one
//!   of the case's types made a method of it the function calls
//!   (`methods.rs`).
//! - `default-copy`: a trait's default method written out in each impl of
//!   it that takes it (`defaults.rs`).
//! - `tuple-struct`: a struct of named fields made a tuple struct, its
//!   fields read by their places (`tuples.rs`).
//! - `loud`: the case made to say what std does with its values, each
//!   struct's and enum's drops and clones, each closure's calls and each
//!   `for` loop's steps a numbered line, `~loud <n> <what>`; with
//!   `METAMORPH_PANIC_AT=<n>`, a panic at the n-th (`loud.rs`, #1203).
//! - `opaque`: each iterator the case makes given on as an `impl Iterator`,
//!   promising what the iterator is of `DoubleEndedIterator`, `Clone`, ..
//!   (`opaque.rs`, #1204); `dyn-fn`, each closure called through a
//!   `Box<dyn Fn>` (`dyn_fn.rs`); each narrowed to the places rustc
//!   accepts (`narrow.rs`).
//! - `generic-instances`: each consumer or adapter the case calls on an
//!   iterator it makes called in generic code, given another instance of
//!   the iterator, a `filter`, a `chain`, a `map`, the net's own; and each
//!   value it copies, clones, makes a `Some` of or drops, done in generic
//!   code (`instances.rs`, #1237).
//! - `fact-<fact>`: the case given a neighbour, never run, that turns on
//!   one of the facts rust-js decides for the whole crate, an address
//!   asked, a capacity, a function's identity (`facts.rs`, #1204).
//!
//! - `emi-probe`, `emi-prune` and `emi-insert`: code a case's run never
//!   reaches, found by the probe's native run, deleted, made
//!   `unreachable!()`, or code never run added, `if black_box(false) {..}`
//!   (`emi.rs`, #1086).
//!
//! `<limit>`: only so many of the places a rewrite finds, in order.
//!
//! And `reduce`, a failing program's reductions (`reduce.rs`, #1088):
//!
//!   metamorph reduce <case.rs> [<n>]   # how many, or the program with the n-th made

mod closures;
mod defaults;
mod dyn_fn;
mod emi;
mod facts;
mod generic;
mod instances;
mod library;
mod loops;
mod loud;
mod matches;
mod methods;
mod narrow;
mod opaque;
mod reduce;
mod tuples;
mod wrap;

use proc_macro2::TokenTree;
use quote::{ToTokens, quote};
use std::collections::BTreeMap;
use syn::visit::{self, Visit};
use syn::{Block, FnArg, Item, Pat, Stmt, Type};

use wrap::Wrap;

const REWRITES: &str = "outline|boxed|option|rc-refcell|field|dyn-ref|impl-ref|impl-owned|generic|closure-fn|library|array|tuple|for-each|for-loop|if-let|method-fn|fn-method|default-copy|tuple-struct|loud|opaque|dyn-fn|generic-instances|fact-<fact>|emi-probe|emi-prune|emi-insert";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // A failing program's reductions (#1088, `reduce.rs`): how many, or one.
    if let [command, file, rest @ ..] = &args[1..]
        && command == "reduce"
    {
        let source = std::fs::read_to_string(file).expect("the program");
        let mut ast = syn::parse_file(&source).expect("the program parses");
        let target = rest.first().map(|n| n.parse::<usize>().expect("a reduction's number"));
        match reduce::reduce(&mut ast, target) {
            reduce::Reduced::Made => print!("{}", ast.to_token_stream()),
            reduce::Reduced::Count(n) if target.is_none() => println!("{n}"),
            reduce::Reduced::Count(_) => std::process::exit(2),
        }
        return;
    }
    let (rewrite, file, limit) = match &args[1..] {
        [rewrite, file] => (rewrite.as_str(), file, usize::MAX),
        [rewrite, file, limit] => (rewrite.as_str(), file, limit.parse().expect("a limit")),
        _ => {
            eprintln!("metamorph <{REWRITES}> <case.rs> [<limit>]");
            std::process::exit(64);
        }
    };
    let source = std::fs::read_to_string(file).expect("the case");
    let mut ast = syn::parse_file(&source).expect("the case parses");
    let mut budget = limit;
    if rewrite == "library" {
        // A function's identity across crates is no promise of Rust's.
        if closures::compares_identity(&source) {
            std::process::exit(2);
        }
        match library::library(&ast, &mut budget) {
            Some(both) => print!("{both}"),
            None => std::process::exit(2),
        }
        return;
    }
    let changed = match rewrite {
        "outline" => outline(&mut ast),
        "boxed" => wrap::wrap(&mut ast, Wrap::Boxed, &mut budget),
        "option" => wrap::wrap(&mut ast, Wrap::Option, &mut budget),
        "rc-refcell" => wrap::wrap(&mut ast, Wrap::RcRefCell, &mut budget),
        "field" => wrap::wrap(&mut ast, Wrap::Field, &mut budget),
        "array" => wrap::wrap(&mut ast, Wrap::Array, &mut budget),
        "tuple" => wrap::wrap(&mut ast, Wrap::Tuple, &mut budget),
        "dyn-ref" => refs(&mut ast, Refs::Dyn, &mut budget),
        "impl-ref" => refs(&mut ast, Refs::Impl, &mut budget),
        "impl-owned" => owned(&mut ast, &mut budget),
        "generic" => generic::generic(&mut ast, &mut budget),
        "closure-fn" => closures::closure_fn(&mut ast, file, &source, &mut budget),
        "for-each" => loops::for_each(&mut ast, &mut budget),
        "for-loop" => loops::for_loop(&mut ast, &mut budget),
        "if-let" => matches::if_let(&mut ast, &mut budget),
        "method-fn" => methods::method_fn(&mut ast, &mut budget),
        "fn-method" => methods::fn_method(&mut ast, &mut budget),
        "default-copy" => defaults::default_copy(&mut ast, &mut budget),
        "tuple-struct" => tuples::tuple_struct(&mut ast, &mut budget),
        "loud" => loud::loud(&mut ast, &mut budget),
        "opaque" => opaque::opaque(&mut ast, &mut budget),
        "generic-instances" => instances::generic_instances(&mut ast, &source, &mut budget),
        "dyn-fn" => dyn_fn::dyn_fn(&mut ast, file, &source, &mut budget),
        fact if facts::FACTS.contains(&fact) => facts::fact(&mut ast, fact),
        "emi-probe" => emi::probe(&mut ast),
        "emi-prune" => match emi::hits_given() {
            Some(hits) => emi::prune(&mut ast, &hits, &mut budget),
            None => {
                eprintln!("emi-prune: METAMORPH_EMI_HITS, the probe's blocks reached");
                std::process::exit(64);
            }
        },
        "emi-insert" => emi::insert(&mut ast, &mut budget),
        _ => {
            eprintln!("no rewrite `{rewrite}`: {REWRITES}");
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

/// Whether `f` is a binding, `#[cfg_attr(rust_js, rust_js::link_name =
/// "Object.assign")]`'s: its JS is another's, which takes what its Rust
/// signature says, so its parameters aren't rewritten.
fn bound_to_js(f: &syn::ItemFn) -> bool {
    (f.attrs.iter()).any(|attr| attr.to_token_stream().to_string().contains("rust_js"))
}

fn refs(ast: &mut syn::File, how: Refs, budget: &mut usize) -> bool {
    let mut changed = false;
    for item in &mut ast.items {
        let Item::Fn(f) = item else { continue };
        if f.sig.ident == "main" || f.sig.constness.is_some() || bound_to_js(f) {
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
            *arg.ty = match mutable {
                false => syn::parse_quote!(& #lifetime #referent),
                true => syn::parse_quote!(& #lifetime mut #referent),
            };
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
        if f.sig.ident == "main" || f.sig.constness.is_some() || bound_to_js(f) {
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
            *arg.ty = syn::parse_quote!(impl MetamorphIs<#ty>);
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
