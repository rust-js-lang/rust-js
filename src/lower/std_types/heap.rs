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
        let item = self
            .slice_item(heap_ty)
            .ok_or_else(|| self.unsupported(span, "this heap"))?;
        self.heap_of(item, span)?;
        let compare = self.cmp_fn(item, false, span)?;
        let mut values = self.operands(args, out)?;
        values.push(compare);
        let (helper, name) = match op {
            HeapOp::Push => (Helper::HeapPush, "$heapPush"),
            HeapOp::Pop => (Helper::HeapPop, "$heapPop"),
            HeapOp::IntoSorted => (Helper::HeapSorted, "$heapSorted"),
            HeapOp::From => (Helper::HeapFrom, "$heapFrom"),
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
