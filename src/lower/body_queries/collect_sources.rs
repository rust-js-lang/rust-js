//! The `Vec` each `collect()` of a body's is made of where std may collect
//! it in place: its chain's `into_iter(v)` of a variable of the body's, or
//! of a place in one, a field's or a `Box`'s, `s.0` or `*b`, the
//! buffer std collects in, `src_cap * src_size / dst_size` of whose capacity
//! is the collect's (#1259). Split from body_queries.rs by concept, as a file
//! is at most 1500 lines.

use super::strip;
use crate::lower::fn_def;
use crate::lower::recognition::{StdItem, is_std_def, std_item, trait_method};
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, PatKind, StmtKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;

/// Where a `collect()`'s source `Vec` is: a variable of the body's, and the
/// fields under it the place reads, each by the type it's of, outermost
/// first; a `Box`'s deref none, as a `Box` is its value (ADR 0023).
#[derive(Clone, Debug, PartialEq)]
pub struct CollectSource<'tcx> {
    pub var: LocalVarId,
    pub(in crate::lower) fields: Vec<(Ty<'tcx>, usize)>,
}

/// The place `at` reads, of a variable, its fields' and its `Box`es'
/// derefs, if it's one.
fn place_of<'tcx>(thir: &Thir<'tcx>, mut at: ExprId) -> Option<CollectSource<'tcx>> {
    let mut fields = Vec::new();
    loop {
        match thir[at].kind {
            ExprKind::VarRef { id } => {
                fields.reverse();
                return Some(CollectSource { var: id, fields });
            }
            ExprKind::Field { lhs, name, .. } => {
                fields.push((thir[lhs].ty, name.as_usize()));
                at = strip(thir, lhs);
            }
            ExprKind::Deref { arg } if thir[arg].ty.is_box() => at = strip(thir, arg),
            _ => return None,
        }
    }
}

/// Each `collect()`, by its THIR node, whose iterator is a chain of
/// `Iterator`'s methods of `IntoIterator::into_iter(v)` of a `Vec` the body
/// names, or a place in one, `v.into_iter().map(f).collect()`, through
/// calls of the crate's functions that return their argument's iterator
/// or a `Vec` argument's `into_iter()` (`returned_iterator`, #1934): `v`, which
/// the chain moves, so nothing grows it before the collect reads its
/// capacity. Whether std collects it in place is the collect lowering's
/// (`collects_in_place`); a chain a `let` holds, or a `Vec` made where it's
/// moved, has none. Two collects a macro's repetition writes share a span,
/// so each is known by its node, and MIR's call at the span asks each node
/// there (`span_join::each_call_at`, `mir/locals.rs`' `in_place_source_names`):
/// of sources apart, neither's is taken of the other (#1788, ADR 1151).
pub(super) fn in_place_collect_sources<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &Thir<'tcx>,
) -> IndexMap<ExprId, CollectSource<'tcx>> {
    let iterator = std_item(tcx, StdItem::Iterator);
    let collect = trait_method(tcx, iterator, "collect");
    let into_iter = trait_method(tcx, std_item(tcx, StdItem::IntoIterator), "into_iter");
    // Each collect's source, by its node.
    let mut found = IndexMap::default();
    for (node, expr) in thir.exprs.iter_enumerated() {
        let ExprKind::Call { fun, ref args, .. } = expr.kind else {
            continue;
        };
        if fn_def(thir[fun].ty).map(|(id, _)| id) != Some(collect) {
            continue;
        }
        let Some(&first) = args.first() else { continue };
        let mut at = strip(thir, first);
        while let ExprKind::Call { fun, ref args, .. } = thir[at].kind {
            let Some((callee, callee_args)) = fn_def(thir[fun].ty) else {
                break;
            };
            let Some(&receiver) = args.first() else { break };
            if callee == into_iter {
                if let Some(source) = place_of(thir, strip(thir, receiver))
                    && let Some(vec) = callee_args.types().next()
                    && matches!(vec.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Vec))
                {
                    found.insert(node, source);
                }
                break;
            }
            // A call of the crate's function that returns its argument's
            // iterator, or a `Vec` argument's `into_iter()`: on into that
            // argument (`returned_iterator`, #1934).
            if tcx.trait_of_assoc(callee) != Some(iterator) {
                use crate::lower::pointers::{Returned, Via, returned_iterator};
                let Some(Returned { param, via }) = returned_iterator(tcx, callee) else {
                    break;
                };
                let Some(&arg) = args.get(param) else { break };
                if via == Via::IntoIter {
                    if let Some(source) = place_of(thir, strip(thir, arg))
                        && matches!(thir[arg].ty.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Vec))
                    {
                        found.insert(node, source);
                    }
                    break;
                }
                at = strip(thir, arg);
                continue;
            }
            at = strip(thir, receiver);
        }
    }
    found
}

/// Each `collect()` whose `Vec` the body asks its capacity, by its call's
/// span: one a `let` not `mut` binds, whose variable is `capacity()`'s
/// receiver, nothing changing it between, or one that's the receiver
/// itself, `(..).collect::<Vec<_>>().capacity()`.
/// Where std may collect it in place and its source isn't known, the
/// capacity can't be, so it's refused where it's compiled (#1926).
pub(super) fn capacity_read_collects<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> IndexSet<Span> {
    use crate::lower::recognition::{CapacityOp, capacity_op};
    let collect = trait_method(tcx, std_item(tcx, StdItem::Iterator), "collect");
    let collect_at = |at: ExprId| match thir[strip(thir, at)].kind {
        ExprKind::Call { fun, fn_span, .. } if fn_def(thir[fun].ty).map(|(id, _)| id) == Some(collect) => Some(fn_span),
        _ => None,
    };
    // Each variable a `let` binds to a collect, and the collect's span.
    let mut bound: IndexMap<LocalVarId, Span> = IndexMap::default();
    for stmt in thir.stmts.iter() {
        if let StmtKind::Let {
            pattern,
            initializer: Some(init),
            ..
        } = &stmt.kind
            && let Some(at) = collect_at(*init)
        {
            // Not `mut`, whose `shrink_to_fit()` would know it again.
            if let PatKind::Binding {
                var,
                subpattern: None,
                mode: rustc_hir::BindingMode(rustc_hir::ByRef::No, rustc_hir::Mutability::Not),
                ..
            } = pattern.kind
            {
                bound.insert(var, at);
            }
        }
    }
    let mut read = IndexSet::default();
    for expr in thir.exprs.iter() {
        let ExprKind::Call { fun, ref args, .. } = expr.kind else {
            continue;
        };
        if !fn_def(thir[fun].ty).is_some_and(|(id, _)| capacity_op(tcx, id) == Some(CapacityOp::Of)) {
            continue;
        }
        let Some(&receiver) = args.first() else { continue };
        let mut at = strip(thir, receiver);
        while let ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } = thir[at].kind {
            at = strip(thir, arg);
        }
        match thir[at].kind {
            ExprKind::VarRef { id } => read.extend(bound.get(&id).copied()),
            _ => read.extend(collect_at(at)),
        }
    }
    read
}
