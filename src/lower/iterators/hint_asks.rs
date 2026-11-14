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
use crate::lower::temporary_names::Role;
use rustc_middle::mir::{Operand, TerminatorKind};
use rustc_middle::ty::TypeVisitableExt;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether a call of `callee`, std's collect or extend, asks the hint of
    /// what it's given at `index`, of type `given`, as it fills, where the
    /// program sees what it asks: `collect()`'s and `from_iter`'s, and
    /// `extend`'s iterable's, where the hint may run a `size_hint` of the
    /// crate's own or a library's (`hint_reaches_own`), which a flatten made
    /// in place, knowing no items' constant size, would ask otherwise than
    /// std's (#1327). A capacity the crate reads is std's already, what
    /// keeps it asking (ADR 0419, #1180).
    pub(in crate::lower) fn collect_hint_seen(
        &self,
        (callee, index): (rustc_span::def_id::DefId, usize),
        given: Ty<'tcx>,
    ) -> bool {
        let tcx = self.tcx;
        let iterator = crate::lower::recognition::std_item(tcx, crate::lower::recognition::StdItem::Iterator);
        let from_iterator = crate::lower::recognition::std_item(tcx, crate::lower::recognition::StdItem::FromIterator);
        let collects = (callee == crate::lower::recognition::trait_method(tcx, iterator, "collect")
            || callee == crate::lower::recognition::trait_method(tcx, from_iterator, "from_iter"))
            && index == 0
            || index == 1
                && tcx
                    .trait_of_assoc(callee)
                    .is_some_and(|t| crate::lower::recognition::is_extend(tcx, t));
        if !collects {
            return false;
        }
        let seen = self.hint_reaches_own(given);
        let answer = match (index == 0, seen) {
            (true, true) => "collect-asks",
            (false, true) => "extend-asks",
            (true, false) => "collect-unseen",
            (false, false) => "extend-unseen",
        };
        crate::decisions::record("hint-seen", answer);
        seen
    }

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

    /// The iterator a call of `callee` grows a `Vec` by, whose hint std asks
    /// as it fills: a `collect()`'s into what grows as a `Vec`, or an
    /// `extend`'s (`extend_grows`), in `typing_env`. The one answer, of what
    /// generic code grows (`hint_asked.rs`) and of a string's `chars()` a
    /// crate that reads capacities asks the hint of (`chars_asked.rs`, #1316).
    pub(in crate::lower) fn vec_grown_by(
        &self,
        callee: rustc_span::def_id::DefId,
        args: ty::GenericArgsRef<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
    ) -> Option<Ty<'tcx>> {
        let iterator = crate::lower::recognition::std_item(self.tcx, crate::lower::recognition::StdItem::Iterator);
        let collect = crate::lower::recognition::trait_method(self.tcx, iterator, "collect");
        if callee == collect && args.types().nth(1).is_some_and(|into| self.grows_as_vec(into)) {
            return args.types().next();
        }
        (self.extend_grows(callee, args))
            .then(|| args.types().nth(2))
            .flatten()
            .map(|iterable| crate::lower::recognition::iterator_type_of(self.tcx, typing_env, iterable))
    }

    /// Whether what a call of `callee` is given at `index` is the iterator
    /// it grows a `Vec` by (`vec_grown_by`): a `collect()`'s receiver, an
    /// `extend`'s iterable. Of a crate that keeps capacities, the program
    /// sees that iterator's hint, whose lower bound is the room std asks
    /// (#1528).
    pub(in crate::lower) fn grows_vec_by(
        &self,
        (callee, args, index): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>, usize),
        typing_env: ty::TypingEnv<'tcx>,
    ) -> bool {
        let iterator = crate::lower::recognition::std_item(self.tcx, crate::lower::recognition::StdItem::Iterator);
        let at = match callee == crate::lower::recognition::trait_method(self.tcx, iterator, "collect") {
            true => 0,
            false => 1,
        };
        index == at && self.vec_grown_by(callee, args, typing_env).is_some()
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
                    // A `dyn`'s too, of one Rust made, not a binding (#1377).
                    let binding = args
                        .get(1)
                        .is_some_and(|arg| self.made_by_a_binding_in(body, &arg.node));
                    self.hint_reaches_own_where(iter_ty, !binding)
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

    /// Whether `operand` of `body` is what a binding's call made, JS's own
    /// iterator, as MIR shows it: a local a binding's call wrote, moved or
    /// copied on, or a `map`, `filter`, `take` or `skip` of one, whose
    /// hint is that one's (`iterators::binding_made`'s, of a value, #1377).
    fn made_by_a_binding_in(
        &self,
        body: &rustc_middle::mir::Body<'tcx>,
        operand: &rustc_middle::mir::Operand<'tcx>,
    ) -> bool {
        use rustc_middle::mir::{Operand, Rvalue, StatementKind, TerminatorKind};
        let mut operand = operand.clone();
        // Each step back is to an earlier write, so it ends, at most once
        // a local.
        for _ in 0..body.local_decls.len() {
            let (Operand::Move(place) | Operand::Copy(place)) = &operand else {
                return false;
            };
            if !place.projection.is_empty() {
                return false;
            }
            let local = place.local;
            let mut next = None;
            for data in body.basic_blocks.iter() {
                for statement in &data.statements {
                    if let StatementKind::Assign(assign) = &statement.kind
                        && assign.0.local == local
                        && assign.0.projection.is_empty()
                        && let Rvalue::Use(used, _) = &assign.1
                    {
                        next = Some(Err(used.clone()));
                    }
                }
                if let TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } = &data.terminator().kind
                    && destination.local == local
                    && destination.projection.is_empty()
                {
                    next = Some(Ok((func.clone(), args.first().map(|arg| arg.node.clone()))));
                }
            }
            match next {
                Some(Err(used)) => operand = used,
                Some(Ok((func, first))) => {
                    let Some((callee, _)) = func.const_fn_def() else {
                        return false;
                    };
                    if crate::lower::bindings::is_binding(self.tcx, callee) {
                        return true;
                    }
                    let iterator = std_item(self.tcx, StdItem::Iterator);
                    let adapts = ["map", "filter", "take", "skip"]
                        .iter()
                        .any(|name| crate::lower::recognition::trait_method(self.tcx, iterator, name) == callee);
                    match (adapts, first) {
                        (true, Some(first)) => operand = first,
                        _ => return false,
                    }
                }
                None => return false,
            }
        }
        false
    }

    /// Whether a collect into `output` of an iterator of `iter_ty` grows a
    /// `Vec` of `item`s whose size is its caller's, a type parameter's or
    /// one built of them, `(T, u8)`, where the hint it asks as that `Vec`
    /// fills may run the crate's own (`hint_reaches_own`, of a `dyn`'s too,
    /// #1359), or, of a crate
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
        // Of a `dyn` or an opaque type, as `grown_ask` asks it of a value
        // Rust made, which no analysis of a type tells from a binding's.
        let reaches_own = self.hint_reaches_own_where(iter_ty, !shunted);
        self.grows_as_vec(output) && item.has_param() && (reaches_own || keeps_capacity && !trusted)
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

    /// What `extend` of `into` by what's of type `iterable` reserves room
    /// for first, and of what iterator (`room_of`). `None` of another.
    pub(in crate::lower) fn extend_room(&self, into: Ty<'tcx>, iterable: Ty<'tcx>) -> Option<(Ty<'tcx>, Room<'tcx>)> {
        let iter_ty = crate::lower::recognition::iterator_type_of(self.tcx, self.typing_env, iterable);
        let item = self.iterator_item(iter_ty)?;
        Some((iter_ty, self.room_of(into, item)?))
    }

    /// What `extend` of `into` by `item`s reserves room for first: a
    /// `String`'s by `char`s, its bytes; a hash table's, its entries. `None`
    /// of another.
    pub(in crate::lower) fn room_of(&self, into: Ty<'tcx>, item: Ty<'tcx>) -> Option<Room<'tcx>> {
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

    /// The entries of a hash table whose room a call of `callee` with
    /// `args` checks by their layout (ADR 0774), as `table_room` asks it:
    /// a `HashMap`'s or a `HashSet`'s `with_capacity` or `reserve`, its
    /// entry's size and alignment, a `(K, V)`'s or a set's `T`'s; an
    /// indexmap's, its buckets' size, `(usize, K, V)`'s. Their caller gives
    /// them where they're its type parameters' (#1437).
    pub(in crate::lower) fn table_room_entry(
        &self,
        callee: rustc_span::def_id::DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Vec<(Ty<'tcx>, TypeFact)> {
        use crate::lower::recognition::StdItem;
        let tcx = self.tcx;
        let Some(imp) = tcx.inherent_impl_of_assoc(callee) else {
            return Vec::new();
        };
        if !matches!(tcx.item_name(callee).as_str(), "with_capacity" | "reserve") {
            return Vec::new();
        }
        let owner = ty::EarlyBinder::bind(tcx, tcx.type_of(imp).instantiate_identity().skip_normalization())
            .instantiate(tcx, args)
            .skip_normalization();
        let ty::Adt(adt, held) = owner.kind() else {
            return Vec::new();
        };
        let set = self.is_std_type(owner, StdItem::HashSet) || self.is_set(owner);
        let hashed = self.is_std_type(owner, StdItem::HashMap) || self.is_std_type(owner, StdItem::HashSet);
        let indexed = crate::lower::recognition::is_index_map(tcx, adt.did());
        let mut types = held.types();
        let (Some(key), value) = (types.next(), types.next()) else {
            return Vec::new();
        };
        let value = if set {
            tcx.types.unit
        } else {
            value.unwrap_or(tcx.types.unit)
        };
        match (hashed, indexed) {
            (true, _) => {
                let entry = if set { key } else { Ty::new_tup(tcx, &[key, value]) };
                vec![(entry, TypeFact::Room), (entry, TypeFact::Align)]
            }
            (false, true) => vec![(Ty::new_tup(tcx, &[tcx.types.usize, key, value]), TypeFact::Room)],
            _ => Vec::new(),
        }
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
    /// Whether each of its parts that's an `ExactSizeIterator`, a chain's
    /// or a zip's, is given knowing its length, whose hints it then sums or
    /// takes the fewer of as std's do (`parts_knowing_lengths`, #1216).
    parts: bool,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Of a crate that keeps capacities, a `Vec` grown by an iterator of
    /// type `iter_ty` that isn't `TrustedLen`, by its hint's lower bound,
    /// refused where JS's isn't std's (`lower_bound_is_stds`), as its
    /// `size_hint()` is: so its capacity would be another (#1180). Not of
    /// one whose hint runs the crate's own, its own.
    fn refuse_unkept_lower_bound(&self, iter_ty: Ty<'tcx>, span: Span) -> R<()> {
        // A string's `chars()` the crate makes `$chars`, whose hint is std's
        // (#1186); and what `size_hint()` finds std's where it's made, a
        // `take(n)` of `successors`, a `rev()` of a `flat_map` (#1316).
        let chars_kept = self.krate.chars_whole;
        if self.hint_reaches_own(iter_ty)
            || self.recognition().lower_bound_is_stds_where(iter_ty, chars_kept)
            || self.recognition().lower_bound_kept_by_parts_where(iter_ty, chars_kept)
            || self.adapted_hint_known(iter_ty)
        {
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
    /// where its type doesn't say which. Of a `dyn` a binding made,
    /// `binding_made`, JS's own iterator, nothing its hint asks runs
    /// (#1359).
    pub(in crate::lower) fn grown_ask(
        &self,
        output: Ty<'tcx>,
        item: Option<Ty<'tcx>>,
        (iter_ty, binding_made): (Ty<'tcx>, bool),
        std_struct: bool,
        span: Span,
    ) -> R<Option<Grown<'tcx>>> {
        let reaches_own = self.recognition().hint_reaches_own_where(iter_ty, !binding_made);
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
        // hint then is std's (#1200); not one whose hint is known as it's
        // made (`adapted_hint_known`), the crate's own's too, which it asks
        // as it is (#1573).
        let exact = kept && self.recognition().grows_by_exact_len(iter_ty) && !self.adapted_hint_known(iter_ty);
        if kept && !exact {
            self.refuse_unkept_lower_bound(iter_ty, span)?;
        }
        // Of a chain or a zip, each exact-sized part given knowing its length
        // (#1216); not one whose lower bound is std's as it is.
        let parts = kept
            && !exact
            && !reaches_own
            && !self.recognition().lower_bound_is_stds(iter_ty)
            && self.recognition().lower_bound_kept_by_parts(iter_ty);
        Ok(item
            .filter(|_| {
                asks_room
                    && (endless
                        || endless_once
                        || std_struct && !self.recognition().bounded_by_items(iter_ty)
                        || reaches_own
                        || kept)
            })
            .map(|item| Grown {
                output,
                item,
                trusted,
                endless: endless || endless_once,
                exact,
                parts,
            }))
    }

    /// `it`, of type `ty`, a chain or a zip, `$lazyChain(a, b)` or
    /// `$lazyZip(a, b, ..)`, or a copy of one, each of whose parts that's
    /// an `ExactSizeIterator` is made knowing its length, `as_exact_size`'s,
    /// so its hint, which the runtime's chain sums and zip takes the fewer
    /// of, is std's (`lower_bound_kept_by_parts`, #1216). A part whose
    /// lower bound is std's as it is stays; another is refused.
    fn parts_knowing_lengths(&mut self, it: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let ty = ty.peel_refs();
        // What's made knowing its hint where it's made, as `hint_asked.rs`
        // asks of this collect's parts, down to its source.
        if self.hint_known_by_its_parts(ty) {
            return Ok(it);
        }
        // A copy of one, which it is.
        if let Some(source) = crate::lower::recognition::copied_source(self.tcx, ty) {
            return self.parts_knowing_lengths(it, source, span);
        }
        if self.recognition().grows_by_exact_len(ty) && !self.compiled_struct(ty, &it) {
            return self.as_exact_size(it, ty, false, span);
        }
        if self.recognition().lower_bound_is_stds(ty) {
            return Ok(it);
        }
        // std's compiled chain or zip, as the build of std's structs makes
        // each (ADR 0908): its hint is std's body's over its parts', std's
        // where each part's is (`compiled_hint_is_stds`, #1737).
        if self.compiled_struct(ty, &it) && self.compiled_hint_is_stds(ty) {
            return Ok(it);
        }
        let sources = crate::lower::recognition::chained_sources(self.tcx, ty)
            .map(|sources| ("$lazyChain", sources))
            .or_else(|| crate::lower::recognition::zipped_sources(self.tcx, ty).map(|sources| ("$lazyZip", sources)));
        if let Some((helper, (a, b))) = sources
            && let js::ExprKind::Call(callee, args) = &it.kind
            && callee.helper_name() == Some(helper)
            && let [first, second, rest @ ..] = args.as_slice()
        {
            let first = self.parts_knowing_lengths(first.clone(), a, span)?;
            let second = self.parts_knowing_lengths(second.clone(), b, span)?;
            let args = [vec![first, second], rest.to_vec()].concat();
            return Ok(Expr::call(Expr::helper(helper), args));
        }
        // One made elsewhere, a variable's, each of whose parts its maker
        // made knowing its hint, which the runtime's chain or zip it's made
        // into combines.
        if self.hint_known_by_its_parts(ty) {
            return Ok(it);
        }
        Err(self.unsupported(
            span,
            &format!(
                "a `Vec` grown by a `{ty}` in a crate that reads a `Vec`'s capacity, which std grows by that iterator's `size_hint()`, whose lower bound JS's doesn't have"
            ),
        ))
    }

    /// Whether an iterator of type `ty`, std's compiled struct's part,
    /// hints std's lower bound: a source whose own is std's, or std's
    /// compiled adapter of such ones, whose `size_hint` is std's body's over
    /// what it holds, as the build of std's structs makes each (ADR 0908),
    /// a chain of an `inspect()` of a `Vec`'s items and an array's (#1737).
    fn compiled_hint_is_stds(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        if self.recognition().lower_bound_is_stds(ty) {
            return true;
        }
        crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty)
            && crate::lower::recognition::adapted_iterators(self.tcx, self.typing_env, ty)
                .is_some_and(|held| held.into_iter().all(|part| self.compiled_hint_is_stds(part)))
    }

    /// Whether a chain or a zip of type `ty`, or a copy of one, knows std's
    /// hint as it's made: each part a `skip(n)` or a `take(n)` made knowing
    /// its hint (`made_knowing_its_hint`), one whose lower bound is std's
    /// as it is, or another such chain or zip.
    fn hint_known_by_its_parts(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        if self.recognition().lower_bound_is_stds(ty) || self.knows_its_length(ty) {
            return true;
        }
        // Of a source that knows it, made so too.
        if let Some((source, _)) = crate::lower::recognition::counted_source(self.tcx, ty)
            && self.made_knowing_its_hint(ty)
        {
            return self.hint_known_by_its_parts(source);
        }
        if let Some(source) = crate::lower::recognition::copied_source(self.tcx, ty) {
            return self.hint_known_by_its_parts(source);
        }
        // A map made knowing its source's hint where it's made, `$mapEnds`.
        if let Some(source) = crate::lower::recognition::mapped_source(self.tcx, ty)
            && self.made_knowing_its_hint(ty)
        {
            return self.hint_known_by_its_parts(source);
        }
        crate::lower::recognition::chained_sources(self.tcx, ty)
            .or_else(|| crate::lower::recognition::zipped_sources(self.tcx, ty))
            .is_some_and(|(a, b)| self.hint_known_by_its_parts(a) && self.hint_known_by_its_parts(b))
    }

    /// `it` grown as `grown` says, `$roomForHint(it, size, trusted)`: the
    /// size it's grown by (`grown_item_size`), whether it's `TrustedLen`,
    /// a caller's `ITrustedLen` where that's its caller's, `callers_trusted`
    /// (#670); and of one not `TrustedLen` that's endless once stepped, or
    /// of items with a destructor, whose first item std takes by `next`
    /// before it asks room, whether it's endless and that item's drop, as a
    /// panic asking it unwinds (#724).
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
            parts,
        } = grown;
        // Not std's compiled struct, whose own hint, its struct's, is std's.
        let it = match exact && !self.compiled_struct(iter_ty, &it) {
            true => self.as_exact_size(it, iter_ty, false, span)?,
            false if parts => self.parts_knowing_lengths(it, iter_ty, span)?,
            false => it,
        };
        let size = self.grown_item_size(output, iter_ty, item, span)?;
        let mut given = vec![it, size, callers_trusted.unwrap_or_else(|| Expr::bool(trusted))];
        let drop = match !trusted && self.drops(item) != crate::lower::drops::Drops::Nothing {
            true => self.drop_function(item, span)?,
            false => None,
        };
        // Of a `TrustedLen`, its hint says it's endless, std's contract,
        // which the runtime reads (ADR 0940, amended): not given.
        let endless = endless && !trusted;
        if endless || drop.is_some() {
            given.push(Expr::bool(endless));
        }
        given.extend(drop);
        Ok(Expr::call_helper("$roomForHint", given))
    }

    /// `it`, a `TrustedLen` of type `iter_ty` collected into an `Rc<[T]>` or
    /// an `Arc<[T]>` of `item`s, as a `Box<[T]>`, `boxed`, would be, given
    /// room as std's `ToRcSlice` asks it, `$roomForCounted(it, size,
    /// align)`: of no upper bound, `capacity overflow`; past what a `Layout`
    /// of the slice after its counts holds, `LayoutError`'s (#1678). Not of
    /// one bounded by items that exist already, which asks no room it could
    /// lack (`bounded_by_items`).
    pub(in crate::lower) fn counted_slice_room(
        &mut self,
        it: Expr,
        (boxed, item): (Ty<'tcx>, Ty<'tcx>),
        iter_ty: Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
        if self.recognition().bounded_by_items(iter_ty) {
            return Ok(it);
        }
        let (size, align) = self.counted_slice_layout((boxed, item), iter_ty, span)?;
        Ok(Expr::call_helper("$roomForCounted", vec![it, size, align]))
    }

    /// The size and the alignment of each of `item`s an `Rc<[T]>`, as a
    /// `Box<[T]>`, `boxed`, would be, std's `from_iter_exact` lays out:
    /// `grown_item_size`'s, and its layout's or its caller's `TAlign`.
    pub(in crate::lower) fn counted_slice_layout(
        &mut self,
        (boxed, item): (Ty<'tcx>, Ty<'tcx>),
        iter_ty: Ty<'tcx>,
        span: Span,
    ) -> R<(Expr, Expr)> {
        let size = self.grown_item_size(boxed, iter_ty, item, span)?;
        let align = match self.layout(item) {
            Some(layout) => Expr::int(layout.align.abi.bytes().into()),
            None => self.type_fact_value(item, TypeFact::Align, span)?,
        };
        Ok((size, align))
    }

    /// The size of each of `item`s a `Vec` std makes of an iterator of
    /// `iter_ty` grows by, which says when it's full and asks the hint
    /// again (`$roomForHint`'s, #728): `room_item_size`; but of items whose
    /// size is its caller's where that hint may run the crate's own
    /// (`grows_by_callers_size`), the caller's, `TSize` (#796).
    pub(in crate::lower) fn grown_item_size(
        &mut self,
        output: Ty<'tcx>,
        iter_ty: Ty<'tcx>,
        item: Ty<'tcx>,
        span: Span,
    ) -> R<Expr> {
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
    /// A `dyn`'s too, of one Rust made, not a binding, `binding_made`
    /// (#1377).
    pub(in crate::lower) fn extend_fills(&self, into: Ty<'tcx>, (iterable, binding_made): (Ty<'tcx>, bool)) -> bool {
        let iter_ty = crate::lower::recognition::iterator_type_of(self.tcx, self.typing_env, iterable);
        self.recognition().grows_as_it_extends(into)
            && (self.recognition().hint_reaches_own_where(iter_ty, !binding_made)
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
        let binding = crate::lower::iterators::binding_made(&items);
        if !self.extend_fills(into, (iterable, binding)) {
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
        let own = self.recognition().hint_reaches_own_where(iter_ty, !binding);
        let known_own = own && !(iterable.has_param() || iterable.has_aliases());
        // Of generic code, by an iterator whose JS hint, of what one of its
        // callers gives, isn't std's: its capacity unknown once it grows
        // (`Hinted::lost`, #1180's (3)); of one whose callers aren't all
        // seen, by each value it's given (`Hinted::by_value`, #1251).
        let site = (
            self.tcx.typeck_root_def_id(self.frame.owner),
            self.tcx.erase_and_anonymize_regions(iter_ty),
        );
        let lost = (
            self.krate.hinted.lost.contains(&site),
            self.krate.hinted.by_value.contains(&site),
        );
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
            .grown_by(
                collection,
                items,
                (into, item, (true, false), (false, false)),
                span,
                out,
            )?
            .1)
    }

    /// The size of a `Vec`'s `item`s, by which std grows it, of a crate
    /// that keeps capacities: rustc's where it knows it, `&T`'s too; its
    /// caller's `TSize` where it's a type parameter's; an associated type's,
    /// `I::Item`'s, or a type's built of them, `[T; 2]`'s, as its caller or
    /// its trait's dictionary gives it (`known_fact_of`, #1260), as the
    /// analysis asks it (`grown_vec_item`, #1180). `None` where neither
    /// gives it: its growth isn't kept, and `capacity()` refuses it.
    /// The size and alignment of a hash table's entry, `entry`, by which
    /// hashbrown checks its table's room (ADR 0774): rustc's where it lays
    /// it out here; else its caller's, `KVSize` and `KVAlign` of a `(K,
    /// V)`, a set's `TSize` and `TAlign` (#1437), as the analysis asks them
    /// (`table_room_entry`); of none given, #934's, `0` and `1`, checked by
    /// the count alone.
    pub(in crate::lower) fn entry_layout(&self, entry: Ty<'tcx>) -> (Expr, Expr) {
        if let Ok(layout) = self.tcx.layout_of(self.typing_env.as_query_input(entry)) {
            return (
                Expr::int(i128::from(layout.size.bytes())),
                Expr::int(i128::from(layout.align.abi.bytes())),
            );
        }
        let align = match entry.kind() {
            ty::Param(param) => self.given_type_fact(param.index, TypeFact::Align),
            _ => self.known_fact_of(entry, TypeFact::Align),
        };
        match (self.room_value(entry), align) {
            (Some(size), Some(align)) if !matches!(size.kind, crate::js::ExprKind::Undefined) => (size, align),
            _ => (Expr::int(0), Expr::int(1)),
        }
    }

    pub(in crate::lower) fn capacity_item_size(&self, item: Ty<'tcx>) -> Option<Expr> {
        self.room_value(item)
    }

    /// The size std grows a `Vec` of `item`s by and checks their room by,
    /// `TypeFact::Room` (#1260): rustc's where it lays it out here, of any
    /// types given, as the analysis's `built_of` asks none of it; a type
    /// parameter's, given as `Room` or `Size` (`settle_room` keeps one);
    /// an associated type's or a built type's, as its caller or its trait's
    /// dictionary gives it (`known_fact_of`). `undefined` of #934's case
    /// alone, an associated type no caller gives and no dictionary carries,
    /// in an impl's method (`in_impl_method`), its `I::Item`; elsewhere, not
    /// given, `None`, loud. Of #934's, its growth then isn't kept and its room is
    /// counted alone; the one place `Room` is given so (held by
    /// test/architecture-generic-facts.test.ts). `None` where it isn't
    /// given.
    /// Whether the function being lowered is a trait impl's method, or a
    /// closure in one: where #934's case is, an associated type its impl's
    /// dictionary can't give (`room_value`).
    fn in_impl_method(&self) -> bool {
        (self.tcx)
            .trait_impl_of_assoc(self.tcx.typeck_root_def_id(self.item))
            .is_some()
    }

    pub(in crate::lower) fn room_value(&self, item: Ty<'tcx>) -> Option<Expr> {
        // As the bounds here normalize it, a caller's `Option<<Wrapper<C> as
        // Iterator>::Item>` its callee's slot names.
        let item = (self.tcx)
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(item))
            .unwrap_or(item);
        if let Ok(layout) = self.tcx.layout_of(self.typing_env.as_query_input(item)) {
            return Some(Expr::int(i128::from(layout.size.bytes())));
        }
        let given = |fact| match item.kind() {
            ty::Param(param) => self.given_type_fact(param.index, fact),
            _ => self.known_fact_of(item, fact),
        };
        given(TypeFact::Room).or_else(|| given(TypeFact::Size)).or_else(|| {
            (crate::lower::recognition::is_associated_type(item) && self.in_impl_method()).then(Expr::js_undefined)
        })
    }

    /// `$grownBy(collection, items, size, deque, drop, own, lost,
    /// byValue)`: of `into`'s items' size, of `item`s' drop, as the
    /// iterator gives them, whether its hint may be the crate's own, `own`,
    /// which a `Vec` whose capacity isn't known can't ask as std's does
    /// (#1180), whether its JS hint may not be std's, `lost`, its capacity
    /// then unknown once it grows (#1180's (3)), and whether that's asked of
    /// each value it's given, `byValue` (#1251). Of items whose
    /// size no caller gives, as it is, its growth not kept; refused where
    /// its hint is the crate's own, its type known here, `known_own`, whose
    /// asking would be seen.
    fn grown_by(
        &mut self,
        collection: Expr,
        items: Expr,
        (into, item, (own, known_own), (lost, by_value)): (Ty<'tcx>, Ty<'tcx>, (bool, bool), (bool, bool)),
        span: Span,
        out: &mut Vec<crate::js::Stmt>,
    ) -> R<(Expr, Expr)> {
        let collection = match collection.reads_same() {
            true => collection,
            false => self.spill(Role::Items, collection, out),
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
        let lost_or_asked = lost || by_value;
        if deque || drop.is_some() || own || lost_or_asked {
            args.push(Expr::bool(deque));
        }
        if drop.is_some() || own || lost_or_asked {
            args.push(drop.unwrap_or_else(Expr::js_undefined));
        }
        if own || lost_or_asked {
            args.push(Expr::bool(own));
        }
        if lost_or_asked {
            args.push(Expr::bool(lost));
        }
        if by_value {
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
        match self.recognition().extend_room(into, iterable) {
            Some((iter_ty, room)) => self.reserved_by_hint(it, Some(iter_ty), room, Some(of)),
            None => it,
        }
    }

    /// Whether `extend` of `into` by what's of type `iterable` asks its
    /// hint first (`extended_reserved`): where it's asked of a collection
    /// not named, it's held first, so it's made once.
    pub(in crate::lower) fn extend_reserves(&self, into: Ty<'tcx>, iterable: Ty<'tcx>) -> bool {
        let reserves = self
            .recognition()
            .extend_room(into, iterable)
            .is_some_and(|(iter_ty, _)| self.recognition().hint_room_seen(iter_ty));
        crate::decisions::record("extend-reserves", if reserves { "reserves" } else { "none" });
        reserves
    }

    /// `extend_reserved`'s, of a dictionary's `extend`, `C: Extend<item>`
    /// of `into`, given any iterator generic code has, which may be the
    /// crate's own: its hint asked, as std's asks it (#849).
    pub(in crate::lower) fn dictionary_reserved(&self, it: Expr, into: Ty<'tcx>, item: Ty<'tcx>, of: Expr) -> Expr {
        let Some(room) = self.recognition().room_of(into, item) else {
            return it;
        };
        let mut args = vec![it];
        match room {
            Room::Nothing => {}
            Room::Bytes => args.extend([Expr::int(1), Expr::js_undefined()]),
            Room::Entries(entry) => {
                let (size, align) = self.entry_layout(entry);
                args.extend([size, align]);
            }
        }
        args.push(of);
        Expr::call_helper("$reserveHint", args)
    }

    /// `it`, of type `iter_ty`, asked its `size_hint` before it's stepped,
    /// as std's `extend` of a `String` or hash table asks it, and room for
    /// its lower bound reserved as `room` says, `$reserveHint(it, size,
    /// align, of)`, counted with what `of` holds, the collection extended,
    /// of a new one none. Only where asking it may change what the program
    /// sees: run the crate's own hint (#733), or, of room it reserves, pass
    /// the room there is, `capacity overflow` before an item's taken
    /// (`hint_room_seen`, #1358). Else std's hints change nothing but that
    /// room, which no JS string or table reaches.
    pub(in crate::lower) fn reserved_by_hint(
        &self,
        it: Expr,
        iter_ty: Option<Ty<'tcx>>,
        room: Room<'tcx>,
        of: Option<Expr>,
    ) -> Expr {
        let seen = |iter_ty: Ty<'tcx>| match room {
            Room::Nothing => self.hint_reaches_own(iter_ty),
            Room::Bytes | Room::Entries(_) => self.recognition().hint_room_seen(iter_ty),
        };
        if !iter_ty.is_some_and(seen) {
            return it;
        }
        let mut args = vec![it];
        match room {
            Room::Nothing => {}
            Room::Bytes => args.push(Expr::int(1)),
            Room::Entries(entry) => {
                let (size, align) = self.entry_layout(entry);
                args.extend([size, align]);
            }
        }
        if let Some(of) = of {
            if args.len() == 2 {
                args.push(Expr::js_undefined());
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
