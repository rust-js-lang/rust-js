//! A table's lengths (ADR 0576, amended): what `vec![item; n]` is as long
//! as (#362), how long at least (#367), and its rows', of a table of
//! tables, `board[r]` (#370); and an integer's `max` and `min` (#367).

use super::known::{ITEM, Known, Range};
use super::{Slice, Walk};
use crate::lower::body_queries::strip;
use crate::lower::fn_def;
use crate::lower::recognition::{SliceLength, is_vec_from_elem, slice_length, vec_view};
use rustc_middle::mir::Mutability;
use rustc_middle::thir::{ExprId, ExprKind};
use rustc_middle::ty::{self, Ty};

/// The range of the larger of `a` and `b`, or the smaller: below a bound
/// both are below, or, the smaller, either's (#367).
pub(super) fn chosen(a: &Range, b: &Range, larger: bool) -> Range {
    match larger {
        true => Range {
            lo: a.lo.max(b.lo),
            hi: a.hi.max(b.hi),
            below: match (&a.below, &b.below) {
                (Some((s, x)), Some((t, y))) if s == t => Some((s.clone(), x.join(*y))),
                _ => None,
            },
        },
        false => Range {
            lo: a.lo.min(b.lo),
            hi: a.hi.min(b.hi),
            below: a.below.clone().or_else(|| b.below.clone()),
        },
    }
}

impl<'a, 'tcx> Walk<'a, 'tcx> {
    /// The item and the count of `init` where it's a table, `vec![item; n]`.
    pub(super) fn table(&self, init: ExprId) -> Option<(ExprId, ExprId)> {
        let thir = self.thir;
        let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, init)].kind else {
            return None;
        };
        let (def_id, _) = fn_def(thir[strip(thir, fun)].ty)?;
        let [item, count] = args[..] else { return None };
        is_vec_from_elem(self.tcx, def_id).then_some((item, count))
    }

    /// What's known once `place` may change length, of a table's row,
    /// `board[r]`: none of its table's rows is known as long as anything.
    pub(super) fn row_reached(&self, place: ExprId, known: &mut Option<Known>) {
        if let (Some((_, var)), Some(k)) = (self.row_of(place), known.as_mut()) {
            k.rows_changed(var);
        }
    }

    /// Whether a table lent as a view of its rows, `board.iter_mut()`'s
    /// `&mut *board`, keeps its rows' lengths: where nothing the body
    /// does can resize a row of its type, no call but a view's, an index's
    /// or a length's given a `&mut` of one, no closure holding one, and no
    /// row assigned whole (#372).
    pub(super) fn rows_kept(&self, table: ExprId) -> bool {
        let thir = self.thir;
        let ty::Adt(_, args) = thir[strip(thir, table)].ty.peel_refs().kind() else {
            return false;
        };
        let Some(row) = args.types().next() else { return false };
        let is_row = |t: Ty<'tcx>| t == row;
        let lent_row = |t: Ty<'tcx>| matches!(t.kind(), ty::Ref(_, of, Mutability::Mut) if is_row(*of));
        let safe = |fun: ExprId| {
            fn_def(thir[strip(thir, fun)].ty).is_some_and(|(def_id, generic_args)| {
                vec_view(self.tcx, def_id, generic_args).is_some()
                    || matches!(
                        slice_length(self.tcx, def_id, generic_args),
                        Some(SliceLength::Len | SliceLength::IsEmpty | SliceLength::Index)
                    )
            })
        };
        !(thir.exprs.iter()).any(|e| match &e.kind {
            ExprKind::Call { fun, args, .. } => args.iter().any(|&arg| lent_row(thir[arg].ty)) && !safe(*fun),
            ExprKind::Closure(closure) => (closure.upvars.iter()).any(|&upvar| {
                thir[upvar]
                    .ty
                    .walk()
                    .filter_map(|part| part.as_type())
                    .any(|t| lent_row(t) || is_row(t))
            }),
            ExprKind::Assign { lhs, .. } => is_row(thir[*lhs].ty),
            _ => false,
        })
    }

    /// The rows of a table `items` is one of, `board[r]`: `([ITEM], board)`,
    /// which only a table of rows each as long is known by (#370).
    pub(super) fn row_of(&self, items: ExprId) -> Option<Slice> {
        let thir = self.thir;
        let mut e = strip(thir, items);
        while let ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } = thir[e].kind {
            e = strip(thir, arg);
        }
        let ExprKind::Call { fun, ref args, .. } = thir[e].kind else {
            return None;
        };
        let (def_id, generic_args) = fn_def(thir[strip(thir, fun)].ty)?;
        if slice_length(self.tcx, def_id, generic_args) != Some(SliceLength::Index) {
            return None;
        }
        let (mut path, var) = self.slice(*args.first()?)?;
        path.push(ITEM);
        Some((path, var))
    }

    /// What a table of `count` items is as long as: the slice `count` is the
    /// length of, `terraces` of `terraces.len()`, or `count` itself, an
    /// integer variable (`VALUE`) (#362).
    pub(super) fn count_base(&self, count: ExprId) -> Option<Slice> {
        match self.asks(count) {
            Some((SliceLength::Len, items)) => self.slice(items),
            _ => self.value_base(count),
        }
    }
}
