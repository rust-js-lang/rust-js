//! `observe-borrows` and `observe-lens` (#2481): what a case's run never
//! prints of its `RefCell`s, said after each statement. A borrow flag the
//! JS keeps otherwise than native, a guard never released or a borrow never
//! counted, is invisible to the corpus unless the program happens to ask
//! it, and programs written to run rarely do; a length behind a guard kept
//! stale, the same unless it's printed.
//!
//! After each statement of each function, each local in scope is given to
//! a probe, `metamorph_observe::at_k(&x)`, which walks what it holds, its
//! fields, boxes, `Rc`s, vectors and options, to each `RefCell`, and prints
//! one line, `~observe <k> x.cells[0]=shared x.cells[1]=free`: of each cell,
//! `free` where `try_borrow_mut` succeeds, `shared` where only `try_borrow`
//! does, `unique` where neither (`observe-borrows`); or `x.*.len()=3`, the
//! length of each `Vec`, `String` and `VecDeque` behind a guard
//! (`observe-lens`). Each probe's guards drop at once, so it changes no
//! flag it reads, and it's valid Rust wherever a shared borrow of the local
//! is: one rustc rejects, a local moved, a unique borrow of it live, a type
//! no impl walks, is taken off by its errors (`narrow.rs`). The case's own
//! structs and enums are walked field by field, each field a place of its
//! own, taken off where its type isn't walked.
//!
//! A closure's body probes only what it binds itself, never what it would
//! capture, so no capture is added; an `async` block none. A case of
//! threads is none: its probes' lines would interleave.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeSet;
use syn::visit_mut::{self, VisitMut};
use syn::{Block, Expr, Fields, Item, Pat, Stmt};

use crate::narrow;

const MODULE: &str = "metamorph_observe";

/// How many probes print: past it they return at once, so a loop of a
/// million steps prints no more, and runs at its speed.
const PRINTED: usize = 2000;

#[derive(Clone, Copy, PartialEq)]
pub enum Observed {
    /// Each cell's flag.
    Borrows,
    /// Each length behind a guard.
    Lens,
}

/// What a place is: a local probed after a statement, by its name, or a
/// field of one of the case's types, walked.
enum Place {
    Local(String),
    Field,
}

struct Probes<'o> {
    /// The places left out, by rustc's errors.
    off: &'o BTreeSet<usize>,
    budget: usize,
    /// Each place found, in order, made or left out.
    places: Vec<Place>,
    /// The names bound in scope, innermost last.
    scope: Vec<String>,
}

