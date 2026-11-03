//! A type inside itself, `Hir` in `HirKind::Capture`'s `Box<Hir>`: its
//! `==`, clone and order are each a function, which calls itself for the
//! ones inside (ADRs 0052, 0053, #511). Expanded in place, each never
//! ended. Of a type its module knows, the module's own function, one for
//! each type and module, which every place that compares, clones or orders
//! one calls (#517); of one only a caller knows, made of a type parameter,
//! the function's, which reads the dictionaries it was given.

use super::{Dependencies, FnCx, R};
use crate::js::{self, Expr, Stmt, StmtKind};
use rustc_middle::ty::{Ty, TypeVisitableExt};
use rustc_span::def_id::{DefId, LocalModId};
use std::collections::{HashMap, HashSet};

/// What the function is of its type.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Kind {
    Eq,
    Clone,
    /// Its order, partial or not.
    Cmp {
        partial: bool,
    },
}

/// The crate's modules' functions, made as their bodies are lowered: each
/// by its module, what it is and its type.
#[derive(Default)]
pub(super) struct ModuleFns<'tcx> {
    made: HashMap<Key<'tcx>, Made<'tcx>>,
    /// Those whose bodies are being lowered, the innermost last.
    building: Vec<Key<'tcx>>,
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
    calls: HashSet<Key<'tcx>>,
    used_by: HashSet<DefId>,
    written: bool,
}

impl<'tcx> ModuleFns<'tcx> {
    /// `item` calls the function of `key`: and so the ones it calls, and
    /// the one being lowered, if any, calls it.
    fn called(&mut self, key: Key<'tcx>, item: DefId) {
        if let Some(&outer) = self.building.last() {
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

    /// The names `module`'s functions have, which no other name of its may
    /// be.
    pub(super) fn names(&self, module: LocalModId) -> impl Iterator<Item = &str> {
        (self.made.iter())
            .filter(move |((at, ..), _)| *at == module)
            .map(|(_, made)| made.name.as_str())
    }

    /// The functions `item` calls not yet written, each then written: its
    /// module's, before `item`.
    pub(super) fn written_before(&mut self, item: DefId) -> Vec<js::Function> {
        let mut first: Vec<_> = (self.made.values_mut())
            .filter(|made| !made.written && made.used_by.contains(&item))
            .filter_map(|made| {
                made.written = true;
                made.lowered
                    .as_ref()
                    .map(|(function, _)| (made.made_before, function.clone()))
            })
            .collect();
        first.sort_by_key(|(made_before, _)| *made_before);
        first.into_iter().map(|(_, function)| function).collect()
    }
}

/// The functions a type only a caller knows has, being made: each by its
/// kind and type, its name, which one inside it calls.
#[derive(Default)]
pub(super) struct Making<'tcx>(Vec<(Kind, Ty<'tcx>, String)>);

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The function that's `ty`'s `kind`, recursive (`is_recursive`), named
    /// of `base` and the type, `eqHir`: made the first time it's asked of
    /// `params`, the names of its parameters, whose body `make` lowers,
    /// given them and its statements; then called. Its module's, where its
    /// type is the module's to know; else declared in `out`.
    pub(super) fn recursive_fn(
        &mut self,
        (kind, ty, base): (Kind, Ty<'tcx>, &str),
        params: &[&str],
        make: impl FnOnce(&mut Self, Vec<Expr>, &mut Vec<Stmt>) -> R<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<String> {
        if ty.has_param() || ty.has_aliases() {
            return self.local_recursive_fn((kind, ty, base), params, make, out);
        }
        let key = (self.module, kind, self.tcx.erase_and_anonymize_regions(ty));
        let mut module_fns = self.krate.module_fns.borrow_mut();
        if module_fns.made.contains_key(&key) {
            module_fns.called(key, self.item);
            let made = &module_fns.made[&key];
            let name = made.name.clone();
            // What it uses, this item's to import and reach too; none yet
            // of the one being made, which its own body calls.
            if let Some((_, uses)) = &made.lowered {
                merge(&mut self.dependencies.borrow_mut(), uses, self.item);
            }
            drop(module_fns);
            self.names.insert(name.clone());
            return Ok(name);
        }
        // Apart from the module's names, its other functions', and this
        // body's, which mustn't hide it.
        let mut taken: HashSet<String> = (self.module_names.iter().cloned())
            .chain(module_fns.names(self.module).map(str::to_owned))
            .collect();
        taken.extend(self.names.iter().cloned());
        let name = super::fresh_in(&mut taken, base);
        let made_before = module_fns.made.len();
        module_fns.made.insert(
            key,
            Made {
                name: name.clone(),
                made_before,
                lowered: None,
                calls: HashSet::new(),
                used_by: HashSet::new(),
                written: false,
            },
        );
        module_fns.called(key, self.item);
        module_fns.building.push(key);
        drop(module_fns);
        self.names.insert(name.clone());
        // Its body, lowered as a function of its own: its names apart from
        // this body's, what it uses its own, given no dictionary of this
        // item's.
        let mut names: HashSet<String> = (self.module_names.iter().cloned())
            .chain(self.krate.module_fns.borrow().names(self.module).map(str::to_owned))
            .collect();
        let params: Vec<String> = params.iter().map(|p| super::fresh_in(&mut names, p)).collect();
        let names = std::mem::replace(&mut self.names, names);
        let uses = self.dependencies.replace(Dependencies::default());
        let given = self.set_given_aside();
        let mut body = Vec::new();
        let value = make(self, params.iter().map(|p| Expr::var(p)).collect(), &mut body);
        self.restore_given(given);
        self.names = names;
        let own = self.dependencies.replace(uses);
        self.krate.module_fns.borrow_mut().building.pop();
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
        let mut module_fns = self.krate.module_fns.borrow_mut();
        let made = module_fns.made.get_mut(&key).expect("made above");
        made.lowered = Some((function, own));
        Ok(name)
    }

    /// `recursive_fn`'s of a type only a caller knows: a function of this
    /// body, made where it's first asked, in `out`, which reads the
    /// dictionaries the body is given.
    fn local_recursive_fn(
        &mut self,
        (kind, ty, base): (Kind, Ty<'tcx>, &str),
        params: &[&str],
        make: impl FnOnce(&mut Self, Vec<Expr>, &mut Vec<Stmt>) -> R<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<String> {
        if let Some((.., name)) = self.recursive.0.iter().find(|(k, t, _)| *k == kind && *t == ty) {
            return Ok(name.clone());
        }
        let name = self.fresh(base);
        let params: Vec<String> = params.iter().map(|p| self.fresh(p)).collect();
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
