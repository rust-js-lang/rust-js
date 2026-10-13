//! Read, borrow and write places. Prepared targets lower each operand once.

use super::{FnCx, Std};
use crate::js::{self, Expr};

use rustc_middle::thir::{ExprId, ExprKind};
use rustc_middle::ty;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `p` of `*f(&mut p)`, where `f` gives back what it's given, as a
    /// `String`'s `deref_mut` gives its `&mut str` (ADR 0334): the place
    /// it writes.
    pub(super) fn through_same(&self, mut place: ExprId) -> ExprId {
        while let ExprKind::Deref { arg } = self.thir[self.strip(place)].kind
            && let ExprKind::Call { fun, ref args, .. } = self.thir[self.strip(arg)].kind
            && self.std_fn(fun) == Some(Std::Same)
            && let Some(&given) = args.first()
            && let ExprKind::Borrow {
                borrow_kind: rustc_middle::mir::BorrowKind::Mut { .. },
                arg: inner,
            } = self.thir[self.strip(given)].kind
        {
            place = inner;
        }
        place
    }

    /// What's kept for good, `s.leak()`, as the `&'static mut` it gives:
    /// a box of it where that's a cell, `{ value: s }` (ADR 0334).
    pub(super) fn leaked(&self, value: Expr, output: ty::Ty<'tcx>) -> Expr {
        match self.is_cell(output) {
            true => Expr::object(vec![js::Prop::Field("value".into(), value)]),
            false => value,
        }
    }

    /// Is `index` a constant below the length of the array `lhs`'s type?
    pub(super) fn in_bounds(&self, lhs: ExprId, index: &Expr) -> bool {
        let ty::Array(_, len) = self.thir[lhs].ty.peel_refs().kind() else {
            return false;
        };
        let (Some(len), Some(i)) = (len.try_to_target_usize(self.tcx), index.as_int()) else {
            return false;
        };
        (0..i128::from(len)).contains(&i)
    }
}
