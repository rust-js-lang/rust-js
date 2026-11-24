//! The callbacks JS gives (#2403 step 2c(a), ADR 1195): a parameter of a
//! function JS gives, an `impl Fn(..) -> R`, a `dyn Fn` or a `fn` pointer,
//! is a function, and where what it gives back is checked, it's wrapped
//! at entry, so each call's result is checked where it returns:
//!
//! ```js
//! rule = $checkCallback(rule, "discounted(rule)", $checkU32);
//! ```
//!
//! One whose result Rust ignores, `()`, or that may be any value, a JS
//! type's, is checked a function and given as it is: a wrapper is another
//! function, which JS can tell apart where Rust gives it back.

use rust_js_program::js::Expr;
use rustc_middle::ty::{self, Ty};

use crate::FnCx;
use crate::declarations::contract::Contract;
use crate::signatures::{Form, JsSignature};

/// A callback parameter's check: the checker of what it gives back, where
/// that's checked.
pub(super) struct Callback {
    pub(super) output: Option<Expr>,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Of a parameter of `form`, of a function of `signature`, its check as
    /// a callback; none of one that isn't a function JS gives.
    pub(super) fn callback(&self, signature: &JsSignature<'tcx>, form: Form<'tcx>) -> Option<Callback> {
        let Form::Value { ty, .. } = form else { return None };
        self.callback_of(signature, ty)
    }

    /// Of a value of `ty`, of a function of `signature`, its check as a
    /// callback: a parameter's, or a part a pattern binds of one.
    pub(super) fn callback_of(&self, signature: &JsSignature<'tcx>, ty: Ty<'tcx>) -> Option<Callback> {
        let output = self.callback_output(signature, ty)?;
        Some(Callback {
            output: self.result_checker(output),
        })
    }

    /// What a function of type `ty` gives back: a type parameter's the
    /// signature takes a JS function of, a `dyn Fn`'s, a `fn` pointer's.
    fn callback_output(&self, signature: &JsSignature<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        if let Contract::Param(param) = self.contract(ty) {
            return (signature.callables.iter())
                .find(|callable| callable.index == param.index)
                .map(|callable| callable.output);
        }
        match ty.kind() {
            ty::Ref(_, inner, _) => self.callback_output(signature, *inner),
            ty::FnPtr(..) => Some(
                self.tcx
                    .instantiate_bound_regions_with_erased(ty.fn_sig(self.tcx))
                    .output(),
            ),
            _ => match self.contract(ty) {
                Contract::DynFn { output, .. } => Some(output),
                Contract::Pointee(inner) => self.callback_output(signature, inner),
                _ => None,
            },
        }
    }

    /// The checker of a callback's result: none of `()` or `!`, which Rust
    /// ignores, nor of what may be any value.
    fn result_checker(&self, output: Ty<'tcx>) -> Option<Expr> {
        match self.contract(output) {
            Contract::Unit | Contract::Never => None,
            _ => self.checker(output),
        }
    }
}
