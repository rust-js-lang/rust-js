//! Read, borrow and write places. Prepared targets lower each operand once.

use super::FnCx;
use crate::js::{self, Expr};

use rustc_middle::ty;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What's kept for good, `s.leak()`, as the `&'static mut` it gives:
    /// a box of it where that's a cell, `{ value: s }` (ADR 0334).
    pub(super) fn leaked(&self, value: Expr, output: ty::Ty<'tcx>) -> Expr {
        match self.is_cell(output) {
            true => Expr::object(vec![js::Prop::Field("value".into(), value)]),
            false => value,
        }
    }
}
