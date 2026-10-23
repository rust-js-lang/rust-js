//! `std::future`'s functions (ADRs 0029, 0571): a future is a JS promise, so
//! `ready(v)`, a future already done, is `Promise.resolve(v)`.

use crate::js::Expr;
use crate::lower::FnCx;
use crate::lower::recognition::{Std, StdItem};
use rustc_middle::ty::{self, Ty};

/// What's asked of `std::future`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum FutureOp {
    /// `future::ready(v)`: `Promise.resolve(v)`.
    Ready,
}

/// Which of `std::future`'s functions `path` is, as std names it.
pub(in crate::lower) fn future_op(path: &str) -> Option<FutureOp> {
    match path {
        "std::future::ready" => Some(FutureOp::Ready),
        _ => None,
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `std::future` function's call: `None` if `known` is another.
    /// `tys`, its arguments' types: what's resolved is settled where a
    /// promise would adopt it (`may_be_thenable`).
    pub(in crate::lower) fn future_call(
        &mut self,
        known: Std,
        values: &mut std::vec::IntoIter<Expr>,
        tys: &[Ty<'tcx>],
    ) -> Option<Expr> {
        let Std::Future(op) = known else {
            return None;
        };
        Some(match op {
            FutureOp::Ready => {
                let value = values.next().expect("its value");
                Expr::call(
                    Expr::member(Expr::var("Promise"), "resolve"),
                    vec![self.settled(value, tys[0])],
                )
            }
        })
    }

    /// Whether a future's output of type `ty` may be a JS thenable, which
    /// a promise resolved with it would adopt (#172): a future itself, a
    /// promise, or what holds one as its value, an `Option`'s, a `Box`'s;
    /// a struct with a function named `then`; or a type only a caller
    /// knows. What resolves one gives `$settled(value)`, and its `.await`
    /// takes `$unsettled(..)` of it: the one answer, of both.
    pub(in crate::lower) fn may_be_thenable(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        if self.is_unknown(ty) || self.recognition().is_dyn_future(ty) {
            return true;
        }
        match ty.kind() {
            ty::Coroutine(..) => true,
            // An `impl Future` that stays one, a future.
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Opaque { .. },
                    ..
                },
            ) => true,
            ty::Adt(..) if let Some(inner) = self.option_of(ty) => self.may_be_thenable(inner),
            ty::Adt(_, args) if ty.is_box() || self.is_std_type(ty, StdItem::Rc) => {
                self.may_be_thenable(args.type_at(0))
            }
            ty::Adt(..) if let Some(pointer) = self.recognition().pinned(ty) => self.may_be_thenable(pointer),
            // A function, or an `Option` or a `Box` of one, each the function
            // in JS, or what may be one.
            ty::Adt(adt, args) if adt.is_struct() => adt.non_enum_variant().fields.iter().any(|field| {
                let held = self.field_ty(field, args);
                let mut held = self.option_of(held).unwrap_or(held);
                // A `Box<dyn Fn>`'s is the function.
                while let ty::Adt(_, inner) = held.kind()
                    && (held.is_box() || self.is_std_type(held, StdItem::Rc))
                {
                    held = inner.type_at(0);
                }
                field.name.as_str() == "then" && (self.is_callable(held) || self.is_unknown(held))
            }),
            _ => false,
        }
    }

    /// `value`, a future's output of type `ty`, as what resolves the
    /// promise gives it: `$settled(value)` where it may be a thenable.
    pub(in crate::lower) fn settled(&self, value: Expr, ty: Ty<'tcx>) -> Expr {
        match self.may_be_thenable(ty) {
            true => Expr::call_helper("$settled", vec![value]),
            false => value,
        }
    }
}
