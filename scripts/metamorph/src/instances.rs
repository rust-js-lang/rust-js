//! `generic-instances` (#1237): each consumer or adapter the case calls on
//! an iterator it makes, `v.iter().size_hint()`, `it.map(f).take(2)`, called
//! in generic code instead, `metamorph_generic::at_0(v.iter())`, of `fn
//! at_0<I: Iterator>(it: I) -> (usize, Option<usize>) { it.size_hint() }`,
//! and given another instance of the iterator than the case's own: the
//! case's, a `filter` that keeps each item, a `chain` of nothing, a `map`
//! of each item to itself, a `fuse`, or the net's own iterator, which
//! steps and hints as the one it holds. Which instance a place is given
//! turns with the place and the case, so the corpus asks each of each
//! method. Most of what generic code decides is of every instance its
//! callers give it; native Rust's run of the rewrite is its oracle. A place
//! rustc rejects is given the case's own instance, then left as it was;
//! an adapter's promises, `DoubleEndedIterator`, .., are what its instance
//! keeps (`narrow.rs`).
//!
//! And what the case does with a value, done in generic code of a `T`
//! that's each value the corpus has, a number, an object, a dropping
//! type, an `Option`: `x.clone()` cloned there, `Some(x)` made there,
//! `drop(x)` dropped there, `*r` given to a call, a macro or a `let`
//! copied there (`VALUES`).

use quote::{ToTokens, format_ident, quote};
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, GenericArgument};

use crate::narrow;
use crate::opaque::MAKERS;

const MODULE: &str = "metamorph_generic";

/// The instances a place may be given of the iterator the case makes,
/// `{it}` its own.
const INSTANCES: &[&str] = &[
    "{it}",
    "{it}.filter(|_| true)",
    "{it}.chain(::std::iter::empty())",
    "{it}.map(|item| item)",
    "{it}.fuse()",
    "metamorph_generic::Own({it})",
];

/// Which of `INSTANCES` answer `method` at the cost the case's own
/// iterator may, without stepping each item it passes, by Rust's own
/// semantics: std's `Range::nth`, `count`, `last`, `min` and `max`, and a
/// `skip`'s and a `step_by`'s, which ask its `nth`, don't step each number,
/// where a `map` calls its function on each item it passes, a `filter`
/// tests each, and the net's own iterator has none of its own; a `chain`
/// and a `fuse` pass `nth`, `count` and `last` on, not `min` or `max`.
fn keeps_its_cost(instance: usize, method: &str) -> bool {
    match INSTANCES[instance] {
        "{it}" => true,
        "{it}.chain(::std::iter::empty())" | "{it}.fuse()" => !matches!(method, "min" | "max"),
        _ => !matches!(method, "count" | "last" | "nth" | "min" | "max" | "skip" | "step_by"),
    }
}

/// The instance of the `n`th place that calls an iterator's `method`, of a
/// case whose instances start turning at `shift`: each in turn. Of a case
/// whose `//@ instances-keep-cost` says its point is what its iterators
/// answer without stepping each item, `METAMORPH_KEEP_COST` (corpus-cases.ts),
/// the case's own where the turn's would answer `method` stepping each
/// (`keeps_its_cost`): skipping four billion numbers at once through a
/// `map` steps each, by Rust's own semantics, which native Rust finishes
/// only as its optimizer folds the loop away (#1109).
fn instance_of(method: usize, n: usize, shift: usize) -> usize {
    let turned = (n + shift) % INSTANCES.len();
    let kept = std::env::var_os("METAMORPH_KEEP_COST").is_none() || keeps_its_cost(turned, METHODS[method].name);
    match kept {
        true => turned,
        false => 0,
    }
}

/// What an adapter made in generic code may promise besides `Iterator`.
const PROMISES: &[(&str, &str)] = &[
    ("DoubleEndedIterator", "::std::iter::DoubleEndedIterator"),
    ("ExactSizeIterator", "::std::iter::ExactSizeIterator"),
    ("FusedIterator", "::std::iter::FusedIterator"),
];

