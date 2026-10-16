//! `BinaryHeap` (ADR 0068): a JS array kept in the order Rust's heap
//! keeps it, by the runtime's `$heapPush` and `$heapPop`.

use rustc_middle::ty::Ty;
use rustc_span::Span;

use crate::js::Expr;
use crate::lower::{FnCx, R};

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
    /// A `BinaryHeap`'s method, of its arguments' values, the heap's type
    /// `heap_ty` as it's given.
    pub(in crate::lower) fn heap_values(
        &mut self,
        op: HeapOp,
        mut values: Vec<Expr>,
        heap_ty: Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        let heap_ty = match op {
            HeapOp::From => heap_ty,
            _ => heap_ty.peel_refs(),
        };
        // A guard's, of its top.
        if let HeapOp::PeekTop { .. } | HeapOp::PeekPop = op {
            let item = self.recognition().peek_mut_of(heap_ty).expect("a `PeekMut`");
            let guard = values.remove(0);
            return Ok(match op {
                HeapOp::PeekTop { mutable: false } => Expr::index(Expr::member(guard, "heap"), Expr::int(0)),
                HeapOp::PeekTop { .. } => {
                    let mut given = vec![guard];
                    if self.is_boxable(item) {
                        given.push(Expr::bool(true));
                    }
                    Expr::call(Expr::helper("$peekMutTop"), given)
                }
                _ => Expr::call(Expr::helper("$peekMutPop"), vec![guard]),
            });
        }
        let item = self
            .slice_item(heap_ty)
            .ok_or_else(|| self.unsupported(span, "this heap"))?;
        if op == HeapOp::Drain {
            let heap = values.remove(0);
            return Ok(Expr::call(Expr::member(heap, "splice"), vec![Expr::int(0)]));
        }
        let compare = self.cmp_fn(item, false, span)?;
        values.push(compare);
        let name = match op {
            HeapOp::Push => "$heapPush",
            HeapOp::Pop => "$heapPop",
            HeapOp::IntoSorted => "$heapSorted",
            HeapOp::From => "$heapFrom",
            HeapOp::Append => "$heapAppend",
            HeapOp::Retain => "$heapRetain",
            HeapOp::PeekMut => "$peekMut",
            HeapOp::Drain | HeapOp::PeekTop { .. } | HeapOp::PeekPop => unreachable!("lowered above"),
        };
        Ok(Expr::call(Expr::helper(name), values))
    }
}
