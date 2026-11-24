//! A slice's fills, copies, order checks, chunks and splits (ADR 0324): its
//! JS array, changed in place where std's is.

use crate::temporary_names::Role;
use rustc_middle::ty;
use rustc_middle::ty::consts::ConstExt as _;
use rustc_span::Span;

use crate::recognition::{Std, TypeFact};
use crate::representation::Num;
use crate::std_types::range::RangeKind;
use crate::{FnCx, R};
use rust_js_program::js::{self, Expr, Op, Stmt, StmtKind};

/// The helper that does what `known` does to a view of part of a slice,
/// given the slice and where the part starts and ends, `(v, start, end?)`,
/// where it has one: JS has no method over part of an array, so a view's
/// is the slice's own, in place, no view made (ADR 1073).
pub fn ranged(known: Std) -> Option<&'static str> {
    match known {
        Std::Method("reverse") => Some("$reverseRange"),
        _ => None,
    }
}

/// What a sort orders by: its items' own order, a comparator (`_by`), or a
/// key (`_by_key`).
#[derive(Clone, Copy, PartialEq)]
pub enum SortWith {
    Order,
    By,
    Key,
}

/// A slice's own methods, of those ADR 0324 takes.
#[derive(Clone, Copy, PartialEq)]
pub enum SliceOp {
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
    /// `slice::range` checks it (#1937), a handle on each number or text, as
    /// `iter_mut`'s.
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
    /// `array::from_fn(f)` and, `given_length`, `Vec::from_fn(n, f)`, Rust
    /// 1.100's: `f` given each index, `Array.from({ length: N }, (_, i) =>
    /// ..)`; of items with a destructor, collected as `(0..N).map(f)` is,
    /// what it made dropped where `f` panics (#2278, #1656).
    FromFn {
        given_length: bool,
    },
    /// `slice::from_ref(&x)` and `array::from_ref(&x)`: `[x]`, of one item
    /// it shares.
    FromRef,
    /// `slice::from_mut(&mut x)` and `array::from_mut(&mut x)` of a number
    /// or text: `$oneItemOf(h)`, a view of one item over the handle `h` on
    /// `x`. Of an object, refused (#1336).
    FromMut,
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
    pub(crate) fn lends(self) -> bool {
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
                // A deque's two halves, and a slice's chunks and its first
                // or last `N` items, each a view of what it holds (#1274).
                | SliceOp::DequeSlices
                | SliceOp::ChunksExact { .. }
                | SliceOp::Rchunks { .. }
                | SliceOp::Chunk { .. }
        )
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether a slice's search's predicate, its method's last type, runs
    /// the program's code (`pattern_runs_code`, #1501).
    fn predicate_runs_code(&self, generic_args: ty::GenericArgsRef<'tcx>) -> bool {
        (generic_args.types().last()).is_some_and(|predicate| self.recognition().pattern_runs_code(predicate))
    }

    /// A call of one of the views' helpers (ADR 0335).
    pub fn view_call(&mut self, name: &'static str, list: Vec<Expr>) -> Expr {
        Expr::call(Expr::helper(name), list)
    }

    /// `view_call`'s, made where a view unmade is used, its panic raised
    /// where it was indexed, `raised` (`Value::View`, #2292).
    pub fn raised_view_call(&mut self, name: &'static str, list: Vec<Expr>, raised: Option<Expr>) -> Expr {
        match raised {
            Some(at) => Expr::raised_at(at, || self.view_call(name, list)),
            None => self.view_call(name, list),
        }
    }