/// A method generic code calls: its name, how many arguments it takes, and
/// the function of a place, given `#at`, the type parameters after `I`,
/// and what it promises (an adapter's).
struct Method {
    name: &'static str,
    args: usize,
    /// The bound `I` needs besides `Iterator`, and what's added to the
    /// function's own type parameters, `B: FromIterator<I::Item>`.
    bound: &'static str,
    params: &'static str,
    output: &'static str,
    body: &'static str,
    adapter: bool,
}

const METHODS: &[Method] = &[
    Method {
        name: "size_hint",
        args: 0,
        bound: "",
        params: "",
        output: "(usize, ::std::option::Option<usize>)",
        body: "it.size_hint()",
        adapter: false,
    },
    Method {
        name: "count",
        args: 0,
        bound: "",
        params: "",
        output: "usize",
        body: "it.count()",
        adapter: false,
    },
    Method {
        name: "last",
        args: 0,
        bound: "",
        params: "",
        output: "::std::option::Option<I::Item>",
        body: "it.last()",
        adapter: false,
    },
    Method {
        name: "nth",
        args: 1,
        bound: "",
        params: "",
        output: "::std::option::Option<I::Item>",
        body: "{ let mut it = it; it.nth(a0) }",
        adapter: false,
    },
    Method {
        name: "len",
        args: 0,
        bound: "+ ::std::iter::ExactSizeIterator",
        params: "",
        output: "usize",
        body: "it.len()",
        adapter: false,
    },
    Method {
        name: "max",
        args: 0,
        bound: "<Item = T>",
        params: ", T: ::std::cmp::Ord",
        output: "::std::option::Option<T>",
        body: "it.max()",
        adapter: false,
    },
    Method {
        name: "min",
        args: 0,
        bound: "<Item = T>",
        params: ", T: ::std::cmp::Ord",
        output: "::std::option::Option<T>",
        body: "it.min()",
        adapter: false,
    },
    Method {
        name: "collect",
        args: 0,
        bound: "",
        params: ", B: ::std::iter::FromIterator<I::Item>",
        output: "B",
        body: "it.collect()",
        adapter: false,
    },
    Method {
        name: "sum",
        args: 0,
        bound: "",
        params: ", S: ::std::iter::Sum<I::Item>",
        output: "S",
        body: "it.sum()",
        adapter: false,
    },
    Method {
        name: "product",
        args: 0,
        bound: "",
        params: ", S: ::std::iter::Product<I::Item>",
        output: "S",
        body: "it.product()",
        adapter: false,
    },
    Method {
        name: "take",
        args: 1,
        bound: "",
        params: "",
        output: "",
        body: "it.take(a0)",
        adapter: true,
    },
    Method {
        name: "skip",
        args: 1,
        bound: "",
        params: "",
        output: "",
        body: "it.skip(a0)",
        adapter: true,
    },
    Method {
        name: "step_by",
        args: 1,
        bound: "",
        params: "",
        output: "",
        body: "it.step_by(a0)",
        adapter: true,
    },
    Method {
        name: "rev",
        args: 0,
        bound: "+ ::std::iter::DoubleEndedIterator",
        params: "",
        output: "",
        body: "it.rev()",
        adapter: true,
    },
];

/// What generic code does with a value the case gives it, each a place of
/// its own, `metamorph_generic::at_k(..)`: its function, `{at}` its name,
/// of a `T` that's each value the corpus has, a number, an object, a
/// dropping type, an `Option` (#1237). `x.clone()` cloned there, `Some(x)`
/// made there, `drop(x)` dropped there, and `*r`, given to a call, a macro
/// or a `let`, copied there.
const VALUES: &[(Value, &str)] = &[
    (
        Value::Clone,
        "pub fn {at}<T: ::std::clone::Clone>(x: &T) -> T { x.clone() }",
    ),
    (
        Value::Some,
        "pub fn {at}<T>(x: T) -> ::std::option::Option<T> { ::std::option::Option::Some(x) }",
    ),
    (Value::Drop, "pub fn {at}<T>(x: T) { ::std::mem::drop(x) }"),
    (Value::Copy, "pub fn {at}<T: ::std::marker::Copy>(x: &T) -> T { *x }"),
];

#[derive(Clone, Copy, PartialEq)]
enum Value {
    Clone,
    Some,
    Drop,
    Copy,
}

