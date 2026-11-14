//! `LazyCell` and `LazyLock` (ADR 0318): `{ init }`, whose `value` its
//! `init` makes the first time it's used, as Rust's does.

use crate::js::{Expr, Op, Prop, Stmt, StmtKind};
use crate::lower::calls::Call;
use crate::lower::recognition::Lend;
use crate::lower::recognition::{Std, StdItem};
use crate::lower::temporary_names::Role;
use crate::lower::{FnCx, R};
use rustc_middle::ty;

/// The property of a `LazyCell`'s or `LazyLock`'s object that makes its
/// value: its `init` until the value's made, `null` while that runs or once
/// it panicked, `undefined` once it's made; the value then the object's
/// held value, `value` (`js::HELD`). The one spelling of the cell's object,
/// which its lowering, its `Debug`, its `Default` and its declaration read
/// (#1752); the runtime's `$force` and `$forceLock`, which JS can't share
/// it with, read it as written there.
pub(in crate::lower) const INIT: &str = "init";

/// A cell that `init` makes the value of, `{ init }`.
pub(in crate::lower) fn lazy_of(init: Expr) -> Expr {
    Expr::object(vec![Prop::Field(INIT.into(), init)])
}

/// Whether `lazy`'s value is made, `lazy.init === undefined`.
pub(in crate::lower) fn is_made(lazy: Expr) -> Expr {
    Expr::bin(Op::Eq, Expr::member(lazy, INIT), Expr::js_undefined())
}

/// A `LazyCell`'s or `LazyLock`'s own functions, and its `Deref`.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum LazyOp {
    New,
    /// `force()` and `Deref`: its value, made if it isn't yet.
    Force,
    /// `force_mut()` and `DerefMut`.
    ForceMut,
    /// `get()`: its value if it's made.
    Get,
    GetMut,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `LazyCell`'s or `LazyLock`'s call: `None` if `known` is another.
    pub(in crate::lower) fn lazy_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Std::Lazy(op) = known else {
            return Ok(None);
        };
        let Call {
            generic_args,
            tys,
            span,
            ..
        } = call;
        // `Deref`'s are of the cell's type, its own functions' of what it holds.
        let item = match *generic_args.type_at(0).kind() {
            ty::Adt(_, args) if self.is_std_type(generic_args.type_at(0), StdItem::LazyCell) => args.type_at(0),
            _ => generic_args.type_at(0),
        };
        let lazy = values.next().expect("rustc checked the arguments");
        if op == LazyOp::New {
            return Ok(Some(lazy_of(lazy)));
        }
        // A `LazyLock`'s init that uses it deadlocks in Rust: rust-js's error.
        let force = match self.recognition().is_lock_cell(tys[0].peel_refs()) {
            true => "$forceLock",
            false => "$force",
        };
        if op == LazyOp::Force || op == LazyOp::ForceMut && !self.mut_ref_form(item).is_cell() {
            return Ok(Some(Expr::call(Expr::var(force), vec![lazy])));
        }
        if op == LazyOp::GetMut && self.boxed_payload(item) {
            return Err(self.unsupported(span, &format!("`get_mut` of a `LazyCell` of `{item}`")));
        }
        let lazy = if lazy.reads_same() {
            lazy
        } else {
            self.spill(Role::Lazy, lazy, out)
        };
        // A `&mut` to a number or text is a `{ value }` box (ADR 0074), which
        // the cell itself is once its value's made.
        let given = match op {
            LazyOp::ForceMut => {
                let force = Expr::call(Expr::var(force), vec![lazy.clone()]);
                out.push(StmtKind::Expr(force).at(self.js_span(span)));
                // In a crate that asks an address, a handle on its place in the
                // cell, past its state's tag (#1474).
                let place = self.lent_place((lazy.clone(), tys[0].peel_refs()), Lend::LazyCell, None);
                return Ok(Some(place.unwrap_or(lazy)));
            }
            LazyOp::GetMut if self.mut_ref_form(item).is_cell() => lazy.clone(),
            LazyOp::Get if self.boxed_payload(item) => self.some(Expr::held(lazy.clone())),
            _ => Expr::held(lazy.clone()),
        };
        Ok(Some(Expr::cond(is_made(lazy), given, Expr::js_undefined())))
    }
}
