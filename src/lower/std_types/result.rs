//! `?` of an `Option` or a `Result`, and the `From` that converts its
//! error (ADRs 0052, 0141). A `Result`'s JS itself is `js::result`'s.

use crate::js::result::{err, held, is};
use crate::js::{self, Expr, Prop, StmtKind};
use crate::lower::recognition::{StdItem, std_item};
use crate::lower::{FnCx, R};
use rustc_middle::ty::{self, Ty};

/// How `?` makes an error the function's: a `From` of the crate's own,
/// called on it, or for a `Box<dyn Error>`, its dictionary (ADR 0141).
pub(in crate::lower) enum Conversion {
    From(Expr),
    Dyn(Expr),
}

impl Conversion {
    /// `error`, converted.
    pub(in crate::lower) fn of(self, error: Expr) -> Expr {
        match self {
            Conversion::From(from) => Expr::call(from, vec![error]),
            Conversion::Dyn(dictionary) => Expr::object(vec![
                Prop::Field(js::HELD.into(), error),
                Prop::Field(js::DICTIONARY.into(), dictionary),
            ]),
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `?` of `subject`, an `Option` or a `Result`, returns early:
    /// it's `None`, `o == null` (ADR 0298, amended), or an `Err`.
    pub(in crate::lower) fn tried_failed(&self, subject: Expr, ty: Ty<'tcx>) -> Expr {
        match self.option_of(ty) {
            Some(_) => super::option::absent(subject),
            None => is(subject, false),
        }
    }

    /// What `?` of `subject` gives when it doesn't return: what's inside.
    pub(in crate::lower) fn tried_value(&mut self, subject: Expr, ty: Ty<'tcx>) -> Expr {
        match self.option_of(ty) {
            Some(inner) if self.boxed_payload(inner) => self.some_value(subject),
            Some(_) => subject,
            None => held(subject),
        }
    }

    /// What `?` of `subject` returns: `undefined` for a `None`; the `Err`
    /// as it is, or `converted`, its error made the function's.
    pub(in crate::lower) fn tried_returned(&self, subject: Expr, ty: Ty<'tcx>, converted: Option<Expr>) -> Expr {
        if self.option_of(ty).is_some() {
            return Expr::undefined();
        }
        match converted {
            Some(converted) => err(converted),
            None => subject,
        }
    }

    /// What converts the error of `?` of a `Result` `ty` to the function's,
    /// `returned`'s, given `subject`'s `_0`: nothing if it's the same, or a
    /// `&str` to a `String`, the same JS string; a `From` of the crate's own;
    /// or for a `Box<dyn Error>`, the error and its dictionary (ADR 0141).
    pub(in crate::lower) fn tried_conversion(
        &mut self,
        ty: Ty<'tcx>,
        returned: Option<Ty<'tcx>>,
        span: rustc_span::Span,
    ) -> R<Option<Conversion>> {
        if self.option_of(ty).is_some() {
            return Ok(None);
        }
        let error = |t: Ty<'tcx>| match t.kind() {
            ty::Adt(_, args) => args.types().nth(1),
            _ => None,
        };
        let (to, from_ty) = (returned.and_then(error), error(ty));
        let same_string = to
            .zip(from_ty)
            .is_some_and(|(to, from_ty)| self.is_string_like(to) && self.is_string_like(from_ty));
        if to == from_ty || same_string {
            return Ok(None);
        }
        let (Some(to), Some(from_ty)) = (to, from_ty) else {
            return Err(self.unsupported(span, "this `?`"));
        };
        if let Some(dictionary) = self.dyn_error_from(to, from_ty, span)? {
            return Ok(Some(Conversion::Dyn(dictionary)));
        }
        let from = self
            .error_from(to, from_ty)?
            .ok_or_else(|| self.unsupported(span, "`?` that converts the error with this `From`"))?;
        Ok(Some(Conversion::From(from)))
    }

    /// The function `?` converts an error with, `<to as From<from>>::from`,
    /// if it's one of the crate's own (ADR 0052).
    pub(in crate::lower) fn error_from(&mut self, to: Ty<'tcx>, from: Ty<'tcx>) -> R<Option<Expr>> {
        let from_trait = std_item(self.tcx, StdItem::From);
        let method = self.tcx.associated_item_def_ids(from_trait)[0];
        let args = self.tcx.mk_args(&[to.into(), from.into()]);
        let Some(instance) = self.resolve_instance(method, args)? else {
            return Ok(None);
        };
        if !self.krate.fns.contains_key(&instance.def_id()) {
            return Ok(None);
        }
        let evidence = self.evidence_args(instance.def_id(), instance.args, self.tcx.def_span(instance.def_id()))?;
        let callee = self.fn_ref(instance.def_id());
        Ok(Some(if evidence.is_empty() {
            callee
        } else {
            let mut values = vec![Expr::var("error")];
            values.extend(evidence);
            Expr::arrow(
                vec!["error".into()],
                vec![StmtKind::Return(Some(Expr::call(callee, values))).at(js::Span::NONE)],
            )
        }))
    }
}