/// What a place calls in generic code: an iterator's method, of
/// `METHODS`, or what's done with a value, of `VALUES`.
#[derive(Clone, Copy, PartialEq)]
enum Op {
    Method(usize),
    Value(Value),
}

/// The arguments' types of a method of `args` arguments, each a `usize`:
/// `nth`'s, `take`'s, `skip`'s and `step_by`'s.
fn arguments(args: usize) -> Vec<syn::FnArg> {
    (0..args)
        .map(|i| {
            let a = format_ident!("a{i}");
            syn::parse_quote!(#a: usize)
        })
        .collect()
}

/// A place: what it calls, its instance, and an adapter's promises; `None`
/// where it's left as it was. A value's place has the case's own instance,
/// `0`, alone.
#[derive(Clone)]
struct Place {
    op: Op,
    instance: usize,
    promised: Vec<bool>,
}

struct Wrap<'p> {
    places: &'p [Option<Place>],
    /// Each place met, what it calls, in order.
    met: Vec<Op>,
    /// How many of them are an iterator's, whose instances turn.
    iterators: usize,
    /// Where the instances start turning, the case's own.
    shift: usize,
    budget: usize,
    depth_const: usize,
}

/// What `e` does with a value, if it's one `VALUES` calls in generic code:
/// `x.clone()`, `Some(x)`, `drop(x)` or `mem::drop(x)`.
fn value_of(e: &Expr) -> Option<Value> {
    match e {
        Expr::MethodCall(call) if call.method == "clone" && call.args.is_empty() && call.turbofish.is_none() => {
            Some(Value::Clone)
        }
        Expr::Call(call) if call.args.len() == 1 => {
            let Expr::Path(path) = &*call.func else { return None };
            if path.qself.is_some() {
                return None;
            }
            let names: Vec<String> = path.path.segments.iter().map(|s| s.ident.to_string()).collect();
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            match names.as_slice() {
                ["Some"] | ["Option", "Some"] => Some(Value::Some),
                ["drop"] | ["mem", "drop"] | ["std", "mem", "drop"] => Some(Value::Drop),
                _ => None,
            }
        }
        _ => None,
    }
}

impl Wrap<'_> {
    /// The next place, calling `op`: its number and its instance, or
    /// `None` where it's left as it was.
    fn place(&mut self, op: Op) -> Option<(usize, usize)> {
        let k = self.met.len();
        self.met.push(op);
        let turned = match op {
            Op::Method(method) => {
                self.iterators += 1;
                instance_of(method, self.iterators - 1, self.shift)
            }
            Op::Value(_) => 0,
        };
        if k >= self.budget {
            return None;
        }
        match self.places.get(k) {
            Some(None) => None,
            Some(Some(place)) => Some((k, place.instance)),
            None => Some((k, turned)),
        }
    }

    /// `e`, what `value` is done with, done in generic code.
    fn value(&mut self, e: &mut Expr, value: Value) {
        let Some((k, _)) = self.place(Op::Value(value)) else {
            return;
        };
        let module = format_ident!("{MODULE}");
        let at = format_ident!("at_{k}");
        *e = match (value, &*e) {
            (Value::Clone, Expr::MethodCall(call)) => {
                let receiver = &call.receiver;
                syn::parse_quote!(#module::#at(&(#receiver)))
            }
            (Value::Copy, Expr::Unary(unary)) => {
                let read = &unary.expr;
                syn::parse_quote!(#module::#at(&*(#read)))
            }
            (_, Expr::Call(call)) => {
                let given = &call.args[0];
                syn::parse_quote!(#module::#at(#given))
            }
            _ => unreachable!("a value's place is of its own shape"),
        };
    }

    /// `e`, given to a call, a macro or a `let`: of `*r`, copied in
    /// generic code.
    fn copied(&mut self, e: &mut Expr) {
        if self.depth_const == 0 && matches!(e, Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_))) {
            self.value(e, Value::Copy);
        }
    }
}