impl Probes<'_> {
    /// The next place's function, `at_k`, or `None` where it's left out.
    fn place(&mut self, place: Place) -> Option<syn::Ident> {
        let k = self.places.len();
        self.places.push(place);
        (k < self.budget && !self.off.contains(&k)).then(|| format_ident!("at_{k}"))
    }

    /// The probes of each name in scope, the innermost of each name.
    fn probes(&mut self) -> Vec<Stmt> {
        let mut seen = BTreeSet::new();
        let names: Vec<String> = self
            .scope
            .iter()
            .rev()
            .filter(|n| seen.insert((*n).clone()))
            .cloned()
            .collect();
        let module = format_ident!("{MODULE}");
        let mut made = Vec::new();
        for name in names.into_iter().rev() {
            if let Some(at) = self.place(Place::Local(name.clone())) {
                let local: TokenStream = name.parse().expect("a name");
                made.push(syn::parse_quote!(#module::#at(&#local);));
            }
        }
        made
    }

    /// A function's body, its parameters in scope and nothing else.
    fn function(&mut self, inputs: &syn::punctuated::Punctuated<syn::FnArg, syn::Token![,]>, block: &mut Block) {
        let outer = std::mem::take(&mut self.scope);
        for input in inputs {
            match input {
                syn::FnArg::Receiver(_) => self.scope.push("self".to_string()),
                syn::FnArg::Typed(arg) => bound(&arg.pat, &mut self.scope),
            }
        }
        self.visit_block_mut(block);
        self.scope = outer;
    }
}

/// Whether a function's body is probed: not a `const fn`'s, which can't
/// print, nor a binding's, whose body is native's stub.
fn probed(sig: &syn::Signature, attrs: &[syn::Attribute]) -> bool {
    sig.constness.is_none() && sig.asyncness.is_none() && !bound_to_js(attrs)
}

/// Whether a type is one rust-js gives JS its own way, a
/// `#[rust_js::class]`'s, which implements no trait of the case's but a
/// marker's: it's walked by none.
fn bound_to_js(attrs: &[syn::Attribute]) -> bool {
    use quote::ToTokens;
    attrs
        .iter()
        .any(|attr| attr.to_token_stream().to_string().contains("rust_js"))
}

/// Each name `pat` binds, in order: never a path's, a constant or a unit
/// variant, `None`, which a pattern writes as a name.
fn bound(pat: &Pat, names: &mut Vec<String>) {
    match pat {
        Pat::Ident(p) => {
            let name = p.ident.to_string();
            if !name.trim_start_matches("r#").starts_with(|c: char| c.is_uppercase()) {
                names.push(name);
            }
            if let Some((_, sub)) = &p.subpat {
                bound(sub, names);
            }
        }
        Pat::Tuple(t) => t.elems.iter().for_each(|p| bound(p, names)),
        Pat::TupleStruct(t) => t.elems.iter().for_each(|p| bound(p, names)),
        Pat::Struct(s) => s.fields.iter().for_each(|f| bound(&f.pat, names)),
        Pat::Slice(s) => s.elems.iter().for_each(|p| bound(p, names)),
        Pat::Reference(r) => bound(&r.pat, names),
        Pat::Type(t) => bound(&t.pat, names),
        Pat::Paren(p) => bound(&p.pat, names),
        // Each case of an `|` binds the same names.
        Pat::Or(o) => {
            if let Some(first) = o.cases.first() {
                bound(first, names);
            }
        }
        _ => {}
    }
}

/// The names the `let`s of a condition bind, `if let Some(x) = a && let
/// Ok(y) = b`, for its branch.
fn bound_by_condition(cond: &Expr, names: &mut Vec<String>) {
    match cond {
        Expr::Let(l) => bound(&l.pat, names),
        Expr::Binary(b) if matches!(b.op, syn::BinOp::And(_)) => {
            bound_by_condition(&b.left, names);
            bound_by_condition(&b.right, names);
        }
        Expr::Paren(p) => bound_by_condition(&p.expr, names),
        _ => {}
    }
}

impl VisitMut for Probes<'_> {
    fn visit_block_mut(&mut self, block: &mut Block) {
        let depth = self.scope.len();
        let stmts = std::mem::take(&mut block.stmts);
        let last = stmts.len();
        for (i, mut stmt) in stmts.into_iter().enumerate() {
            self.visit_stmt_mut(&mut stmt);
            let tail = i + 1 == last && matches!(stmt, Stmt::Expr(_, None));
            let item = matches!(stmt, Stmt::Item(_));
            if let Stmt::Local(local) = &stmt {
                bound(&local.pat, &mut self.scope);
            }
            block.stmts.push(stmt);
            if !tail && !item {
                let probes = self.probes();
                block.stmts.extend(probes);
            }
        }
        self.scope.truncate(depth);
    }

    fn visit_expr_for_loop_mut(&mut self, e: &mut syn::ExprForLoop) {
        self.visit_expr_mut(&mut e.expr);
        let depth = self.scope.len();
        bound(&e.pat, &mut self.scope);
        self.visit_block_mut(&mut e.body);
        self.scope.truncate(depth);
    }

    fn visit_expr_if_mut(&mut self, e: &mut syn::ExprIf) {
        self.visit_expr_mut(&mut e.cond);
        let depth = self.scope.len();
        bound_by_condition(&e.cond, &mut self.scope);
        self.visit_block_mut(&mut e.then_branch);
        self.scope.truncate(depth);
        if let Some((_, otherwise)) = &mut e.else_branch {
            self.visit_expr_mut(otherwise);
        }
    }

    fn visit_expr_while_mut(&mut self, e: &mut syn::ExprWhile) {
        self.visit_expr_mut(&mut e.cond);
        let depth = self.scope.len();
        bound_by_condition(&e.cond, &mut self.scope);
        self.visit_block_mut(&mut e.body);
        self.scope.truncate(depth);
    }

    fn visit_arm_mut(&mut self, arm: &mut syn::Arm) {
        let depth = self.scope.len();
        bound(&arm.pat, &mut self.scope);
        if let Some((_, guard)) = &mut arm.guard {
            self.visit_expr_mut(guard);
        }
        self.visit_expr_mut(&mut arm.body);
        self.scope.truncate(depth);
    }

    // A closure probes what it binds, never what it would capture.
    fn visit_expr_closure_mut(&mut self, e: &mut syn::ExprClosure) {
        if e.asyncness.is_some() {
            return;
        }
        let outer = std::mem::take(&mut self.scope);
        e.inputs.iter().for_each(|p| bound(p, &mut self.scope));
        self.visit_expr_mut(&mut e.body);
        self.scope = outer;
    }

    fn visit_expr_async_mut(&mut self, _: &mut syn::ExprAsync) {}
    fn visit_expr_const_mut(&mut self, _: &mut syn::ExprConst) {}

    fn visit_item_fn_mut(&mut self, f: &mut syn::ItemFn) {
        if probed(&f.sig, &f.attrs) {
            self.function(&f.sig.inputs, &mut f.block);
        }
    }

    fn visit_impl_item_fn_mut(&mut self, f: &mut syn::ImplItemFn) {
        if probed(&f.sig, &f.attrs) {
            self.function(&f.sig.inputs, &mut f.block);
        }
    }

    fn visit_trait_item_fn_mut(&mut self, f: &mut syn::TraitItemFn) {
        if let Some(block) = &mut f.default
            && probed(&f.sig, &f.attrs)
        {
            self.function(&f.sig.inputs, block);
        }
    }

    // A constant's or a static's initializer is no function's body.
    fn visit_item_const_mut(&mut self, _: &mut syn::ItemConst) {}
    fn visit_item_static_mut(&mut self, _: &mut syn::ItemStatic) {}

    fn visit_item_mod_mut(&mut self, m: &mut syn::ItemMod) {
        visit_mut::visit_item_mod_mut(self, m);
        // What probes is the crate root's, which a module asks its parent
        // for, as the native harness includes the case in a module.
        if let Some((_, items)) = &mut m.content {
            let module = format_ident!("{MODULE}");
            items.insert(0, syn::parse_quote!(#[allow(unused_imports)] use super::#module;));
        }
    }
}

/// The walk of each struct and enum of `items`, the case's own, each field
/// a place: `impl Observe for T`, beside it.
fn walks(items: &mut Vec<Item>, probes: &mut Probes) {
    let module = format_ident!("{MODULE}");
    let mut added = Vec::new();
    for item in items.iter_mut() {
        match item {
            Item::Mod(m) => {
                if let Some((_, inner)) = &mut m.content {
                    walks(inner, probes);
                }
            }
            Item::Struct(s) if !bound_to_js(&s.attrs) => {
                let calls = fields(&s.fields, probes, |member, _| quote!(&self.#member), "");
                let (ig, tg, wc) = s.generics.split_for_impl();
                let name = &s.ident;
                added.push(syn::parse_quote!(
                    impl #ig #module::Observe for #name #tg #wc {
                        fn observe(&self, path: &str, seen: &mut #module::Seen) {
                            let _ = (&path, &seen);
                            #(#calls)*
                        }
                    }
                ));
            }
            Item::Enum(e) if !bound_to_js(&e.attrs) => {
                let name = &e.ident;
                let mut arms = Vec::new();
                for variant in &e.variants {
                    let v = &variant.ident;
                    let label = v.to_string();
                    let binders: Vec<syn::Ident> = (0..variant.fields.len())
                        .map(|i| format_ident!("metamorph_f{i}"))
                        .collect();
                    let calls = fields(
                        &variant.fields,
                        probes,
                        |_, i| {
                            let b = &binders[i];
                            quote!(#b)
                        },
                        &label,
                    );
                    let pattern = match &variant.fields {
                        Fields::Unit => quote!(#name::#v),
                        Fields::Unnamed(_) => quote!(#name::#v(#(#binders),*)),
                        Fields::Named(named) => {
                            let names = named.named.iter().map(|f| f.ident.as_ref().expect("a named field"));
                            quote!(#name::#v { #(#names: #binders),* })
                        }
                    };
                    arms.push(quote!(#pattern => { #(#calls)* }));
                }
                let (ig, tg, wc) = e.generics.split_for_impl();
                added.push(syn::parse_quote!(
                    impl #ig #module::Observe for #name #tg #wc {
                        #[allow(unreachable_patterns, unused_variables)]
                        fn observe(&self, path: &str, seen: &mut #module::Seen) {
                            let _ = (&path, &seen);
                            match self {
                                #(#arms)*
                                _ => {}
                            }
                        }
                    }
                ));
            }
            _ => {}
        }
    }
    items.extend(added);
}

/// Each field of `fields` walked, a place each, `at_k(<value>, path, seen)`,
/// its path `.<variant>.<field>`.
fn fields(
    fields: &Fields,
    probes: &mut Probes,
    value: impl Fn(syn::Member, usize) -> TokenStream,
    variant: &str,
) -> Vec<TokenStream> {
    let module = format_ident!("{MODULE}");
    let mut calls = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        let member = match &field.ident {
            Some(ident) => syn::Member::Named(ident.clone()),
            None => syn::Member::Unnamed(syn::Index::from(i)),
        };
        let label = match &field.ident {
            Some(ident) => ident.to_string(),
            None => i.to_string(),
        };
        let segment = if variant.is_empty() {
            format!(".{label}")
        } else {
            format!(".{variant}.{label}")
        };
        if let Some(at) = probes.place(Place::Field) {
            let given = value(member, i);
            calls.push(quote!(#module::#at(#given, &::std::format!("{}{}", path, #segment), seen);));
        }
    }
    calls
}

/// What walks, and each place's function.
fn prelude(how: Observed, places: &[Place], off: &BTreeSet<usize>, budget: usize) -> Item {
    let lens = how == Observed::Lens;
    let fns = places
        .iter()
        .enumerate()
        .filter(|(k, _)| *k < budget && !off.contains(k))
        .map(|(k, place)| {
            let at = format_ident!("at_{k}");
            match place {
                Place::Local(name) => quote!(
                    pub fn #at<T: ?Sized + Observe>(x: &T) { point(#k, #name, x) }
                ),
                Place::Field => quote!(
                    pub fn #at<T: ?Sized + Observe>(x: &T, path: &str, seen: &mut Seen) { seen.walk(x, path, false) }
                ),
            }
        });
    let module = format_ident!("{MODULE}");
    syn::parse_quote!(
        #[allow(dead_code, unused_imports, unused_variables, clippy::all)]
        mod #module {
            use ::std::cell::{Cell, Ref, RefCell, RefMut};
            use ::std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
            use ::std::rc::Rc;
            use ::std::sync::Arc;
            use ::std::sync::atomic::{AtomicUsize, Ordering};

            const LENS: bool = #lens;
            static PRINTED: AtomicUsize = AtomicUsize::new(0);

            /// What a probe found, and how deep it is.
            pub struct Seen {
                lines: Vec<String>,
                depth: usize,
                guarded: bool,
            }

            impl Seen {
                /// `x` walked at `path`, behind a guard if `guarded` or
                /// already; no deeper than a cycle of `Rc`s lets it end.
                pub fn walk<T: ?Sized + Observe>(&mut self, x: &T, path: &str, guarded: bool) {
                    if self.depth >= 8 || self.lines.len() >= 64 {
                        return;
                    }
                    let was = self.guarded;
                    self.depth += 1;
                    self.guarded |= guarded;
                    x.observe(path, self);
                    self.guarded = was;
                    self.depth -= 1;
                }
            }

            pub trait Observe {
                fn observe(&self, path: &str, seen: &mut Seen);
            }

            /// A probe of the local `name` at the place `k`: a line of what
            /// it holds, where it holds anything.
            fn point<T: ?Sized + Observe>(k: usize, name: &str, x: &T) {
                if PRINTED.load(Ordering::Relaxed) >= #PRINTED {
                    return;
                }
                let mut seen = Seen { lines: Vec::new(), depth: 0, guarded: false };
                seen.walk(x, name, false);
                if !seen.lines.is_empty() {
                    PRINTED.fetch_add(1, Ordering::Relaxed);
                    println!("~observe {} {}", k, seen.lines.join(" "));
                }
            }

            impl<T: Observe> Observe for RefCell<T> {
                fn observe(&self, path: &str, seen: &mut Seen) {
                    if !LENS {
                        let state = match (self.try_borrow_mut().is_ok(), self.try_borrow().is_ok()) {
                            (true, _) => "free",
                            (false, true) => "shared",
                            (false, false) => "unique",
                        };
                        seen.lines.push(format!("{path}={state}"));
                    }
                    if let Ok(inner) = self.try_borrow() {
                        seen.walk(&*inner, &format!("{path}.*"), true);
                    }
                }
            }

            /// A length, behind a guard.
            fn len(seen: &mut Seen, path: &str, n: usize) {
                if LENS && seen.guarded {
                    seen.lines.push(format!("{path}.len()={n}"));
                }
            }

            /// The first of a sequence's items walked, `path[i]`.
            fn items<'a, T: Observe + 'a>(seen: &mut Seen, path: &str, items: impl Iterator<Item = &'a T>) {
                for (i, item) in items.take(16).enumerate() {
                    seen.walk(item, &format!("{path}[{i}]"), false);
                }
            }

            impl<T: Observe> Observe for Vec<T> {
                fn observe(&self, path: &str, seen: &mut Seen) {
                    len(seen, path, self.len());
                    items(seen, path, self.iter());
                }
            }
            impl<T: Observe> Observe for VecDeque<T> {
                fn observe(&self, path: &str, seen: &mut Seen) {
                    len(seen, path, self.len());
                    items(seen, path, self.iter());
                }
            }
            impl Observe for String {
                fn observe(&self, path: &str, seen: &mut Seen) {
                    len(seen, path, self.len());
                }
            }
            impl<T: Observe> Observe for [T] {
                fn observe(&self, path: &str, seen: &mut Seen) {
                    items(seen, path, self.iter());
                }
            }
            impl<T: Observe, const N: usize> Observe for [T; N] {
                fn observe(&self, path: &str, seen: &mut Seen) {
                    items(seen, path, self.iter());
                }
            }
            impl<K: Ord, V: Observe> Observe for BTreeMap<K, V> {
                fn observe(&self, path: &str, seen: &mut Seen) {
                    items(seen, path, self.values());
                }
            }
            impl<T: Observe> Observe for Option<T> {
                fn observe(&self, path: &str, seen: &mut Seen) {
                    if let Some(x) = self {
                        seen.walk(x, path, false);
                    }
                }
            }
            impl<T: Observe, E: Observe> Observe for Result<T, E> {
                fn observe(&self, path: &str, seen: &mut Seen) {
                    match self {
                        Ok(x) => seen.walk(x, path, false),
                        Err(e) => seen.walk(e, path, false),
                    }
                }
            }
            impl<'a, T: ?Sized + Observe> Observe for Ref<'a, T> {
                fn observe(&self, path: &str, seen: &mut Seen) {
                    seen.walk(&**self, &format!("{path}.*"), true);
                }
            }
            impl<'a, T: ?Sized + Observe> Observe for RefMut<'a, T> {
                fn observe(&self, path: &str, seen: &mut Seen) {
                    seen.walk(&**self, &format!("{path}.*"), true);
                }
            }

            /// What holds one value, walked as it.
            macro_rules! through {
                ($($t:ident)*) => {$(
                    impl<T: ?Sized + Observe> Observe for $t<T> {
                        fn observe(&self, path: &str, seen: &mut Seen) {
                            seen.walk(&**self, path, false);
                        }
                    }
                )*};
            }
            through!(Box Rc Arc);
            impl<'a, T: ?Sized + Observe> Observe for &'a T {
                fn observe(&self, path: &str, seen: &mut Seen) {
                    seen.walk(&**self, path, false);
                }
            }
            impl<'a, T: ?Sized + Observe> Observe for &'a mut T {
                fn observe(&self, path: &str, seen: &mut Seen) {
                    seen.walk(&**self, path, false);
                }
            }

            macro_rules! tuples {
                ($(($($t:ident $i:tt),*))*) => {$(
                    impl<$($t: Observe),*> Observe for ($($t,)*) {
                        fn observe(&self, path: &str, seen: &mut Seen) {
                            $(seen.walk(&self.$i, &format!("{path}.{}", $i), false);)*
                        }
                    }
                )*};
            }
            tuples!((A 0) (A 0, B 1) (A 0, B 1, C 2) (A 0, B 1, C 2, D 3));

            /// What holds no cell a probe can reach: a hash table's, whose
            /// order is its seed's, a `Cell`'s, which can't be read, a weak
            /// count's.
            macro_rules! leaves {
                ($($t:ty),*) => {$(
                    impl Observe for $t {
                        fn observe(&self, _: &str, _: &mut Seen) {}
                    }
                )*};
            }
            leaves!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64, bool, char, str, ());
            impl<T: ?Sized> Observe for Cell<T> {
                fn observe(&self, _: &str, _: &mut Seen) {}
            }
            impl<T: ?Sized> Observe for ::std::rc::Weak<T> {
                fn observe(&self, _: &str, _: &mut Seen) {}
            }
            impl<T: ?Sized> Observe for ::std::sync::Weak<T> {
                fn observe(&self, _: &str, _: &mut Seen) {}
            }
            impl<K, V, S> Observe for HashMap<K, V, S> {
                fn observe(&self, _: &str, _: &mut Seen) {}
            }
            impl<T, S> Observe for HashSet<T, S> {
                fn observe(&self, _: &str, _: &mut Seen) {}
            }
            impl<T: Ord> Observe for BTreeSet<T> {
                fn observe(&self, _: &str, _: &mut Seen) {}
            }
            impl<T: Ord> Observe for BinaryHeap<T> {
                fn observe(&self, _: &str, _: &mut Seen) {}
            }

            #(#fns)*
        }
    )
}

fn made(ast: &syn::File, how: Observed, off: &BTreeSet<usize>, budget: usize) -> (syn::File, usize) {
    let mut ast = ast.clone();
    let mut probes = Probes {
        off,
        budget,
        places: Vec::new(),
        scope: Vec::new(),
    };
    probes.visit_file_mut(&mut ast);
    walks(&mut ast.items, &mut probes);
    // The probes of locals made: a walk of a field alone says nothing.
    let count = (probes.places.iter().enumerate())
        .filter(|(k, place)| *k < budget && !off.contains(k) && matches!(place, Place::Local(_)))
        .count();
    ast.items.push(prelude(how, &probes.places, off, budget));
    (ast, count)
}

pub fn observe(ast: &mut syn::File, source: &str, how: Observed, budget: &mut usize) -> bool {
    // Of a case of cells; and not one of threads, whose lines interleave.
    if !source.contains("RefCell") || source.contains("thread::") {
        return false;
    }
    let mut off = BTreeSet::new();
    for _ in 0..32 {
        let (program, count) = made(ast, how, &off, *budget);
        if count == 0 {
            return false;
        }
        use quote::ToTokens;
        let text = program.to_token_stream().to_string();
        let Some(errors) = narrow::diagnostics(&text) else {
            *ast = program;
            return true;
        };
        let sites = narrow::sites(&text, MODULE);
        let before = off.len();
        for error in &errors {
            off.extend(narrow::holding(&sites, error));
        }
        // An error no probe holds, of a place made before it, as other
        // rewrites map one.
        if off.len() == before {
            for error in &errors {
                off.extend(narrow::places_of(&text, &sites, error));
            }
        }
        if off.len() == before {
            return false;
        }
    }
    false
}
