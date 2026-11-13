//! A write in place, into the object a name holds (ADR 0147): the one way
//! one is made, each asking the copy policy whether the written type's
//! copies are left out (#1665, #1672).

use super::super::refusal::{Refused, refuse};
use super::super::{FnCx, R};
use crate::js;
use crate::js::Expr;
use rustc_middle::ty::Ty;
use rustc_span::Span;

/// What writes a value in place: `$assign(place, value)`, `$take(place,
/// value)`, `$exchange(a, b)`, or a cell on an object, `$cellOf(object)`,
/// whose `value` written writes the object.
#[derive(Clone, Copy, Debug)]
pub(in crate::lower) enum InPlaceWriter {
    Assign,
    Take,
    Exchange,
    CellOf,
}

impl InPlaceWriter {
    fn helper(self) -> &'static str {
        match self {
            InPlaceWriter::Assign => "$assign",
            InPlaceWriter::Take => "$take",
            InPlaceWriter::Exchange => "$exchange",
            InPlaceWriter::CellOf => "$cellOf",
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `writer`'s call of `args`, writing a `written` in place. Where
    /// `written` is a `Copy` type whose copies the copy policy leaves out,
    /// as nothing it knows changes one in place, a write in place would
    /// change every name sharing the value, `a` of `let mut b = a`: refused,
    /// in release too, as rust-js's own error, never a wrong answer.
    pub(in crate::lower) fn in_place(
        &self,
        writer: InPlaceWriter,
        written: Ty<'tcx>,
        args: Vec<Expr>,
        span: Span,
    ) -> R<Expr> {
        if self.copies_left_out(written) {
            let what = format!("writing a `{written}` in place, `{}`", writer.helper());
            let why = "rust-js leaves out its copies, as it doesn't know the type is changed in place: an internal error (#1665)";
            return Err(refuse(self.tcx, span, Refused::Because(&what, why)));
        }
        Ok(Expr::call_helper(writer.helper(), args))
    }

    /// A `&mut dyn`'s pair of `object`, a `written`, whose write of the
    /// whole changes it in place, `$assign` (`Storage::Assigned`, #968):
    /// refused as `in_place` refuses.
    pub(in crate::lower) fn in_place_pair(
        &self,
        object: Expr,
        dictionary: Expr,
        written: Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        self.in_place(InPlaceWriter::Assign, written, Vec::new(), span)?;
        Ok(Expr::stored_pair(object, dictionary, js::Storage::Assigned))
    }

    /// Whether a value of `ty` is shared where Rust copies it: a `Copy`
    /// type nothing is known to change in place, whose copy is a value of
    /// its own, an object or an array, not the value itself, a number's.
    fn copies_left_out(&self, ty: Ty<'tcx>) -> bool {
        let value = Expr::var("value");
        self.is_copy(ty) && !self.contains_mutated(ty) && !js::same_path(&self.copy(value.clone(), ty), &value)
    }
}
