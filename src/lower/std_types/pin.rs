//! `Pin` (ADR 0329): its pointer, as JS never moves a value, so what Rust
//! promises of one is kept already.

use crate::js::{Expr, Stmt, StmtKind};
use crate::lower::calls::{Call, apply};
use crate::lower::recognition::{PinTarget, Std};
use crate::lower::{FnCx, R};
use rustc_middle::ty::{self, Ty};

/// A `Pin`'s functions, and its `Deref` and `DerefMut`.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum PinOp {
    /// `as_mut()`, `get_mut()` and `DerefMut`: its pointer, which a `&mut`
    /// to what it points at is, a box for a number or text (ADR 0074).
    Mut,
    /// `as_ref()`, `into_ref()` and `Deref`: what it points at, a pinned
    /// `&mut` to a number or text its box's `value`.
    Ref,
    /// `set(value)`: what it points at written.
    Set,
    /// `map_unchecked(f)`, `map_unchecked_mut(f)`: `f` of its pointer.
    Map,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `value`, a `Pin` of type `pin`, as its `==`, `!=`, order and `Hash`
    /// read it, std's `Ptr::Target` (`pinned_target`, #1446, #1435): of
    /// std's pointer, that pointer through its references, which is its
    /// target in JS; of the crate's own, its `deref()`. And the type read.
    pub(in crate::lower) fn pin_target(
        &mut self,
        value: Expr,
        pin: Ty<'tcx>,
        span: rustc_span::Span,
        out: &mut Vec<Stmt>,
    ) -> R<(Expr, Ty<'tcx>)> {
        match self.recognition().pinned_target(pin) {
            Some(PinTarget::Pointer(pointer)) => Ok(self.through_refs(value, pointer)),
            Some(PinTarget::Deref(pointer, target)) => Ok((self.own_deref(value, pointer, span, out)?, target)),
            None => Err(self.unsupported(span, &format!("what a `{pin}` points at"))),
        }
    }

    /// `value.deref()` of the crate's own smart pointer of type `pointer`.
    pub(in crate::lower) fn own_deref(
        &mut self,
        value: Expr,
        pointer: Ty<'tcx>,
        span: rustc_span::Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let deref = self
            .tcx
            .require_lang_item(rustc_hir::attrs::lang_items::LangItem::Deref, span);
        let method = crate::lower::recognition::trait_method(self.tcx, deref, "deref");
        let args = self.tcx.mk_args(&[pointer.into()]);
        match self.trait_call(method, args, vec![value], span, out)? {
            Some(read) => Ok(read),
            None => Err(self.unsupported(span, &format!("the `Deref` of a `{pointer}` a `Pin` reads through"))),
        }
    }

    /// A `Pin`'s call: `None` if `known` is another.
    pub(in crate::lower) fn pin_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Std::Pin(op) = known else {
            return Ok(None);
        };
        let Call { tys, span, .. } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        let given = tys[0];
        let pin = given.peel_refs();
        let pointer = self.recognition().pinned(pin).expect("a `Pin`");
        let target = match *pointer.kind() {
            ty::Ref(_, target, _) => target,
            ty::Adt(_, args) if pointer.is_box() => args.type_at(0),
            // A `ManuallyDrop`, what it holds in JS, as a `Box` is (#1454).
            ty::Adt(..) if let Some(held) = self.recognition().std_wrapped(pointer) => held,
            _ => return self.pin_of_another_call(op, pin, values, span, out).map(Some),
        };
        // A `&mut` to a pinned `Box` is a box of it, whose `value` is the
        // `Box`, what it points at (ADR 0074).
        let handle = matches!(given.kind(), ty::Ref(_, _, ty::Mutability::Mut)) && self.mut_ref_form(pin).is_cell();
        Ok(Some(match op {
            PinOp::Map => {
                let (pin, f) = (arg(), arg());
                apply(f, vec![pin])
            }
            PinOp::Mut if handle && !self.mut_ref_form(target).is_cell() => Expr::held(arg()),
            PinOp::Mut => arg(),
            PinOp::Ref => match self.boxed_target(pointer) {
                true => Expr::held(arg()),
                false => arg(),
            },
            // An object a `&mut` points at replaced whole in place (ADR 0147),
            // anything else its box's `value`.
            PinOp::Set => {
                let (pin, value) = (arg(), arg());
                let js_span = self.js_span(span);
                match handle || self.mut_ref_form(target).is_cell() {
                    true => out.push(StmtKind::Assign(Expr::held(pin), value).at(js_span)),
                    false => {
                        out.push(StmtKind::Expr(Expr::call(Expr::helper("$assign"), vec![pin, value])).at(js_span));
                    }
                }
                Expr::undefined()
            }
        }))
    }

    /// A `Pin`'s call of a pointer other than a reference or a `Box`, an
    /// `Rc`, an `Arc` or the crate's own smart pointer (#1448): `as_ref()`
    /// and its `Deref` what it points at (`pin_read`), `map_unchecked(f)` `f`
    /// of that; `as_mut()` and `set(v)` through the crate's own pointer's
    /// `deref_mut()`, as std's `Pin<Ptr: DerefMut>`'s are.
    fn pin_of_another_call(
        &mut self,
        op: PinOp,
        pin: Ty<'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        span: rustc_span::Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut arg = || values.next().expect("rustc checked the arguments");
        let target = self.recognition().pinned_target(pin);
        Ok(match (op, target) {
            (PinOp::Ref, _) => self.pin_read(arg(), pin, span, out)?,
            (PinOp::Map, _) => {
                let (pin_value, f) = (arg(), arg());
                apply(f, vec![self.pin_read(pin_value, pin, span, out)?])
            }
            (PinOp::Mut, Some(PinTarget::Deref(pointer, _))) => self.own_deref_mut(arg(), pointer, span, out)?,
            (PinOp::Set, Some(PinTarget::Deref(pointer, target))) => {
                let (pin_value, value) = (arg(), arg());
                let place = self.own_deref_mut(pin_value, pointer, span, out)?;
                let js_span = self.js_span(span);
                out.push(match self.mut_ref_form(target).is_cell() {
                    true => StmtKind::Assign(Expr::held(place), value).at(js_span),
                    false => StmtKind::Expr(Expr::call(Expr::helper("$assign"), vec![place, value])).at(js_span),
                });
                Expr::undefined()
            }
            _ => return Err(self.unsupported(span, &format!("`as_mut()` or `set()` of a `{pin}`"))),
        })
    }

    /// What `value`, a `Pin` of type `pin`, points at, as `*pin` reads it:
    /// a counted `Rc`'s `value` (ADR 0320), another std pointer itself,
    /// which is its target in JS, or the crate's own pointer's `deref()`.
    fn pin_read(&mut self, value: Expr, pin: Ty<'tcx>, span: rustc_span::Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let (read, read_ty) = self.pin_target(value, pin, span, out)?;
        Ok(match self.counted_rc(read_ty) {
            Some(pointee) => self.counted_pointee(read, pointee),
            None => read,
        })
    }

    /// `value.deref_mut()` of the crate's own smart pointer of type `pointer`.
    fn own_deref_mut(
        &mut self,
        value: Expr,
        pointer: Ty<'tcx>,
        span: rustc_span::Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let deref_mut = self
            .tcx
            .require_lang_item(rustc_hir::attrs::lang_items::LangItem::DerefMut, span);
        let method = crate::lower::recognition::trait_method(self.tcx, deref_mut, "deref_mut");
        let args = self.tcx.mk_args(&[pointer.into()]);
        match self.trait_call(method, args, vec![value], span, out)? {
            Some(place) => Ok(place),
            None => Err(self.unsupported(span, &format!("the `DerefMut` of a `{pointer}` a `Pin` writes through"))),
        }
    }

    /// Is `pointer`, a `Pin`'s, a `&mut` to a number or text, which is a
    /// box?
    fn boxed_target(&self, pointer: Ty<'tcx>) -> bool {
        matches!(*pointer.kind(), ty::Ref(_, target, ty::Mutability::Mut) if self.mut_ref_form(target).is_cell())
    }
}
