//! A slice's fills, copies, order checks, chunks and splits (ADR 0324): its
//! JS array, changed in place where std's is.

use rustc_middle::thir::ExprId;
use rustc_middle::ty;
use rustc_middle::ty::TypeVisitableExt;
use rustc_span::Span;

use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::lower::recognition::trait_method;
use crate::lower::representation::Num;
use crate::lower::std_types::range::RangeKind;
use crate::lower::{FnCx, R};
use crate::runtime::Helper;

/// What an unstable sort orders by: its items' own order, a comparator
/// (`_by`), or a key (`_by_key`).
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum SortWith {
    Order,
    By,
    Key,
}

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
    /// `chunks_exact(n)`, and `chunks_exact_mut(n)` (`mutable`), whose
    /// chunks are views (ADR 0335); so of each below.
    ChunksExact {
        mutable: bool,
    },
    Rchunks {
        exact: bool,
        mutable: bool,
    },
    /// A `ChunksExact`'s or an `RChunksExact`'s `remainder()`, and a
    /// `_mut` one's `into_remainder()`.
    Remainder,
    /// `first_chunk::<N>()` or `last_chunk::<N>()`.
    Chunk {
        last: bool,
        mutable: bool,
    },
    /// `split_first_chunk::<N>()` or `split_last_chunk::<N>()`: that chunk
    /// and the rest.
    SplitChunk {
        last: bool,
        mutable: bool,
    },
    /// `split`, `splitn`, `rsplit`, `rsplitn` and `split_inclusive` by a
    /// predicate.
    SplitBy {
        limited: bool,
        inclusive: bool,
        back: bool,
        mutable: bool,
    },
    /// `chunk_by(p)`: the runs `p` holds of each two in a row of.
    ChunkBy {
        mutable: bool,
    },
    /// `&mut v[a..b]` and `get_unchecked_mut(a..b)`: a view of those items
    /// (ADR 0335); `get_mut(a..b)`, one or `None` (`checked`).
    View {
        checked: bool,
    },
    /// `chunks_mut(n)`: views of each `n` items, the last fewer.
    ChunksMut,
    /// `split_at_mut(mid)`, or `split_at_mut_checked` (`checked`): views
    /// before and after `mid`.
    SplitAtMut {
        checked: bool,
    },
    /// `split_first_mut()` or `split_last_mut()` (`last`): the item at
    /// that end, a handle on a number or text, and a view of the rest.
    SplitEndMut {
        last: bool,
    },
    /// `as_array::<N>()` and `as_mut_array::<N>()`: itself, if it's `N`
    /// long (ADR 0337).
    AsArray,
    /// `array_windows::<N>()`: `windows(N)`'s.
    ArrayWindows,
    /// `as_chunks::<N>()`, or `as_rchunks` (`back`): its whole chunks and
    /// what's left; `as_chunks_unchecked` (`unchecked`) the chunks alone.
    AsChunks {
        back: bool,
        mutable: bool,
        unchecked: bool,
    },
    /// `as_flattened()` of arrays: their items, in order.
    Flattened {
        mutable: bool,
    },
    /// `s.split_off(range)` of a one-sided range, or `split_off_first()`
    /// (`end: Some(false)`) or `split_off_last()`: what it takes off `s`,
    /// which is then what's left (ADR 0338).
    SplitOff {
        end: Option<bool>,
        mutable: bool,
    },
    /// `get_disjoint_mut(indices)`: `Ok` of a `&mut` to each item or range,
    /// or `Err` of why not; `get_disjoint_unchecked_mut` (`unchecked`) the
    /// `&mut`s (ADR 0339).
    GetDisjointMut {
        unchecked: bool,
    },
    /// `write_copy_of_slice(src)` of `MaybeUninit`s, or
    /// `write_clone_of_slice` (`clone`): each item written, and the slice
    /// (ADR 0332).
    UninitWrite {
        clone: bool,
    },
    /// `assume_init_drop()` of `MaybeUninit`s: each item dropped.
    UninitDrop,
    /// `sort_unstable`, `_by` and `_by_key`, where items that compare equal
    /// can be told apart: std's own algorithm, which leaves them where it
    /// does (ADR 0342).
    SortUnstable {
        with: SortWith,
    },
    /// `select_nth_unstable(index)`, `_by` and `_by_key`: std's introselect.
    SelectNth {
        with: SortWith,
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
    /// `strip_prefix(p)`, `strip_suffix(p)`, or `strip_circumfix(p, s)`
    /// (`circumfix`), of items compared by their own `==` (ADR 0336).
    Strip {
        suffix: bool,
        circumfix: bool,
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
    /// A call of one of the views' helpers (ADR 0335).
    fn view_call(&mut self, name: &str, list: Vec<Expr>) -> Expr {
        self.runtime.insert(Helper::View);
        Expr::call(Expr::var(name), list)
    }

    /// How a slice's `starts_with` and `strip_*` compare its items (ADR
    /// 0336): `None` where JS's `===` is their `==`, numbers, text and
    /// `bool`s, or a function of two of them their own `==` is.
    pub(in crate::lower) fn item_eq(&mut self, item: ty::Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        if self.eq_is_identity(item) || (!self.has_cell_layer(item) && Num::of(item.peel_refs()).is_some()) {
            return Ok(None);
        }
        let mut body = Vec::new();
        let same = self.eq_value(Expr::var("a"), Expr::var("b"), item, span, &mut body)?;
        // `(a, b) => $eq(a, b)` is `$eq`: the helper gives it the two alone.
        if body.is_empty()
            && let js::ExprKind::Call(callee, given) = &same.kind
            && matches!(callee.kind, js::ExprKind::Var(_))
            && let [a, b] = given.as_slice()
            && matches!((&a.kind, &b.kind), (js::ExprKind::Var(a), js::ExprKind::Var(b)) if a == "a" && b == "b")
        {
            return Ok(Some(*callee.clone()));
        }
        body.push(StmtKind::Return(Some(same)).at(js::Span::NONE));
        Ok(Some(Expr::arrow(vec!["a".into(), "b".into()], body)))
    }

    /// What std's unstable sort picks by an item's type (ADR 0342): its
    /// small sort, by whether it's `Freeze` and `Copy` and its size, and a
    /// Hoare partition of one of more than 96 bytes.
    fn sort_kind(&self, item: ty::Ty<'tcx>, span: Span) -> R<&'static str> {
        if item.has_param() {
            return Err(self.unsupported(
                span,
                "an unstable sort of a type parameter's items, whose order std's algorithm picks by their layout",
            ));
        }
        let layout = self
            .tcx
            .layout_of(ty::TypingEnv::fully_monomorphized().as_query_input(item))
            .map_err(|_| self.unsupported(span, "an unstable sort of these items"))?;
        let size = layout.size.bytes();
        let freeze = item.is_freeze(self.tcx, self.typing_env);
        Ok(if freeze && self.is_copy(item) && size <= 8 {
            "network"
        } else if freeze && size * 48 <= 4096 {
            if size <= 16 { "general8" } else { "general" }
        } else if size <= 96 {
            "fallback"
        } else {
            "hoare"
        })
    }

    /// `a < b` of `ty` values, as `PartialOrd::lt` has it: JS's `<` of
    /// numbers and `bool`s, else of their `cmp`.
    fn less_value(&mut self, a: Expr, b: Expr, ty: ty::Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        let (_, inner) = self.through_refs(Expr::undefined(), ty);
        if Num::of(inner).is_some() || inner.is_bool() {
            let (a, _) = self.through_refs(a, ty);
            let (b, _) = self.through_refs(b, ty);
            return Ok(Expr::bin(Op::Lt, a, b));
        }
        let order = self.cmp_value(a, b, ty, false, span, out)?;
        Ok(Expr::bin(Op::Lt, order, Expr::int(0)))
    }

    /// `$sortUnstable(items, isLess, kind)`, or `$selectNthUnstable` of an
    /// `index`, a handle on a number or text it gives where `item` is one.
    fn unstable_sort_call(
        &mut self,
        op: SliceOp,
        items: Expr,
        index: Option<Expr>,
        is_less: Expr,
        kind: &str,
        item: ty::Ty<'tcx>,
    ) -> Expr {
        let select = matches!(op, SliceOp::SelectNth { .. });
        let mut list = vec![items];
        list.extend(index);
        list.extend([is_less, Expr::str(kind)]);
        if select && self.is_boxable(item) {
            list.push(Expr::bool(true));
        }
        self.runtime.insert(Helper::SortUnstable);
        let name = if select { "$selectNthUnstable" } else { "$sortUnstable" };
        Expr::call(Expr::var(name), list)
    }

    /// `value` where it's read more than once: a variable, or a `const`.
    fn named_once(&mut self, base: &str, value: Expr, out: &mut Vec<Stmt>) -> Expr {
        match value.kind {
            js::ExprKind::Var(_) => value,
            _ => self.spill(base, value, out),
        }
    }

    /// A chunk's `N`, as the call has it.
    fn chunk_length(&self, generic_args: ty::GenericArgsRef<'tcx>, span: Span) -> R<u64> {
        generic_args
            .consts()
            .next()
            .and_then(|n| n.try_to_target_usize(self.tcx))
            .ok_or_else(|| self.unsupported(span, "a chunk of a length only a caller knows"))
    }

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
            // `s.split_off(..n)`: `s` given what's left, `s = split[0]`, and what
            // it takes, `split[1]` (ADR 0338), as a `String`'s `pop` is.
            SliceOp::SplitOff { end, mutable } => {
                let Some(place) = self.mut_borrowed(args[0]) else {
                    return Err(self.unsupported(span, "splitting this slice"));
                };
                let (target, _) = self.prepare_assignment_target(place, true, Expr::undefined(), span, out)?;
                let mut list = vec![target.read()];
                match end {
                    None => match self.range_bounds(args[1], span, out)? {
                        (_, Some(n)) => list.push(n),
                        (n, None) => list.extend([n, Expr::bool(true)]),
                    },
                    Some(last) => {
                        list.push(Expr::bool(last));
                        if mutable && self.is_boxable(item()) {
                            list.push(Expr::bool(true));
                        }
                    }
                }
                let name = match (end, mutable) {
                    (None, false) => "$sliceSplitOff",
                    (None, true) => "$sliceSplitOffMut",
                    (Some(_), false) => "$sliceSplitOffEnd",
                    (Some(_), true) => "$sliceSplitOffEndMut",
                };
                let split = match mutable {
                    false => helper(self, name, list),
                    true => self.view_call(name, list),
                };
                let split = self.spill("split", split, out);
                target.write(Expr::index(split.clone(), Expr::int(0)), self.js_span(span), out);
                Expr::index(split, Expr::int(1))
            }
            // `$copyFromSlice(slots, src)`, then `slots`; a clone of each from
            // `$writeCloneOfSlice`, whose `assert_eq!` says why it panics.
            SliceOp::UninitWrite { clone } => {
                let mut values = self.operands(args, out)?.into_iter();
                let (slots, source) = (values.next().expect("the slots"), values.next().expect("the source"));
                let slots = if slots.reads_same() {
                    slots
                } else {
                    self.spill("slots", slots, out)
                };
                self.runtime.insert(Helper::SliceOps);
                if clone {
                    let clone = self.clone_arg(generic_args.type_at(0), span)?;
                    return Ok(Expr::call(Expr::var("$writeCloneOfSlice"), vec![slots, source, clone]));
                }
                let copy = Expr::call(Expr::var("$copyFromSlice"), vec![slots.clone(), source]);
                out.push(StmtKind::Expr(copy).at(self.js_span(span)));
                slots
            }
            SliceOp::UninitDrop => {
                let slots = self.operands(args, out)?.remove(0);
                let ty = ty::Ty::new_slice(self.tcx, generic_args.type_at(0));
                self.drop_value(slots, ty, span, out)?;
                Expr::undefined()
            }
            // `$sortUnstable(v, isLess, kind)`: std's `is_less` and what std
            // picks by the item's type (ADR 0342).
            SliceOp::SortUnstable { with } | SliceOp::SelectNth { with } => {
                let item = generic_args.type_at(0);
                let kind = self.sort_kind(item, span)?;
                let mut values = self.operands(args, out)?.into_iter();
                let mut next = || values.next().expect("rustc checked the arguments");
                let items = next();
                let index = matches!(op, SliceOp::SelectNth { .. }).then(&mut next);
                let mut body = Vec::new();
                let (a, b) = (Expr::var("a"), Expr::var("b"));
                let less = match with {
                    SortWith::Order => self.less_value(a, b, item, span, &mut body)?,
                    // `(a, b) => $cmp(b, a) < 0` of a closure of one expression.
                    SortWith::By => {
                        let compare = next();
                        if let js::ExprKind::Arrow(params, stmts) = &compare.kind
                            && params.len() == 2
                            && let [
                                Stmt {
                                    kind: StmtKind::Return(Some(order)),
                                    ..
                                },
                            ] = stmts.as_slice()
                        {
                            let less = Expr::bin(Op::Lt, order.clone(), Expr::int(0));
                            let returned = vec![StmtKind::Return(Some(less)).at(js::Span::NONE)];
                            let is_less = Expr::arrow(params.clone(), returned);
                            return Ok(self.unstable_sort_call(op, items, index, is_less, kind, item));
                        }
                        let compare = self.named_once("compare", compare, out);
                        Expr::bin(Op::Lt, Expr::call(compare, vec![a, b]), Expr::int(0))
                    }
                    SortWith::Key => {
                        let key = self.named_once("key", next(), out);
                        let key_ty = generic_args.type_at(1);
                        let (a, b) = (Expr::call(key.clone(), vec![a]), Expr::call(key, vec![b]));
                        self.less_value(a, b, key_ty, span, &mut body)?
                    }
                };
                body.push(StmtKind::Return(Some(less)).at(js::Span::NONE));
                let is_less = Expr::arrow(vec!["a".into(), "b".into()], body);
                self.unstable_sort_call(op, items, index, is_less, kind, item)
            }
            SliceOp::View { checked } => {
                let items = self.operands(&args[..1], out)?.remove(0);
                let (start, end) = self.range_bounds(args[1], span, out)?;
                // `&mut v[..]`: all of it, which is `v`.
                if !checked && start.as_int() == Some(0) && end.is_none() {
                    return Ok(items);
                }
                let mut list = vec![items, start];
                list.extend(end);
                self.view_call(if checked { "$viewGet" } else { "$view" }, list)
            }
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
                    SliceOp::ChunksExact { mutable: false } => helper(self, "$chunksExact", vec![arg(), arg()]),
                    SliceOp::Rchunks { exact, mutable: false } => {
                        helper(self, "$rchunks", vec![arg(), arg(), Expr::bool(exact)])
                    }
                    SliceOp::ChunksMut => self.view_call("$chunksMut", vec![arg(), arg()]),
                    SliceOp::ChunksExact { mutable: true } => self.view_call("$chunksExactMut", vec![arg(), arg()]),
                    SliceOp::Rchunks { exact, mutable: true } => {
                        self.view_call("$rchunksMut", vec![arg(), arg(), Expr::bool(exact)])
                    }
                    SliceOp::SplitAtMut { checked } => {
                        let mut list = vec![arg(), arg()];
                        if checked {
                            list.push(Expr::bool(true));
                        }
                        self.view_call("$splitAtMut", list)
                    }
                    SliceOp::SplitEndMut { last } => {
                        let mut list = vec![arg(), Expr::bool(last)];
                        if self.is_boxable(item()) {
                            list.push(Expr::bool(true));
                        }
                        self.view_call("$splitEndMut", list)
                    }
                    SliceOp::ChunkBy { mutable: false } => helper(self, "$chunkBy", vec![arg(), arg()]),
                    SliceOp::ChunkBy { mutable: true } => self.view_call("$chunkByMut", vec![arg(), arg()]),
                    SliceOp::SplitChunk { last, mutable } => {
                        let n = self.chunk_length(generic_args, span)?;
                        let list = vec![arg(), Expr::int(n as i128), Expr::bool(last)];
                        match mutable {
                            false => helper(self, "$splitChunk", list),
                            true => self.view_call("$splitChunkMut", list),
                        }
                    }
                    SliceOp::View { .. }
                    | SliceOp::SortUnstable { .. }
                    | SliceOp::SelectNth { .. }
                    | SliceOp::SplitOff { .. }
                    | SliceOp::UninitWrite { .. }
                    | SliceOp::UninitDrop => unreachable!("lowered above"),
                    // Each index's item, a handle on a number or text, or each
                    // range's view, `$getDisjointMut(v, [0, 5], true)`.
                    SliceOp::GetDisjointMut { unchecked } => {
                        let index = generic_args.type_at(1);
                        let mut list = vec![arg(), arg()];
                        let handle = index.is_usize() && self.is_boxable(item());
                        let inclusive = self.range_kind(index) == Some(RangeKind::Inclusive);
                        if handle || inclusive {
                            list.push(Expr::bool(handle));
                        }
                        if inclusive {
                            list.push(Expr::bool(true));
                        }
                        let result = self.view_call("$getDisjointMut", list);
                        match unchecked {
                            true => Expr::member(result, "_0"),
                            false => result,
                        }
                    }
                    SliceOp::AsArray => {
                        let n = self.chunk_length(generic_args, span)?;
                        let items = arg();
                        let items = if items.reads_same() {
                            items
                        } else {
                            self.spill("items", items, out)
                        };
                        let length = Expr::member(items.clone(), "length");
                        Expr::cond(
                            Expr::bin(Op::Eq, length, Expr::int(n as i128)),
                            items,
                            Expr::undefined(),
                        )
                    }
                    SliceOp::ArrayWindows => {
                        let n = self.chunk_length(generic_args, span)?;
                        self.runtime.insert(Helper::Windows);
                        Expr::call(Expr::var("$windows"), vec![arg(), Expr::int(n as i128)])
                    }
                    SliceOp::AsChunks {
                        back,
                        mutable,
                        unchecked,
                    } => {
                        let n = self.chunk_length(generic_args, span)?;
                        let mut list = vec![arg(), Expr::int(n as i128)];
                        if back {
                            list.push(Expr::bool(true));
                        }
                        let pair = match mutable {
                            false => helper(self, "$asChunks", list),
                            true => self.view_call("$asChunksMut", list),
                        };
                        match unchecked {
                            true => Expr::index(pair, Expr::int(0)),
                            false => pair,
                        }
                    }
                    SliceOp::Flattened { mutable: false } => Expr::call(Expr::member(arg(), "flat"), Vec::new()),
                    SliceOp::Flattened { mutable: true } => {
                        let n = self.chunk_length(generic_args, span)?;
                        self.view_call("$flatView", vec![arg(), Expr::int(n as i128)])
                    }
                    // Of one stepped through a `&mut`, which is `$iter`'s, of its
                    // `items` (ADR 0071).
                    SliceOp::Remainder if self.is_stepping(args[0]) => {
                        Expr::member(Expr::member(arg(), "items"), "remainder")
                    }
                    SliceOp::Remainder => Expr::member(arg(), "remainder"),
                    // `Some` of its first `N`, or its last, if it has as many:
                    // a copy, or a view (`mutable`).
                    SliceOp::Chunk { last, mutable } => {
                        let n = self.chunk_length(generic_args, span)?;
                        let items = arg();
                        let items = if items.reads_same() {
                            items
                        } else {
                            self.spill("items", items, out)
                        };
                        let length = Expr::member(items.clone(), "length");
                        let chunk = match (last, mutable) {
                            (false, false) => {
                                Expr::call(Expr::member(items, "slice"), vec![Expr::int(0), Expr::int(n as i128)])
                            }
                            (true, false) => Expr::call(Expr::member(items, "slice"), vec![Expr::int(-(n as i128))]),
                            (false, true) => self.view_call("$view", vec![items, Expr::int(0), Expr::int(n as i128)]),
                            (true, true) => {
                                let start = Expr::bin(Op::Sub, length.clone(), Expr::int(n as i128));
                                self.view_call("$view", vec![items, start])
                            }
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
                        mutable,
                    } => {
                        let items = arg();
                        let limit = if limited { arg() } else { Expr::undefined() };
                        let predicate = arg();
                        let list = vec![items, predicate, limit, Expr::bool(inclusive), Expr::bool(back)];
                        match mutable {
                            false => helper(self, "$sliceSplitBy", list),
                            true => self.view_call("$sliceSplitByMut", list),
                        }
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
                    SliceOp::Strip { suffix, circumfix } => {
                        let mut list = vec![arg(), arg()];
                        let name = match circumfix {
                            true => {
                                list.push(arg());
                                "$sliceStripCircumfix"
                            }
                            false => {
                                list.push(Expr::bool(suffix));
                                "$sliceStrip"
                            }
                        };
                        list.extend(self.item_eq(item(), span)?);
                        helper(self, name, list)
                    }
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
