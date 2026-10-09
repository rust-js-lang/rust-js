//! `OnceCell` and `OnceLock` (ADR 0317): a `{ value }` of an `Option`, as
//! a `Cell<Option<T>>` is, set once.

use crate::js::{Expr, Op, Prop, Stmt};
use crate::lower::calls::Call;
use crate::lower::recognition::Std;
use crate::lower::{FnCx, R};
use crate::runtime::Helper;

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
        let Call { generic_args, span, .. } = call;
        let item = generic_args.type_at(0);
        let mut arg = || values.next().expect("rustc checked the arguments");
        Ok(Some(match op {
            OnceOp::New => Expr::object(vec![Prop::Field("value".into(), Expr::undefined())]),
            OnceOp::Get => Expr::member(arg(), "value"),
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
                    Expr::bin(Op::LooseEq, Expr::member(cell.clone(), "value"), Expr::null()),
                    Expr::undefined(),
                    cell,
                )
            }
            OnceOp::GetMut => Expr::member(arg(), "value"),
            // What's set is `Some` of it, boxed where it looks like `None`
            // (ADR 0051), and std's panic of an init that inits it again.
            OnceOp::Set => {
                self.runtime.insert(Helper::OnceSet);
                Expr::call(Expr::var("$onceSet"), vec![arg(), arg()])
            }
            OnceOp::GetOrInit => {
                self.runtime.insert(Helper::GetOrInit);
                Expr::call(Expr::var("$getOrInit"), vec![arg(), arg()])
            }
            OnceOp::Take => {
                self.runtime.insert(Helper::CellReplace);
                Expr::call(Expr::var("$cellReplace"), vec![arg(), Expr::undefined()])
            }
        }))
    }
}
