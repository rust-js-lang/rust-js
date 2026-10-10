//! `MaybeUninit`, and a `Box` made before its value (ADR 0332): what it
//! holds, or `undefined` before it's written, as JS has no memory that isn't
//! a value. A `&mut` to one is a box, the place it's written to.

use crate::js::{self, Expr, Prop, Stmt, StmtKind};
use crate::lower::calls::Call;
use crate::lower::recognition::Std;
use crate::lower::representation::Num;
use crate::lower::{FnCx, R};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

/// What's asked of a `MaybeUninit` or a `Box` of one.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum UninitOp {
    /// `MaybeUninit::uninit()`, `Box::new_uninit()`: nothing yet.
    Uninit,
    /// `MaybeUninit::zeroed()`, `Box::new_zeroed()`: its type's zero.
    Zeroed,
    /// `Box::new_uninit_slice(n)`, `new_zeroed_slice(n)`.
    Slice { zeroed: bool },
    /// `Box::write(b, v)`: `v`.
    BoxWrite,
    /// A `MaybeUninit`'s `write(v)`: written, and a `&mut` to it given.
    Write,
    /// `assume_init_mut()`: a `&mut` to what it holds.
    InitMut,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `MaybeUninit`'s call, or a `Box`'s of one: `None` if `known` is another.
    pub(in crate::lower) fn uninit_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Std::Uninit(op) = known else {
            return Ok(None);
        };
        let Call {
            generic_args,
            args,
            span,
            ..
        } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        // What it holds: the function's type argument, or a `Box`'s item.
        let item = generic_args.types().next().expect("a type argument");
        Ok(Some(match op {
            UninitOp::Uninit => Expr::undefined(),
            UninitOp::Zeroed => self.zero(item, span)?,
            // `Array.from({ length: n })`; of a zero, `new Array(n).fill(0)`,
            // or an object made for each.
            UninitOp::Slice { zeroed } => {
                let item = match item.kind() {
                    ty::Slice(item) => *item,
                    _ => item,
                };
                let n = arg();
                let zero = if zeroed { Some(self.zero(item, span)?) } else { None };
                if let Some(zero) = zero.as_ref().filter(|zero| zero.is_constant()) {
                    let made = Expr::new_(Expr::var("Array"), vec![n]);
                    return Ok(Some(Expr::call(Expr::member(made, "fill"), vec![zero.clone()])));
                }
                let mut given = vec![Expr::object(vec![Prop::Field("length".into(), n)])];
                given.extend(
                    zero.map(|zero| Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(zero)).at(js::Span::NONE)])),
                );
                Expr::call(Expr::member(Expr::var("Array"), "from"), given)
            }
            UninitOp::BoxWrite => {
                let boxed = arg();
                if boxed.has_effects() {
                    out.push(StmtKind::Expr(boxed).at(self.js_span(span)));
                }
                arg()
            }
            // Written through its box; a `&mut` to a number or text is the box,
            // to an object the object (ADR 0074).
            UninitOp::Write | UninitOp::InitMut => {
                let slot = arg();
                // A handle on a place is the place itself, written as it is.
                let place = match slot.kind {
                    js::ExprKind::Handle(place) => *place,
                    kind => {
                        let slot = Expr { kind, ..slot };
                        let slot = if slot.reads_same() {
                            slot
                        } else {
                            self.spill("slot", slot, out)
                        };
                        Expr::member(slot, "value")
                    }
                };
                if op == UninitOp::Write {
                    let value = arg();
                    out.push(StmtKind::Assign(place.clone(), value).at(self.js_span(span)));
                }
                let target = self.thir[args[0]].ty.peel_refs();
                let held = self.recognition().uninit_of(target).unwrap_or(target);
                match self.is_boxable(held) {
                    true => Expr::handle(place),
                    false => place,
                }
            }
        }))
    }

    /// A `ty` of all zero bytes, where that's one: a number's `0`, `false`,
    /// `'\0'`, and a tuple or an array of them.
    fn zero(&mut self, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        if let Some(num) = Num::of(ty) {
            return Ok(match num.big() {
                true => Expr::bigint(0),
                false => Expr::int(0),
            });
        }
        match ty.kind() {
            ty::Bool => Ok(Expr::bool(false)),
            ty::Char => Ok(Expr::str("\0")),
            ty::Tuple(items) if items.is_empty() => Ok(Expr::undefined()),
            ty::Tuple(items) => Ok(Expr::array(
                items.iter().map(|t| self.zero(t, span)).collect::<R<Vec<_>>>()?,
            )),
            ty::Array(item, len) => {
                let len = len
                    .try_to_target_usize(self.tcx)
                    .ok_or_else(|| self.unsupported(span, "a zeroed array of a generic length"))?;
                let zero = self.zero(*item, span)?;
                Ok(Expr::array((0..len).map(|_| zero.clone()).collect()))
            }
            _ => Err(self.unsupported(span, &format!("a zeroed `{ty}`, which may be no value of it"))),
        }
    }
}
