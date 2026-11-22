//! What a test that an item is there shows of its index (ADR 0881,
//! amended): where `let Some(x) = v.get(i) else { .. }` holds, `i` is an
//! index of a slice, below its length, so below `usize`'s most, and
//! `i + 1` can't wrap; through `copied()` or `cloned()` too. What MIR's
//! hints ask of a `let`-`else` (`mir/hints/present_breaks.rs`), kept of
//! THIR's ranges as M4 deleted them (#1672).

use crate::body_queries::strip;
use crate::fn_def;
use crate::recognition::{PresentItem, present_item};
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, Pat, PatKind, Thir};
use rustc_middle::ty::{TyCtxt, TypeVisitableExt};

/// Whether `init`, an `Option` a `let pattern = init else { .. }` tests,
/// `pattern` a `Some(..)`, is an item of a slice by its index, `v.get(i)`,
/// `.copied()` too, of items that take up room: where it's `Some`, `i` is
/// below the slice's length, at most `isize`'s most (ADR 0701), so `i + n`
/// stays in `usize` for any `n` up to it, and where it's `None`, `i` is the
/// length or past it (rd260).
pub fn present_item_of_sized<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, pattern: &Pat<'tcx>, init: ExprId) -> bool {
    if !some_pattern(tcx, pattern) {
        return false;
    }
    let rustc_middle::ty::Adt(_, args) = thir[init].ty.kind() else {
        return false;
    };
    // The slice's item: `get`'s `Option<&T>`'s `T`, which may be a
    // reference itself, `&Element` of a `Vec<&Element>`; one reference of
    // `copied()`'s too, which only leaves out one of no size.
    let item = args.types().next().map(|t| match t.kind() {
        rustc_middle::ty::Ref(_, item, _) => *item,
        _ => t,
    });
    let sized = item.filter(|item| !item.has_param()).is_some_and(|item| {
        (tcx.layout_of(rustc_middle::ty::TypingEnv::fully_monomorphized().as_query_input(item)))
            .is_ok_and(|layout| !layout.is_zst())
    });
    sized && present_index(tcx, thir, init).is_some()
}

/// Whether `pat` is `Some(..)`'s.
fn some_pattern(tcx: TyCtxt<'_>, pat: &Pat<'_>) -> bool {
    matches!(&pat.kind, PatKind::Variant { adt_def, variant_index, .. }
        if tcx.lang_items().option_some_variant() == Some(adt_def.variant(*variant_index).def_id))
}

/// The variable whose value `e`, an `Option`, being `Some` shows is an
/// item's index: `i` of `v.get(i)` and `v.get(i).copied()`.
fn present_index<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, e: ExprId) -> Option<LocalVarId> {
    let e = strip(thir, e);
    let ExprKind::Call { fun, ref args, .. } = thir[e].kind else {
        return match thir[e].kind {
            // A closure's body of one value, `{ v.get(i) }`.
            ExprKind::Block { block } if thir[block].stmts.is_empty() => present_index(tcx, thir, thir[block].expr?),
            _ => None,
        };
    };
    let (def_id, generic_args) = fn_def(thir[strip(thir, fun)].ty)?;
    match present_item(tcx, def_id, generic_args) {
        Some(PresentItem::Get) => var_of(thir, *args.get(1)?),
        Some(PresentItem::Passed | PresentItem::Filtered) => present_index(tcx, thir, *args.first()?),
        None => None,
    }
}

/// The variable `e` reads, `i` of `v.get(i)`.
fn var_of(thir: &Thir<'_>, e: ExprId) -> Option<LocalVarId> {
    match thir[strip(thir, e)].kind {
        ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => Some(id),
        _ => None,
    }
}
