//! A join of MIR to THIR, whose only key rustc gives is a span: the one
//! THIR node of a kind there, or none. Two nodes of one kind may share a
//! span inside a macro expansion, and a join that took either would let a
//! fact of one leave out a check of the other (#1671, #1672). Every join
//! that licenses leaving out a check, a bound's, a wrap's, a cast's or a
//! panic's, asks this alone.

use rustc_middle::mir;
use rustc_middle::thir::{ExprId, LocalVarId, Thir};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// The one call of `thir` whose `fn_span` is `span`, MIR's call there,
/// or none: none there, or more than one, of a macro's expansion.
pub(in crate::lower) fn the_call_at(thir: &Thir<'_>, span: Span) -> Option<ExprId> {
    the_one((thir.exprs.iter_enumerated()).filter_map(|(id, e)| match e.kind {
        rustc_middle::thir::ExprKind::Call { fn_span, .. } if fn_span == span => Some(id),
        _ => None,
    }))
}

/// The one call of `callee` in `thir` whose `fn_span` is `span`, MIR's
/// call of it there, or none: none there, or more than one of it, of a
/// macro's expansion. Of two calls of other functions at one span, the
/// one of `callee`, which MIR's names (#2175).
pub(in crate::lower) fn the_call_of_at(thir: &Thir<'_>, callee: DefId, span: Span) -> Option<ExprId> {
    the_one((thir.exprs.iter_enumerated()).filter_map(|(id, e)| match e.kind {
        rustc_middle::thir::ExprKind::Call { fun, fn_span, .. }
            if fn_span == span && matches!(*thir[fun].ty.kind(), ty::FnDef(id, _) if id == callee) =>
        {
            Some(id)
        }
        _ => None,
    }))
}

/// The MIR local THIR's variable `var` is bound to: the one whole local
/// `mir`'s debug info names at its binding's span, or none (#2158).
pub(in crate::lower) fn var_local(tcx: TyCtxt<'_>, mir: &mir::Body<'_>, var: LocalVarId) -> Option<mir::Local> {
    let span = tcx.hir_span(var.0);
    the_one(mir.var_debug_info.iter().filter_map(|info| match info.value {
        mir::VarDebugInfoContents::Place(place) if place.projection.is_empty() && info.source_info.span == span => {
            Some(place.local)
        }
        _ => None,
    }))
}

/// The MIR local the call of `callee` whose `fn_span` is `span` gives its
/// value to: the one call of it in `mir` there, its destination a whole
/// local; none of none there or more than one. A `for` loop's `into_iter`
/// shares its iterator's call's span, and is another function (#2158).
pub(in crate::lower) fn call_destination(mir: &mir::Body<'_>, callee: DefId, span: Span) -> Option<mir::Local> {
    the_one(
        mir.basic_blocks
            .iter()
            .filter_map(|block| match &block.terminator().kind {
                mir::TerminatorKind::Call {
                    func,
                    fn_span,
                    destination,
                    ..
                } if *fn_span == span && func.const_fn_def().is_some_and(|(id, _)| id == callee) => Some(*destination),
                _ => None,
            }),
    )
    .and_then(|destination| destination.projection.is_empty().then_some(destination.local))
}

/// The MIR local a value written at `span` is made in: the one whole
/// local `mir` assigns there, `_5 = Down(const 2)`, or none (#2158).
pub(in crate::lower) fn assigned_local(mir: &mir::Body<'_>, span: Span) -> Option<mir::Local> {
    the_one(
        mir.basic_blocks
            .iter()
            .flat_map(|block| &block.statements)
            .filter_map(|statement| match &statement.kind {
                mir::StatementKind::Assign(assign)
                    if statement.source_info.span == span && assign.0.projection.is_empty() =>
                {
                    Some(assign.0.local)
                }
                _ => None,
            }),
    )
}

/// Whether a fact `holds` of MIR's call at `span`, whichever THIR call
/// there it is: of each, and there's one. Two a macro's repetition writes
/// share a span, `$(f.call(..);)*`, and of two alike, both, the fact is
/// the call's whichever it is; of two apart, neither's is taken of the
/// other (#1671 change 3 M5). A fact that licenses a form, never one
/// that's of a call alone, which `the_call_at` joins.
pub(in crate::lower) fn each_call_at(thir: &Thir<'_>, span: Span, holds: impl Fn(ExprId) -> bool) -> bool {
    let mut at = (thir.exprs.iter_enumerated()).filter_map(|(id, e)| match e.kind {
        rustc_middle::thir::ExprKind::Call { fn_span, .. } if fn_span == span => Some(id),
        _ => None,
    });
    let Some(first) = at.next() else { return false };
    holds(first) && at.all(holds)
}

/// The one of `found`, or none: none found, or more than one.
pub(in crate::lower) fn the_one<T>(mut found: impl Iterator<Item = T>) -> Option<T> {
    let one = found.next()?;
    found.next().is_none().then_some(one)
}
