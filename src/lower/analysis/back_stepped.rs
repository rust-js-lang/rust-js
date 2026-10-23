//! The adapter types the crate steps from their back (ADR 0545): a `map` or
//! a `filter` whose `next_back`, `rfold` and the like are called, of a
//! source not an array's, whose own forms step it, or of an array's items
//! with a destructor, an owning source closed to drop them, which they
//! don't. A map of such a type is
//! made knowing its back where it's made, as the iterators draft asks of
//! one owner, before lowering: so one held in a variable before it's
//! stepped, as a library's drop flags make one, still steps from its back.

use crate::lower::recognition::{
    Recognition, filtered_source, flattened_source, is_double_ended, is_iter_adapter_def, mapped_source,
};
use crate::lower::{Body, fn_def};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use std::collections::HashSet;

/// Each such type `all_bodies` step from its back, regions erased.
pub(super) fn back_stepped<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> HashSet<Ty<'tcx>> {
    let mut stepped = HashSet::new();
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            // What it asks to be double-ended holds such a type, as an
            // `Either` of filters does, stepped from its back by the impl
            // that asks it of what it holds; the type asked itself is its
            // call's own, below or at its `rev()`.
            for (clause, _) in tcx.clauses_of(callee).instantiate(tcx, args) {
                let Some(asked) = clause.skip_normalization().as_trait_clause() else {
                    continue;
                };
                if !is_double_ended(tcx, asked.def_id()) {
                    continue;
                }
                let asked = recognition.reveal(asked.self_ty().skip_binder().peel_refs());
                // Not through a std adapter, whose own call steps what it
                // holds: `filter(p).map(f).rev()`'s is `rev()`'s.
                let mut walk = asked.walk();
                while let Some(part) = walk.next() {
                    let Some(held) = part.as_type() else { continue };
                    if matches!(held.kind(), ty::Adt(adapter, _) if is_iter_adapter_def(tcx, adapter.did())) {
                        // The type asked itself too where it's of an array's
                        // items with a destructor, which `rev()` can't step
                        // from its back once it's held in a variable.
                        if held != asked || dropping_items(recognition, held, &drops) {
                            stepped.extend(back_stepped_adapter(tcx, recognition, held, &drops));
                        }
                        walk.skip_current_subtree();
                    }
                }
            }
            if !tcx.trait_of_assoc(callee).is_some_and(|tr| is_double_ended(tcx, tr)) {
                continue;
            }
            let Some(adapter) = args.types().next().map(|ty| ty.peel_refs()) else {
                continue;
            };
            stepped.extend(back_stepped_adapter(tcx, recognition, adapter, &drops));
        }
    }
    stepped
}

/// `adapter`, regions erased, where it's a map or a filter of a source not
/// an array's, which its own forms step from their back, or of an array's
/// items whose drop runs, `drops`'s.
fn back_stepped_adapter<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    adapter: Ty<'tcx>,
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> Option<Ty<'tcx>> {
    // A flatten, of whatever source: its own forms never step from the back.
    if flattened_source(tcx, adapter).is_some() {
        return Some(tcx.erase_and_anonymize_regions(adapter));
    }
    let source = mapped_source(tcx, adapter)
        .or_else(|| filtered_source(tcx, adapter))?
        .peel_refs();
    let native = recognition.array_source(source).is_some() && !dropping_items(recognition, adapter, &drops);
    (!native).then(|| tcx.erase_and_anonymize_regions(adapter))
}

/// Whether `adapter`, a map or a filter, is of an array's items whose drop
/// runs, `drops`'s: an owning source closed to drop them (ADR 0465).
fn dropping_items<'tcx>(
    recognition: Recognition<'_, 'tcx>,
    adapter: Ty<'tcx>,
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> bool {
    let tcx = recognition.tcx;
    mapped_source(tcx, adapter)
        .or_else(|| filtered_source(tcx, adapter))
        .map(|source| source.peel_refs())
        .is_some_and(|source| {
            recognition.array_source(source).is_some() && recognition.iterator_item(source).is_some_and(drops)
        })
}
