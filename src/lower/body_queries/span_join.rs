//! A join of MIR to THIR, whose only key rustc gives is a span: the one
//! THIR node of a kind there, or none. Two nodes of one kind may share a
//! span inside a macro expansion, and a join that took either would let a
//! fact of one leave out a check of the other (#1671, #1672). Every join
//! that licenses leaving out a check, a bound's, a wrap's, a cast's or a
//! panic's, asks this alone.

use rustc_middle::thir::{ExprId, Thir};
use rustc_span::Span;

/// The one call of `thir` whose `fn_span` is `span`, MIR's call there,
/// or none: none there, or more than one, of a macro's expansion.
pub(in crate::lower) fn the_call_at(thir: &Thir<'_>, span: Span) -> Option<ExprId> {
    the_one((thir.exprs.iter_enumerated()).filter_map(|(id, e)| match e.kind {
        rustc_middle::thir::ExprKind::Call { fn_span, .. } if fn_span == span => Some(id),
        _ => None,
    }))
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
