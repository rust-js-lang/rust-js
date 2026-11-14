//! A type inside itself, `Hir` in `HirKind::Capture`'s `Box<Hir>`: its
//! `==`, clone and order are each a function, which calls itself for the
//! ones inside (ADRs 0052, 0053, #511). Expanded in place, each never
//! ended. Of a type its module knows, the module's own function, one for
//! each type and module, which every place that compares, clones or orders
//! one calls (#517); of one only a caller knows, made of a type parameter,
//! the function's, which reads the dictionaries it was given. And a type's
//! dictionary of std's order its module makes, `pointPartialOrd`, made once,
//! a `const` of its module, as the crate's own impls' are (#944).

use super::{Dependencies, FnCx, R};
use crate::collections::{IndexMap, IndexSet};
use crate::js::{self, Expr, Stmt, StmtKind};
use crate::lower::temporary_names::Base;
use rustc_middle::ty::{Ty, TypeVisitableExt};
use rustc_span::def_id::{DefId, LocalModId};

/// What the function is of its type.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Kind {
    Eq,
    Clone,
    /// Its order, partial or not.
    Cmp {
        partial: bool,
    },
    /// Its dictionary of the trait, made by rust-js, a `const`.
    Dictionary(DefId),
    /// A generic function's instance as a value, given its dictionaries,
    /// a `const`: one function, as Rust's pointer to it is one (ADR 0395,
    /// #974).
    FnValue,
    /// A generic function's instance as a `fn` pointer that takes its
    /// type's forms, adapting them to the instance's, a `const` (#1187).
    FnPointer,
}

impl Kind {
    /// Whether it's a function's value, its instance's or its pointer's.
    fn is_fn_value(self) -> bool {
        matches!(self, Kind::FnValue | Kind::FnPointer)
    }
}

/// The crate's modules' functions, made as their bodies are lowered: each
/// by its module, what it is and its type. What the bodies lowered before
/// made, `Settled`'s, which the pipeline settles each body's into
/// (`settle`), never a body itself (#1671 change 4, M2).
#[derive(Default)]
pub(super) struct ModuleFns<'tcx> {
    made: IndexMap<Key<'tcx>, Made<'tcx>>,
}

/// What a body made and called of its module's functions, beside those
/// settled before it: its own, given back, which the pipeline settles
/// (`ModuleFns::settle`, ADR 1148).
#[derive(Default)]
pub(super) struct MadeHere<'tcx> {
    made: IndexMap<Key<'tcx>, Made<'tcx>>,
    /// Those whose bodies are being lowered, the innermost last.
    building: Vec<Key<'tcx>>,
    /// Each call of one, by the item, in the one being made then, if any,
    /// in order: what `ModuleFns::called` makes of each as it's settled.
    calls: Vec<(Option<Key<'tcx>>, Key<'tcx>, DefId)>,
}

type Key<'tcx> = (LocalModId, Kind, Ty<'tcx>);

/// A module's function: its name, given before its body is lowered, so the
/// ones inside call it; then its function and what it uses, the others of
/// these among it; and the items that call it, whose module writes it
/// where it writes the first of them.
struct Made<'tcx> {
    name: String,
    /// How many were made before it: those written together, in the order
    /// they were made.
    made_before: usize,
    lowered: Option<(js::Function, Dependencies)>,
    calls: IndexSet<Key<'tcx>>,
    used_by: IndexSet<DefId>,
    written: bool,
    /// A `const` of the value its function returns, not a function.
    constant: bool,
}

/// What `written_before` gives an item to write before it: its module's
/// functions, and its dictionaries' `const`s, which no load runs.
#[derive(Default)]
pub(super) struct WrittenBefore {
    pub(super) functions: Vec<js::Function>,
    pub(super) consts: Vec<js::Const>,
}

impl WrittenBefore {
    /// Its `const`s' names, each a dictionary's.
    pub(super) fn const_names(&self) -> impl Iterator<Item = String> + '_ {
        self.consts.iter().map(|c| c.name.clone())
    }
}

impl<'tcx> ModuleFns<'tcx> {
    /// What a body made and called, `here`, settled: each it made, in the
    /// order it made them, then each call, in the order it made them.
    pub(super) fn settle(&mut self, here: MadeHere<'tcx>) {
        debug_assert!(
            here.building.is_empty(),
            "a module's function is settled while it's made"
        );
        self.made.extend(here.made);
        for (outer, key, item) in here.calls {
            self.called(outer, key, item);
        }
    }

