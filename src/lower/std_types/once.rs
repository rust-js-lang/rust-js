//! `OnceCell` and `OnceLock` (ADR 0317): a `{ value }` of an `Option`, as
//! a `Cell<Option<T>>` is, set once; in a field, the property, absent until
//! it's set, and set where it's known empty by an assignment (ADR 0847).

use crate::js::{Expr, Op, Prop, Stmt, StmtKind, result};
use crate::lower::calls::Call;
use crate::lower::recognition::Std;
use crate::lower::{FnCx, R};

/// A `OnceCell`'s or `OnceLock`'s own methods.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum OnceOp {
    New,
    /// `get()` and `into_inner()`: the `Option` it holds.
    Get,
    GetMut,
    Set,
    GetOrInit,
    Take,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `OnceCell`'s or `OnceLock`'s call: `None` if `known` is another.
    pub(in crate::lower) fn once_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Std::Once(op) = known else {
            return Ok(None);
        };
        let Call {
            generic_args,
            tys,
            span,
            ..
        } = call;
        let item = generic_args.type_at(0);
        let mut arg = || values.next().expect("rustc checked the arguments");
        Ok(Some(match op {
            OnceOp::New => Expr::object(vec![Prop::Field(crate::js::HELD.into(), Expr::undefined())]),
            OnceOp::Get => Expr::held(arg()),
            // A `&mut` to an object is the object; to a number or text, a
            // `{ value }` box (ADR 0074), which the cell itself is once set.
            OnceOp::GetMut if self.boxed_payload(item) => {
                return Err(self.unsupported(span, &format!("`get_mut` of a `OnceCell` of `{item}`")));
            }
            OnceOp::GetMut if self.is_boxable(item) => {
                let cell = arg();
                let cell = if cell.reads_same() {
                    cell
                } else {
                    self.spill("cell", cell, out)
                };
                Expr::cond(
                    Expr::bin(Op::LooseEq, Expr::held(cell.clone()), Expr::null()),
                    Expr::undefined(),
                    cell,
                )
            }
            OnceOp::GetMut => Expr::held(arg()),
            // Lowered by MIR's call, which knows whether it's of a cell still
            // empty and whether its `Result` is read (`once_set`); else
            // neither.
            OnceOp::Set => {
                let (cell, value) = (arg(), arg());
                self.once_set((cell, value), item, (false, true), span, out)
            }
            // A `OnceLock`'s that sets it deadlocks in Rust: rust-js's error.
            OnceOp::GetOrInit if self.recognition().is_lock_cell(tys[0].peel_refs()) => {
                Expr::call(Expr::helper("$getOrInitLock"), vec![arg(), arg()])
            }
            OnceOp::GetOrInit => Expr::call(Expr::helper("$getOrInit"), vec![arg(), arg()]),
            // What it held, and empty again, as a `Cell`'s `take` is:
            // `const old = cell.value; cell.value = undefined;`.
            OnceOp::Take => {
                let place = self.fixed_place_of(Expr::held(arg()), out);
                let old = self.spill("old", place.clone(), out);
                out.push(StmtKind::Assign(place, Expr::undefined()).at(self.js_span(span)));
                old
            }
        }))
    }

    /// `cell.set(value)` of a `OnceCell` of `item`: what's set is `Some`
    /// of it, boxed where it looks like `None` (ADR 0051). Where the
    /// body's ranges show the cell still empty, `empty`, it's assigned,
    /// `payload._ioInfo = ioInfo`, and `Ok`; else where it's empty, `if
    /// (payload._ioInfo === undefined) { .. }`, and where its `Result` is
    /// `read`, `Err` of it where it wasn't (ADR 0847).
    pub(in crate::lower) fn once_set(
        &mut self,
        (cell, value): (Expr, Expr),
        item: rustc_middle::ty::Ty<'tcx>,
        (empty, read): (bool, bool),
        span: rustc_span::Span,
        out: &mut Vec<Stmt>,
    ) -> Expr {
        let js_span = self.js_span(span);
        let place = self.fixed_place_of(Expr::held(cell), out);
        if empty {
            let stored = self.once_stored(value, item);
            out.push(StmtKind::Assign(place, stored).at(js_span));
            return result::ok(Expr::undefined());
        }
        // Made before it's tested, as Rust makes it before the call.
        let value = match value.reads_same() || (!read && !value.has_effects()) {
            true => value,
            false => self.spill("value", value, out),
        };
        let unset = Expr::bin(Op::Eq, place.clone(), Expr::undefined());
        let unset = match read {
            true => self.spill("unset", unset, out),
            false => unset,
        };
        let stored = self.once_stored(value.clone(), item);
        let set = vec![StmtKind::Assign(place, stored).at(js_span)];
        out.push(StmtKind::If(unset.clone(), set, None).at(js_span));
        match read {
            true => Expr::cond(unset, result::ok(Expr::undefined()), result::err(value)),
            false => Expr::undefined(),
        }
    }

    /// What a `OnceCell` of `item` holds of `value` set: `Some` of it, as
    /// `$some` boxes it where it looks like `None` (ADR 0051), so a cell
    /// set is never `undefined`.
    fn once_stored(&mut self, value: Expr, item: rustc_middle::ty::Ty<'tcx>) -> Expr {
        match self.boxed_payload(item) {
            true => self.some(value),
            false => value,
        }
    }
}
