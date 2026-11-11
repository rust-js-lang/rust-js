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
        let args = self.args_of(iterator, ty.peel_refs());
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
            std_path(self.tcx, adt.did()),
            "std::iter::Map"
                | "std::iter::Filter"
                | "std::iter::FilterMap"
                | "std::iter::Skip"
                | "std::iter::Take"
                | "std::iter::Rev"
                | "std::iter::Enumerate"
                | "std::iter::Peekable"
                | "std::iter::Fuse"
                | "std::iter::Inspect"
                | "std::iter::StepBy"
                | "std::iter::TakeWhile"
                | "std::iter::SkipWhile"
                | "std::iter::MapWhile"
                | "std::iter::Copied"
                | "std::iter::Cloned"
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
        match std_path(self.tcx, adt.did()) {
            "std::iter::Repeat" | "std::iter::RepeatN" | "std::iter::RepeatWith" | "std::ops::RangeFrom" => true,
            "std::ops::Range" | "std::ops::RangeInclusive" => sources.next().is_some_and(wide),
            "std::iter::Cycle" => sources
                .next()
                .is_some_and(|source| self.lower_bound_kept(source) != Some(false)),
            "std::iter::Map"
            | "std::iter::Enumerate"
            | "std::iter::Inspect"
            | "std::iter::Fuse"
            | "std::iter::Peekable"
            | "std::iter::Copied"
            | "std::iter::Cloned"
            | "std::iter::Rev"
            | "std::iter::Skip"
            | "std::iter::Take"
            | "std::iter::StepBy" => sources
                .next()
                .is_some_and(|source| self.lower_bound_may_pass_room(source)),
            "std::iter::Chain" => sources.take(2).any(|source| self.lower_bound_may_pass_room(source)),
            "std::iter::Zip" => sources.take(2).all(|source| self.lower_bound_may_pass_room(source)),
            _ => false,
        }
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
        match std_path(self.tcx, adt.did()) {
            "std::ops::RangeFrom" | "std::iter::Repeat" => true,
            "std::iter::Map"
            | "std::iter::Enumerate"
            | "std::iter::Inspect"
            | "std::iter::Fuse"
            | "std::iter::Peekable"
            | "std::iter::Copied"
            | "std::iter::Cloned" => sources.next().is_some_and(|source| self.endless_by_type(source)),
            "std::iter::Chain" => sources.take(2).any(|source| self.endless_by_type(source)),
            "std::iter::Zip" => sources.take(2).all(|source| self.endless_by_type(source)),
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
        match std_path(self.tcx, adt.did()) {
            // Its hint is `usize::MAX` where what it started with has a
            // lower bound above 0, and none where it's 0.
            "std::iter::Cycle" => sources.next().map_or(Some(false), |of| self.lower_bound_kept(of)),
            "std::iter::Map"
            | "std::iter::Skip"
            | "std::iter::Enumerate"
            | "std::iter::Inspect"
            | "std::iter::Fuse"
            | "std::iter::Peekable"
            | "std::iter::Copied"
            | "std::iter::Cloned" => source(),
            "std::iter::StepBy" => match source() {
                Some(true) => None,
                other => other,
            },
            "std::iter::Chain" => match (source(), source()) {
                (Some(a), Some(b)) if a == b => Some(a),
                _ => None,
            },
            // An item of both, each stepped.
            "std::iter::Zip" => match (source(), source()) {
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
                std_path(self.tcx, adt.did()),
                "std::str::SplitN"
                    | "std::str::RSplitN"
                    | "std::str::RSplit"
                    | "std::str::SplitTerminator"
                    | "std::str::RSplitTerminator"
                    | "std::str::SplitInclusive"
                    | "std::str::MatchIndices"
                    | "std::str::RMatchIndices"
                    | "std::str::Matches"
                    | "std::str::RMatches"
                    | "std::str::SplitWhitespace"
                    | "std::str::SplitAsciiWhitespace"
                    | "std::str::Lines"
                    | "std::collections::btree_map::Range"
                    | "std::collections::btree_map::RangeMut"
                    | "std::collections::btree_set::Range"
                    | "std::collections::btree_set::Intersection"
                    | "std::collections::btree_set::SymmetricDifference"
                    | "std::collections::hash_set::Difference"
                    | "std::collections::hash_set::Intersection"
                    | "std::collections::hash_set::SymmetricDifference"
            )
    }

    /// Whether std's hint of an iterator of type `ty` has a lower bound its
    /// merge's state gives as it's stepped, which JS's array of the result
    /// doesn't keep: a B-tree set's `difference(..)`, what's left of the
    /// first less what's left of the second, and `union(..)`, the more of
    /// the two, and a hash set's `union(..)`, what's left of the larger
    /// (#1215).
    fn hints_merged_lower_bound(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        let ty::Adt(adt, _) = ty.kind() else { return false };
        matches!(
            std_path(self.tcx, adt.did()),
            "std::collections::btree_set::Difference"
                | "std::collections::btree_set::Union"
                | "std::collections::hash_set::Union"
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
        // A set's merge, whose lower bound JS's array doesn't keep (#1215).
        if self.hints_merged_lower_bound(ty) {
            return false;
        }
        // A string's `chars()` or `char_indices()`, lazy (#1186): JS's own
        // string iterator knows no hint, and JS's adapters of the runtime's
        // `$chars`, `drop(n)` and the like, keep none of its: refused, until
        // this asks what each adapter's JS keeps (#1200).
        if self.is_str_chars(ty) || self.is_char_indices(ty) {
            return false;
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
        let ty = self.reveal(ty.peel_refs());
        if let Some(source) = super::copied_source(self.tcx, ty).or_else(|| super::mapped_source(self.tcx, ty)) {
            return self.lower_bound_kept_by_parts(source);
        }
        let Some((a, b)) = super::chained_sources(self.tcx, ty).or_else(|| super::zipped_sources(self.tcx, ty)) else {
            return false;
        };
        [a, b].into_iter().all(|part| {
            self.lower_bound_is_stds(part) || self.grows_by_exact_len(part) || self.lower_bound_kept_by_parts(part)
        })
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
        match std_path(self.tcx, adt.did()) {
            "std::iter::Filter"
            | "std::iter::FilterMap"
            | "std::iter::TakeWhile"
            | "std::iter::SkipWhile"
            | "std::iter::MapWhile"
            | "std::iter::Scan"
            | "std::iter::FromFn" => Some(false),
            "std::iter::Map"
            | "std::iter::Enumerate"
            | "std::iter::Inspect"
            | "std::iter::Fuse"
            | "std::iter::Copied"
            | "std::iter::Cloned" => source(),
            "std::iter::Skip" | "std::iter::Take" | "std::iter::StepBy" => match source() {
                Some(false) => Some(false),
                _ => None,
            },
            "std::iter::Peekable" => match source() {
                Some(true) => Some(true),
                _ => None,
            },
            "std::iter::Chain" => match (source(), source()) {
                (Some(a), Some(b)) if a == b => Some(a),
                _ => None,
            },
            "std::iter::Zip" => match (source(), source()) {
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
        matches!(iter.peel_refs().kind(), ty::Adt(adt, _) if matches!(
            std_path(self.tcx, adt.did()),
            "std::slice::Iter" | "std::slice::IterMut" | "std::vec::IntoIter" | "std::array::IntoIter" | "std::ops::Range"
        ))
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