impl VisitMut for Wrap<'_> {
    fn visit_expr_mut(&mut self, e: &mut Expr) {
        visit_mut::visit_expr_mut(self, e);
        if self.depth_const > 0 {
            return;
        }
        match e {
            Expr::Call(call) => call.args.iter_mut().for_each(|arg| self.copied(arg)),
            Expr::MethodCall(call) => call.args.iter_mut().for_each(|arg| self.copied(arg)),
            _ => {}
        }
        if let Some(value) = value_of(e) {
            self.value(e, value);
            return;
        }
        let Expr::MethodCall(call) = e else { return };
        let Some(method) = METHODS
            .iter()
            .position(|m| m.name == call.method.to_string() && m.args == call.args.len())
        else {
            return;
        };
        // Of an iterator the case makes here, which is moved into the call:
        // a variable's might be used again.
        let Expr::MethodCall(made) = &*call.receiver else {
            return;
        };
        if !MAKERS.contains(&made.method.to_string().as_str()) {
            return;
        }
        let Some((k, instance)) = self.place(Op::Method(method)) else {
            return;
        };
        let receiver = call.receiver.to_token_stream().to_string();
        let given: Expr =
            syn::parse_str(&INSTANCES[instance].replace("{it}", &format!("({receiver})"))).expect("an instance");
        let module = format_ident!("{MODULE}");
        let at = format_ident!("at_{k}");
        let args = call.args.iter();
        // `collect::<Vec<_>>()`'s and `sum::<u32>()`'s type, the function's
        // second type parameter.
        let turbofish = call.turbofish.as_ref().and_then(|t| match t.args.first() {
            Some(GenericArgument::Type(ty)) => Some(ty.clone()),
            _ => None,
        });
        *e = match turbofish {
            Some(ty) => syn::parse_quote!(#module::#at::<_, #ty>(#given #(, #args)*)),
            None => syn::parse_quote!(#module::#at(#given #(, #args)*)),
        };
    }

    fn visit_local_mut(&mut self, local: &mut syn::Local) {
        visit_mut::visit_local_mut(self, local);
        if let Some(init) = &mut local.init {
            self.copied(&mut init.expr);
        }
    }

    // What std's macros of expressions are given, `println!("{:?}",
    // v.iter().size_hint())`, which is where a case says most of what it
    // asks.
    fn visit_macro_mut(&mut self, m: &mut syn::Macro) {
        const EXPRESSIONS: &[&str] = &[
            "print",
            "println",
            "eprint",
            "eprintln",
            "format",
            "write",
            "writeln",
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
            self.copied(arg);
        }
        m.tokens = args.to_token_stream();
    }

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
        // The module of the places is the crate root's (the native harness
        // includes the case in a module).
        if let Some((_, items)) = &mut m.content {
            let module = format_ident!("{MODULE}");
            items.insert(0, syn::parse_quote!(#[allow(unused_imports)] use super::#module;));
        }
    }
}

/// `ast` with each place `places` keeps called in generic code, and the
/// module of their functions; and each place's method, in order.
fn made(ast: &syn::File, places: &[Option<Place>], shift: usize, budget: usize) -> (syn::File, Vec<Op>) {
    let mut ast = ast.clone();
    let mut wrap = Wrap {
        places,
        met: Vec::new(),
        iterators: 0,
        shift,
        budget,
        depth_const: 0,
    };
    wrap.visit_file_mut(&mut ast);
    let fns: Vec<_> = (wrap.met.iter().enumerate())
        .filter(|&(k, _)| k < budget)
        .filter_map(|(k, &op)| {
            let place = match places.get(k) {
                Some(place) => place.clone()?,
                None => Place {
                    op,
                    instance: 0,
                    promised: vec![true; PROMISES.len()],
                },
            };
            let at = format_ident!("at_{k}");
            let method = match place.op {
                Op::Method(method) => method,
                Op::Value(value) => {
                    let (_, item) = VALUES.iter().find(|(v, _)| *v == value).expect("a value's function");
                    let item: syn::Item = syn::parse_str(&item.replace("{at}", &at.to_string())).expect("a function");
                    return Some(item.to_token_stream());
                }
            };
            let m = &METHODS[method];
            let bound: proc_macro2::TokenStream = m.bound.parse().expect("a bound");
            let params: proc_macro2::TokenStream = m.params.parse().expect("parameters");
            let body: Expr = syn::parse_str(m.body).expect("a body");
            let args = arguments(m.args);
            Some(if m.adapter {
                let promises: Vec<syn::Path> = (PROMISES.iter().zip(&place.promised))
                    .filter(|((name, _), kept)| **kept && !m.bound.contains(name))
                    .map(|((_, path), _)| syn::parse_str(path).expect("a trait's path"))
                    .collect();
                quote!(
                    pub fn #at<I: ::std::iter::Iterator #bound #(+ #promises)*>(it: I #(, #args)*) -> impl ::std::iter::Iterator<Item = I::Item> #bound #(+ #promises)* {
                        #body
                    }
                )
            } else {
                let output: syn::Type = syn::parse_str(m.output).expect("a type");
                quote!(
                    pub fn #at<I: ::std::iter::Iterator #bound #params>(it: I #(, #args)*) -> #output {
                        #body
                    }
                )
            })
        })
        .collect();
    let module = format_ident!("{MODULE}");
    ast.items.push(syn::parse_quote!(
        #[allow(dead_code)]
        mod #module {
            /// The net's own iterator: the one it holds, stepped and hinted
            /// as that one is, from either end.
            pub struct Own<I>(pub I);

            impl<I: ::std::iter::Iterator> ::std::iter::Iterator for Own<I> {
                type Item = I::Item;
                fn next(&mut self) -> ::std::option::Option<I::Item> {
                    self.0.next()
                }
                fn size_hint(&self) -> (usize, ::std::option::Option<usize>) {
                    self.0.size_hint()
                }
            }

            impl<I: ::std::iter::DoubleEndedIterator> ::std::iter::DoubleEndedIterator for Own<I> {
                fn next_back(&mut self) -> ::std::option::Option<I::Item> {
                    self.0.next_back()
                }
            }

            impl<I: ::std::iter::ExactSizeIterator> ::std::iter::ExactSizeIterator for Own<I> {}

            #(#fns)*
        }
    ));
    (ast, wrap.met)
}

