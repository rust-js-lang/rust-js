//! A slice's fills, copies, order checks, chunks and splits (ADR 0324): its
//! JS array, changed in place where std's is.

use rustc_middle::thir::ExprId;
use rustc_middle::ty;
use rustc_span::Span;

use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::lower::recognition::trait_method;
use crate::lower::representation::Num;
use crate::lower::{FnCx, R};
use crate::runtime::Helper;

/// A slice's own methods, of those ADR 0324 takes.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum SliceOp {
    Fill,
    FillWith,
    CopyFromSlice,
    CloneFromSlice,
    SwapWithSlice,
    CopyWithin,
    IsSorted,
    IsSortedBy,
    IsSortedByKey,
    PartitionPoint,
    ChunksExact,
    Rchunks {
        exact: bool,
    },
    /// A `ChunksExact`'s or an `RChunksExact`'s `remainder()`.
    Remainder,
    /// `first_chunk::<N>()` or `last_chunk::<N>()`.
    Chunk {
        last: bool,
    },
    /// `split`, `splitn`, `rsplit`, `rsplitn` and `split_inclusive` by a
    /// predicate.
    SplitBy {
        limited: bool,
        inclusive: bool,
        back: bool,
    },
    SortByCachedKey,
    /// A byte slice's `to_ascii_uppercase()` or `to_ascii_lowercase()`, and
    /// `make_ascii_*` in place.
    AsciiCase {
        upper: bool,
        in_place: bool,
    },
    IsAscii,
    TrimAscii {
        start: bool,
        end: bool,
    },
    /// `strip_prefix(p)` or `strip_suffix(p)`, of items compared by value.
    Strip {
        suffix: bool,
    },
    Repeat,
    /// A `VecDeque`'s `swap_remove_back(i)` or `swap_remove_front(i)`.
    DequeSwapRemove {
        front: bool,
    },
    /// `retain_mut(f)` and a `VecDeque`'s `pop_front_if(f)`: `f` given each
    /// item, or a handle on a number or a string.
    RetainMut,
    PopFrontIf,
    /// `push_mut(x)`, `insert_mut(i, x)`: a `&mut` to the item put in.
    PushMut {
        at: bool,
    },
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// One of `SliceOp`'s, of `args`, with the call's `generic_args`.
    pub(in crate::lower) fn slice_call(
        &mut self,
        op: SliceOp,
        args: &[ExprId],
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let helper = |cx: &mut Self, name: &str, list: Vec<Expr>| {
            cx.runtime.insert(Helper::SliceOps);
            Expr::call(Expr::var(name), list)
        };
        let item = || generic_args.type_at(0);
        Ok(match op {
            SliceOp::CopyWithin => {
                let items = self.operands(&args[..1], out)?.remove(0);
                let (start, end) = self.range_bounds(args[1], span, out)?;
                let dest = self.operands(&args[2..], out)?.remove(0);
                helper(
                    self,
                    "$copyWithin",
                    vec![items, start, end.unwrap_or_else(Expr::undefined), dest],
                )
            }
            _ => {
                let mut values = self.operands(args, out)?.into_iter();
                let mut arg = || values.next().expect("rustc checked the arguments");
                match op {
                    // A clone of the value in each but the last, as std's is.
                    SliceOp::Fill => {
                        let (items, value) = (arg(), arg());
                        match self.needs_clone(item()) {
                            true => {
                                let clone = self.clone_arg(item(), span)?;
                                helper(self, "$fill", vec![items, value, clone])
                            }
                            false => Expr::call(Expr::member(items, "fill"), vec![value]),
                        }
                    }
                    SliceOp::FillWith => helper(self, "$fillWith", vec![arg(), arg()]),
                    SliceOp::CopyFromSlice => helper(self, "$copyFromSlice", vec![arg(), arg()]),
                    // Each by its type's own `clone_from` where it has one, as
                    // std's calls it.
                    SliceOp::CloneFromSlice if self.recognition().own_clone_from(item()) => {
                        let (items, source) = (arg(), arg());
                        let place = Expr::index(Expr::var("items"), Expr::var("i"));
                        let target = match self.is_boxable(item()) {
                            true => Expr::handle(place),
                            false => place,
                        };
                        let clone_from = trait_method(self.tcx, self.clone_trait(), "clone_from");
                        let args = self.args_of(self.clone_trait(), item());
                        let called = self.impl_call(clone_from, args, vec![target, Expr::var("from")], span)?;
                        let f = Expr::arrow(
                            vec!["items".into(), "i".into(), "from".into()],
                            vec![StmtKind::Expr(called).at(js::Span::NONE)],
                        );
                        helper(self, "$cloneFromSlice", vec![items, source, Expr::undefined(), f])
                    }
                    SliceOp::CloneFromSlice if self.recognition().reaches_clone_from(item()) => {
                        return Err(self.unsupported(
                            span,
                            "`clone_from_slice` of items whose `clone_from` may be the crate's, inside them or generic",
                        ));
                    }
                    SliceOp::CloneFromSlice => {
                        let (items, source) = (arg(), arg());
                        let clone = self.clone_arg(item(), span)?;
                        helper(self, "$cloneFromSlice", vec![items, source, clone])
                    }
                    SliceOp::SwapWithSlice => helper(self, "$swapWithSlice", vec![arg(), arg()]),
                    // Each two in a row, by their `PartialOrd`.
                    // A number's is JS's `<=`, `false` of a NaN as `partial_cmp`'s is.
                    SliceOp::IsSorted => {
                        let items = arg();
                        let (a, b) = (Expr::var("a"), Expr::var("b"));
                        let le = match Num::of(item()) {
                            Some(_) => Expr::bin(Op::Le, a, b),
                            None => {
                                let cmp = self.cmp_fn(item(), true, span)?;
                                Expr::bin(Op::Le, Expr::call(cmp, vec![a, b]), Expr::int(0))
                            }
                        };
                        let in_order = Expr::arrow(
                            vec!["a".into(), "b".into()],
                            vec![crate::js::StmtKind::Return(Some(le)).at(crate::js::Span::NONE)],
                        );
                        helper(self, "$isSortedBy", vec![items, in_order])
                    }
                    SliceOp::IsSortedBy => helper(self, "$isSortedBy", vec![arg(), arg()]),
                    SliceOp::IsSortedByKey => {
                        let (items, key) = (arg(), arg());
                        // `is_sorted_by_key<'a, F, K>`: its key, the third type.
                        let key_ty = generic_args.types().nth(2).expect("its key's type");
                        let cmp = self.cmp_fn(key_ty, true, span)?;
                        helper(self, "$isSortedByKey", vec![items, key, cmp])
                    }
                    SliceOp::PartitionPoint => helper(self, "$partitionPoint", vec![arg(), arg()]),
                    SliceOp::ChunksExact => helper(self, "$chunksExact", vec![arg(), arg()]),
                    SliceOp::Rchunks { exact } => helper(self, "$rchunks", vec![arg(), arg(), Expr::bool(exact)]),
                    SliceOp::Remainder => Expr::member(arg(), "remainder"),
                    // `Some` of its first `N`, or its last, if it has as many.
                    SliceOp::Chunk { last } => {
                        let Some(n) = generic_args
                            .consts()
                            .next()
                            .and_then(|n| n.try_to_target_usize(self.tcx))
                        else {
                            return Err(self.unsupported(span, "a chunk of a length only a caller knows"));
                        };
                        let items = arg();
                        let items = if items.reads_same() {
                            items
                        } else {
                            self.spill("items", items, out)
                        };
                        let length = Expr::member(items.clone(), "length");
                        let chunk = match last {
                            false => Expr::call(Expr::member(items, "slice"), vec![Expr::int(0), Expr::int(n as i128)]),
                            true => Expr::call(Expr::member(items, "slice"), vec![Expr::int(-(n as i128))]),
                        };
                        let chunk = match (n, last) {
                            // `slice(-0)` is all of it.
                            (0, true) => Expr::array(Vec::new()),
                            _ => chunk,
                        };
                        Expr::cond(
                            Expr::bin(Op::Ge, length, Expr::int(n as i128)),
                            chunk,
                            Expr::undefined(),
                        )
                    }
                    SliceOp::SplitBy {
                        limited,
                        inclusive,
                        back,
                    } => {
                        let items = arg();
                        let limit = if limited { arg() } else { Expr::undefined() };
                        let predicate = arg();
                        helper(
                            self,
                            "$sliceSplitBy",
                            vec![items, predicate, limit, Expr::bool(inclusive), Expr::bool(back)],
                        )
                    }
                    SliceOp::SortByCachedKey => {
                        let (items, key) = (arg(), arg());
                        // `sort_by_cached_key<K, F>`: its key, the second type.
                        let cmp = self.cmp_fn(generic_args.type_at(1), false, span)?;
                        helper(self, "$sortByCachedKey", vec![items, key, cmp])
                    }
                    SliceOp::AsciiCase { upper, in_place } => {
                        let name = if in_place { "$makeAsciiBytes" } else { "$asciiBytes" };
                        helper(self, name, vec![arg(), Expr::bool(upper)])
                    }
                    SliceOp::IsAscii => {
                        let byte = Expr::arrow(
                            vec!["b".into()],
                            vec![
                                crate::js::StmtKind::Return(Some(Expr::bin(Op::Lt, Expr::var("b"), Expr::int(128))))
                                    .at(crate::js::Span::NONE),
                            ],
                        );
                        Expr::call(Expr::member(arg(), "every"), vec![byte])
                    }
                    SliceOp::TrimAscii { start, end } => {
                        helper(self, "$trimAsciiBytes", vec![arg(), Expr::bool(start), Expr::bool(end)])
                    }
                    SliceOp::Strip { suffix } => helper(self, "$sliceStrip", vec![arg(), arg(), Expr::bool(suffix)]),
                    SliceOp::Repeat => helper(self, "$repeatItems", vec![arg(), arg()]),
                    SliceOp::PushMut { at } => {
                        let mut given = vec![arg(), arg()];
                        if at {
                            given.push(arg());
                        }
                        if self.is_boxable(item()) {
                            given.push(Expr::bool(true));
                        }
                        helper(self, if at { "$insertMut" } else { "$pushMut" }, given)
                    }
                    SliceOp::DequeSwapRemove { front } => {
                        helper(self, "$dequeSwapRemove", vec![arg(), arg(), Expr::bool(front)])
                    }
                    SliceOp::RetainMut | SliceOp::PopFrontIf => {
                        let handles = self.is_boxable(item());
                        let name = if op == SliceOp::RetainMut {
                            "$retainMut"
                        } else {
                            "$popFrontIf"
                        };
                        helper(self, name, vec![arg(), arg(), Expr::bool(handles)])
                    }
                    SliceOp::CopyWithin => unreachable!("lowered above"),
                }
            }
        })
    }
}
