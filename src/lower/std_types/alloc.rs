//! The allocator (ADR 0417): an allocation is a JS array of slots, given as
//! a pointer to its first, as bytes, `alloc(layout)`; `realloc` keeps it,
//! as an array grows, and `dealloc` leaves it to JS's collector. A
//! `Layout` is `{ size, align }`, and `handle_alloc_error` ends the
//! program, as Rust's does. `Vec::from_raw_parts(p, len, cap)` is the array
//! `p` points to the first of, `len` long, and `v.set_len(n)` leaves what's
//! past `n` where it was (ADR 0420).

use crate::js::{Expr, Prop, Stmt, StmtKind};
use crate::lower::calls::Call;
use crate::lower::recognition::{Std, TypeFact};
use crate::lower::{FnCx, R};

/// What's asked of the allocator, or of a `Layout`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum AllocOp {
    /// `alloc(layout)`: a new allocation, a pointer to its bytes.
    Alloc,
    /// `realloc(p, layout, size)`: the same allocation, which grows.
    Realloc,
    /// `dealloc(p, layout)`: nothing JS's collector doesn't do.
    Dealloc,
    /// `handle_alloc_error(layout)`: the end, as Rust's says it.
    Error,
    /// `Layout::from_size_align(size, align)`: `Ok` of one, or `Err` where
    /// `align` isn't a power of two or `size` too big.
    Layout,
    /// `Layout::new::<T>()`: `T`'s size and alignment (ADR 0145).
    New,
    /// `Layout::from_size_align_unchecked(size, align)`: one, unchecked.
    Unchecked,
    /// `layout.size()`, `layout.align()`.
    Size,
    Align,
    /// `Vec::from_raw_parts(p, len, cap)`: the array `p` is into the first
    /// of, `len` long.
    VecFromRaw,
    /// `v.set_len(n)`: `n` long, what's past it left where it was.
    SetLen,
}

/// Which of the allocator's functions `path` is.
pub(in crate::lower) fn alloc_op(path: &str) -> Option<AllocOp> {
    Some(match path {
        "std::alloc::alloc" => AllocOp::Alloc,
        "std::alloc::realloc" => AllocOp::Realloc,
        "std::alloc::dealloc" => AllocOp::Dealloc,
        "std::alloc::handle_alloc_error" => AllocOp::Error,
        "std::alloc::Layout::from_size_align" => AllocOp::Layout,
        "std::alloc::Layout::new" => AllocOp::New,
        "std::alloc::Layout::from_size_align_unchecked" => AllocOp::Unchecked,
        "std::alloc::Layout::size" => AllocOp::Size,
        "std::alloc::Layout::align" => AllocOp::Align,
        "std::vec::Vec::<T>::from_raw_parts" => AllocOp::VecFromRaw,
        "std::vec::Vec::<T, A>::set_len" => AllocOp::SetLen,
        _ => return None,
    })
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of the allocator's: its helper, or a `Layout`'s field. `None`
    /// if `known` is another.
    pub(in crate::lower) fn alloc_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Std::Alloc(op) = known else {
            return Ok(None);
        };
        let span = call.span;
        let helper = |name: &'static str, values: Vec<Expr>| Expr::call(Expr::helper(name), values);
        let layout = |size: Expr, align: Expr| {
            Expr::object(vec![
                Prop::Field("size".into(), size),
                Prop::Field("align".into(), align),
            ])
        };
        Ok(Some(match op {
            AllocOp::Alloc => helper("$alloc", values.collect()),
            AllocOp::Realloc => helper("$realloc", values.collect()),
            AllocOp::Error => helper("$allocError", values.collect()),
            AllocOp::Layout => helper("$layout", values.collect()),
            AllocOp::VecFromRaw => helper("$vecFromRaw", values.collect()),
            AllocOp::SetLen => helper("$setLen", values.collect()),
            AllocOp::New => {
                let of = call.generic_args.type_at(0);
                layout(
                    self.type_fact_value(of, TypeFact::Size, span)?,
                    self.type_fact_value(of, TypeFact::Align, span)?,
                )
            }
            AllocOp::Unchecked => {
                let size = values.next().expect("a size");
                layout(size, values.next().expect("an alignment"))
            }
            AllocOp::Size => Expr::member(values.next().expect("a layout"), "size"),
            AllocOp::Align => Expr::member(values.next().expect("a layout"), "align"),
            AllocOp::Dealloc => {
                for value in values.by_ref() {
                    if value.has_effects() {
                        out.push(StmtKind::Expr(value).at(self.js_span(span)));
                    }
                }
                Expr::undefined()
            }
        }))
    }
}