    /// How a slice's `starts_with` and `strip_*` compare its items (ADR
    /// 0336), and an iterator's `eq` another's, `item` to `rhs`: `None` where
    /// JS's `===` is their `==`, numbers, text and `bool`s, or a function of
    /// two of them their own `==` is, a `PartialEq<Rhs>`'s too. A slice's,
    /// `by_ne`, compares by std's slice `==`, which asks `!(a != b)` of
    /// each two, an impl's own `ne` included (#962); an iterator's `eq`
    /// asks `==`.
    pub(crate) fn item_eq(
        &mut self,
        (item, rhs): (ty::Ty<'tcx>, ty::Ty<'tcx>),
        by_ne: bool,
        span: Span,
    ) -> R<Option<Expr>> {
        let made_b = self.unspelled("b");
        let made_a = self.unspelled("a");
        if item == rhs
            && (self.eq_is_identity(item) || (!self.has_cell_layer(item) && Num::of(item.peel_refs()).is_some()))
        {
            return Ok(None);
        }
        let mut body = Vec::new();
        let (a, b) = (Expr::var(&made_a), Expr::var(&made_b));
        let same = match by_ne && item.peel_refs() == rhs.peel_refs() && self.own_ne(item) {
            true => Expr::unary(js::UnaryOp::Not, self.ne_value(a, b, item, span, &mut body)?),
            false => self.eq_values(a, b, (item, rhs), false, span, &mut body)?,
        };
        // `(a, b) => $eq(a, b)` is `$eq`: the helper gives it the two alone.
        if body.is_empty()
            && let js::ExprKind::Call(callee, given) = &same.kind
            && matches!(callee.kind, js::ExprKind::Var(_))
            && let [a, b] = given.as_slice()
            && matches!((&a.kind, &b.kind), (js::ExprKind::Var(a), js::ExprKind::Var(b)) if *a == made_a && *b == made_b)
        {
            return Ok(Some(*callee.clone()));
        }
        body.push(StmtKind::Return(Some(same)).at(js::Span::NONE));
        Ok(Some(Expr::arrow(
            vec![made_a.clone().into(), made_b.clone().into()],
            body,
        )))
    }

    /// `value` where it's read more than once: a variable, or a `const`.
    pub(super) fn named_once(&mut self, base: Role, value: Expr, out: &mut Vec<Stmt>) -> Expr {
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

    /// `v.extract_if(range, f)`: the items `f` holds of, from `start` to `end`,
    /// taken out (`$extractIf`), each in a box where an `Option` of it would
    /// look like `None`, of `item`.
    pub fn extract_if_values(
        &mut self,
        items: Expr,
        (start, end, inclusive): (Expr, Option<Expr>, Option<Expr>),
        f: Expr,
        item: ty::Ty<'tcx>,
    ) -> Expr {
        let mut list = vec![items, start, end.unwrap_or_else(Expr::js_undefined), f];
        let handle = self.handles_mut(item);
        if handle || inclusive.is_some() {
            list.push(Expr::bool(handle));
        }
        list.extend(inclusive);
        // `slice::range`'s panic, where std's `ExtractIf::new` checks (#2292).
        Expr::raised_at(Expr::std_site("vecExtractIf", 0), || {
            Expr::call(Expr::helper("$extractIf"), list)
        })
    }

    /// `slots.write_copy_of_slice(src)`, of `values`, the slots and the
    /// source: `$copyFromSlice(slots, src)`, then `slots`; a clone of each
    /// from `$writeCloneOfSlice`, whose `assert_eq!` says why it panics.
    pub fn uninit_write_values(
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
            self.spill(Role::Slots, slots, out)
        };
        if clone {
            let clone = self.clone_arg(item, span)?;
            return Ok(Expr::call(
                Expr::helper("$writeCloneOfSlice"),
                vec![slots, source, clone],
            ));
        }
        // `copy_from_slice`'s panic, where std's `write_copy_of_slice` calls
        // it (#2292).
        let copy = Expr::raised_at(Expr::std_site("writeCopyOfSlice", 0), || {
            Expr::call(Expr::helper("$copyFromSlice"), vec![slots.clone(), source])
        });
        out.push(StmtKind::Expr(copy).at(self.js_span(span)));
        Ok(slots)
    }

    /// `s.split_off(..n)` and the like of a slice read as `read`, with the
    /// range's bounds, or `split_off_first`'s or `last`'s, `end`: a `const`
    /// of what's left, `[0]`, which the slice's place is given, and what it
    /// takes, `[1]` (ADR 0338).
    pub fn split_off_values(
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
        self.spill(Role::Split, split, out)
    }

    /// `v.copy_within(start..end, dest)`: `$copyWithin`.
    pub fn copy_within_values(
        &mut self,
        items: Expr,
        (start, end, inclusive): (Expr, Option<Expr>, Option<Expr>),
        dest: Expr,
    ) -> Expr {
        let list = vec![items, start, end.unwrap_or_else(Expr::js_undefined), dest];
        Expr::call(Expr::helper("$copyWithin"), list.into_iter().chain(inclusive).collect())
    }