pub fn generic_instances(ast: &mut syn::File, source: &str, budget: &mut usize) -> bool {
    let shift = source.len() % INSTANCES.len();
    let (_, met) = made(ast, &[], shift, *budget);
    if met.is_empty() {
        return false;
    }
    // An iterator's instance turns with the iterators' places alone; a
    // value's is its own.
    let mut iterators = 0;
    let mut places: Vec<Option<Place>> = (met.iter())
        .map(|&op| {
            let instance = match op {
                Op::Method(method) => {
                    iterators += 1;
                    instance_of(method, iterators - 1, shift)
                }
                Op::Value(_) => 0,
            };
            Some(Place {
                op,
                instance,
                promised: vec![true; PROMISES.len()],
            })
        })
        .collect();
    for _ in 0..24 {
        let (program, _) = made(ast, &places, shift, *budget);
        let text = program.to_token_stream().to_string();
        let Some(errors) = narrow::diagnostics(&text) else {
            *ast = program;
            return places.iter().take(*budget).any(Option::is_some);
        };
        let sites = narrow::sites(&text, MODULE);
        let mut changed = false;
        // A place stepped down once a round, however many errors name it.
        let mut stepped = Vec::new();
        for error in &errors {
            for k in narrow::places_of(&text, &sites, error) {
                let Some(Some(place)) = places.get_mut(k) else { continue };
                if stepped.contains(&k) {
                    continue;
                }
                let named: Vec<usize> = (PROMISES.iter().enumerate())
                    .filter(|(_, (name, _))| error.message.contains(name))
                    .map(|(i, _)| i)
                    .collect();
                // An adapter's promise its instance doesn't keep taken off;
                // else the case's own instance; else the place as it was.
                // One already off, `Skip<I>: DoubleEndedIterator` asking
                // `I: ExactSizeIterator`: every promise off.
                let adapter = matches!(place.op, Op::Method(m) if METHODS[m].adapter)
                    && error.code == "E0277"
                    && !error.message.contains("impl Iterator");
                if adapter && named.iter().any(|&i| place.promised[i]) {
                    for i in named {
                        place.promised[i] = false;
                    }
                } else if adapter && !named.is_empty() && place.promised.iter().any(|&kept| kept) {
                    place.promised = vec![false; PROMISES.len()];
                } else if place.instance != 0 {
                    place.instance = 0;
                    place.promised = vec![true; PROMISES.len()];
                    stepped.push(k);
                } else {
                    places[k] = None;
                    stepped.push(k);
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
