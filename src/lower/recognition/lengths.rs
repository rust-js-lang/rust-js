//! What an iterator's type says of how many items it gives, which what
//! collects it asks room for (ADR 0940).

use rustc_middle::ty::TypeVisitableExt;

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether the crate's iterator `ty`'s impl writes a `size_hint` of its
    /// own, which rust-js compiled, not std's `(0, None)`. The one answer,
    /// of what's given it and what may reach it (`hint_reaches_own`).
    pub(in crate::lower) fn writes_size_hint(&self, ty: ty::Ty<'tcx>) -> bool {
        let iterator = std_item(self.tcx, StdItem::Iterator);
        let size_hint = trait_method(self.tcx, iterator, "size_hint");
        let args = self.args_of(iterator, super::iterator_held(ty));
        crate::lower::items::resolve_instance_in(self.tcx, self.typing_env, size_hint, args)
            .ok()
            .flatten()
            .is_some_and(|instance| {
                instance.def_id() != size_hint
                    && (instance.def_id().is_local() || self.foreign.compiled(instance.def_id()))
            })
    }

    /// Whether asking `size_hint` of an iterator of type `ty` may run what
    /// the program can see: the crate's own `size_hint`, or one that std's
    /// adapter of type `ty` asks, as each asks its source's (ADR 1092's
    /// table). Where it can't, asking it changes nothing but the room a
    /// collect asks, which is std's own (#733). Of a type only its caller
    /// knows, a type parameter's or an associated type's, always: its
    /// caller's may be the crate's own, whose hint `$given` passes on from
    /// its dictionary, and where it's JS's own, asking it runs nothing (the
    /// user's decision, ADR 1092). The one answer, of the collect that asks
    /// it and of what a generic function is given for that, its items'
    /// `TSize` (#796).
    pub(in crate::lower) fn hint_reaches_own(&self, ty: ty::Ty<'tcx>) -> bool {
        self.hint_reaches_own_where(ty, false)
    }

    /// `hint_reaches_own`, where what a `dyn Iterator`, boxed or lent, or
    /// an opaque type that can't be revealed, holds may be the crate's
    /// own, `unseen_own`, or what has no end, which only its hint says once
    /// it runs: as a collect asks it of a value, so `capacity overflow`
    /// panics after one item where the hint's lower bound is `usize::MAX`
    /// (`grown_ask`); not of one a binding made, JS's own iterator, whose
    /// hint is its contract's (`iterators::binding_made`, the user's
    /// decision, #1359).
    pub(in crate::lower) fn hint_reaches_own_where(&self, ty: ty::Ty<'tcx>, unseen_own: bool) -> bool {
        let ty = self.reveal(ty.peel_refs());
        if self.is_user_iterator(ty) {
            return self.writes_size_hint(ty);
        }
        if self.is_unknown(ty) {
            return true;
        }
        let held = ty.boxed_ty().unwrap_or(ty);
        if matches!(
            held.kind(),
            ty::Dynamic(..)
                | ty::Alias(
                    _,
                    ty::AliasTy {
                        kind: ty::Opaque { .. },
                        ..
                    }
                )
        ) {
            return unseen_own;
        }
        adapted_iterators(self.tcx, self.typing_env, ty).is_some_and(|held| {
            held.into_iter()
                .any(|held| self.hint_reaches_own_where(held, unseen_own))
        })
    }

    /// Whether an iterator of type `ty` gives no more items than an array's
    /// it steps, which exist already: std's adapters that give at most what
    /// their source does, over a slice's or a `Vec`'s items. What collects
    /// it asks no room it could lack (ADR 0940).
    pub(in crate::lower) fn bounded_by_items(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        if self.array_source(ty).is_some() {
            return true;
        }
        let ty::Adt(adt, args) = ty.kind() else { return false };
        let at_most_its_source = matches!(
            std_path_item(self.tcx, adt.did()),
            Some(
                StdPath::IterMap
                    | StdPath::IterFilter
                    | StdPath::IterFilterMap
                    | StdPath::IterSkip
                    | StdPath::IterTake
                    | StdPath::IterRev
                    | StdPath::IterEnumerate
                    | StdPath::IterPeekable
                    | StdPath::IterFuse
                    | StdPath::IterInspect
                    | StdPath::IterStepBy
                    | StdPath::IterTakeWhile
                    | StdPath::IterSkipWhile
                    | StdPath::IterMapWhile
                    | StdPath::IterCopied
                    | StdPath::IterCloned
            )
        );
        at_most_its_source && args.types().next().is_some_and(|source| self.bounded_by_items(source))
    }

    /// Whether asking the hint of an iterator of type `ty`, for room std
    /// reserves by its lower bound before it's stepped, a `String`'s bytes
    /// or a hash table's entries, may change what the program sees: where it
    /// may run the crate's own (`hint_reaches_own`), or where that bound may
    /// pass the room there is, `capacity overflow` before an item's taken
    /// (`lower_bound_may_pass_room`, #1358).
    pub(in crate::lower) fn hint_room_seen(&self, ty: Ty<'tcx>) -> bool {
        self.hint_reaches_own(ty) || self.lower_bound_may_pass_room(ty)
    }

    /// Whether JS's own adapter, made in place, of an iterator of type
    /// `ty` loses std's hint: one over a string's `chars()` or
    /// `char_indices()`, the runtime's `$chars`, whose hint JS's adapters
    /// keep none of (#1186), through std's adapters. Where its hint is asked,
    /// a `cycle()` above it, it's held, std's struct, whose hint asks its
    /// source's (#1367).
    pub(in crate::lower) fn in_place_hint_lost(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        adapted_iterators(self.tcx, self.typing_env, ty).is_some_and(|held| {
            (held.into_iter()).any(|held| {
                let held = self.reveal(held.peel_refs());
                self.is_str_chars(held) || self.is_char_indices(held) || self.in_place_hint_lost(held)
            })
        })
    }

    /// Whether std's lower bound of an iterator of type `ty` may pass the
    /// room a `String` or a hash table has, `isize::MAX` bytes: of an
    /// endless one, a `repeat`, `repeat_n`'s or `repeat_with`'s, an `n..`;
    /// a range of a type wider than 31 bits, `0..usize::MAX / 2 + 1`; a
    /// `cycle()` of what may have items, whose bound is then `usize::MAX`;
    /// through std's adapters that keep their source's lower bound or a
    /// part of it, a chain of either side, a zip of both. Not of what gives
    /// no more than what exists already, a collection's, an `Option`'s, a
    /// string's `chars()`, nor of a char's or a narrow type's range, nor of
    /// an adapter whose lower bound is none, a `filter`, a `flat_map` (#1358).
    pub(in crate::lower) fn lower_bound_may_pass_room(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        let ty::Adt(adt, args) = ty.kind() else { return false };
        let mut sources = args.types();
        let wide = |item: Ty<'tcx>| {
            !matches!(
                item.kind(),
                ty::Int(ty::IntTy::I8 | ty::IntTy::I16) | ty::Uint(ty::UintTy::U8 | ty::UintTy::U16)
            ) && item.is_integral()
        };
        match std_path_item(self.tcx, adt.did()) {
            Some(StdPath::IterRepeat | StdPath::IterRepeatN | StdPath::IterRepeatWith | StdPath::OpsRangeFrom) => true,
            Some(StdPath::OpsRange | StdPath::OpsRangeInclusive) => sources.next().is_some_and(wide),
            Some(StdPath::IterCycle) => sources
                .next()
                .is_some_and(|source| self.lower_bound_kept(source) != Some(false)),
            Some(
                StdPath::IterMap
                | StdPath::IterEnumerate
                | StdPath::IterInspect
                | StdPath::IterFuse
                | StdPath::IterPeekable
                | StdPath::IterCopied
                | StdPath::IterCloned
                | StdPath::IterRev
                | StdPath::IterSkip
                | StdPath::IterTake
                | StdPath::IterStepBy,
            ) => sources
                .next()
                .is_some_and(|source| self.lower_bound_may_pass_room(source)),
            Some(StdPath::IterChain) => sources.take(2).any(|source| self.lower_bound_may_pass_room(source)),
            Some(StdPath::IterZip) => sources.take(2).all(|source| self.lower_bound_may_pass_room(source)),
            _ => false,
        }
    }

    /// Whether a `collect()` into a `Vec` of an iterator of type `ty` checks
    /// std's room upfront by a count the program gives (ADR 0774): a
    /// `TrustedLen` whose count is a range's, a `repeat_n`'s, a `take(n)`'s of
    /// an endless one or of one counted so, a range's `step_by`, through
    /// adapters that keep it, or
    /// a chain or a zip of such; not a collection's items', whose room
    /// fits where they are (the guide's differences). The one answer of
    /// which generic collects ask their items' size, `Room` (#1436): the
    /// analysis asks it of these, and their lowering's room checks read it
    /// (`room_item_size`), so no other collect takes a `TSize`.
    pub(in crate::lower) fn counted_by_program(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        is_trusted_len(self.tcx, self.typing_env, ty) && self.count_of_program(ty)
    }

    /// Whether the count of an iterator of type `ty` is the program's
    /// (`counted_by_program`).
    fn count_of_program(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        if self.range_kind(ty).is_some() || is_repeat_n(self.tcx, ty) || self.endless_by_type(ty) {
            return true;
        }
        // A `take(n)`'s of an endless one, `n`, or of one counted so, the
        // smaller (#1441).
        if let Some((source, false)) = counted_source(self.tcx, ty) {
            return self.endless_by_type(source) || self.count_of_program(source);
        }
        if let Some(source) = stepped_by_source(self.tcx, ty) {
            return self.range_kind(self.reveal(source.peel_refs())).is_some();
        }
        if let Some(source) = length_kept_source(self.tcx, ty) {
            return self.count_of_program(source);
        }
        (zipped_sources(self.tcx, ty).or_else(|| chained_sources(self.tcx, ty)))
            .is_some_and(|(a, b)| self.count_of_program(a) && self.count_of_program(b))
    }

    /// Whether an iterator of type `ty` has no end its type knows: a
    /// `RangeFrom` or a `repeat()`, through std's adapters that keep each
    /// item, or a chain or a zip of such. Of a `TrustedLen` one, `collect()`
    /// into a `Vec` panics `capacity overflow` before it steps it, its upper
    /// bound none (ADR 0940).
    pub(in crate::lower) fn endless_by_type(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        let ty::Adt(adt, args) = ty.kind() else { return false };
        let mut sources = args.types();
        match std_path_item(self.tcx, adt.did()) {
            Some(StdPath::OpsRangeFrom | StdPath::IterRepeat) => true,
            Some(
                StdPath::IterMap
                | StdPath::IterEnumerate
                | StdPath::IterInspect
                | StdPath::IterFuse
                | StdPath::IterPeekable
                | StdPath::IterCopied
                | StdPath::IterCloned,
            ) => sources.next().is_some_and(|source| self.endless_by_type(source)),
            Some(StdPath::IterChain) => sources.take(2).any(|source| self.endless_by_type(source)),
            Some(StdPath::IterZip) => sources.take(2).all(|source| self.endless_by_type(source)),
            _ => false,
        }
    }

    /// Whether an iterator of type `ty` has no end its hint knows once it
    /// has given an item: its lower bound is then `usize::MAX`. That holds
    /// of what `endless_by_type` finds, of a `cycle()` of what has items
    /// (`lower_bound_kept`), and of a `skip(n)` of such, whose `n` is gone
    /// once it gives one, through std's adapters that keep each item, or of
    /// a chain or a zip of such. `collect()` into a `Vec` of one that isn't
    /// `TrustedLen` steps it once, as std's `SpecFromIter` does, then asks
    /// room for its lower bound and one more, and panics `capacity overflow`
    /// (ADR 0940, amended). `None` where only what it holds as it runs says:
    /// a chain of one such and another, whose hint has no end where its
    /// first item is the first side's; a `cycle()` of what its type doesn't
    /// say gives a lower bound above 0 where it has items; a `step_by(n)` of
    /// one such, whose lower bound is then `usize::MAX / n`.
    pub(in crate::lower) fn endless_once_stepped(&self, ty: Ty<'tcx>) -> Option<bool> {
        if self.endless_by_type(ty) {
            return Some(true);
        }
        let ty = self.reveal(ty.peel_refs());
        let ty::Adt(adt, args) = ty.kind() else {
            return Some(false);
        };
        let mut sources = args.types();
        let mut source = || {
            sources
                .next()
                .map_or(Some(false), |source| self.endless_once_stepped(source))
        };
        match std_path_item(self.tcx, adt.did()) {
            // Its hint is `usize::MAX` where what it started with has a
            // lower bound above 0, and none where it's 0.
            Some(StdPath::IterCycle) => sources.next().map_or(Some(false), |of| self.lower_bound_kept(of)),
            Some(
                StdPath::IterMap
                | StdPath::IterSkip
                | StdPath::IterEnumerate
                | StdPath::IterInspect
                | StdPath::IterFuse
                | StdPath::IterPeekable
                | StdPath::IterCopied
                | StdPath::IterCloned,
            ) => source(),
            Some(StdPath::IterStepBy) => match source() {
                Some(true) => None,
                other => other,
            },
            Some(StdPath::IterChain) => match (source(), source()) {
                (Some(a), Some(b)) if a == b => Some(a),
                _ => None,
            },
            // An item of both, each stepped.
            Some(StdPath::IterZip) => match (source(), source()) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            },
            _ => Some(false),
        }
    }

    /// Whether std's hint of an iterator of type `ty` has no lower bound, 0
    /// wherever it's asked after its first item, where JS counts what's
    /// left of the array it steps, `$iter`'s: a B-tree's `range(..)`, whose
    /// `size_hint` is std's default, `(0, None)`, and a set's
    /// `intersection(..)` and `symmetric_difference(..)`, and a hash set's
    /// `difference(..)`, each `(0, upper)`. Stepped so, of a crate that keeps
    /// capacities, its hint is std's, `$iter(items, true)` (#1215).
    pub(in crate::lower) fn hints_no_lower_bound(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        let ty::Adt(adt, _) = ty.kind() else { return false };
        // A string's pattern's, whose `size_hint` is std's default, and
        // what's filtered or mapped of one (#1180's (3)); a B-tree's range
        // and the sets' (#1215).
        self.is_str_split(ty)
            || matches!(
                std_path_item(self.tcx, adt.did()),
                Some(
                    StdPath::StrSplitN
                        | StdPath::StrRSplitN
                        | StdPath::StrRSplit
                        | StdPath::StrSplitTerminator
                        | StdPath::StrRSplitTerminator
                        | StdPath::StrSplitInclusive
                        | StdPath::StrMatchIndices
                        | StdPath::StrRMatchIndices
                        | StdPath::StrMatches
                        | StdPath::StrRMatches
                        | StdPath::StrSplitWhitespace
                        | StdPath::StrSplitAsciiWhitespace
                        | StdPath::StrLines
                        | StdPath::BtreeMapRange
                        | StdPath::BtreeMapRangeMut
                        | StdPath::BtreeSetRange
                        | StdPath::BtreeSetIntersection
                        | StdPath::BtreeSetSymmetricDifference
                        | StdPath::HashSetDifference
                        | StdPath::HashSetIntersection
                        | StdPath::HashSetSymmetricDifference
                )
            )
    }

    /// Whether std's hint of an iterator of type `ty` has a lower bound its
    /// merge's state gives as it's stepped: a B-tree set's `difference(..)`,
    /// what's left of the first less what's left of the second, and
    /// `union(..)`, the more of the two, and a hash set's `union(..)`,
    /// what's left of the larger (#1215), each made as std's where a crate
    /// keeps capacities (#1219).
    fn hints_merged_lower_bound(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        let ty::Adt(adt, _) = ty.kind() else { return false };
        matches!(
            std_path_item(self.tcx, adt.did()),
            Some(StdPath::BtreeSetDifference | StdPath::BtreeSetUnion | StdPath::HashSetUnion)
        )
    }

    /// Whether a `Vec` std grows of an iterator of type `ty` that isn't
    /// `TrustedLen` is given it knowing its length (`as_exact_size`), so
    /// JS's hint is std's: an `ExactSizeIterator`'s, whose lower bound is
    /// its length, of no array JS counts and no adapter's of JS's own,
    /// `drop(n)`'s, which know none (#1200). Not one whose lower bound is
    /// always 0, which JS's adapters' is too.
    pub(in crate::lower) fn grows_by_exact_len(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        self.lower_bound_kept(ty) != Some(false)
            && self
                .exact_size_iterator()
                .is_some_and(|exact| self.implements_trait(ty, exact))
    }

    /// Whether JS's hint of an iterator of type `ty` has std's lower bound,
    /// by which std grows a `Vec` it collects or extends where it isn't
    /// `TrustedLen` (#1180): always where std's is always 0
    /// (`lower_bound_kept`'s `Some(false)`), which JS's adapters' is too;
    /// else not where it steps, through std's adapters, an array JS counts
    /// of an iterator std doesn't, no `ExactSizeIterator`, a string's
    /// `chars()` or `split(..)` among them (`is_array_iter`).
    pub(in crate::lower) fn lower_bound_is_stds(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        if self.lower_bound_kept(ty) == Some(false) {
            return true;
        }
        if let Some(held) = crate::lower::recognition::adapted_iterators(self.tcx, self.typing_env, ty) {
            // An `ExactSizeIterator` adapter held by what isn't one, a
            // `chain`'s part, `a.iter().skip(1)`: JS's own adapters of an
            // array's iterator, `a.values().drop(1)`, which know no hint, so
            // its lower bound, its length, is lost. One that's collected is
            // given knowing its length (`grows_by_exact_len`) and never asked
            // here (#1200).
            if self.grows_by_exact_len(ty) {
                return false;
            }
            return held.into_iter().all(|held| self.lower_bound_is_stds(held));
        }
        // A set's merge, made as std's where a crate keeps capacities, a
        // B-tree's `$treeSetAlgebra` and a hash set's `$hashSetUnion`, whose
        // hint is its merge's as it's stepped, which JS's array of the
        // result doesn't keep (#1215, #1219).
        if self.hints_merged_lower_bound(ty) {
            return true;
        }
        // A string's `chars()` or `char_indices()`, lazy (#1186): JS's own
        // string iterator knows no hint, and JS's adapters of the runtime's
        // `$chars`, `drop(n)` and the like, keep none of its: refused, until
        // this asks what each adapter's JS keeps (#1200).
        if self.is_str_chars(ty) || self.is_char_indices(ty) {
            return false;
        }
        // A slice's split, whose lower bound std's says is at most 1, `(1,
        // Some(len + 1))` till it's done: JS's, the lazy form's the same and
        // the array of its pieces' 0 (`Iterator.from`), grows a `Vec` as std's
        // does, as std's first capacity is RawVec's least, 4 of a slice's
        // pieces, above `lower + 1` either way, and each growth doubles past
        // `len + lower + 1` (#1892).
        if is_slice_split(self.tcx, ty) {
            return true;
        }
        !self.is_array_iter(ty)
            || self
                .exact_size_iterator()
                .is_some_and(|exact| self.implements_trait(ty, exact))
    }

    /// Whether JS's hint of an iterator of type `ty`, a chain or a zip, or a
    /// copy of one, keeps std's lower bound once each part of it that's an
    /// `ExactSizeIterator` is given knowing its length
    /// (`hint_known_by_its_parts`): `a.iter().skip(1).chain(rest)`,
    /// whose parts' JS, `drop(1)`, knows no hint, and whose runtime chain
    /// sums its parts' as std's `Chain` does, a zip's the fewer (#1216).
    pub(in crate::lower) fn lower_bound_kept_by_parts(&self, ty: Ty<'tcx>) -> bool {
        self.lower_bound_kept_by_parts_where(ty, false)
    }

    /// `lower_bound_kept_by_parts`, where a string's `chars()` and
    /// `char_indices()` are the runtime's `$chars`, `chars_kept`, whose hint
    /// is std's: of a part of a chain or a zip, whose runtime one sums its
    /// parts' hints (`lower_bound_is_stds_where`, #1316).
    pub(in crate::lower) fn lower_bound_kept_by_parts_where(&self, ty: Ty<'tcx>, chars_kept: bool) -> bool {
        let ty = self.reveal(ty.peel_refs());
        if let Some(source) = super::copied_source(self.tcx, ty).or_else(|| super::mapped_source(self.tcx, ty)) {
            return self.lower_bound_kept_by_parts_where(source, chars_kept);
        }
        let Some((a, b)) = super::chained_sources(self.tcx, ty).or_else(|| super::zipped_sources(self.tcx, ty)) else {
            return false;
        };
        [a, b].into_iter().all(|part| {
            self.lower_bound_is_stds_where(part, chars_kept)
                || self.grows_by_exact_len(part)
                || self.lower_bound_kept_by_parts_where(part, chars_kept)
        })
    }

    /// `lower_bound_is_stds`, of an iterator as it's made, a chain's or a
    /// zip's part too, whose runtime one sums its parts' hints: a string's
    /// `chars()` or `char_indices()` where they're the runtime's `$chars`,
    /// `chars_kept`, as a crate that asks them more than their front's steps
    /// makes each (#1186), `((len + 3) / 4, Some(len))`; and a std source
    /// whose form gives std's hint, `repeat`'s `$repeating` and the like
    /// (`source_hint_kept`, #1357). Not an adapter of one, JS's own
    /// `drop(n)` of `$chars`, which keeps none (#1316).
    ///
    /// And of what passes on the hint of a source whose runtime form keeps
    /// std's (`hint_kept_by_form`): `s.char_indices().map(f).rev()` (#1572).
    pub(in crate::lower) fn lower_bound_is_stds_where(&self, ty: Ty<'tcx>, chars_kept: bool) -> bool {
        let made = self.reveal(ty.peel_refs());
        self.hint_kept_by_form(made, chars_kept) || self.lower_bound_is_stds(ty)
    }

    /// Whether the runtime form of an iterator of type `ty` keeps std's
    /// hint, as it's made: a string's `chars()` or `char_indices()` where
    /// they're `$chars`, `chars_kept` (#1186), and a std source whose form
    /// gives std's, `$repeating` and the like (`source_hint_kept`, #1357);
    /// and a `rev()`, a `take(n)`, a `copied()` or a `map()` of one, each a
    /// `$Steps` that passes its source's on as std's adapter does (#1572),
    /// and an `enumerate()` of one, `$enumerate`'s or std's compiled
    /// struct's, which each pass it on, `s.chars().enumerate()` (#1892); and a
    /// `rev()` of a flatten of what isn't arrays, `$flattenEnds`'s, std's
    /// `FlattenCompat`'s (#1892). Not an array's iterator, JS's own, nor a map of one,
    /// whose JS `map` knows none, however exactly std counts it; nor a
    /// `skip(n)`, whose JS `drop(n)` keeps none (#1316).
    pub(in crate::lower) fn hint_kept_by_form(&self, ty: Ty<'tcx>, chars_kept: bool) -> bool {
        let ty = self.reveal(ty.peel_refs());
        if chars_kept && (self.is_str_chars(ty) || self.is_char_indices(ty)) || super::source_hint_kept(self.tcx, ty) {
            return true;
        }
        // A `rev()` of a flatten, made `$flattenEnds` as it's made knowing
        // its back, which hints as std's `FlattenCompat`, its inner ones'
        // left and its source's once done (`$flatHint`, #1892): where its
        // inner iterators hint as std's. Not of arrays, whose
        // `ConstSizeIntoIterator` bound std's struct keeps, its source's
        // count times their length.
        if let Some(flattened) = super::reversed_source(self.tcx, ty)
            && let Some(inner) = self.flattened_inner(flattened.peel_refs())
            && let Some((source, made)) = super::flattened_source(self.tcx, flattened.peel_refs())
            && let Some(item) = made.or_else(|| self.iterator_item(source))
        {
            let item = item.peel_refs();
            return !item.is_array() && self.lower_bound_is_stds_where(inner, chars_kept);
        }
        super::copied_source(self.tcx, ty)
            .or_else(|| super::mapped_source(self.tcx, ty))
            .or_else(|| super::reversed_source(self.tcx, ty))
            .or_else(|| super::enumerated_source(self.tcx, ty))
            .or_else(|| {
                (super::counted_source(self.tcx, ty))
                    .filter(|(_, skip)| !skip)
                    .map(|(source, _)| source)
            })
            .is_some_and(|source| self.hint_kept_by_form(source, chars_kept))
    }

    /// `lower_bound_is_stds`, of an iterator of type `ty` a caller gives
    /// generic code that grows a `Vec` by it (#1180's (3)): a string's
    /// `chars()` or `char_indices()` given is the runtime's `$chars`, whose
    /// hint is std's, not JS's own string iterator, and so is a `rev()` of
    /// one, `$rev`'s, its source's. What JS adapts of one is as
    /// `lower_bound_is_stds` says, a `skip(n)`'s `drop(n)` none.
    pub(in crate::lower) fn given_hint_is_stds(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        if self.is_str_chars(ty) || self.is_char_indices(ty) {
            return true;
        }
        match crate::lower::recognition::reversed_source(self.tcx, ty) {
            Some(source) => self.given_hint_is_stds(source),
            None => self.lower_bound_is_stds(ty),
        }
    }

    /// Whether the hint of an iterator of type `ty` has a lower bound above
    /// 0 wherever it has items, `Some(true)`: an `ExactSizeIterator`'s, a
    /// `TrustedLen`'s, one with no end once stepped, or a chain of such. Or
    /// always 0, `Some(false)`: a `filter`'s and the
    /// others that may give none of what they step, through std's adapters
    /// that keep each item. `None` where only what it holds as it runs
    /// says: a chain of one of each, a `peekable()` of one always 0.
    fn lower_bound_kept(&self, ty: Ty<'tcx>) -> Option<bool> {
        let ty = self.reveal(ty.peel_refs());
        let exact = self
            .exact_size_iterator()
            .is_some_and(|exact| self.implements_trait(ty, exact));
        if exact
            || crate::lower::recognition::is_trusted_len(self.tcx, self.typing_env, ty)
            || self.endless_once_stepped(ty) == Some(true)
        {
            return Some(true);
        }
        if self.hints_no_lower_bound(ty) {
            return Some(false);
        }
        let ty::Adt(adt, args) = ty.kind() else { return None };
        let mut sources = args.types();
        let mut source = || sources.next().and_then(|source| self.lower_bound_kept(source));
        match std_path_item(self.tcx, adt.did()) {
            Some(
                StdPath::IterFilter
                | StdPath::IterFilterMap
                | StdPath::IterTakeWhile
                | StdPath::IterSkipWhile
                | StdPath::IterMapWhile
                | StdPath::IterScan
                | StdPath::IterFromFn,
            ) => Some(false),
            Some(
                StdPath::IterMap
                | StdPath::IterEnumerate
                | StdPath::IterInspect
                | StdPath::IterFuse
                | StdPath::IterCopied
                | StdPath::IterCloned,
            ) => source(),
            Some(StdPath::IterSkip | StdPath::IterTake | StdPath::IterStepBy) => match source() {
                Some(false) => Some(false),
                _ => None,
            },
            Some(StdPath::IterPeekable) => match source() {
                Some(true) => Some(true),
                _ => None,
            },
            Some(StdPath::IterChain) => match (source(), source()) {
                (Some(a), Some(b)) if a == b => Some(a),
                _ => None,
            },
            Some(StdPath::IterZip) => match (source(), source()) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            },
            _ => None,
        }
    }
}

