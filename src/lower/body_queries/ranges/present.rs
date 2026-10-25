//! What a test that an item is there shows of its index (ADR 0881,
//! amended): where `let Some(x) = v.get(i)` holds, `i` is an index of a
//! slice, below its length, so below `usize`'s most, and `i + 1` can't
//! wrap; through `copied()` or `cloned()`, and of a thread-local's
//! `with_borrow(|v| v.get(i))`, its closure's body, which the crate's
//! bodies have.

use std::collections::HashMap;

use super::known::{Known, Range};
use super::var_of;
use crate::lower::body_queries::strip;
use crate::lower::fn_def;
use crate::lower::recognition::{PresentItem, is_local_borrow, present_item};
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, Pat, PatKind, Thir};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// Each of the crate's bodies walked, by what it is: its THIR and its root.
pub(super) type Bodies<'a, 'tcx> = HashMap<DefId, (&'a Thir<'tcx>, ExprId)>;

/// Whether `pat` is `Some(..)`'s.
pub(super) fn some_pattern(tcx: TyCtxt<'_>, pat: &Pat<'_>) -> bool {
    matches!(&pat.kind, PatKind::Variant { adt_def, variant_index, .. }
        if tcx.lang_items().option_some_variant() == Some(adt_def.variant(*variant_index).def_id))
}

/// The variable whose value `e`, an `Option`, being `Some` shows is an
/// item's index: `i` of `v.get(i)`, `v.get(i).copied()` and
/// `LOCAL.with_borrow(|v| v.get(i))`.
pub(super) fn present_index<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &Thir<'tcx>,
    e: ExprId,
    bodies: &Bodies<'_, 'tcx>,
) -> Option<LocalVarId> {
    let e = strip(thir, e);
    let ExprKind::Call { fun, ref args, .. } = thir[e].kind else {
        return match thir[e].kind {
            // A closure's body of one value, `{ v.get(i) }`.
            ExprKind::Block { block } if thir[block].stmts.is_empty() => {
                present_index(tcx, thir, thir[block].expr?, bodies)
            }
            _ => None,
        };
    };
    let (def_id, generic_args) = fn_def(thir[strip(thir, fun)].ty)?;
    match present_item(tcx, def_id, generic_args) {
        Some(PresentItem::Get) => var_of(thir, *args.get(1)?),
        Some(PresentItem::Passed) => present_index(tcx, thir, *args.first()?, bodies),
        None if is_local_borrow(tcx, def_id) => {
            let ExprKind::Closure(ref closure) = thir[strip(thir, *args.get(1)?)].kind else {
                return None;
            };
            let &(inner, root) = bodies.get(&closure.closure_id.to_def_id())?;
            present_index(tcx, inner, root, bodies)
        }
        None => None,
    }
}

/// `var`, an index of a slice where it's known so: below `usize`'s most,
/// as a slice is no longer than it.
pub(super) fn below_most(var: LocalVarId, volatile: bool, known: &mut Option<Known>) {
    const MOST: i128 = u32::MAX as i128;
    if let Some(k) = known.as_mut()
        && !volatile
    {
        let current = k.vars.get(&var).cloned().unwrap_or(Range::of(0, MOST));
        k.vars.insert(
            var,
            Range {
                hi: current.hi.min(MOST - 1),
                ..current
            },
        );
    }
}