    /// `item` calls the function of `key`, in `outer`, the one being
    /// lowered then, if any: and so the ones it calls, and `outer` calls it.
    fn called(&mut self, outer: Option<Key<'tcx>>, key: Key<'tcx>, item: DefId) {
        if let Some(outer) = outer {
            self.made.get_mut(&outer).expect("being made").calls.insert(key);
        }
        let mut todo = vec![key];
        while let Some(key) = todo.pop() {
            let made = self.made.get_mut(&key).expect("made");
            if made.used_by.insert(item) {
                todo.extend(made.calls.iter().copied());
            }
        }
    }

    /// Each made function's lowered function, which the pipeline settles
    /// as it does the items' bodies: its drop arguments (ADR 0300), an
    /// instance's as a value's (#974).
    pub(super) fn functions_mut(&mut self) -> impl Iterator<Item = &mut js::Function> {
        (self.made.values_mut()).filter_map(|made| made.lowered.as_mut().map(|(function, _)| function))
    }

    /// The instances' values, once their drop arguments are settled: each
    /// that's the function itself, `keep`, read as it in `items`, the
    /// crate's lowered functions, and in the module's own; each nothing
    /// reads, never written (#974).
    pub(super) fn settle_values<'f>(&mut self, items: impl Iterator<Item = &'f mut js::Function>) {
        let mut items: Vec<&'f mut js::Function> = items.collect();
        let aliases = self.take_aliases();
        if !aliases.is_empty() {
            let alias = |e: &mut Expr| {
                if let js::ExprKind::Var(name) = &e.kind
                    && let Some(target) = aliases.get(name.as_str())
                {
                    *e = target.clone();
                }
            };
            for function in items.iter_mut() {
                js::each_expr_mut(&mut function.body, &mut |e| alias(e));
            }
            for function in self.functions_mut() {
                js::each_expr_mut(&mut function.body, &mut |e| alias(e));
            }
        }
        let mut read: IndexSet<String> = IndexSet::default();
        items.iter().for_each(|function| {
            function.visit_vars(&mut |name: &str| {
                read.insert(name.to_owned());
            })
        });
        self.drop_unread_values(&read);
    }

    /// Each instance's value that its settled drop arguments left the
    /// function itself, `const keepLoud = keep`: by its name, that
    /// function, which each use reads in its place, never written (#974).
    fn take_aliases(&mut self) -> IndexMap<String, Expr> {
        let mut aliases = IndexMap::default();
        for ((_, kind, _), made) in self.made.iter_mut() {
            if !kind.is_fn_value() {
                continue;
            }
            let Some((function, _)) = &made.lowered else { continue };
            if let [
                js::Stmt {
                    kind: StmtKind::Return(Some(value)),
                    ..
                },
            ] = function.body.as_slice()
                && matches!(value.kind, js::ExprKind::Var(_))
            {
                aliases.insert(made.name.clone(), value.clone());
                made.lowered = None;
            }
        }
        aliases
    }

    /// Each instance's value nothing reads, of the names the items read,
    /// `by_items`, or a module's function still written: made where a body
    /// took the function as a value, which its tidy then left unread, `let
    /// twice = count::<4>` of a direct call. Never written (#974), nor what
    /// only such a value read, the value of a `fn` pointer's instance that
    /// adapts it (#1324).
    fn drop_unread_values(&mut self, by_items: &IndexSet<String>) {
        loop {
            let mut read = by_items.clone();
            self.functions_mut().for_each(|function| {
                function.visit_vars(&mut |name: &str| {
                    read.insert(name.to_owned());
                })
            });
            let mut dropped = false;
            for ((_, kind, _), made) in self.made.iter_mut() {
                if kind.is_fn_value() && made.lowered.is_some() && !read.contains(&made.name) {
                    made.lowered = None;
                    dropped = true;
                }
            }
            if !dropped {
                return;
            }
        }
    }

    /// The names `module`'s functions have, which no other name of its may
    /// be.
    pub(super) fn names(&self, module: LocalModId) -> impl Iterator<Item = &str> {
        names(&self.made, module)
    }

    /// The functions `item` calls not yet written, each then written: its
    /// module's, before `item`.
    pub(super) fn written_before(&mut self, item: DefId) -> WrittenBefore {
        let mut first: Vec<_> = (self.made.values_mut())
            .filter(|made| !made.written && made.used_by.contains(&item))
            .filter_map(|made| {
                made.written = true;
                made.lowered
                    .as_ref()
                    .map(|(function, _)| (made.made_before, made.constant, function.clone()))
            })
            .collect();
        first.sort_by_key(|(made_before, ..)| *made_before);
        let mut written = WrittenBefore::default();
        for (_, constant, function) in first {
            match (constant, function.body.as_slice()) {
                (
                    true,
                    [
                        js::Stmt {
                            kind: StmtKind::Return(Some(value)),
                            ..
                        },
                    ],
                ) => written.consts.push(js::Const {
                    name: function.name,
                    value: value.clone(),
                    mutable: false,
                    fields: js::Fields::default(),
                    export: false,
                    span: js::Span::NONE,
                }),
                _ => written.functions.push(function),
            }
        }
        written
    }
}

