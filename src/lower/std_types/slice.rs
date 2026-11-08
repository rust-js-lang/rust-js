//! A slice's fills, copies, order checks, chunks and splits (ADR 0324): its
//! JS array, changed in place where std's is.

use rustc_middle::ty;
use rustc_middle::ty::TypeVisitableExt;
use rustc_span::Span;

use crate::js::{self, Expr, Op, Stmt, StmtKind};
use crate::lower::recognition::{Std, TypeFact, trait_method};
use crate::lower::representation::Num;
use crate::lower::std_types::range::RangeKind;
use crate::lower::{FnCx, R};

/// The helper that does what `known` does to a view of part of a slice,
/// given the slice and where the part starts and ends, `(v, start, end?)`,
/// where it has one: JS has no method over part of an array, so a view's
/// is the slice's own, in place, no view made (ADR 1073).
pub(in crate::lower) fn ranged(known: Std) -> Option<&'static str> {
    match known {
        Std::Method("reverse") => Some("$reverseRange"),
        _ => None,
    }
}

/// What a sort orders by: its items' own order, a comparator (`_by`), or a
/// key (`_by_key`).
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum SortWith {
    Order,
    By,
    Key,
}

/// A slice's own methods, of those ADR 0324 takes.
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum SliceOp {
    /// `v[i]` or `&mut v[i]` of a type parameter's index, `I: SliceIndex<[T]>`:
    /// its dictionary's `index(i, v)` or `index_mut(i, v)` (ADR 0418).
    GenericIndex {
        mutable: bool,
    },
    Fill,
    FillWith,
    CopyFromSlice,
    CloneFromSlice,
    SwapWithSlice,
    /// A `VecDeque`'s `as_slices()` and `as_mut_slices()`: the one array
    /// it is, then none, `[d, []]`, writes through the first its own.
    DequeSlices,
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
    /// A `VecDeque`'s `range_mut(a..b)`: a view of those items, checked as
    /// `&mut v[a..b]` is, a handle on each number or text, as `iter_mut`'s.
    DequeRangeMut,
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
    /// `array::from_fn(f)`: `Array.from({ length: N }, (_, i) => ..)`, `f`
    /// given each index.
    FromFn,
    /// `slice::from_ref(&x)` and `array::from_ref(&x)`: `[x]`, of one item
    /// it shares.
    FromRef,
    /// A `VecDeque`'s `swap_remove_back(i)` or `swap_remove_front(i)`.
    DequeSwapRemove {
        front: bool,
    },
    /// `retain_mut(f)` and a `VecDeque`'s `pop_front_if(f)`: `f` given each
    /// item, or a handle on a number or a string. A `VecDeque`'s
    /// `retain_mut` is std's own (`deque`, #909).
    RetainMut {
        deque: bool,
    },
    PopFrontIf,
    /// `push_mut(x)`, `insert_mut(i, x)`: a `&mut` to the item put in.
    PushMut {
        at: bool,
    },
    /// A deque's `push_front_mut(x)`.
    PushFrontMut,
    /// `v.extract_if(range, f)` of a `Vec`, or of all of a list
    /// (`range: false`): a JS iterator, taking out what it gives as it's
    /// asked (ADR 0344).
    ExtractIf {
        range: bool,
    },
    /// A `VecDeque`'s `insert(i, x)`, or `insert_mut` (`mutable`): checked
    /// with its own message, `index out of bounds`.
    DequeInsert {
        mutable: bool,
    },
}

impl SliceOp {
    /// Does it lend items, or views of them, and drop none: `split_at_mut`,
    /// `chunks_mut` and the rest (ADR 0507)?
    pub(in crate::lower) fn lends(self) -> bool {
        matches!(
            self,
            SliceOp::SplitChunk { .. }
                | SliceOp::SplitBy { .. }
                | SliceOp::ChunkBy { .. }
                | SliceOp::View { .. }
                | SliceOp::DequeRangeMut
                | SliceOp::ChunksMut
                | SliceOp::SplitAtMut { .. }
                | SliceOp::SplitEndMut { .. }
                | SliceOp::AsArray
                | SliceOp::ArrayWindows
                | SliceOp::AsChunks { .. }
                | SliceOp::Flattened { .. }
                | SliceOp::GetDisjointMut { .. }
        )
    }
}

/// A value written over in place by a clone of another, as std's own
/// `clone_from` of its type does (`clone_into`): `helper(to, from,
/// ...args)`.
pub(in crate::lower) struct CloneInto {
    helper: &'static str,
    args: Vec<Expr>,
}

