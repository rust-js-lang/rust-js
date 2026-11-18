//! What a library's JS leaves copies of out, each function's and trait
//! impl's, which its contract says (`Library::elided`, ADR 0914, amended;
//! #2112's change D): a consumer that changes one compiles its own.

use super::{Foreign, key};
use crate::collections::{IndexMap, IndexSet};
use crate::library::RcShape;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;
use std::collections::BTreeSet;

/// Each function's elided copies (`Caches::elided`, #2112's change D), its
/// own and those of each function of its crate it calls or names, through
/// their calls in turn: a consumer that compiles a function itself
/// compiles the private ones it calls too (ADR 0914), so what any of them
/// leaves out is the function's to say.
pub(in crate::lower) fn elided_through_calls(
    tcx: TyCtxt<'_>,
    bodies: &[crate::lower::Body<'_>],
) -> IndexMap<DefId, BTreeSet<RcShape>> {
    // What each function calls or names, its closures' too.
    let mut callees: IndexMap<DefId, IndexSet<DefId>> = IndexMap::default();
    for body in bodies {
        let Some(mir) = &body.mir else { continue };
        let named = crate::lower::instances::called(&mir.body)
            .into_iter()
            .map(|(callee, _)| callee);
        callees
            .entry(tcx.typeck_root_def_id(body.def_id))
            .or_default()
            .extend(named);
    }
    let callees = &callees;
    let own = crate::lower::session::caches(tcx).elided.borrow().clone();
    let mut all: IndexMap<DefId, BTreeSet<RcShape>> = IndexMap::default();
    for &function in callees.keys().chain(own.keys()) {
        let mut seen = IndexSet::default();
        let mut work = vec![function];
        let mut found = std::collections::BTreeSet::new();
        while let Some(at) = work.pop() {
            if !at.is_local() || !seen.insert(at) {
                continue;
            }
            found.extend(own.get(&at).into_iter().flatten().cloned());
            work.extend(callees.get(&at).into_iter().flatten().copied());
            // A trait's method, called through a dictionary or resolved
            // where it's called: each of the crate's impls' of it.
            if let Some(trait_id) = tcx.trait_of_assoc(at) {
                work.extend(
                    (tcx.all_impls(trait_id).filter(|imp| imp.is_local()))
                        .filter_map(|imp| tcx.impl_item_implementor_ids(imp).get(&at).copied()),
                );
            }
        }
        if !found.is_empty() {
            all.insert(function, found);
        }
    }
    // A trait impl, its contract's item, a consumer's call of whose
    // methods compiles the impl: what each of its methods leaves out, and
    // each default of its trait it uses, through their calls.
    let impls: Vec<DefId> = (all.keys().chain(callees.keys()).copied())
        .filter_map(|id| tcx.trait_impl_of_assoc(id))
        .collect::<crate::collections::IndexSet<_>>()
        .into_iter()
        .collect();
    for imp in impls {
        let found: BTreeSet<RcShape> = (tcx.associated_item_def_ids(imp).iter().copied())
            .chain(crate::lower::instances::defaults_used(tcx, imp))
            .flat_map(|method| all.get(&method).into_iter().flatten().cloned().collect::<Vec<_>>())
            .collect();
        if !found.is_empty() {
            all.entry(imp).or_default().extend(found);
        }
    }
    all
}

impl<'a> Foreign<'a, '_> {
    /// What a library's function or trait impl leaves copies of out, the
    /// shapes of their types, as its contract says (`Library::elided`,
    /// #2112's change D): of a closure, its function's. `None` of what no
    /// library's JS has: the crate's own, an instance, what isn't a
    /// library's.
    pub(in crate::lower) fn elided(&self, id: DefId) -> Option<&'a [RcShape]> {
        if id.is_local() || self.is_instance(id) {
            return None;
        }
        let library = self.library(id.krate)?;
        let owner = match self.tcx.def_kind(id) {
            rustc_hir::def::DefKind::Impl { .. } => id,
            _ => self.tcx.typeck_root_def_id(id),
        };
        Some(library.elided.get(&key(self.tcx, owner)).map_or(&[], Vec::as_slice))
    }
}
