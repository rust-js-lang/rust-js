//! `BinaryHeap` (ADR 0068): a JS array kept in the order Rust's heap
//! keeps it, by the runtime's `$heapPush` and `$heapPop`.

use rustc_middle::thir::ExprId;
use rustc_middle::ty::Ty;
use rustc_span::Span;

use crate::js::{Expr, Stmt};
use crate::lower::{FnCx, R};
use crate::runtime::Helper;

/// A `BinaryHeap`'s: a JS array kept in the order Rust's heap keeps it,
/// by the same steps, so `{:?}` and `into_vec()` show what Rust's do.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum HeapOp {
    Push,
    Pop,
    IntoSorted,
    /// `BinaryHeap::from(v)`, and `collect()` into one.
    From,
    /// `append(&mut other)` and `retain(f)`, by std's steps (ADR 0333).
    Append,
    Retain,
    /// `drain()`: its items in its order, and it left empty, `heap.splice(0)`.
    Drain,
    /// `peek_mut()`: a guard of its top, `{ heap, cmp, changed }`; its
    /// `Deref` and `DerefMut`, and `PeekMut::pop` (ADR 0333).
    PeekMut,
    PeekTop {
        mutable: bool,
    },
    PeekPop,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// One of `HeapOp`'s, with the items' `cmp` (ADR 0057).
    pub(in crate::lower) fn heap_call(
        &mut self,
        op: HeapOp,
        args: &[ExprId],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let heap_ty = match op {
            HeapOp::From => self.thir[args[0]].ty,
            _ => self.thir[args[0]].ty.peel_refs(),
        };
        // A guard's, of its top.
        if let HeapOp::PeekTop { .. } | HeapOp::PeekPop = op {
            let item = self.recognition().peek_mut_of(heap_ty).expect("a `PeekMut`");
            let guard = self.expr(args[0], out)?;
            self.runtime.insert(Helper::HeapOps);
            return Ok(match op {
                HeapOp::PeekTop { mutable: false } => Expr::index(Expr::member(guard, "heap"), Expr::int(0)),
                HeapOp::PeekTop { .. } => {
                    let mut given = vec![guard];
                    if self.is_boxable(item) {
                        given.push(Expr::bool(true));
                    }
                    Expr::call(Expr::var("$peekMutTop"), given)
                }
                _ => Expr::call(Expr::var("$peekMutPop"), vec![guard]),
            });
        }
        let item = self
            .slice_item(heap_ty)
            .ok_or_else(|| self.unsupported(span, "this heap"))?;
        self.heap_of(item, span)?;
        if op == HeapOp::Drain {
            let heap = self.expr(args[0], out)?;
            return Ok(Expr::call(Expr::member(heap, "splice"), vec![Expr::int(0)]));
        }
        let compare = self.cmp_fn(item, false, span)?;
        let mut values = self.operands(args, out)?;
        values.push(compare);
        let (helper, name) = match op {
            HeapOp::Push => (Helper::HeapPush, "$heapPush"),
            HeapOp::Pop => (Helper::HeapPop, "$heapPop"),
            HeapOp::IntoSorted => (Helper::HeapSorted, "$heapSorted"),
            HeapOp::From => (Helper::HeapFrom, "$heapFrom"),
            HeapOp::Append => (Helper::HeapOps, "$heapAppend"),
            HeapOp::Retain => (Helper::HeapOps, "$heapRetain"),
            HeapOp::PeekMut => (Helper::HeapOps, "$peekMut"),
            HeapOp::Drain | HeapOp::PeekTop { .. } | HeapOp::PeekPop => unreachable!("lowered above"),
        };
        self.runtime.insert(helper);
        Ok(Expr::call(Expr::var(name), values))
    }

    /// A heap of `item`s: `pop` gives `undefined` for `None`, so an item
    /// that could look like it is an error, as for a map's values.
    pub(in crate::lower) fn heap_of(&self, item: Ty<'tcx>, span: Span) -> R<()> {
        if self.can_be_nullish(item) {
            return Err(self.unsupported(
                span,
                &format!("a heap of `{item}`, whose `pop` would look like `None` in JS"),
            ));
        }
        Ok(())
    }
}
