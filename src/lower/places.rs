//! A place leaked, `Box::leak` and its kin (`leaked`): what it's given
//! is the value, kept for good.

use super::FnCx;
use crate::js::{self, Expr};

use rustc_middle::ty;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What's kept for good, `s.leak()`, as the `&'static mut` it gives:
    /// a box of it where that's a cell, `{ value: s }` (ADR 0334).
    pub(super) fn leaked(&self, value: Expr, output: ty::Ty<'tcx>) -> Expr {
        match self.is_cell(output) {
            true => Expr::object(vec![js::Prop::Field(js::HELD.into(), value)]),
            false => value,
        }
    }
}
