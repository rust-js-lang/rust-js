//! When a collect asks its iterator's `size_hint`, and for what room
//! (#820): one model of std's paths, `SpecFromIter` of a `TrustedLen`
//! iterator, its hint first, `SpecFromIterNested` of another, its first
//! item then its hint, and `extend_desugared` as the `Vec` fills; and the
//! reserves of a `String`'s and a hash table's `extend`, by destination
//! and source. Every collect asks it, and so does the analysis of what a
//! grown `Vec`'s items' size is asked of a caller (`grows_by_callers_size`).
//! What the runtime's `$roomForHint` and `$reserveHint` are given is made
//! here alone.

use super::*;
use crate::collections::{IndexMap, IndexSet};
use crate::lower::recognition::iterator_type_of;
use crate::lower::recognition::{Recognition, TypeFact, is_trusted_len};
use rustc_middle::mir::{Operand, TerminatorKind};
use rustc_middle::ty::TypeVisitableExt;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether std's `extend` of `ty` asks its iterator's hint as it fills:
    /// a `Vec`'s, `extend_desugared`, a `BinaryHeap`'s, its `Vec`'s, and a
    /// `VecDeque`'s, `spec_extend` (#849). Not a `LinkedList`'s, which asks
    /// none.
    pub(in crate::lower) fn grows_as_it_extends(&self, ty: Ty<'tcx>) -> bool {
        self.is_vec_like(ty) && !self.is_std_type(ty, crate::lower::recognition::StdItem::LinkedList)
    }

    /// Whether std's growth of a `Vec` by `instance`, an iterator a caller
    /// gives generic code that extends one, is kept in JS (`Hinted::lost`,
    /// #1180's (3)): as `refuse_unkept_lower_bound` asks of a type known
    /// where it grows, by a hint whose lower bound is std's as it's given;
    /// or by one that reaches the crate's own, which runs as it is, a `map`
    /// of its own exact-sized iterator's too.
    pub(in crate::lower) fn given_growth_kept(&self, instance: Ty<'tcx>) -> bool {
        self.given_hint_is_stds(instance) || self.hint_reaches_own(instance)
    }

    /// Whether `def_id` is an `extend` std's collection asks its hint of as
    /// it fills: a `Vec`'s, a `VecDeque`'s or a `BinaryHeap`'s.
    pub(in crate::lower) fn extend_grows(
        &self,
        def_id: rustc_span::def_id::DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
    ) -> bool {
        self.tcx
            .trait_of_assoc(def_id)
            .is_some_and(|t| crate::lower::recognition::is_extend(self.tcx, t))
            && generic_args
                .types()
                .next()
                .is_some_and(|ty| self.grows_as_it_extends(ty.peel_refs()))
    }

    /// Whether any of `bodies` extends a `Vec`, a `VecDeque` or a
    /// `BinaryHeap` by an iterator whose hint may run the crate's own,
    /// which std asks as the collection fills, as its capacity says: then
    /// each one's capacity is native Rust's where it's asked (ADR 0419,
    /// #849).
    pub(in crate::lower) fn extends_as_it_fills(&self, bodies: &[&crate::lower::Body<'tcx>]) -> bool {
        let extend = |func: &rustc_middle::mir::Operand<'tcx>,
                      args: &[rustc_span::Spanned<rustc_middle::mir::Operand<'tcx>>],
                      body: &rustc_middle::mir::Body<'tcx>| {
            let Some((def_id, generic_args)) = func.const_fn_def() else {
                return false;
            };
            let iterable = args.get(1).map(|arg| arg.node.ty(body, self.tcx));
            self.tcx
                .trait_of_assoc(def_id)
                .is_some_and(|t| crate::lower::recognition::is_extend(self.tcx, t))
                && generic_args
                    .types()
                    .next()
                    .is_some_and(|ty| self.grows_as_it_extends(ty.peel_refs()))
                && iterable.is_some_and(|iterable| {
                    let iter_ty = crate::lower::recognition::iterator_type_of(self.tcx, self.typing_env, iterable);
                    self.hint_reaches_own(iter_ty)
                })
        };
        (bodies.iter())
            .filter_map(|body| body.mir.as_ref().map(|mir| &mir.body))
            .any(|body| {
                body.basic_blocks.iter().any(|data| {
                    matches!(&data.terminator().kind,
                    rustc_middle::mir::TerminatorKind::Call { func, args, .. } if extend(func, args, body))
                })
            })
    }

    /// Whether a collect into `output` of an iterator of `iter_ty` grows a
    /// `Vec` of `item`s whose size is its caller's, a type parameter's or
    /// one built of them, `(T, u8)`, where the hint it asks as that `Vec`
    /// fills may run the crate's own (`hint_reaches_own`), or, of a crate
    /// that keeps capacities, `keeps_capacity`, where it isn't `TrustedLen`,
    /// as its capacity says (#1180): so the size its caller gives, `TSize`
    /// (#796). Of a collect into a `Result` or an `Option`, `shunted`, the
    /// collection std's `GenericShunt` collects and its values', `T` of
    /// `Result<T, E>` (#848), which isn't `TrustedLen`. The one answer of
    /// the lowering's `grown_item_size` and `shunted_ask` and the
    /// analysis's `grown_asked`.
    pub(in crate::lower) fn grows_by_callers_size(
        &self,
        output: Ty<'tcx>,
        iter_ty: Ty<'tcx>,
        item: Ty<'tcx>,
        (shunted, keeps_capacity): (bool, bool),
    ) -> bool {
        let trusted = !shunted && is_trusted_len(self.tcx, self.typing_env, iter_ty);
        self.grows_as_vec(output) && item.has_param() && (self.hint_reaches_own(iter_ty) || keeps_capacity && !trusted)
    }

    /// The items of a `Vec`, a `VecDeque` or a `BinaryHeap` that a call of
    /// `callee` given `args` adds to, `grows_vec`'s or an `extend`'s
    /// (`extend_grows`), or reserves room in, whose size std grows it by,
    /// or asks the capacity of, `usize::MAX` of items of no size (#1180):
    /// of a type parameter's, its caller's (`capacity_item_size`).
    pub(in crate::lower) fn grown_vec_item(
        &self,
        callee: rustc_span::def_id::DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Option<Ty<'tcx>> {
        use crate::lower::recognition::CapacityOp;
        let sized = matches!(
            crate::lower::recognition::capacity_op(self.tcx, callee),
            Some(CapacityOp::Of | CapacityOp::Reserve { exact: false })
        );
        if sized || crate::lower::recognition::grows_vec(self.tcx, callee).is_some() {
            return args.types().next();
        }
        if !self.extend_grows(callee, args) {
            return None;
        }
        match args.types().next()?.peel_refs().kind() {
            ty::Adt(_, held) => held.types().next(),
            _ => None,
        }
    }

    /// Of a collect into `output`, a `Result` or an `Option` of a
    /// collection, of `item`s, `Result<T, E>` or `Option<T>`: what std's
    /// `GenericShunt` collects, that collection of `T`s, as `try_process`
    /// makes it (#848). `None` of another.
    fn shunted(&self, output: Ty<'tcx>, item: Ty<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
        let first = |ty: Ty<'tcx>| match ty.kind() {
            ty::Adt(_, args) => args.types().next(),
            _ => None,
        };
        let collection = match self.option_of(output) {
            Some(collection) => collection,
            None if self.is_result(output) => first(output)?,
            None => return None,
        };
        Some((collection, first(item)?))
    }
}

