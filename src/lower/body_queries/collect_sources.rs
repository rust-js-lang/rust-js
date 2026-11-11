//! The `Vec` each `collect()` of a body's is made of where std may collect
//! it in place: its chain's `into_iter(v)` of a variable of the body's, or
//! of a place in one, a field's or a `Box`'s, `s.0` or `*b`, the
//! buffer std collects in, `src_cap * src_size / dst_size` of whose capacity
//! is the collect's (#1259). Split from body_queries.rs by concept, as a file
//! is at most 1500 lines.

use super::strip;
use crate::collections::IndexMap;
use crate::lower::fn_def;
use crate::lower::recognition::{StdItem, is_std_def, std_item, trait_method};
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;

/// Where a `collect()`'s source `Vec` is: a variable of the body's, and the
/// fields under it the place reads, each by the type it's of, outermost
/// first; a `Box`'s deref none, as a `Box` is its value (ADR 0023).
#[derive(Clone, Debug)]
pub(in crate::lower) struct CollectSource<'tcx> {
    pub(in crate::lower) var: LocalVarId,
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

/// Each `collect()`, by its call's span, whose iterator is a chain of
/// `Iterator`'s methods of `IntoIterator::into_iter(v)` of a `Vec` the body
/// names, or a place in one, `v.into_iter().map(f).collect()`: `v`, which
/// the chain moves, so nothing grows it before the collect reads its
/// capacity. Whether std collects it in place is the collect lowering's
/// (`collects_in_place`); a chain a `let` holds, or a `Vec` made where it's
/// moved, has none.
pub(super) fn in_place_collect_sources<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &Thir<'tcx>,
) -> IndexMap<Span, CollectSource<'tcx>> {
    let iterator = std_item(tcx, StdItem::Iterator);
    let collect = trait_method(tcx, iterator, "collect");
    let into_iter = trait_method(tcx, std_item(tcx, StdItem::IntoIterator), "into_iter");
    let mut found = IndexMap::default();
    for expr in thir.exprs.iter() {
        let ExprKind::Call {
            fun, ref args, fn_span, ..
        } = expr.kind
        else {
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
                    found.insert(fn_span, source);
                }
                break;
            }
            if tcx.trait_of_assoc(callee) != Some(iterator) {
                break;
            }
            at = strip(thir, receiver);
        }
    }
    found
}