/// A side of generic code's `a.eq(b)` (`eq_sides`): how its length, which
/// std's `SpecIterEq` compares first of two `TrustedLen` ones, is known.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum EqSide<'tcx> {
    /// Its type's, known here.
    Known,
    /// A type parameter's it's given as, `I` itself or `<I as
    /// IntoIterator>::IntoIter`, whose `LengthFirst` its caller gives
    /// (#1182).
    Given(Ty<'tcx>),
    /// Another made of type parameters, `Map<I, F>`, which its callers decide
    /// (`callers_decide`, ADR 1014).
    Built,
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether an iterator of type `iter`, given to generic code, is one
    /// whose JS knows how many it has left, its `len()` or its hint: an
    /// array's or a `Vec`'s, `$iter`, or a range of numbers, `$lazyRange`
    /// (#1182).
    pub(in crate::lower) fn counts_where_given(&self, iter: Ty<'tcx>) -> bool {
        matches!(iter.peel_refs().kind(), ty::Adt(adt, _) if matches!(std_path_item(self.tcx, adt.did()), Some(StdPath::SliceIter | StdPath::SliceIterMut | StdPath::VecIntoIter | StdPath::ArrayIntoIter | StdPath::OpsRange)))
    }

    /// The sides of `a.eq(b)`, of an iterator of type `this` and an
    /// `IntoIterator` of type `other`, each as its length is known
    /// (`EqSide`); `None` where std never compares their lengths first, of
    /// a side whose type is known not to be `TrustedLen`. The one answer of
    /// what generic code's `eq` asks its callers and of how it's lowered.
    pub(in crate::lower) fn eq_sides(&self, this: Ty<'tcx>, other: Ty<'tcx>) -> Option<[EqSide<'tcx>; 2]> {
        let trusted = super::fact_trait(self.tcx, TypeFact::TrustedLen)?;
        let into_iterator = super::std_item(self.tcx, StdItem::IntoIterator);
        // Of what an `impl Iterator` hides, revealed first as a body's MIR
        // is (`mir::revealed`, #1142), in any typing environment, as std's
        // specialization sees it: an opaque type is no projection, and the
        // type it hides is the side, a type parameter of the caller's or
        // one known here (#1229).
        let side = |ty: Ty<'tcx>| {
            let ty = crate::lower::mir::revealed(self.tcx, ty);
            match *ty.kind() {
                ty::Param(_) => Some(EqSide::Given(ty)),
                // A projection's alone: an opaque one left unrevealed has
                // no trait.
                ty::Alias(_, alias)
                    if matches!(alias.kind, ty::Projection { .. })
                        && alias.trait_def_id(self.tcx) == into_iterator
                        && matches!(alias.self_ty().kind(), ty::Param(_)) =>
                {
                    Some(EqSide::Given(alias.self_ty()))
                }
                _ if ty.has_param() => Some(EqSide::Built),
                _ => {
                    let iter = super::iterator_type_of(self.tcx, self.typing_env, ty);
                    self.implements_trait(iter, trusted).then_some(EqSide::Known)
                }
            }
        };
        Some([side(this)?, side(other)?])
    }
}

/// Whether `ty` is a slice's split by a predicate, `split`'s, `splitn`'s,
/// `rsplit`'s, `rsplitn`'s or `split_inclusive`'s, of shared items or
/// mutable ones, each of whose pieces is a slice, a pointer and a length.
fn is_slice_split<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    let ty::Adt(adt, _) = ty.kind() else { return false };
    matches!(
        std_path_item(tcx, adt.did()),
        Some(
            StdPath::SliceSplit
                | StdPath::SliceSplitMut
                | StdPath::SliceSplitN
                | StdPath::SliceSplitNMut
                | StdPath::SliceRSplit
                | StdPath::SliceRSplitMut
                | StdPath::SliceRSplitN
                | StdPath::SliceRSplitNMut
                | StdPath::SliceSplitInclusive
                | StdPath::SliceSplitInclusiveMut
        )
    )
}