/// What a reserve by an iterator's hint makes room for (`reserved_by_hint`).
pub(in crate::lower) enum Room<'tcx> {
    /// A `String`'s bytes.
    Bytes,
    /// A hash table's entries of this type, of a type parameter's of their
    /// count alone, as `table_room_for` asks it.
    Entries(Ty<'tcx>),
    /// None: the hint of a `GenericShunt`, whose lower bound is none (#848).
    Nothing,
}

/// What a collect into a `Result` or an `Option` of a collection asks of
/// its iterator's hint through std's `GenericShunt`, whose hint is its
/// source's, its lower bound none, `(0, upper)`, asked while no `Err` or
/// `None` has stopped it (#848).
pub(in crate::lower) enum Shunted {
    /// Nothing the program sees.
    Never,
    /// As a `Vec` of values of this size grows (`$shuntGrowth`).
    Grown(Expr),
    /// Once, before it's stepped, as a `String`'s or a hash table's
    /// `extend` reserves, room for none (`$reserveHint(it)`).
    Reserved,
}

/// What a collect into a `Vec` std grows asks of its iterator's hint, as
/// std's `SpecFromIter` does, where it asks any (`FnCx::grown_ask`).
pub(in crate::lower) struct Grown<'tcx> {
    /// What it grows, a `Vec` std makes, of `item`s.
    output: Ty<'tcx>,
    item: Ty<'tcx>,
    /// Whether it's `TrustedLen`, its hint asked before it's stepped.
    trusted: bool,
    /// Whether it has no end its type knows, or none once it gives an
    /// item, a `cycle()`'s: a `capacity overflow` panic (ADR 0940).
    endless: bool,
    /// Whether it's given knowing its length, `as_exact_size`'s, whose
    /// hint is then std's: of a crate that keeps capacities, an
    /// `ExactSizeIterator` std grows by its lower bound, which JS's own
    /// adapters, `drop(n)`'s, know nothing of (#1200).
    exact: bool,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Of a crate that keeps capacities, a `Vec` grown by an iterator of
    /// type `iter_ty` that isn't `TrustedLen`, by its hint's lower bound,
    /// refused where JS's isn't std's (`lower_bound_is_stds`), as its
    /// `size_hint()` is: so its capacity would be another (#1180). Not of
    /// one whose hint runs the crate's own, its own.
    fn refuse_unkept_lower_bound(&self, iter_ty: Ty<'tcx>, span: Span) -> R<()> {
        if self.hint_reaches_own(iter_ty) || self.recognition().lower_bound_is_stds(iter_ty) {
            return Ok(());
        }
        Err(self.unsupported(
            span,
            &format!(
                "a `Vec` grown by a `{iter_ty}` in a crate that reads a `Vec`'s capacity, which std grows by that iterator's `size_hint()`, whose lower bound JS's doesn't have"
            ),
        ))
    }

    /// What `collect()` of an iterator of type `iter_ty` into `output`, a
    /// `Vec` std grows (`grows_as_vec`), of `item`s, asks of its hint: of
    /// std's struct made here, `std_struct`, not bounded by an array's
    /// items, the room its hint asks first, as std's `SpecFromIter` does;
    /// of one whose type has no end, the `capacity overflow` a
    /// `TrustedLen`'s none asks (ADR 0940); of one whose hint may run the
    /// crate's own, when it's asked (#733). `None` where it asks nothing
    /// the program sees, a `LinkedList`'s, whose `from_iter` extends a new
    /// one, among them. Of another whose hint has no end once it gives an
    /// item, a `cycle()`'s, std steps it once, then asks room for its lower
    /// bound and one more, `usize::MAX`: a `capacity overflow` of items of
    /// any size, the item it made dropped (ADR 0940, amended); of items of
    /// no size, room for any number, so it goes on as JS does. Refused
    /// where its type doesn't say which.
    pub(in crate::lower) fn grown_ask(
        &self,
        output: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        iter_ty: Ty<'tcx>,
        std_struct: bool,
        span: Span,
    ) -> R<Option<Grown<'tcx>>> {
        let trusted = is_trusted_len(self.tcx, self.typing_env, iter_ty);
        let endless = trusted && self.recognition().endless_by_type(iter_ty);
        let sized = |item: Ty<'tcx>| self.layout(item).is_some_and(|layout| layout.size.bytes() > 0);
        let asks_room = self.recognition().grows_as_vec(output);
        let endless_once = match !trusted && asks_room && item.is_some_and(sized) {
            true => self.recognition().endless_once_stepped(iter_ty).ok_or_else(|| {
                self.unsupported(
                    span,
                    "`collect()` into a `Vec` of what may have no end only as what it holds says, a `cycle()` chained with what has an end, a `cycle()` of what's partly filtered or a `step_by(n)` of what has no end, which panics `capacity overflow` only then",
                )
            })?,
            false => false,
        };
        // Of a crate that keeps capacities, of what isn't `TrustedLen`,
        // whose capacity std's growth gives (#1180); not of what std may
        // collect in place, whose capacity is its source's.
        let kept = asks_room
            && self.krate.keeps_capacity
            && !trusted
            && item.is_some_and(|item| item.has_param() || sized(item))
            && self.collects_in_place(Some(iter_ty), item) == Some(false);
        // Of an `ExactSizeIterator`, it's given knowing its length, whose
        // hint then is std's (#1200); not one whose hint runs the crate's
        // own, which it asks as it is.
        let exact = kept && !self.hint_reaches_own(iter_ty) && self.recognition().grows_by_exact_len(iter_ty);
        if kept && !exact {
            self.refuse_unkept_lower_bound(iter_ty, span)?;
        }
        Ok(item
            .filter(|_| {
                asks_room
                    && (endless
                        || endless_once
                        || std_struct && !self.recognition().bounded_by_items(iter_ty)
                        || self.hint_reaches_own(iter_ty)
                        || kept)
            })
            .map(|item| Grown {
                output,
                item,
                trusted,
                endless: endless || endless_once,
                exact,
            }))
    }

    /// `it` grown as `grown` says, `$roomForHint(it, size, trusted)`: the
    /// size it's grown by (`grown_item_size`), whether it's `TrustedLen`,
    /// a caller's `ITrustedLen` where that's its caller's, `callers_trusted`
    /// (#670); and of one that's endless, or not `TrustedLen` of items with
    /// a destructor, whose first item std takes by `next` before it asks
    /// room, that item's drop, as a panic asking it unwinds (#724).
    pub(in crate::lower) fn grown_by_hint(
        &mut self,
        it: Expr,
        grown: Grown<'tcx>,
        iter_ty: Ty<'tcx>,
        callers_trusted: Option<Expr>,
        span: Span,
    ) -> R<Expr> {
        let Grown {
            output,
            item,
            trusted,
            endless,
            exact,
        } = grown;
        // Not std's compiled struct, whose own hint, its struct's, is std's.
        let it = match exact && !self.compiled_struct(iter_ty, &it) {
            true => self.as_exact_size(it, iter_ty, false, span)?,
            false => it,
        };
        let size = self.grown_item_size(output, iter_ty, item, span)?;
        let mut given = vec![it, size, callers_trusted.unwrap_or_else(|| Expr::bool(trusted))];
        let drop = match !trusted && self.drops(item) != crate::lower::drops::Drops::Nothing {
            true => self.drop_function(item, span)?,
            false => None,
        };
        if endless || drop.is_some() {
            given.push(Expr::bool(endless));
        }
        given.extend(drop);
        Ok(Expr::call_helper("$roomForHint", given))
    }

    /// The size of each of `item`s a `Vec` std makes of an iterator of
    /// `iter_ty` grows by, which says when it's full and asks the hint
    /// again (`$roomForHint`'s, #728): `room_item_size`; but of items whose
    /// size is its caller's where that hint may run the crate's own
    /// (`grows_by_callers_size`), the caller's, `TSize` (#796).
    fn grown_item_size(&mut self, output: Ty<'tcx>, iter_ty: Ty<'tcx>, item: Ty<'tcx>, span: Span) -> R<Expr> {
        match (self.recognition()).grows_by_callers_size(output, iter_ty, item, (false, self.krate.keeps_capacity)) {
            true => self.type_fact_value(item, TypeFact::Size, span),
            false => Ok(self.room_item_size(item)),
        }
    }

    /// What a collect of an iterator of `iter_ty` into `target`, a `Result`
    /// or an `Option` of a collection, of `item`s, asks of its hint
    /// through std's `GenericShunt` (`Shunted`): of a collection std grows
    /// as a `Vec`, as it grows, by its values' size, a caller's `TSize`
    /// where it's its caller's (`grows_by_callers_size`); of a `String` of
    /// `char`s or a hash table, which reserve first, once; of another, a
    /// `LinkedList` or a `String` of strings, none. Only where asking it
    /// may run the crate's own (`hint_reaches_own`), or, of a `Vec` of a
    /// crate that keeps capacities, always, which std grows by the shunt's
    /// lower bound, none, whatever its source's (#1180).
    pub(in crate::lower) fn shunted_ask(
        &mut self,
        target: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        iter_ty: Option<Ty<'tcx>>,
        span: Span,
    ) -> R<Shunted> {
        let (Some(item), Some(iter_ty)) = (item, iter_ty) else {
            return Ok(Shunted::Never);
        };
        let Some((collection, value)) = self.recognition().shunted(target, item) else {
            return Ok(Shunted::Never);
        };
        let keeps = self.krate.keeps_capacity;
        let grows = self.recognition().grows_as_vec(collection);
        if !self.hint_reaches_own(iter_ty) && !(keeps && grows) {
            return Ok(Shunted::Never);
        }
        if grows {
            return Ok(Shunted::Grown(
                match self
                    .recognition()
                    .grows_by_callers_size(collection, iter_ty, value, (true, keeps))
                {
                    true => self.type_fact_value(value, TypeFact::Size, span)?,
                    false => self.room_item_size(value),
                },
            ));
        }
        let chars =
            self.is_lang_adt(collection, rustc_hir::attrs::lang_items::LangItem::String) && value.peel_refs().is_char();
        Ok(
            match chars || (self.is_map(collection) && !self.is_sorted(collection)) {
                true => Shunted::Reserved,
                false => Shunted::Never,
            },
        )
    }

    /// Whether `extend` of `into` by `iterable` grows it as std's does as
    /// it fills, `$grownBy` (`extended_grown`): a `Vec`'s, a `VecDeque`'s
    /// or a `BinaryHeap`'s (`grows_as_it_extends`), by an iterator whose
    /// hint may run the crate's own (#849), or, of a crate that keeps
    /// capacities, by one that isn't `TrustedLen`, which std's grows item by
    /// item, `extend_desugared` (#1180). Else by as many as it's given,
    /// which a `TrustedLen` one reserves first, `$grew` where capacities are
    /// kept (`mir_std_call`).
    pub(in crate::lower) fn extend_fills(&self, into: Ty<'tcx>, iterable: Ty<'tcx>) -> bool {
        let iter_ty = crate::lower::recognition::iterator_type_of(self.tcx, self.typing_env, iterable);
        self.recognition().grows_as_it_extends(into)
            && (self.hint_reaches_own(iter_ty)
                || self.krate.keeps_capacity && !is_trusted_len(self.tcx, self.typing_env, iter_ty))
    }

    /// `extend` of `collection`, of type `into`, by `items`, of type
    /// `iterable`, asking its hint as std's does as the collection fills
    /// (`extend_fills`): of a `Vec`'s, a `BinaryHeap`'s, its `Vec`'s, and a
    /// `VecDeque`'s, `$grownBy(collection, items, size, deque, drop)`, its
    /// capacity std's, kept where its crate keeps capacities (ADR 0419,
    /// amended, #1180); which a crate whose `extend` asks the crate's own
    /// hint so does (`extends_as_it_fills`). Of items whose size is a
    /// caller's, refused where the hint may run the crate's own; of an
    /// iterator its caller gives, its caller's to refuse
    /// (`refuse_unknown_capacity_extends`). What it's extended by, and the
    /// collection, held first where it's read again.
    pub(in crate::lower) fn extended_grown(
        &mut self,
        collection: Expr,
        items: Expr,
        (iterable, into): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<crate::js::Stmt>,
    ) -> R<(Expr, Expr)> {
        if !self.extend_fills(into, iterable) {
            return Ok((collection, items));
        }
        let iter_ty = crate::lower::recognition::iterator_type_of(self.tcx, self.typing_env, iterable);
        let Some(item) = self.iterator_item(iter_ty) else {
            return Ok((collection, items));
        };
        // A crate whose `extend` asks the crate's own hint keeps capacities
        // (`extends_as_it_fills`): of one that doesn't, an iterator its
        // caller gives, its caller's to refuse.
        if !self.krate.keeps_capacity {
            return Ok((collection, items));
        }
        self.refuse_unkept_lower_bound(iter_ty, span)?;
        // Of one whose hint is the crate's own, its type known here, not its
        // caller's, whose callers decide (`refuse_unknown_capacity_extends`).
        let own = self.hint_reaches_own(iter_ty);
        let known_own = own && !(iterable.has_param() || iterable.has_aliases());
        // Of generic code, by an iterator whose JS hint, of what one of its
        // callers gives, isn't std's: its capacity unknown once it grows
        // (`Hinted::lost`, #1180's (3)).
        let lost = (self.krate.hinted.lost).contains(&(
            self.tcx.typeck_root_def_id(self.frame.owner),
            self.tcx.erase_and_anonymize_regions(iter_ty),
        ));
        self.grown_by(collection, items, (into, item, (own, known_own), lost), span, out)
    }

    /// Generic code's `extend` of `into`, a std collection, through its
    /// dictionary, by `items`, whatever iterable it's given, of `item`s
    /// (`extend_dictionary`): of a crate that keeps capacities, of one that
    /// grows as it's extended, as std's grows by any (#1180), `$grownBy`,
    /// which of a `TrustedLen` iterator, whose hint is exact, reserves as
    /// many as it has left where it's first full, as std's reserve of them
    /// all leaves it. Refused of items whose size is a caller's, which its
    /// dictionary isn't given.
    pub(in crate::lower) fn dictionary_grown(
        &mut self,
        collection: Expr,
        items: Expr,
        (into, item): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
        out: &mut Vec<crate::js::Stmt>,
    ) -> R<Expr> {
        if !self.krate.keeps_capacity || !self.recognition().grows_as_it_extends(into) {
            return Ok(items);
        }
        if item.has_param() {
            return Err(self.unsupported(
                span,
                &format!("a bound's `extend` of a `{into}`, whose items' size std grows it by, a type parameter's, in a crate that reads a `Vec`'s capacity: call `extend` of the `Vec` itself"),
            ));
        }
        // Of an iterator its caller gives, which may be the crate's own.
        Ok(self
            .grown_by(collection, items, (into, item, (true, false), false), span, out)?
            .1)
    }

    /// The size of a `Vec`'s `item`s, by which std grows it, of a crate
    /// that keeps capacities: its caller's `TSize` where it's a type
    /// parameter's, as the analysis asks it (`grown_vec_item`, #1180).
    /// `None` of another made of type parameters, an associated type's or
    /// `vec::IntoIter<I::Item>`, whose size no caller gives: its growth
    /// isn't kept, as before #1180.
    pub(in crate::lower) fn capacity_item_size(&self, item: Ty<'tcx>) -> Option<Expr> {
        match item.kind() {
            _ if !item.has_param() => Some(self.room_item_size(item)),
            ty::Param(param) => self.given_type_fact(param.index, TypeFact::Size),
            _ => None,
        }
    }

    /// `$grownBy(collection, items, size, deque, drop, own, lost)`: of
    /// `into`'s items' size, of `item`s' drop, as the iterator gives them,
    /// whether its hint may be the crate's own, `own`, which a `Vec` whose
    /// capacity isn't known can't ask as std's does (#1180), and whether
    /// its JS hint may not be std's, `lost`, its capacity then unknown
    /// once it grows (#1180's (3)). Of items whose
    /// size no caller gives, as it is, its growth not kept; refused where
    /// its hint is the crate's own, its type known here, `known_own`, whose
    /// asking would be seen.
    fn grown_by(
        &mut self,
        collection: Expr,
        items: Expr,
        (into, item, (own, known_own), lost): (Ty<'tcx>, Ty<'tcx>, (bool, bool), bool),
        span: Span,
        out: &mut Vec<crate::js::Stmt>,
    ) -> R<(Expr, Expr)> {
        let collection = match collection.reads_same() {
            true => collection,
            false => self.spill("items", collection, out),
        };
        let deque = self.is_std_type(into, crate::lower::recognition::StdItem::VecDeque);
        let drop = match self.drops(item) != crate::lower::drops::Drops::Nothing {
            true => self.drop_function(item, span)?,
            false => None,
        };
        let Some(size) = self.capacity_item_size(self.slice_item(into).unwrap_or(item)) else {
            if known_own {
                return Err(self.unsupported(
                    span,
                    "an `extend` of a `Vec`, a `VecDeque` or a `BinaryHeap` by an iterator whose `size_hint` is the crate's own, which std asks as it fills, as its capacity says, of items whose size no caller gives: extend one of items of a known type",
                ));
            }
            return Ok((collection, items));
        };
        let mut args = vec![collection.clone(), items, size];
        if deque || drop.is_some() || own || lost {
            args.push(Expr::bool(deque));
        }
        if drop.is_some() || own || lost {
            args.push(drop.unwrap_or_else(Expr::undefined));
        }
        if own || lost {
            args.push(Expr::bool(own));
        }
        if lost {
            args.push(Expr::bool(true));
        }
        Ok((collection, Expr::call_helper("$grownBy", args)))
    }

    /// What `extend` of `into` by what it's given, `it`, of type `iterable`,
    /// asks of its hint before it's stepped, as std's does (#849): of a
    /// `String` by `char`s, `reserve(lower)` (string.rs `Extend<char>`); of
    /// a hash table, hashbrown's `reserve` of its lower bound, half of it
    /// rounded up where the table has entries already; each counted with
    /// what `of`, the collection, holds. `it` as it is of another, which
    /// asks none first, a `String` by strings, a B-tree's; a `Vec`'s and a
    /// `VecDeque`'s, which ask it as they grow, are their own.
    pub(in crate::lower) fn extended_reserved(&self, it: Expr, into: Ty<'tcx>, iterable: Ty<'tcx>, of: Expr) -> Expr {
        match self.extend_room(into, iterable) {
            Some((iter_ty, room)) => self.reserved_by_hint(it, Some(iter_ty), room, Some(of)),
            None => it,
        }
    }

    /// Whether `extend` of `into` by what's of type `iterable` asks its
    /// hint first (`extended_reserved`): where it's asked of a collection
    /// not named, it's held first, so it's made once.
    pub(in crate::lower) fn extend_reserves(&self, into: Ty<'tcx>, iterable: Ty<'tcx>) -> bool {
        self.extend_room(into, iterable)
            .is_some_and(|(iter_ty, _)| self.hint_reaches_own(iter_ty))
    }

    /// What `extend` of `into` by what's of type `iterable` reserves room
    /// for first, and of what iterator (`room_of`). `None` of another.
    fn extend_room(&self, into: Ty<'tcx>, iterable: Ty<'tcx>) -> Option<(Ty<'tcx>, Room<'tcx>)> {
        let iter_ty = crate::lower::recognition::iterator_type_of(self.tcx, self.typing_env, iterable);
        let item = self.iterator_item(iter_ty)?;
        Some((iter_ty, self.room_of(into, item)?))
    }

    /// What `extend` of `into` by `item`s reserves room for first: a
    /// `String`'s by `char`s, its bytes; a hash table's, its entries. `None`
    /// of another.
    fn room_of(&self, into: Ty<'tcx>, item: Ty<'tcx>) -> Option<Room<'tcx>> {
        match () {
            _ if self.is_lang_adt(into, rustc_hir::attrs::lang_items::LangItem::String)
                && item.peel_refs().is_char() =>
            {
                Some(Room::Bytes)
            }
            _ if self.is_map(into) && !self.is_sorted(into) => Some(Room::Entries(item)),
            _ => None,
        }
    }

    /// `extend_reserved`'s, of a dictionary's `extend`, `C: Extend<item>`
    /// of `into`, given any iterator generic code has, which may be the
    /// crate's own: its hint asked, as std's asks it (#849).
    pub(in crate::lower) fn dictionary_reserved(&self, it: Expr, into: Ty<'tcx>, item: Ty<'tcx>, of: Expr) -> Expr {
        let Some(room) = self.room_of(into, item) else {
            return it;
        };
        let mut args = vec![it];
        match room {
            Room::Nothing => {}
            Room::Bytes => args.extend([Expr::int(1), Expr::undefined()]),
            Room::Entries(entry) => {
                let (size, align) = match self.layout(entry) {
                    Some(layout) => (i128::from(layout.size.bytes()), i128::from(layout.align.abi.bytes())),
                    None => (0, 1),
                };
                args.extend([Expr::int(size), Expr::int(align)]);
            }
        }
        args.push(of);
        Expr::call_helper("$reserveHint", args)
    }

    /// `it`, of type `iter_ty`, asked its `size_hint` before it's stepped,
    /// as std's `extend` of a `String` or hash table asks it, and room for
    /// its lower bound reserved as `room` says, `$reserveHint(it, size,
    /// align, of)`, counted with what `of` holds, the collection extended,
    /// of a new one none. Only where asking it may run what the program
    /// sees (`hint_reaches_own`), as std's own hints change nothing but
    /// that room, which no JS string or table reaches (#733).
    pub(in crate::lower) fn reserved_by_hint(
        &self,
        it: Expr,
        iter_ty: Option<Ty<'tcx>>,
        room: Room<'tcx>,
        of: Option<Expr>,
    ) -> Expr {
        if !iter_ty.is_some_and(|iter_ty| self.hint_reaches_own(iter_ty)) {
            return it;
        }
        let mut args = vec![it];
        match room {
            Room::Nothing => {}
            Room::Bytes => args.push(Expr::int(1)),
            Room::Entries(entry) => {
                let (size, align) = match self.layout(entry) {
                    Some(layout) => (i128::from(layout.size.bytes()), i128::from(layout.align.abi.bytes())),
                    None => (0, 1),
                };
                args.extend([Expr::int(size), Expr::int(align)]);
            }
        }
        if let Some(of) = of {
            if args.len() == 2 {
                args.push(Expr::undefined());
            }
            args.push(of);
        }
        Expr::call_helper("$reserveHint", args)
    }
}

