//! What the crate's bodies do with a `Vec`: change one in place, or ask its
//! capacity.

use crate::collections::IndexSet;
use crate::lower::Body;
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, Ty, TyCtxt};

/// The `Vec` types something takes `&mut` of: `push`, `sort`, `v[i] = x`
/// and every other change to one does. A clone of any other `Vec` can be
/// the same array (ADR 0052).
pub(super) fn changed_vecs<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> IndexSet<Ty<'tcx>> {
    let mut changed = IndexSet::default();
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            if let ExprKind::Borrow {
                borrow_kind: BorrowKind::Mut { .. },
                arg,
            } = expr.kind
                && let ty = body.thir[arg].ty
                && matches!(ty.kind(), ty::Adt(adt, _) if crate::lower::recognition::is_vec_like_def(tcx, adt.did()))
            {
                changed.insert(ty);
            }
        }
    }
    changed
}

/// Whether any body asks a `Vec` its capacity, `v.capacity()` (ADR 0419).
pub(super) fn asks_capacity<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> bool {
    use crate::lower::recognition::{CapacityOp, capacity_op};
    bodies.iter().any(|body| {
        body.thir.exprs.iter().any(|expr| {
            matches!(expr.kind, ExprKind::ZstLiteral { .. })
                && crate::lower::fn_def(expr.ty).is_some_and(|(id, _)| capacity_op(tcx, id) == Some(CapacityOp::Of))
        })
    })
}