/// The names `module`'s functions of `made` have.
fn names<'m>(made: &'m IndexMap<Key<'_>, Made<'_>>, module: LocalModId) -> impl Iterator<Item = &'m str> {
    (made.iter())
        .filter(move |((at, ..), _)| *at == module)
        .map(|(_, made)| made.name.as_str())
}

/// The functions a type only a caller knows has, being made: each by its
/// kind and type, its name, which one inside it calls.
#[derive(Default)]
pub(super) struct Making<'tcx>(Vec<(Kind, Ty<'tcx>, String)>);

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The names this body's module's functions have, those the bodies
    /// before it made and its own, which none of its names may be.
    pub(super) fn module_fn_names(&self) -> Vec<String> {
        let here = self.module_fns_made.borrow();
        (self.settled.module_fns.names(self.module))
            .chain(names(&here.made, self.module))
            .map(str::to_owned)
            .collect()
    }

    /// The function that's `ty`'s `kind`, recursive (`is_recursive`), named
    /// of `base` and the type, `eqHir`: made the first time it's asked of
    /// `params`, the names of its parameters, whose body `make` lowers,
    /// given them and its statements; then called. Its module's, where its
    /// type is the module's to know; else declared in `out`.
    pub(super) fn recursive_fn(
        &mut self,
        (kind, ty, base): (Kind, Ty<'tcx>, &str),
        params: &[Base<'_>],
        make: impl FnOnce(&mut Self, Vec<Expr>, &mut Vec<Stmt>) -> R<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<String> {
        if ty.has_param() || ty.has_aliases() {
            return self.local_recursive_fn((kind, ty, base), params, make, out);
        }
        self.module_fn((kind, ty, base), params, make)
    }

    /// `ty`'s dictionary of the trait `trait_id` that `make` makes, std's
    /// order of a type the module knows, a `const` of its module named
    /// `base`, `pointPartialOrd`, made the first time it's asked; then read
    /// (#944).
    pub(super) fn module_dictionary(
        &mut self,
        (trait_id, ty, base): (DefId, Ty<'tcx>, &str),
        make: impl FnOnce(&mut Self) -> R<Expr>,
    ) -> R<Expr> {
        let name = self.module_fn((Kind::Dictionary(trait_id), ty, base), &[], |cx, _, _| make(cx))?;
        Ok(Expr::var(&name))
    }

    /// A generic function's instance as a value, of its type `fn_ty`, the
    /// arrow `make` makes that gives it its dictionaries: a `const` of its
    /// module named `base`, `noopU32`, made the first time it's asked; then
    /// read, so each use is the one function (ADR 0395, #974).
    pub(super) fn module_fn_value(
        &mut self,
        (fn_ty, base): (Ty<'tcx>, &str),
        make: impl FnOnce(&mut Self) -> R<Expr>,
    ) -> R<Expr> {
        let name = self.module_fn((Kind::FnValue, fn_ty, base), &[], |cx, _, _| make(cx))?;
        Ok(Expr::var(&name).item())
    }

    /// A generic function's instance as a `fn` pointer, the instance's own
    /// type `fn_def_ty`, the arrow `make` makes: a `const` of its module
    /// named `base`, one for the instance, as `module_fn_value`'s is (#1187).
    pub(super) fn module_fn_pointer(
        &mut self,
        (fn_def_ty, base): (Ty<'tcx>, &str),
        make: impl FnOnce(&mut Self) -> R<Expr>,
    ) -> R<Expr> {
        let name = self.module_fn((Kind::FnPointer, fn_def_ty, base), &[], |cx, _, _| make(cx))?;
        Ok(Expr::var(&name).item())
    }

    /// `recursive_fn`'s of a type its module knows: its module's function,
    /// or of a dictionary its `const`.
    fn module_fn(
        &mut self,
        (kind, ty, base): (Kind, Ty<'tcx>, &str),
        params: &[Base<'_>],
        make: impl FnOnce(&mut Self, Vec<Expr>, &mut Vec<Stmt>) -> R<Expr>,
    ) -> R<String> {
        let key = (self.module, kind, self.tcx.erase_and_anonymize_regions(ty));
        let mut here = self.module_fns_made.borrow_mut();
        let outer = here.building.last().copied();
        if let Some(made) = (self.settled.module_fns.made.get(&key)).or_else(|| here.made.get(&key)) {
            let name = made.name.clone();
            // What it uses, this item's to import and reach too; none yet
            // of the one being made, which its own body calls.
            if let Some((_, uses)) = &made.lowered {
                merge(&mut self.dependencies.borrow_mut(), uses, self.item);
            }
            here.calls.push((outer, key, self.item));
            drop(here);
            self.names.borrow_mut().insert(name.clone());
            return Ok(name);
        }
        // Apart from the module's names, its other functions', and this
        // body's, which mustn't hide it.
        drop(here);
        let mut taken: IndexSet<String> = (self.module_names.iter().cloned())
            .chain(self.module_fn_names())
            .collect();
        taken.extend(self.names.borrow().iter().cloned());
        let name = super::fresh_in(&mut taken, base);
        let mut here = self.module_fns_made.borrow_mut();
        let made_before = self.settled.module_fns.made.len() + here.made.len();
        here.made.insert(
            key,
            Made {
                name: name.clone(),
                made_before,
                lowered: None,
                calls: IndexSet::default(),
                used_by: IndexSet::default(),
                written: false,
                constant: matches!(kind, Kind::Dictionary(_)) || kind.is_fn_value(),
            },
        );
        here.calls.push((outer, key, self.item));
        here.building.push(key);
        drop(here);
        self.names.borrow_mut().insert(name.clone());
        // Its body, lowered as a function of its own: its names apart from
        // this body's, what it uses its own, given no dictionary of this
        // item's.
        let mut names: IndexSet<String> = (self.module_names.iter().cloned())
            .chain(self.module_fn_names())
            .collect();
        let params: Vec<String> = params.iter().map(|p| super::fresh_in(&mut names, &p.word())).collect();
        let names = self.names.replace(names);
        let uses = self.dependencies.replace(Dependencies::default());
        let given = self.set_given_aside();
        let mut body = Vec::new();
        let value = make(self, params.iter().map(|p| Expr::var(p)).collect(), &mut body);
        self.restore_given(given);
        self.names.replace(names);
        let own = self.dependencies.replace(uses);
        self.module_fns_made.borrow_mut().building.pop();
        body.push(StmtKind::Return(Some(value?)).at(js::Span::NONE));
        merge(&mut self.dependencies.borrow_mut(), &own, self.item);
        let function = js::Function {
            name: name.clone(),
            params: params.into_iter().map(js::Pattern::Name).collect(),
            body,
            export: false,
            is_async: false,
            noinline: false,
            span: js::Span::NONE,
            name_span: js::Span::NONE,
            param_spans: Vec::new(),
            set_apart: false,
        };
        let mut here = self.module_fns_made.borrow_mut();
        let made = here.made.get_mut(&key).expect("made above");
        made.lowered = Some((function, own));
        Ok(name)
    }

    /// `recursive_fn`'s of a type only a caller knows: a function of this
    /// body, made where it's first asked, in `out`, which reads the
    /// dictionaries the body is given.
    fn local_recursive_fn(
        &mut self,
        (kind, ty, base): (Kind, Ty<'tcx>, &str),
        params: &[Base<'_>],
        make: impl FnOnce(&mut Self, Vec<Expr>, &mut Vec<Stmt>) -> R<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<String> {
        if let Some((.., name)) = self.recursive.0.iter().find(|(k, t, _)| *k == kind && *t == ty) {
            return Ok(name.clone());
        }
        let name = self.fresh(Base::Named(base));
        let params: Vec<String> = params.iter().map(|&p| self.fresh(p)).collect();
        self.recursive.0.push((kind, ty, name.clone()));
        let mut body = Vec::new();
        let value = make(self, params.iter().map(|p| Expr::var(p)).collect(), &mut body);
        self.recursive.0.pop();
        body.push(StmtKind::Return(Some(value?)).at(js::Span::NONE));
        let f = Expr::arrow(params.into_iter().map(Into::into).collect(), body);
        out.push(StmtKind::Const(name.clone(), f).at(js::Span::NONE));
        Ok(name)
    }
}

/// `into` uses what `uses` does, of the function `item` calls: its imports,
/// and the items it reaches, reached by `item`.
fn merge(into: &mut Dependencies, uses: &Dependencies, item: DefId) {
    into.references.extend(uses.references.iter().copied());
    into.copy_references.extend(uses.copy_references.iter().copied());
    into.through_copies.extend(uses.through_copies.iter().copied());
    into.package_uses.extend(uses.package_uses.iter().cloned());
    into.uses.extend(uses.uses.iter().map(|&(_, to)| (item, to)));
    into.held.extend(uses.held.iter().cloned());
    into.reexporters.extend(uses.reexporters.iter().copied());
}