    /// One of `SliceOp`'s that its operands, as JS, say all of, `values`;
    /// `stepping`, of `Remainder`, whether what it's called on is stepped
    /// through a `&mut`, a `$iter`.
    pub fn slice_values(
        &mut self,
        op: SliceOp,
        values: Vec<Expr>,
        stepping: bool,
        generic_args: ty::GenericArgsRef<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let made_b = self.unspelled("b");
        let helper = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        let item = || generic_args.type_at(0);
        let mut values = values.into_iter();
        let mut arg = || values.next().expect("rustc checked the arguments");
        // Of items std clones as their bits, `clone_from_slice` is std's
        // specialization's `copy_from_slice`, its message too (#2318).
        let op = match op {
            SliceOp::CloneFromSlice
                if crate::std_impls::trivially_cloned(self.tcx, self.recognition().typing_env, item()) =>
            {
                SliceOp::CopyFromSlice
            }
            op => op,
        };
        Ok(match op {
            SliceOp::GenericIndex { mutable } => {
                let (items, index) = (arg(), arg());
                let item = match item().kind() {
                    ty::Slice(item) | ty::Array(item, _) => *item,
                    ty::Adt(_, args) => args.type_at(0),
                    _ => return Err(self.unsupported(span, "this index")),
                };
                let slice = ty::Ty::new_slice(self.tcx, item);
                let slice_index = crate::recognition::std_item(self.tcx, crate::recognition::StdItem::SliceIndex);
                let tr = ty::TraitRef::new(self.tcx, slice_index, [generic_args.type_at(1), slice]);
                let dictionary = self.dictionary(tr, span)?;
                let method = if mutable { "index_mut" } else { "index" };
                // std's impls track their caller: where generic code indexes,
                // its panic's place (ADR 0427, #2324).
                let mut given = vec![index, items];
                // Its declaration tracks its caller, as rustc's every impl of
                // it does (`tracked_without_default`, #2350).
                let index_fn = crate::recognition::trait_method(self.tcx, slice_index, method);
                if self.takes_caller_location(index_fn) {
                    given.push(self.caller_location(span));
                }
                let method = crate::bindings::trait_method_name(self.tcx, slice_index, method);
                let indexed = Expr::call(Expr::member(dictionary, method), given);
                // `index_mut` gives generic code's `&mut`, a box: where what
                // it gives is known here, a bound's `IndexMut<J, Output =
                // [u32]>`, what's in it, as a generic function's result is
                // (ADR 0534).
                let index_trait = self
                    .tcx
                    .require_lang_item(rustc_hir::attrs::lang_items::LangItem::Index, span);
                let output = crate::recognition::trait_method(self.tcx, index_trait, "Output");
                let output = ty::Ty::new_projection(
                    self.tcx,
                    ty::IsRigid::No,
                    output,
                    [generic_args.type_at(0), generic_args.type_at(1)],
                );
                let known = (self.tcx)
                    .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(output))
                    .ok()
                    .filter(|&ty| !crate::recognition::known_by_bounds(ty));
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
            // Each a copy where something changes one in place: its bits,
            // `Copy`'s, as std's `memcpy` copies them, never a hand-written
            // `Clone` of a `Copy` type; and `clone_from_slice`'s of items std
            // clones as their bits, `TrivialClone`'s (#2318).
            SliceOp::CopyFromSlice => {
                let (items, source) = (arg(), arg());
                let copy = self.bits_copy_fn(item());
                helper("$copyFromSlice", Expr::optional_args(vec![items, source], vec![copy]))
            }
            // Each by its type's `clone_from` where it may have one, as std's
            // calls it, else a clone of each; of items with a destructor,
            // each it replaces dropped (ADR 0507). Of items only the caller
            // says std clones as their bits, its `TTriviallyCloned`, which
            // picks `copy_from_slice`'s message (#2318).
            SliceOp::CloneFromSlice => {
                let (items, source) = (arg(), arg());
                let (clone, clone_from) = self.clone_over(item(), "clone_from_slice", span)?;
                let drop = self.std_drop_function(item(), span)?;
                let fact = TypeFact::TriviallyCloned;
                let trivial = match crate::recognition::fact_trait(self.tcx, fact) {
                    Some(trivial) if self.recognition().trait_holds(item(), trivial).is_none() => {
                        Some(self.type_fact_value(item(), fact, span)?)
                    }
                    _ => None,
                };
                helper(
                    "$cloneFromSlice",
                    Expr::optional_args(vec![items, source], vec![clone, clone_from, drop, trivial]),
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
            SliceOp::SplitAtMut { checked } => self.view_call(
                if checked { "$checkedSplitAtMut" } else { "$splitAtMut" },
                vec![arg(), arg()],
            ),
            SliceOp::SplitEndMut { last } => {
                let mut list = vec![arg(), Expr::bool(last)];
                if self.handles_mut(item()) {
                    list.push(Expr::bool(true));
                }
                self.view_call("$splitEndMut", list)
            }
            SliceOp::ChunkBy { mutable } => {
                let list = vec![arg(), arg()];
                match (mutable, self.predicate_runs_code(generic_args)) {
                    (false, false) => helper("$chunkBy", list),
                    (false, true) => helper("$lazyChunkBy", list),
                    (true, false) => self.view_call("$chunkByMut", list),
                    (true, true) => self.view_call("$lazyChunkByMut", list),
                }
            }
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
                    true => rust_js_program::js::result::held(result),
                    false => result,
                }
            }
            SliceOp::AsArray => {
                let n = self.chunk_length(generic_args, span)?;
                let items = arg();
                let items = if items.reads_same() {
                    items
                } else {
                    self.spill(Role::Items, items, out)
                };
                let length = Expr::member(items.clone(), "length");
                Expr::cond(
                    Expr::bin(Op::Eq, length, Expr::int(n as i128)),
                    items,
                    Expr::js_undefined(),
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
                    self.spill(Role::Items, items, out)
                };
                let length = Expr::member(items.clone(), "length");
                let chunk = match (last, mutable) {
                    (false, false) => self.slice_part(items, Expr::int(0), Some(Expr::int(n as i128))),
                    (true, false) => self.slice_part(items, Expr::int(-(n as i128)), None),
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
                    Expr::js_undefined(),
                )
            }
            SliceOp::SplitBy {
                limited,
                inclusive,
                back,
                mutable,
            } => {
                let items = arg();
                let limit = if limited { arg() } else { Expr::js_undefined() };
                let predicate = arg();
                let list = vec![items, predicate, limit, Expr::bool(inclusive), Expr::bool(back)];
                // Of a predicate that runs the program's code, std's iterator,
                // asked as it's stepped (#1501); else the array of its pieces.
                match (mutable, self.predicate_runs_code(generic_args)) {
                    (false, false) => helper("$sliceSplitBy", list),
                    (false, true) => helper("$lazySliceSplitBy", list),
                    (true, false) => self.view_call("$sliceSplitByMut", list),
                    (true, true) => self.view_call("$lazySliceSplitByMut", list),
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
                        let cmp = self.cmp_fn(key_ty, true, span)?;
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
                    vec![made_b.clone().into()],
                    vec![
                        rust_js_program::js::StmtKind::Return(Some(Expr::bin(
                            Op::Lt,
                            Expr::var(&made_b),
                            Expr::int(128),
                        )))
                        .at(rust_js_program::js::Span::NONE),
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
                list.extend(self.item_eq((item(), item()), true, span)?);
                helper(name, list)
            }
            // Its first copy each item's clone, as std's `extend(self)` makes
            // it, the rest its bits' copies, `copy_nonoverlapping`'s (ADR
            // 0623, amended).
            SliceOp::Repeat => {
                let (items, n) = (arg(), arg());
                let clone = match self.needs_clone(item()) {
                    true => Some(self.clone_fn("item", item(), span)?),
                    false => None,
                };
                let copy = self.bits_copy_fn(item());
                // Room for them, which std checks first (ADR 0774).
                let size = self.room_item_size(item());
                // Its clone given only where it isn't its copy, a hand-written
                // `clone`'s, the two alike but for their parameters' names
                // (#1681).
                let clone = clone.filter(|clone| !copy.as_ref().is_some_and(|copy| js::scopes::alpha_eq(copy, clone)));
                let copy = copy.or_else(|| clone.is_some().then(Expr::js_undefined));
                helper(
                    "$repeatItems",
                    Expr::optional_args(vec![items, n, size], vec![copy, clone]),
                )
            }
            SliceOp::FromRef => Expr::array(vec![arg()]),
            // A view over the handle JS holds a `&mut` to a number or text as:
            // an object's `&mut` is the object, whose place `s[0] = other`
            // would have to replace (#1336).
            SliceOp::FromMut => {
                if !self.handles_mut(item()) {
                    return Err(self.unsupported(
                        span,
                        &format!(
                            "`slice::from_mut` of a `{}`, an object, whose place `s[0] = other` would replace (#1336)",
                            item()
                        ),
                    ));
                }
                helper("$oneItemOf", vec![arg()])
            }
            SliceOp::FromFn { given_length } => {
                // `Vec::from_fn(n, f)`'s, its first argument, read once, its
                // room asked first, as its collect asks it; `from_fn::<T, N,
                // F>`'s, its second generic argument.
                let length = match given_length {
                    true => {
                        let length = arg();
                        let length = match length.reads_same() {
                            true => length,
                            false => self.spill(Role::Length, length, out),
                        };
                        if let Some(size) = self.room_size_of(&length, item()) {
                            self.room_for((None, length.clone()), size, span, out);
                        }
                        length
                    }
                    false => {
                        let n = generic_args.const_at(1);
                        match n.try_to_target_usize(self.tcx) {
                            Some(n) => Expr::int(i128::from(n)),
                            None => self.const_arg(n, span)?,
                        }
                    }
                };
                let f = arg();
                // Of items with a destructor, collected as `(0..n).map(f)`
                // is, what's made dropped where `f` panics, as std's guard
                // drops it (#2278).
                if self.drops(item()) != crate::drops::Drops::Nothing {
                    let drop = self.drop_function(item(), span)?;
                    let made = Expr::call(
                        Expr::member(crate::std_types::range::lazy_range_of(Expr::int(0), length), "map"),
                        vec![f],
                    );
                    let mut list = vec![made];
                    list.extend(drop);
                    list.push(Expr::bool(true));
                    return Ok(helper("$collectDropping", list));
                }
                // Given the index, second, as `Array.from` gives it: an arrow
                // of one parameter takes it after `_`.
                let made = match f {
                    Expr {
                        kind: js::ExprKind::Arrow(mut params, body),
                        ..
                    } if params.len() == 1 => {
                        params.insert(0, js::Name::unread("_".to_string()).into());
                        Expr::arrow(params, body)
                    }
                    f => {
                        let i = js::Name::counter("i");
                        let call = Expr::call(f, vec![Expr::var(&i)]);
                        Expr::arrow(
                            vec![js::Name::unread("_".to_string()).into(), i.into()],
                            vec![StmtKind::Return(Some(call)).at(js::Span::NONE)],
                        )
                    }
                };
                let length = Expr::object(vec![js::Prop::Field("length".into(), length)]);
                Expr::builtin(Expr::js_global("Array"), "from", vec![length, made])
            }
            SliceOp::PushMut { at } => {
                let mut given = vec![arg(), arg()];
                if at {
                    given.push(arg());
                }
                let handle = self.handles_mut(item());
                // What `insert_mut` took, dropped as its index past the end
                // panics, as `insert`'s is (#1267, #1274).
                let drop = match at && self.drops(item()) != crate::drops::Drops::Nothing {
                    true => self.drop_function(item(), span)?,
                    false => None,
                };
                if handle || drop.is_some() {
                    given.push(Expr::bool(handle));
                }
                given.extend(drop);
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
                let handle = mutable && self.handles_mut(item());
                // What it took, dropped as its index past the end panics (#1171).
                let drop = match self.drops(item()) != crate::drops::Drops::Nothing {
                    true => self.drop_function(item(), span)?,
                    false => None,
                };
                if handle || drop.is_some() {
                    given.push(Expr::bool(handle));
                }
                given.extend(drop);
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
