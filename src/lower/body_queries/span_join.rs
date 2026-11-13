//! A join of MIR to THIR, whose only key rustc gives is a span: the one
//! THIR node of a kind there, or none. Two nodes of one kind may share a
//! span inside a macro expansion, and a join that took either would let a
//! fact of one leave out a check of the other (#1671, #1672). Every join
//! that licenses leaving out a check, a bound's, a wrap's, a cast's or a
//! panic's, asks this alone.

use rustc_middle::thir::{ExprId, Thir};
use rustc_span::Span;

/// The one expression of `thir` written at `span` that `of_kind` takes,
/// or none: none there, or more than one.
pub(in crate::lower) fn the_one_at<'tcx>(
    thir: &Thir<'tcx>,
    span: Span,
    of_kind: impl Fn(&rustc_middle::thir::Expr<'tcx>) -> bool,
) -> Option<ExprId> {
    the_one(
        (thir.exprs.iter_enumerated())
            .filter(|(_, e)| e.span == span && of_kind(e))
            .map(|(id, _)| id),
    )
}

/// The one of `found`, or none: none found, or more than one.
pub(in crate::lower) fn the_one<T>(mut found: impl Iterator<Item = T>) -> Option<T> {
    let one = found.next()?;
    found.next().is_none().then_some(one)
}
