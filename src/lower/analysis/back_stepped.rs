//! The adapter types the crate steps from their back (ADR 0545): a `map` or
//! a `filter` whose `next_back`, `rfold` and the like are called, of a
//! source not an array's, whose own forms step it. A map of such a type is
//! made knowing its back where it's made, as the iterators draft asks of
//! one owner, before lowering: so one held in a variable before it's
//! stepped, as a library's drop flags make one, still steps from its back.

use crate::lower::recognition::{Recognition, filtered_source, is_double_ended, mapped_source};
use crate::lower::{Body, fn_def};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{Ty, TyCtxt};
use std::collections::HashSet;

/// Each such type `all_bodies` step from its back, regions erased.
pub(super) fn back_stepped<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
) -> HashSet<Ty<'tcx>> {
    let mut stepped = HashSet::new();
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            if !tcx.trait_of_assoc(callee).is_some_and(|tr| is_double_ended(tcx, tr)) {
                continue;
            }
            let Some(adapter) = args.types().next().map(|ty| ty.peel_refs()) else {
                continue;
            };
            let source = mapped_source(tcx, adapter).or_else(|| filtered_source(tcx, adapter));
            if source.is_some_and(|source| recognition.array_source(source.peel_refs()).is_none()) {
                stepped.insert(tcx.erase_and_anonymize_regions(adapter));
            }
        }
    }
    stepped
}
