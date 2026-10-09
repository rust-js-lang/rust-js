//! `LazyCell` and `LazyLock` (ADR 0318): `{ init }`, whose `value` its
//! `init` makes the first time it's used, as Rust's does.

use crate::js::{Expr, Op, Prop, Stmt, StmtKind};
use crate::lower::calls::Call;
use crate::lower::recognition::{Std, StdItem};
use crate::lower::{FnCx, R};
use crate::runtime::Helper;
use rustc_middle::ty;

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
            args,
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
            return Ok(Some(Expr::object(vec![Prop::Field("init".into(), lazy)])));
        }
        // A `LazyLock`'s init that uses it deadlocks in Rust: rust-js's error.
        let (helper, force) = match self.recognition().is_lock_cell(self.thir[args[0]].ty.peel_refs()) {
            true => (Helper::ForceLock, "$forceLock"),
            false => (Helper::Force, "$force"),
        };
        if op == LazyOp::Force || op == LazyOp::ForceMut && !self.is_boxable(item) {
            self.runtime.insert(helper);
            return Ok(Some(Expr::call(Expr::var(force), vec![lazy])));
        }
        if op == LazyOp::GetMut && self.boxed_payload(item) {
            return Err(self.unsupported(span, &format!("`get_mut` of a `LazyCell` of `{item}`")));
        }
        let lazy = if lazy.reads_same() {
            lazy
        } else {
            self.spill("lazy", lazy, out)
        };
        // A `&mut` to a number or text is a `{ value }` box (ADR 0074), which
        // the cell itself is once its value's made.
        let given = match op {
            LazyOp::ForceMut => {
                self.runtime.insert(helper);
                let force = Expr::call(Expr::var(force), vec![lazy.clone()]);
                out.push(StmtKind::Expr(force).at(self.js_span(span)));
                return Ok(Some(lazy));
            }
            LazyOp::GetMut if self.is_boxable(item) => lazy.clone(),
            LazyOp::Get if self.boxed_payload(item) => self.some(Expr::member(lazy.clone(), "value")),
            _ => Expr::member(lazy.clone(), "value"),
        };
        let made = Expr::bin(Op::Eq, Expr::member(lazy, "init"), Expr::undefined());
        Ok(Some(Expr::cond(made, given, Expr::undefined())))
    }
}