// Generic code's `extend` of a collection its caller gives, `c.extend(it)`
// of a `C: Extend<T>`, through its dictionary: where its caller gives a
// `Vec`, a `VecDeque` or a `BinaryHeap` and an iterator whose `size_hint`
// the crate's own may run, std asks that hint as the collection fills, as
// its capacity says, which a crate that reads no capacity doesn't keep
// (#849, the user's (b); #1180). Refused
// where those types are known, at the call that gives them; relayed
// where they're a caller's own type parameters, as #796 relays `TSize`, so
// the outermost caller that knows them decides. One of std's iterators, or
// the crate's that keeps std's default `size_hint`, compiles.
/// The message of a call refused: what it gives, and what to write.
const REFUSED: &str = "rust-js: this call gives generic code a `Vec`, a `VecDeque` or a `BinaryHeap` to `extend` by an iterator whose `size_hint` is the crate's own, which std asks as the collection fills, as its capacity says, which a crate that reads no capacity doesn't keep (#849, #1180): collect the iterator first and give that, or extend the collection here";

/// How many times what each function extends is relayed to its callers,
/// and how much of it each holds, past which what's left isn't: a
/// recursive generic function's types may grow without end.
const ROUNDS: usize = 16;
const HELD: usize = 64;

/// Refuses each call of `bodies` that gives generic code's `extend` a
/// collection that grows as it's extended and an iterator whose hint may
/// run the crate's own, both known there, of a crate that doesn't keep
/// capacities, `keeps_capacity`.
pub(in crate::lower) fn refuse_unknown_capacity_extends<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    bodies: &[&crate::lower::Body<'tcx>],
    keeps_capacity: bool,
) {
    // Of a crate that keeps capacities, each `extend` grows as std's does
    // where it's made, a dictionary's too (`dictionary_grown`, #1180).
    if keeps_capacity {
        return;
    }
    let tcx = recognition.tcx;
    let mirs: Vec<(DefId, &rustc_middle::mir::Body<'tcx>)> = (bodies.iter())
        .filter_map(|body| Some((body.def_id, &body.mir.as_ref()?.body)))
        .collect();
    // What each function extends of what it doesn't know: the collection
    // and what it's extended by, in its own type parameters' terms.
    let mut extends: IndexMap<DefId, IndexSet<(Ty<'tcx>, Ty<'tcx>)>> = IndexMap::default();
    for &(def_id, body) in &mirs {
        for data in body.basic_blocks.iter() {
            let TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
                continue;
            };
            let Some((callee, generic_args)) = func.const_fn_def() else {
                continue;
            };
            let extend = tcx
                .trait_of_assoc(callee)
                .is_some_and(|t| crate::lower::recognition::is_extend(tcx, t));
            let Some(collection) = generic_args.types().next().map(|ty| ty.peel_refs()) else {
                continue;
            };
            let Some(iterable) = args.get(1).map(|arg| arg.node.ty(body, tcx)) else {
                continue;
            };
            // Of a collection it doesn't know, whose dictionary's `extend`
            // can't ask its hint as std's does where capacities aren't
            // kept. One that grows as it's extended by an iterator its
            // caller gives keeps capacities (`extends_as_it_fills`).
            let given = extend && recognition.is_unknown(collection);
            if given {
                extends.entry(def_id).or_default().insert((collection, iterable));
            }
        }
    }
    // Each call of a function of the crate's, resolved where it's a
    // trait's, with what it's given in its caller's terms.
    let mut calls: Vec<(DefId, DefId, ty::GenericArgsRef<'tcx>, Span)> = Vec::new();
    for &(def_id, body) in &mirs {
        let typing_env = ty::TypingEnv::post_analysis(tcx, def_id);
        for data in body.basic_blocks.iter() {
            let TerminatorKind::Call { func, fn_span, .. } = &data.terminator().kind else {
                continue;
            };
            let Some((callee, generic_args)) = Operand::const_fn_def(func) else {
                continue;
            };
            let (callee, generic_args) =
                match crate::lower::items::resolve_instance_in(tcx, typing_env, callee, generic_args) {
                    Ok(Some(instance)) => (instance.def_id(), instance.args),
                    _ => (callee, generic_args),
                };
            if callee.is_local() {
                calls.push((def_id, callee, generic_args, *fn_span));
            }
        }
    }
    let mut refused: IndexSet<Span> = IndexSet::default();
    for _ in 0..ROUNDS {
        // A closure's, the function's it's written in, whose type
        // parameters it has: what std's code that calls it isn't seen to.
        let mut changed = false;
        let closures: Vec<(DefId, IndexSet<(Ty<'tcx>, Ty<'tcx>)>)> = (extends.iter())
            .filter(|(def_id, _)| tcx.is_closure_like(**def_id))
            .map(|(&def_id, held)| (tcx.typeck_root_def_id(def_id), held.clone()))
            .collect();
        for (root, held) in closures {
            let theirs = extends.entry(root).or_default();
            for each in held {
                if theirs.len() < HELD {
                    changed |= theirs.insert(each);
                }
            }
        }
        for &(caller, callee, generic_args, span) in &calls {
            let Some(theirs) = extends.get(&callee).cloned() else {
                continue;
            };
            let typing_env = ty::TypingEnv::post_analysis(tcx, caller);
            for (collection, iterable) in theirs {
                let given = |ty: Ty<'tcx>| {
                    let ty = ty::EarlyBinder::bind(tcx, ty)
                        .instantiate(tcx, generic_args)
                        .skip_normalization();
                    tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))
                        .unwrap_or(ty)
                };
                let (collection, iterable) = (given(collection).peel_refs(), given(iterable));
                let grows = recognition.grows_as_it_extends(collection);
                if !grows && !recognition.is_unknown(collection) {
                    continue;
                }
                if grows && !iterable.has_param() && !iterable.has_aliases() {
                    let iter_ty = iterator_type_of(tcx, typing_env, iterable);
                    if recognition.hint_reaches_own(iter_ty) && refused.insert(span) {
                        tcx.dcx().span_err(span, REFUSED);
                    }
                    continue;
                }
                let held = extends.entry(caller).or_default();
                if held.len() < HELD {
                    changed |= held.insert((collection, iterable));
                }
            }
        }
        if !changed {
            break;
        }
    }
}