impl CloneInto {
    /// `to` written over by a clone of `from`.
    pub(in crate::lower) fn call(self, to: Expr, from: Expr) -> Expr {
        Expr::call_helper(self.helper, [to, from].into_iter().chain(self.args).collect())
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// How an `item` is written over by a clone of another, as std's
    /// `clone_from` does it: a clone, where a clone is more than the value,
    /// and `(items, i, from) => ...`, its type's own `clone_from`, or a type
    /// parameter's dictionary's where its impl writes one. The one answer
    /// `fill` and `clone_from_slice` ask (ADR 0507).
    fn clone_over(&mut self, item: ty::Ty<'tcx>, what: &str, span: Span) -> R<(Option<Expr>, Option<Expr>)> {
        let written = |called: Expr| {
            Expr::arrow(
                vec!["items".into(), "i".into(), "from".into()],
                vec![StmtKind::Expr(called).at(js::Span::NONE)],
            )
        };
        if self.recognition().own_clone_from(item) {
            let place = Expr::index(Expr::var("items"), Expr::var("i"));
            let target = match self.handles_mut(item) {
                true => Expr::handle(place),
                false => place,
            };
            let clone_from = trait_method(self.tcx, self.clone_trait(), "clone_from");
            let args = self.args_of(self.clone_trait(), item);
            let called = self.impl_call(clone_from, args, vec![target, Expr::var("from")], span)?;
            return Ok((None, Some(written(called))));
        }
        // A type parameter's, or an associated type's, `A::Item`.
        if self.is_unknown(item) {
            let clone = self.clone_arg(item, span)?;
            let clone_from = self.given_clone_from(item, span)?;
            let place = Expr::call(Expr::helper("$ptr"), vec![Expr::var("items"), Expr::var("i")]);
            let called = Expr::call(clone_from.clone(), vec![place, Expr::var("from")]);
            return Ok((Some(clone), Some(Expr::bin(Op::And, clone_from, written(called)))));
        }
        // A `Vec`'s, a `VecDeque`'s, a map's, std's own, written in place,
        // where its order of clones and drops is seen: `clone_into`'s
        // (#919, #922).
        let slot = || Expr::index(Expr::var("items"), Expr::var("i"));
        if let Some(into) = self.clone_into(item, span)? {
            return Ok((None, Some(written(into.call(slot(), Expr::var("from"))))));
        }
        // An `Option`'s: its value's, where both are `Some`, and otherwise
        // a clone written over it, what it held dropped (#922).
        let pointer = Expr::call(Expr::helper("$ptr"), vec![Expr::var("items"), Expr::var("i")]);
        if let Some(over) = self.option_clone_over(item, (slot(), pointer), Expr::var("from"), span)? {
            return Ok((
                None,
                Some(Expr::arrow(vec!["items".into(), "i".into(), "from".into()], over)),
            ));
        }
        if self.recognition().reaches_clone_from(item) {
            return Err(self.unsupported(
                span,
                &format!("`{what}` of items whose `clone_from` may be the crate's, inside them or generic"),
            ));
        }
        Ok((Some(self.clone_arg(item, span)?), None))
    }

    /// `to.clone_from(from)` of an `Option` whose value is written in place,
    /// `written_over`'s, as std's is: of two `Some`s, its value's; otherwise
    /// `*to = from.clone()`, the clone written over it, then what it held
    /// dropped (#922). `to` is a place, read more than once, as `from` is,
    /// and the box generic code has of it, which is its value's too, as a
    /// `Some` of it is the value. `None` where its value isn't written in
    /// place.
    pub(in crate::lower) fn option_clone_over(
        &mut self,
        ty: ty::Ty<'tcx>,
        (to, pointer): (Expr, Expr),
        from: Expr,
        span: Span,
    ) -> R<Option<Vec<Stmt>>> {
        let Some(inner) = self.option_of(ty).filter(|&inner| !self.boxed_payload(inner)) else {
            return Ok(None);
        };
        let Some((asked, written)) = self.written_over(inner, (to.clone(), pointer), from.clone(), span)? else {
            return Ok(None);
        };
        let some = |e: &Expr| Expr::bin(Op::LooseNe, e.clone(), Expr::null());
        let mut both = Expr::bin(Op::And, some(&to), some(&from));
        if let Some(asked) = asked {
            both = Expr::bin(Op::And, both, asked);
        }
        let mut replaced = Vec::new();
        let clone = self.clone_value(from.clone(), ty, span, &mut replaced)?;
        let old = self.fresh("old");
        replaced.push(StmtKind::Const(old.clone(), to.clone()).at(js::Span::NONE));
        replaced.push(StmtKind::Assign(to.clone(), clone).at(js::Span::NONE));
        self.drop_value(Expr::var(&old), ty, span, &mut replaced)?;
        let written = StmtKind::Expr(written).at(js::Span::NONE);
        Ok(Some(vec![
            StmtKind::If(both, vec![written], Some(replaced)).at(js::Span::NONE),
        ]))
    }

    /// `held` at `place`, `pointer` the box generic code has of it, written
    /// over in place by `from` as std's `clone_from` of it writes it, where
    /// that isn't `*place = from.clone()`: `clone_into`'s, a `Box`'s its
    /// value's; or of a type only a caller knows, its `Clone` dictionary's
    /// `clone_from`, which the caller's type has where std's or its impl's
    /// is its own (`dictionary_clone_from`), so the JS asks it first (#928):
    /// what's asked, and the call. `None` where it's `*place =
    /// from.clone()` here.
    pub(in crate::lower) fn written_over(
        &mut self,
        held: ty::Ty<'tcx>,
        (place, pointer): (Expr, Expr),
        from: Expr,
        span: Span,
    ) -> R<Option<(Option<Expr>, Expr)>> {
        if let Some(into) = self.clone_into(held, span)? {
            return Ok(Some((None, into.call(place, from))));
        }
        let mut value = held;
        while value.is_box() {
            value = value.boxed_ty().expect("a `Box`");
        }
        if !self.clone_given(value) {
            return Ok(None);
        }
        let clone_from = self.given_clone_from(value, span)?;
        Ok(Some((
            Some(clone_from.clone()),
            Expr::call(clone_from, vec![pointer, from]),
        )))
    }

    /// Whether `ty` is a type only a caller knows whose `Clone` dictionary
    /// a caller gives. Where none does, its dictionary is made here, of a
    /// value JS gives, and its `clone_from` is std's provided.
    pub(in crate::lower) fn clone_given(&self, ty: ty::Ty<'tcx>) -> bool {
        self.is_unknown(ty) && (self.evidence_for(ty::TraitRef::new(self.tcx, self.clone_trait(), [ty]))).is_some()
    }

    /// The `clone_from` of a type only a caller knows, its `Clone`
    /// dictionary's, `TClone.cloneFrom`: `undefined` where it's std's
    /// provided `*dest = source.clone()`.
    fn given_clone_from(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Expr> {
        let dictionary = self.dictionary(ty::TraitRef::new(self.tcx, self.clone_trait(), [ty]), span)?;
        Ok(Expr::member(
            dictionary,
            crate::lower::bindings::trait_method_name(self.tcx, self.clone_trait(), "clone_from"),
        ))
    }

    /// The `clone_from` of a std type's `Clone` dictionary, where std's
    /// isn't `*dest = source.clone()`, `(to, from) => ..`, `to` the box
    /// generic code has of its `&mut`: `clone_into`'s, an `Option`'s of its
    /// value's (`written_over`), so a generic `clone_from` of it is std's
    /// (#928), as an impl's own `clone_from` is its dictionary's. `None`
    /// where std's is the provided one. Refused where std's forwards to a
    /// value's that rust-js doesn't write (#927), which a generic caller's
    /// `clone_from` would otherwise skip.
    pub(in crate::lower) fn dictionary_clone_from(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        // A type only a caller knows, whose dictionary is made here, of no
        // caller's: its `clone_from` is no caller's either, std's provided.
        if self.is_unknown(ty) {
            return Ok(None);
        }
        let (to, from) = (Expr::var("to"), Expr::var("from"));
        let place = Expr::member(to.clone(), js::HELD);
        let body = match self.written_over(ty, (place.clone(), to.clone()), from.clone(), span)? {
            Some((None, written)) => vec![StmtKind::Expr(written).at(js::Span::NONE)],
            // A `Box` of a type parameter's, which is its value: the
            // value's dictionary's, given, where it has one.
            Some((Some(given), _)) => return Ok(Some(given)),
            None => match self.option_clone_over(ty, (place, to), from, span)? {
                Some(over) => over,
                None if self.forwards_clone_from(ty, span)? => {
                    return Err(self.unsupported(
                        span,
                        &format!("a `Clone` dictionary of `{ty}` given to generic code, whose std `clone_from` is its value's, which the crate's or std's own writes in place (#927)"),
                    ));
                }
                None => return Ok(None),
            },
        };
        Ok(Some(Expr::arrow(vec!["to".into(), "from".into()], body)))
    }

    /// How a `ty`, a JS object, is written over in place by a clone of
    /// another, as std's own `clone_from` of its type does, where its order
    /// of clones and drops is seen (#919, #922): a `Vec`'s and a
    /// `BinaryHeap`'s, `$cloneIntoVec`; a `VecDeque`'s, which clears, then
    /// clones each, `$cloneIntoDeque`; a `LinkedList`'s, which writes each it
    /// keeps over, pops the rest from its back, then clones,
    /// `$cloneIntoList`; an array's, each item written over,
    /// `$cloneFromSlice`; a hashed map's or set's, which drops its entries,
    /// stopping at one that panics, as hashbrown's does, then clones each,
    /// `$cloneIntoMap`; a `Box`'s, its value's. `None` where std's is
    /// `*dest = source.clone()`, a B-tree's, or where no order is seen: of
    /// items with no destructor, no `clone_from` of their own and a clone
    /// that can't panic.
    pub(in crate::lower) fn clone_into(&mut self, ty: ty::Ty<'tcx>, span: Span) -> R<Option<CloneInto>> {
        use crate::lower::recognition::StdItem;
        let seen = |cx: &Self, t| cx.drops(t) != crate::lower::drops::Drops::Nothing || cx.clone_may_panic(t);
        let ty::Adt(_, args) = ty.kind() else {
            // An array's `clone_from`, its slice's `clone_from_slice`.
            let ty::Array(item, _) = *ty.kind() else {
                return Ok(None);
            };
            let (clone, clone_from) = self.clone_over(item, "clone_from", span)?;
            if clone_from.is_none() && !seen(self, item) {
                return Ok(None);
            }
            let drop = self.std_drop_function(item, span)?;
            let args = Expr::optional_args(Vec::new(), vec![clone, clone_from, drop]);
            return Ok(Some(CloneInto {
                helper: "$cloneFromSlice",
                args,
            }));
        };
        if ty.is_box() {
            return self.clone_into(args.type_at(0), span);
        }
        if self.is_std_type(ty, StdItem::Vec) || self.is_std_type(ty, StdItem::BinaryHeap) {
            let args = self.vec_clone_into(args.type_at(0), span)?;
            return Ok(args.map(|args| CloneInto {
                helper: "$cloneIntoVec",
                args,
            }));
        }
        let deque = self.is_std_type(ty, StdItem::VecDeque);
        if deque || self.is_std_type(ty, StdItem::LinkedList) {
            let item = args.type_at(0);
            // A `VecDeque`'s asks no item's `clone_from`.
            let clone_from = match deque {
                true => None,
                false => self.clone_over(item, "clone_from", span)?.1,
            };
            if clone_from.is_none() && !seen(self, item) {
                return Ok(None);
            }
            let clone = match self.needs_clone(item) {
                true => Some(self.clone_fn("item", item, span)?),
                false => None,
            };
            let drop = self.std_drop_function(item, span)?;
            let (helper, list) = match deque {
                true => ("$cloneIntoDeque", vec![clone, drop]),
                false => ("$cloneIntoList", vec![clone, clone_from, drop]),
            };
            let args = Expr::optional_args(Vec::new(), list);
            return Ok(Some(CloneInto { helper, args }));
        }
        if self.is_map(ty) && !self.is_sorted(ty) {
            let mut out = Vec::new();
            let make = self.entry_clone(ty, span, &mut out)?;
            assert!(out.is_empty(), "an entry's clone is an arrow");
            let (make, entry, may_panic) = match make {
                Some((f, entry, may_panic)) => (Some(f), entry, may_panic),
                None if self.is_set(ty) => (None, args.type_at(0), false),
                None => (
                    None,
                    ty::Ty::new_tup(self.tcx, &[args.type_at(0), args.type_at(1)]),
                    false,
                ),
            };
            if self.drops(entry) == crate::lower::drops::Drops::Nothing && !may_panic {
                return Ok(None);
            }
            let drop = self.std_drop_function(entry, span)?;
            let args = Expr::optional_args(Vec::new(), vec![make, drop]);
            return Ok(Some(CloneInto {
                helper: "$cloneIntoMap",
                args,
            }));
        }
        Ok(None)
    }

    /// A `Vec`'s `clone_from` of `item`s, std's `SpecCloneIntoVec`: what it
    /// has past the source's length dropped, each it keeps written over by
    /// `clone_over`'s, then a clone of each of the rest pushed. The
    /// arguments `$cloneIntoVec(w, v, ..)` takes after the two, its items'
    /// clone, `clone_from` and drop, where that order is seen: of items with
    /// a destructor, or a `clone_from` of their own or a `Vec`'s, or a clone
    /// that may panic, which leaves `w` half written (#922); of none,
    /// `None`, as `*w = v.clone()` does alike, every clone then no drop
    /// that can be seen (#919).
    pub(in crate::lower) fn vec_clone_into(&mut self, item: ty::Ty<'tcx>, span: Span) -> R<Option<Vec<Expr>>> {
        let (_, clone_from) = self.clone_over(item, "clone_from", span)?;
        if clone_from.is_none()
            && self.drops(item) == crate::lower::drops::Drops::Nothing
            && !self.clone_may_panic(item)
        {
            return Ok(None);
        }
        // What the rest are cloned by, though those kept are by `clone_from`.
        let clone = match self.needs_clone(item) {
            true => Some(self.clone_fn("item", item, span)?),
            false => None,
        };
        let drop = self.std_drop_function(item, span)?;
        Ok(Some(Expr::optional_args(Vec::new(), vec![clone, clone_from, drop])))
    }

    /// A call of one of the views' helpers (ADR 0335).
    pub(in crate::lower) fn view_call(&mut self, name: &'static str, list: Vec<Expr>) -> Expr {
        Expr::call(Expr::helper(name), list)
    }

    /// How a slice's `starts_with` and `strip_*` compare its items (ADR
    /// 0336), and an iterator's `eq` another's, `item` to `rhs`: `None` where
    /// JS's `===` is their `==`, numbers, text and `bool`s, or a function of
    /// two of them their own `==` is, a `PartialEq<Rhs>`'s too.
    pub(in crate::lower) fn item_eq(
        &mut self,
        (item, rhs): (ty::Ty<'tcx>, ty::Ty<'tcx>),
        span: Span,
    ) -> R<Option<Expr>> {
        if item == rhs
            && (self.eq_is_identity(item) || (!self.has_cell_layer(item) && Num::of(item.peel_refs()).is_some()))
        {
            return Ok(None);
        }
        let mut body = Vec::new();
        let same = self.eq_values(Expr::var("a"), Expr::var("b"), (item, rhs), span, &mut body)?;
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

    /// What std's unstable sort picks by an item's type, of a type rustc
    /// knows the layout of (ADR 0342): its small sort, by whether it's
    /// `Freeze` and `Copy` and its size, and a Hoare partition of one of more
    /// than 96 bytes. A type parameter's is its caller's (ADR 0471).
    pub(in crate::lower) fn sort_kind(&self, item: ty::Ty<'tcx>, span: Span) -> R<&'static str> {
        if item.has_param() {
            return Err(self.unsupported(
                span,
                "an unstable sort of items that hold a type parameter, whose order std's algorithm picks by their layout",
            ));
        }
        let layout = self
            .layout(item)
            .ok_or_else(|| self.unsupported(span, "an unstable sort of these items"))?;
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

    /// What std's stable sort picks by an item's type (#925): its small
    /// sort, `"general8"`, sorting eight at a time, for a `Freeze` one of at
    /// most 16 bytes, `"general"` for a larger `Freeze` one, else
    /// `"fallback"`, insertion. A type parameter's is its caller's,
    /// `TStableSort`.
    pub(in crate::lower) fn stable_sort_kind(&self, item: ty::Ty<'tcx>, span: Span) -> R<&'static str> {
        if item.has_param() {
            return Err(self.unsupported(
                span,
                "a stable sort of items that hold a type parameter, whose comparison can be seen, which std's algorithm picks by their type",
            ));
        }
        let layout = (self.layout(item)).ok_or_else(|| self.unsupported(span, "a stable sort of these items"))?;
        Ok(self.stable_sort_kind_of(item, layout.size.bytes()))
    }

    /// `stable_sort_kind`'s, of an item of `size` bytes.
    fn stable_sort_kind_of(&self, item: ty::Ty<'tcx>, size: u64) -> &'static str {
        match item.is_freeze(self.tcx, self.typing_env) {
            true if size <= 16 => "general8",
            true => "general",
            false => "fallback",
        }
    }

    /// Whether a stable sort is std's own, step for step, not JS's `sort`,
    /// which asks another sequence and, of an order that isn't total, leaves
    /// another (#925, #936): where its calls can be seen, or its order may
    /// not be total. Of a type parameter's items, its caller's type's,
    /// always; of its items' order, `order_seen`'s and `order_total`'s; of a
    /// comparator, `call_seen`'s and `call_total`'s; of a key, its
    /// function's, its order's, or its drop's, which runs after each
    /// comparison. Of `sort`'s, `sort_by`'s or `sort_by_key`'s `generic_args`.
    pub(in crate::lower) fn stable_sort_is_std(&self, with: SortWith, generic_args: ty::GenericArgsRef<'tcx>) -> bool {
        let item = generic_args.type_at(0);
        match with {
            // A type parameter's items, its caller's type's, which may be seen.
            _ if self.is_unknown(item) => true,
            SortWith::Order => self.order_seen(item) || !self.order_total(item),
            // One by UTF-16 units, JS's own order of strings, asks nothing
            // a program sees (ADR 0945).
            SortWith::By => {
                let compare = generic_args.type_at(1);
                (self.call_seen(compare) || !self.call_total(compare)) && !self.sorts_as_js_strings(compare, item)
            }
            SortWith::Key => {
                let key = generic_args.type_at(1);
                let order = self.order_seen(key) || !self.order_total(key);
                self.call_seen(generic_args.type_at(2)) || order || self.has_drops(key)
            }
        }
    }

    /// `$sortStable(items, isLess, kind, size)`: std's stable sort (#925),
    /// given `is_less` as std's sort of `with` asks it, and what std picks
    /// by the item's type and its size, which sizes its scratch.
    pub(in crate::lower) fn stable_sort_call(
        &mut self,
        with: SortWith,
        values: Vec<Expr>,
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let item = generic_args.type_at(0);
        // Of a type whose layout is known here, though it holds a type
        // parameter behind a reference, `(&T, usize)`: its own; else its
        // caller's.
        let (kind, size) = match self.tcx.layout_of(self.typing_env.as_query_input(item)) {
            Ok(layout) if item.has_param() => (
                Expr::str(self.stable_sort_kind_of(item, layout.size.bytes())),
                Expr::int(layout.size.bytes().into()),
            ),
            _ => (
                self.type_fact_value(item, TypeFact::StableSort, span)?,
                self.type_fact_value(item, TypeFact::Size, span)?,
            ),
        };
        let mut values = values.into_iter();
        let items = values.next().expect("the items");
        let key_ty = (with == SortWith::Key).then(|| generic_args.type_at(1));
        let is_less = self.sort_is_less(with, values.next(), item, key_ty, span, out)?;
        Ok(Expr::call(
            Expr::helper("$sortStable"),
            vec![items, is_less, kind, size],
        ))
    }

    /// std's `is_less` of a sort of `item`s by `with`, given its comparator
    /// or key: `(a, b) => a < b` by their `PartialOrd::lt`; a comparator's
    /// `compare(a, b) < 0`, of a closure of one expression in place, `(a,
    /// b) => $cmp(b, a) < 0`; a key's `key(a) < key(b)`, by its `lt`.
    fn sort_is_less(
        &mut self,
        with: SortWith,
        by: Option<Expr>,
        item: ty::Ty<'tcx>,
        key_ty: Option<ty::Ty<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut body = Vec::new();
        let (a, b) = (Expr::var("a"), Expr::var("b"));
        let less = match with {
            SortWith::Order => self.less_value(a, b, item, span, &mut body)?,
            SortWith::By => {
                let compare = by.expect("rustc checked the arguments");
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
                    return Ok(Expr::arrow(params.clone(), returned));
                }
                let compare = self.named_once("compare", compare, out);
                Expr::bin(Op::Lt, Expr::call(compare, vec![a, b]), Expr::int(0))
            }
            SortWith::Key => {
                let key = self.named_once("key", by.expect("rustc checked the arguments"), out);
                let key_ty = key_ty.expect("`_by_key` names its key");
                let (a, b) = (Expr::call(key.clone(), vec![a]), Expr::call(key, vec![b]));
                self.less_value(a, b, key_ty, span, &mut body)?
            }
        };
        body.push(StmtKind::Return(Some(less)).at(js::Span::NONE));
        Ok(Expr::arrow(vec!["a".into(), "b".into()], body))
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
        kind: Expr,
        item: ty::Ty<'tcx>,
    ) -> Expr {
        let select = matches!(op, SliceOp::SelectNth { .. });
        let mut list = vec![items];
        list.extend(index);
        list.extend([is_less, kind]);
        if select && self.handles_mut(item) {
            list.push(Expr::bool(true));
        }
        let name = if select { "$selectNthUnstable" } else { "$sortUnstable" };
        Expr::call(Expr::helper(name), list)
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

    /// An unstable sort or a selection, `v.sort_unstable_by(f)`, of its
    /// operands as JS: JS's stable sort where ties can't be told apart, else
    /// std's own, picked by the item's type (ADR 0342).
    pub(in crate::lower) fn unstable_sort_values(
        &mut self,
        op: SliceOp,
        with: SortWith,
        values: Vec<Expr>,
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let item = generic_args.type_at(0);
        let kind = self.type_fact_value(item, TypeFact::Sort, span)?;
        let mut values = values.into_iter();
        let mut next = || values.next().expect("rustc checked the arguments");
        let items = next();
        let index = matches!(op, SliceOp::SelectNth { .. }).then(&mut next);
        // By UTF-16 units, JS's own order of strings, whose ties are the
        // same string, which no order of them tells apart (ADR 0945).
        if matches!(op, SliceOp::SortUnstable { .. })
            && with == SortWith::By
            && self.sorts_as_js_strings(generic_args.type_at(1), item)
        {
            return Ok(Expr::call(Expr::member(items, "sort"), vec![]));
        }
        let by = (with != SortWith::Order).then(next);
        let key_ty = (with == SortWith::Key).then(|| generic_args.type_at(1));
        let is_less = self.sort_is_less(with, by, item, key_ty, span, out)?;
        Ok(self.unstable_sort_call(op, items, index, is_less, kind, item))
    }

    /// `v.extract_if(range, f)`: the items `f` holds of, from `start` to `end`,
    /// taken out (`$extractIf`), each in a box where an `Option` of it would
    /// look like `None`, of `item`.
    pub(in crate::lower) fn extract_if_values(
        &mut self,
        items: Expr,
        (start, end): (Expr, Option<Expr>),
        f: Expr,
        item: ty::Ty<'tcx>,
    ) -> Expr {
        let mut list = vec![items, start, end.unwrap_or_else(Expr::undefined), f];
        if self.handles_mut(item) {
            list.push(Expr::bool(true));
        }
        Expr::call(Expr::helper("$extractIf"), list)
    }

    /// `slots.write_copy_of_slice(src)`, of `values`, the slots and the
    /// source: `$copyFromSlice(slots, src)`, then `slots`; a clone of each
    /// from `$writeCloneOfSlice`, whose `assert_eq!` says why it panics.
    pub(in crate::lower) fn uninit_write_values(
        &mut self,
        clone: bool,
        values: Vec<Expr>,
        item: ty::Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut values = values.into_iter();
        let (slots, source) = (values.next().expect("the slots"), values.next().expect("the source"));
        let slots = if slots.reads_same() {
            slots
        } else {
            self.spill("slots", slots, out)
        };
        if clone {
            let clone = self.clone_arg(item, span)?;
            return Ok(Expr::call(
                Expr::helper("$writeCloneOfSlice"),
                vec![slots, source, clone],
            ));
        }
        let copy = Expr::call(Expr::helper("$copyFromSlice"), vec![slots.clone(), source]);
        out.push(StmtKind::Expr(copy).at(self.js_span(span)));
        Ok(slots)
    }

    /// `s.split_off(..n)` and the like of a slice read as `read`, with the
    /// range's bounds, or `split_off_first`'s or `last`'s, `end`: a `const`
    /// of what's left, `[0]`, which the slice's place is given, and what it
    /// takes, `[1]` (ADR 0338).
    pub(in crate::lower) fn split_off_values(
        &mut self,
        (end, mutable): (Option<bool>, bool),
        read: Expr,
        bounds: Option<(Expr, Option<Expr>)>,
        item: ty::Ty<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> Expr {
        let mut list = vec![read];
        match (end, bounds) {
            (None, Some((_, Some(n)))) => list.push(n),
            (None, Some((n, None))) => list.extend([n, Expr::bool(true)]),
            (Some(last), _) => {
                list.push(Expr::bool(last));
                if mutable && self.handles_mut(item) {
                    list.push(Expr::bool(true));
                }
            }
            (None, None) => unreachable!("a range's bounds"),
        }
        let name = match (end, mutable) {
            (None, false) => "$sliceSplitOff",
            (None, true) => "$sliceSplitOffMut",
            (Some(_), false) => "$sliceSplitOffEnd",
            (Some(_), true) => "$sliceSplitOffEndMut",
        };
        let split = match mutable {
            false => Expr::call(Expr::helper(name), list),
            true => self.view_call(name, list),
        };
        self.spill("split", split, out)
    }

    /// `v.copy_within(start..end, dest)`: `$copyWithin`.
    pub(in crate::lower) fn copy_within_values(
        &mut self,
        items: Expr,
        (start, end): (Expr, Option<Expr>),
        dest: Expr,
    ) -> Expr {
        Expr::call(
            Expr::helper("$copyWithin"),
            vec![items, start, end.unwrap_or_else(Expr::undefined), dest],
        )
    }

    /// One of `SliceOp`'s that its operands, as JS, say all of, `values`;
    /// `stepping`, of `Remainder`, whether what it's called on is stepped
    /// through a `&mut`, a `$iter`.
    pub(in crate::lower) fn slice_values(
        &mut self,
        op: SliceOp,
        values: Vec<Expr>,
        stepping: bool,
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let helper = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        let item = || generic_args.type_at(0);
        let mut values = values.into_iter();
        let mut arg = || values.next().expect("rustc checked the arguments");
        Ok(match op {
            SliceOp::GenericIndex { mutable } => {
                let (items, index) = (arg(), arg());
                let item = match item().kind() {
                    ty::Slice(item) | ty::Array(item, _) => *item,
                    ty::Adt(_, args) => args.type_at(0),
                    _ => return Err(self.unsupported(span, "this index")),
                };
                let slice = ty::Ty::new_slice(self.tcx, item);
                let slice_index =
                    crate::lower::recognition::std_item(self.tcx, crate::lower::recognition::StdItem::SliceIndex);
                let tr = ty::TraitRef::new(self.tcx, slice_index, [generic_args.type_at(1), slice]);
                let dictionary = self.dictionary(tr, span)?;
                let method = if mutable { "index_mut" } else { "index" };
                let method = crate::lower::bindings::trait_method_name(self.tcx, slice_index, method);
                let indexed = Expr::call(Expr::member(dictionary, method), vec![index, items]);
                // `index_mut` gives generic code's `&mut`, a box: where what
                // it gives is known here, a bound's `IndexMut<J, Output =
                // [u32]>`, what's in it, as a generic function's result is
                // (ADR 0534).
                let index_trait = self
                    .tcx
                    .require_lang_item(rustc_hir::attrs::lang_items::LangItem::Index, span);
                let output = crate::lower::recognition::trait_method(self.tcx, index_trait, "Output");
                let output = ty::Ty::new_projection(
                    self.tcx,
                    ty::IsRigid::No,
                    output,
                    [generic_args.type_at(0), generic_args.type_at(1)],
                );
                let known = (self.tcx)
                    .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(output))
                    .ok()
                    .filter(|&ty| !crate::lower::recognition::known_by_bounds(ty));
                match known {
                    Some(output) if mutable && !self.mut_ref_form(output).is_cell() => Expr::held(indexed),
                    _ => indexed,
                }
            }
            // A clone of the value in each but the last, by its type's
            // `clone_from` where it may have one, as std's is; of items with
            // a destructor, each it replaces dropped (ADR 0507). One whose
            // copies can't be told apart, JS's: its `clone_from` is std's.
            SliceOp::Fill => {
                let (items, value) = (arg(), arg());
                match self.needs_clone(item()) {
                    true => {
                        let (clone, clone_from) = self.clone_over(item(), "fill", span)?;
                        let drop = self.std_drop_function(item(), span)?;
                        helper(
                            "$fill",
                            Expr::optional_args(vec![items, value], vec![clone, clone_from, drop]),
                        )
                    }
                    false => Expr::call(Expr::member(items, "fill"), vec![value]),
                }
            }
            // Each made, then what it replaces dropped (ADR 0507).
            SliceOp::FillWith => {
                let (items, f) = (arg(), arg());
                let drop = self.std_drop_function(item(), span)?;
                helper("$fillWith", Expr::optional_args(vec![items, f], vec![drop]))
            }
            // Each a copy where something changes one in place.
            SliceOp::CopyFromSlice => {
                let (items, source) = (arg(), arg());
                let copy = match self.needs_clone(item()) {
                    true => Some(self.clone_fn("item", item(), span)?),
                    false => None,
                };
                helper("$copyFromSlice", Expr::optional_args(vec![items, source], vec![copy]))
            }
            // Each by its type's `clone_from` where it may have one, as std's
            // calls it, else a clone of each; of items with a destructor,
            // each it replaces dropped (ADR 0507).
            SliceOp::CloneFromSlice => {
                let (items, source) = (arg(), arg());
                let (clone, clone_from) = self.clone_over(item(), "clone_from_slice", span)?;
                let drop = self.std_drop_function(item(), span)?;
                helper(
                    "$cloneFromSlice",
                    Expr::optional_args(vec![items, source], vec![clone, clone_from, drop]),
                )
            }
            SliceOp::SwapWithSlice => helper("$swapWithSlice", vec![arg(), arg()]),
            SliceOp::DequeSlices => Expr::array(vec![arg(), Expr::array(Vec::new())]),
            // Each two in a row, by their `PartialOrd`, `is_sorted_call`'s.
            SliceOp::IsSorted => self.is_sorted_call(arg(), item(), None, span)?,
            SliceOp::IsSortedBy => helper("$isSortedBy", vec![arg(), arg()]),
            SliceOp::IsSortedByKey => {
                let (items, key) = (arg(), arg());
                // `is_sorted_by_key<'a, F, K>`: its key, the third type.
                let key_ty = generic_args.types().nth(2).expect("its key's type");
                let cmp = self.cmp_fn(key_ty, true, span)?;
                // A key with a destructor, given its drop, which std's
                // `self.iter().is_sorted_by_key(f)` drops as it replaces
                // one and where two are out of order (#783).
                let mut args = vec![items, key, cmp];
                args.extend(self.std_drop_function(key_ty, span)?);
                helper("$isSortedByKey", args)
            }
            SliceOp::PartitionPoint => helper("$partitionPoint", vec![arg(), arg()]),
            SliceOp::ChunksExact { mutable: false } => helper("$chunksExact", vec![arg(), arg()]),
            SliceOp::Rchunks { exact, mutable: false } => helper("$rchunks", vec![arg(), arg(), Expr::bool(exact)]),
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
                if self.handles_mut(item()) {
                    list.push(Expr::bool(true));
                }
                self.view_call("$splitEndMut", list)
            }
            SliceOp::ChunkBy { mutable: false } => helper("$chunkBy", vec![arg(), arg()]),
            SliceOp::ChunkBy { mutable: true } => self.view_call("$chunkByMut", vec![arg(), arg()]),
            SliceOp::SplitChunk { last, mutable } => {
                let n = self.chunk_length(generic_args, span)?;
                let list = vec![arg(), Expr::int(n as i128), Expr::bool(last)];
                match mutable {
                    false => helper("$splitChunk", list),
                    true => self.view_call("$splitChunkMut", list),
                }
            }
            SliceOp::View { .. }
            | SliceOp::DequeRangeMut
            | SliceOp::ExtractIf { .. }
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
                let handle = index.is_usize() && self.handles_mut(item());
                let inclusive = self.range_kind(index) == Some(RangeKind::Inclusive);
                if handle || inclusive {
                    list.push(Expr::bool(handle));
                }
                if inclusive {
                    list.push(Expr::bool(true));
                }
                let result = self.view_call("$getDisjointMut", list);
                match unchecked {
                    true => crate::js::result::held(result),
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
                Expr::call(Expr::helper("$windows"), vec![arg(), Expr::int(n as i128)])
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
                    false => helper("$asChunks", list),
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
            SliceOp::Remainder if stepping => Expr::member(Expr::member(arg(), "items"), "remainder"),
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
                    false => helper("$sliceSplitBy", list),
                    true => self.view_call("$sliceSplitByMut", list),
                }
            }
            // std's sorts the `(key, index)` pairs unstably, which is
            // ipnsort's sequence of the keys' `partial_cmp` where that can
            // be seen, given what it picks by the pair's type (#925).
            SliceOp::SortByCachedKey => {
                let (items, key) = (arg(), arg());
                // `sort_by_cached_key<K, F>`: its key, the second type.
                let key_ty = generic_args.type_at(1);
                match self.order_seen(key_ty) {
                    true => {
                        let cmp = self.cmp_fn(key_ty, true, span)?;
                        let pair = ty::Ty::new_tup(self.tcx, &[key_ty, self.tcx.types.usize]);
                        let kind = self.type_fact_value(pair, TypeFact::Sort, span)?;
                        helper("$sortByCachedKey", vec![items, key, cmp, kind])
                    }
                    false => {
                        let cmp = self.cmp_fn(key_ty, false, span)?;
                        helper("$sortByCachedKey", vec![items, key, cmp])
                    }
                }
            }
            SliceOp::AsciiCase { upper, in_place } => {
                let name = if in_place { "$makeAsciiBytes" } else { "$asciiBytes" };
                helper(name, vec![arg(), Expr::bool(upper)])
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
                helper("$trimAsciiBytes", vec![arg(), Expr::bool(start), Expr::bool(end)])
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
                list.extend(self.item_eq((item(), item()), span)?);
                helper(name, list)
            }
            SliceOp::Repeat => {
                let (items, n) = (arg(), arg());
                let copy = match self.needs_clone(item()) {
                    true => Some(self.clone_fn("item", item(), span)?),
                    false => None,
                };
                // Room for them, which std checks first (ADR 0774).
                let size = self.room_item_size(item());
                helper("$repeatItems", Expr::optional_args(vec![items, n, size], vec![copy]))
            }
            SliceOp::FromRef => Expr::array(vec![arg()]),
            SliceOp::FromFn => {
                // `from_fn::<T, N, F>`: its length, the second.
                let n = generic_args.const_at(1);
                let length = match n.try_to_target_usize(self.tcx) {
                    Some(n) => Expr::int(i128::from(n)),
                    None => self.const_arg(n, span)?,
                };
                // Given the index, second, as `Array.from` gives it: an arrow
                // of one parameter takes it after `_`.
                let made = match arg() {
                    Expr {
                        kind: js::ExprKind::Arrow(mut params, body),
                        ..
                    } if params.len() == 1 => {
                        params.insert(0, "_".into());
                        Expr::arrow(params, body)
                    }
                    f => {
                        let i = self.fresh("i");
                        let call = Expr::call(f, vec![Expr::var(&i)]);
                        Expr::arrow(
                            vec!["_".into(), i.into()],
                            vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
                        )
                    }
                };
                let length = Expr::object(vec![js::Prop::Field("length".into(), length)]);
                Expr::call(Expr::member(Expr::var("Array"), "from"), vec![length, made])
            }
            SliceOp::PushMut { at } => {
                let mut given = vec![arg(), arg()];
                if at {
                    given.push(arg());
                }
                if self.handles_mut(item()) {
                    given.push(Expr::bool(true));
                }
                helper(if at { "$insertMut" } else { "$pushMut" }, given)
            }
            SliceOp::PushFrontMut => {
                let mut given = vec![arg(), arg()];
                if self.handles_mut(item()) {
                    given.push(Expr::bool(true));
                }
                helper("$pushFrontMut", given)
            }
            SliceOp::DequeInsert { mutable } => {
                let mut given = vec![arg(), arg(), arg()];
                if mutable && self.handles_mut(item()) {
                    given.push(Expr::bool(true));
                }
                helper("$dequeInsert", given)
            }
            SliceOp::DequeSwapRemove { front } => helper("$dequeSwapRemove", vec![arg(), arg(), Expr::bool(front)]),
            // What a `Vec`'s `retain_mut` doesn't keep, dropped as it goes
            // (ADR 0499); a `VecDeque`'s, std's, each kept swapped forward,
            // then what's past them truncated (ADR 0499, amended, #909).
            SliceOp::RetainMut { deque } => {
                let handles = self.handles_mut(item());
                let mut given = vec![arg(), arg(), Expr::bool(handles)];
                given.extend(self.std_drop_function(item(), span)?);
                helper(if deque { "$retainDeque" } else { "$retainMut" }, given)
            }
            SliceOp::PopFrontIf => {
                let handles = self.handles_mut(item());
                helper("$popFrontIf", vec![arg(), arg(), Expr::bool(handles)])
            }
            SliceOp::CopyWithin => unreachable!("lowered above"),
        })
    }
}
