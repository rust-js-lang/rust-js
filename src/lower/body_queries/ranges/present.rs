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
use crate::lower::recognition::{PresentItem, SliceLength, is_local_borrow, present_item, slice_length};
use rustc_middle::mir::UnOp;
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, Pat, PatKind, Thir};
use rustc_middle::ty::{TyCtxt, TypeVisitableExt};
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
        Some(PresentItem::Passed | PresentItem::Filtered) => present_index(tcx, thir, *args.first()?, bodies),
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

/// The variable `pat`, a `Some(v)`, binds where `e`, the `Option` it
/// tests, being `Some` shows `v` isn't empty: an `Option`'s `filter` whose
/// closure tests that what it's given isn't, `queue.filter(|queue|
/// !queue.is_empty())`, its body, which the crate's bodies have (rd376).
pub(super) fn present_not_empty<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &Thir<'tcx>,
    e: ExprId,
    pat: &Pat<'tcx>,
    bodies: &Bodies<'_, 'tcx>,
) -> Option<LocalVarId> {
    let PatKind::Variant { ref subpatterns, .. } = pat.kind else {
        return None;
    };
    let [field] = subpatterns.as_slice() else {
        return None;
    };
    let PatKind::Binding {
        var, subpattern: None, ..
    } = field.pattern.kind
    else {
        return None;
    };
    let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, e)].kind else {
        return None;
    };
    let (def_id, generic_args) = fn_def(thir[strip(thir, fun)].ty)?;
    if present_item(tcx, def_id, generic_args) != Some(PresentItem::Filtered) {
        return None;
    }
    let ExprKind::Closure(ref closure) = thir[strip(thir, *args.get(1)?)].kind else {
        return None;
    };
    let &(inner, root) = bodies.get(&closure.closure_id.to_def_id())?;
    let PatKind::Binding { var: given, .. } = inner.params.iter().nth(1)?.pat.as_deref()?.kind else {
        return None;
    };
    tests_not_empty(tcx, inner, root, given).then_some(var)
}

/// Whether `e`, a closure's body, is `!v.is_empty()` of `v`, through its
/// references.
fn tests_not_empty<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, e: ExprId, v: LocalVarId) -> bool {
    match thir[strip(thir, e)].kind {
        ExprKind::Block { block } if thir[block].stmts.is_empty() => {
            thir[block].expr.is_some_and(|e| tests_not_empty(tcx, thir, e, v))
        }
        ExprKind::Unary { op: UnOp::Not, arg } => {
            let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, arg)].kind else {
                return false;
            };
            fn_def(thir[strip(thir, fun)].ty).is_some_and(|(def_id, generic_args)| {
                slice_length(tcx, def_id, generic_args) == Some(SliceLength::IsEmpty)
            }) && args.first().is_some_and(|&items| given_var(thir, items) == Some(v))
        }
        _ => false,
    }
}

/// The variable `e` is, through its references: `queue` of `&*queue`.
fn given_var(thir: &Thir<'_>, mut e: ExprId) -> Option<LocalVarId> {
    loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = arg,
            ExprKind::VarRef { id } => return Some(id),
            _ => return None,
        }
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

/// Whether `init`, an `Option` a `let pattern = init else { .. }` tests,
/// `pattern` a `Some(..)`,
/// is an item of a slice by its index, `v.get(i)`, `.copied()` too
/// (`present::present_index`), of items that take up room: where it's
/// `Some`, `i` is below the slice's length, at most `isize`'s most (ADR
/// 0701), so `i + n` stays in `usize` for any `n` up to it, and where it's
/// `None`, `i` is the length or past it (rd260).
pub(in crate::lower) fn present_item_of_sized<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &Thir<'tcx>,
    pattern: &Pat<'tcx>,
    init: ExprId,
) -> bool {
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
    sized && present_index(tcx, thir, init, &HashMap::new()).is_some()
}
