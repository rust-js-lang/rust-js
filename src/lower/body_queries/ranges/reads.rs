//! What reads without changing anything, so its value where a condition
//! is tested is its value after (ADRs 0576, 0806): `i < xs.len()`, of the
//! variables it reads.

use super::reads_utf16_length;
use crate::lower::bindings::reads_string_length;
use crate::lower::body_queries::strip;
use crate::lower::fn_def;
use crate::lower::recognition::{CellView, SliceLength, cell_view, makes_utf16_units, slice_length};
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, Thir};
use rustc_middle::ty::TyCtxt;

/// Whether `e` reads without changing anything, so its value where a
/// condition is tested is its value after: `i < xs.len()`.
pub(super) fn pure<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, e: ExprId) -> bool {
    read_vars(tcx, thir, e).is_some()
}

/// The variables `e` reads, where it reads without changing anything
/// (`pure`): `i` and `xs` of `i < xs.len()`.
pub(super) fn read_vars<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, e: ExprId) -> Option<Vec<LocalVarId>> {
    fn reads<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, e: ExprId, vars: &mut Vec<LocalVarId>) -> bool {
        match thir[e].kind {
            ExprKind::Scope { value: inner, .. }
            | ExprKind::Use { source: inner }
            | ExprKind::Deref { arg: inner }
            | ExprKind::Field { lhs: inner, .. }
            | ExprKind::Unary { arg: inner, .. }
            | ExprKind::Cast { source: inner }
            | ExprKind::Borrow {
                borrow_kind: BorrowKind::Shared,
                arg: inner,
            } => reads(tcx, thir, inner, vars),
            ExprKind::Binary { lhs, rhs, .. } => reads(tcx, thir, lhs, vars) && reads(tcx, thir, rhs, vars),
            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => {
                vars.push(id);
                true
            }
            ExprKind::Literal { .. }
            | ExprKind::NonHirLiteral { .. }
            | ExprKind::NamedConst { .. }
            | ExprKind::ConstParam { .. } => true,
            ExprKind::Call { fun, ref args, .. } => {
                fn_def(thir[strip(thir, fun)].ty).is_some_and(|(def_id, generic_args)| {
                    matches!(
                        slice_length(tcx, def_id, generic_args),
                        Some(SliceLength::Len | SliceLength::IsEmpty)
                    ) || reads_string_length(tcx, def_id)
                        // A `RefCell`'s read of what it holds, `queue.borrow()`,
                        // and its guard's `deref`, the value (ADR 0881).
                        || matches!(
                            cell_view(tcx, def_id, generic_args),
                            Some(CellView::Guard { mutable: false } | CellView::Value)
                        )
                        // A string's `encode_utf16()`, std's, which reads it, and
                        // its `count()`, its `length` (ADR 0632).
                        || makes_utf16_units(tcx, def_id, thir[e].ty)
                        || reads_utf16_length(tcx, def_id, generic_args)
                }) && args.iter().all(|&arg| reads(tcx, thir, arg, vars))
            }
            _ => false,
        }
    }
    let mut vars = Vec::new();
    reads(tcx, thir, e, &mut vars).then_some(vars)
}
