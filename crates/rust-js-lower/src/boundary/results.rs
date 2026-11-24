//! What a binding gives back (#2403 step 2c(a), ADR 1195): the crate's own
//! binding's result, checked where its call returns, as Rust takes its
//! word for its type, `$checkResult(width(), "../ts/box.ts#width",
//! $checkU32)`. Not a binding crate's, `js`'s, `webapi`'s, `react`'s,
//! written from WebIDL and TypeScript's own declarations; nor yet a result
//! of text, a `String`'s, a `char`'s, or of what may hold one, whose lone
//! surrogate the DOM may give: the user's decision (ADR 1195's open case).

use rust_js_program::js::Expr;
use rustc_span::def_id::DefId;

use crate::FnCx;
use crate::declarations::contract::Contract;
use crate::signatures::Form;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `made`, the crate's binding `def_id`'s call, its result checked where
    /// it's checked; else `made` as it is.
    pub fn checked_binding_result(&self, def_id: DefId, made: Expr) -> Expr {
        if !def_id.is_local() || self.krate.compiled_once {
            return made;
        }
        let (ty, none) = match self.js_signature(def_id).result {
            Form::Value { ty, .. } => match self.contract(ty) {
                Contract::Option(inner) if !self.boxed_payload(inner) => (inner, Some("nullish")),
                _ => (ty, None),
            },
            Form::Nullable(inner) => (inner, Some("null")),
            Form::Undefinable { inner, .. } => (inner, Some("undefined")),
            _ => return made,
        };
        if !self.plain_result(ty) || (matches!(self.contract(ty), Contract::Bool) && boolean_operator(self.tcx, def_id))
        {
            return made;
        }
        let Some(check) = self.checker(ty) else { return made };
        let mut args = vec![made, Expr::str(crate::bindings::js_name(self.tcx, def_id)), check];
        args.extend(none.map(Expr::str));
        Expr::call(Expr::helper("$checkResult"), args)
    }

    /// Whether a binding's result of `ty` is one checked here: a number, a
    /// `bool`, a fieldless enum, or a shared borrow of one; not text.
    fn plain_result(&self, ty: rustc_middle::ty::Ty<'tcx>) -> bool {
        match self.contract(ty) {
            Contract::Num(_) | Contract::Bool => true,
            Contract::Shared(inner) => self.plain_result(inner),
            Contract::Local(did, _) => {
                self.tcx.def_kind(did) == rustc_hir::def::DefKind::Enum
                    && self.shallow_check(ty).is_some_and(|check| check.names.is_some())
            }
            _ => false,
        }
    }
}

/// Whether binding `def_id`'s JS is an operator JS defines to give a
/// boolean, `entry instanceof Directory`, `key in object`, a `typeof` test:
/// its `bool` can't be another value, so it isn't checked (#2403, ADR 1195).
fn boolean_operator(tcx: rustc_middle::ty::TyCtxt<'_>, def_id: DefId) -> bool {
    use crate::bindings::JsForm;
    matches!(
        crate::bindings::js_form(tcx, def_id),
        JsForm::InstanceOf(_)
            | JsForm::InstanceOfGiven
            | JsForm::In
            | JsForm::Truthy
            | JsForm::StrictEquals
            | JsForm::LooseEquals
            | JsForm::Asks(_)
            | JsForm::Delete
            | JsForm::TypeofObject
            | JsForm::IsWhen { .. }
    )
}
