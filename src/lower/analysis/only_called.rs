//! The crate's functions only calls reach (ADR 0577): each one no JS
//! caller sees, never named as a value, a trait's method or exported, by
//! how many calls of it the crate's code makes. What each call gives it is
//! all it's ever given, so what's known of its parameters is what's known
//! where it's called.

use rustc_hir as hir;
use rustc_hir::def::{DefKind, Res};
use rustc_hir::intravisit::{self, Visitor};
use rustc_middle::ty::{TyCtxt, TypeckResults};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// Each function only calls reach, by how many there are.
pub(in crate::lower) fn only_called(tcx: TyCtxt<'_>) -> HashMap<DefId, usize> {
    struct Mentions<'tcx> {
        typeck: &'tcx TypeckResults<'tcx>,
        calls: HashMap<DefId, usize>,
        values: HashSet<DefId>,
    }
    impl<'tcx> Mentions<'tcx> {
        fn named(&self, qpath: &hir::QPath<'tcx>, id: hir::HirId) -> Option<DefId> {
            match self.typeck.qpath_res(qpath, id) {
                Res::Def(DefKind::Fn | DefKind::AssocFn, def_id) => Some(def_id),
                _ => None,
            }
        }
    }
    impl<'tcx> Visitor<'tcx> for Mentions<'tcx> {
        fn visit_expr(&mut self, e: &'tcx hir::Expr<'tcx>) {
            match e.kind {
                hir::ExprKind::Call(callee, args)
                    if let hir::ExprKind::Path(ref qpath) = callee.kind
                        && let Some(def_id) = self.named(qpath, callee.hir_id) =>
                {
                    *self.calls.entry(def_id).or_default() += 1;
                    for arg in args {
                        self.visit_expr(arg);
                    }
                    return;
                }
                hir::ExprKind::MethodCall(..) => {
                    if let Some(def_id) = self.typeck.type_dependent_def_id(e.hir_id) {
                        *self.calls.entry(def_id).or_default() += 1;
                    }
                }
                // Named as a value: a function pointer, a callback, what
                // anything may call with anything.
                hir::ExprKind::Path(ref qpath) => self.values.extend(self.named(qpath, e.hir_id)),
                _ => {}
            }
            intravisit::walk_expr(self, e);
        }
    }
    let mut calls: HashMap<DefId, usize> = HashMap::new();
    let mut values: HashSet<DefId> = HashSet::new();
    for owner in tcx.hir_body_owners() {
        let mut mentions = Mentions {
            typeck: tcx.typeck(owner),
            calls: HashMap::new(),
            values: HashSet::new(),
        };
        mentions.visit_body(tcx.hir_body_owned_by(owner));
        for (def_id, n) in mentions.calls {
            *calls.entry(def_id).or_default() += n;
        }
        values.extend(mentions.values);
    }
    let visibilities = tcx.effective_visibilities(());
    calls.retain(|&def_id, _| {
        def_id.as_local().is_some_and(|local| !visibilities.is_exported(local))
            && !values.contains(&def_id)
            && (tcx.def_kind(def_id) == DefKind::Fn
                || tcx.def_kind(def_id) == DefKind::AssocFn
                    && tcx.trait_of_assoc(def_id).is_none()
                    && tcx.trait_impl_of_assoc(def_id).is_none())
    });
    calls
}
