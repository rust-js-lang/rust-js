//! `Pin` (ADR 0329): its pointer, as JS never moves a value, so what Rust
//! promises of one is kept already.

use crate::js::{Expr, Stmt, StmtKind};
use crate::lower::calls::{Call, apply};
use crate::lower::recognition::Std;
use crate::lower::{FnCx, R};
use crate::runtime::Helper;
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
            _ => return Err(self.unsupported(span, "a `Pin` of this pointer")),
        };
        // A `&mut` to a pinned `Box` is a box of it, whose `value` is the
        // `Box`, what it points at (ADR 0074).
        let handle = matches!(given.kind(), ty::Ref(_, _, ty::Mutability::Mut)) && self.is_boxable(pin);
        Ok(Some(match op {
            PinOp::Map => {
                let (pin, f) = (arg(), arg());
                apply(f, vec![pin])
            }
            PinOp::Mut if handle && !self.is_boxable(target) => Expr::member(arg(), "value"),
            PinOp::Mut => arg(),
            PinOp::Ref => match self.boxed_target(pointer) {
                true => Expr::member(arg(), "value"),
                false => arg(),
            },
            // An object a `&mut` points at replaced whole in place (ADR 0147),
            // anything else its box's `value`.
            PinOp::Set => {
                let (pin, value) = (arg(), arg());
                let js_span = self.js_span(span);
                match handle || self.is_boxable(target) {
                    true => out.push(StmtKind::Assign(Expr::member(pin, "value"), value).at(js_span)),
                    false => {
                        self.runtime.insert(Helper::Assign);
                        out.push(StmtKind::Expr(Expr::call(Expr::var("$assign"), vec![pin, value])).at(js_span));
                    }
                }
                Expr::undefined()
            }
        }))
    }

    /// Is `pointer`, a `Pin`'s, a `&mut` to a number or text, which is a
    /// box?
    fn boxed_target(&self, pointer: Ty<'tcx>) -> bool {
        matches!(*pointer.kind(), ty::Ref(_, target, ty::Mutability::Mut) if self.is_boxable(target))
    }
}
