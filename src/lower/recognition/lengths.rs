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
        let ty = self.reveal(ty.peel_refs());
        if self.is_user_iterator(ty) {
            return self.writes_size_hint(ty);
        }
        if self.is_unknown(ty) {
            return true;
        }
        adapted_iterators(self.tcx, self.typing_env, ty)
            .is_some_and(|held| held.into_iter().any(|held| self.hint_reaches_own(held)))
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
            return held.into_iter().all(|held| self.lower_bound_is_stds(held));
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
        let side = |ty: Ty<'tcx>| match *ty.kind() {
            ty::Param(_) => Some(EqSide::Given(ty)),
            ty::Alias(_, alias)
                if alias.trait_def_id(self.tcx) == into_iterator && matches!(alias.self_ty().kind(), ty::Param(_)) =>
            {
                Some(EqSide::Given(alias.self_ty()))
            }
            _ if ty.has_param() => Some(EqSide::Built),
            _ => {
                let iter = super::iterator_type_of(self.tcx, self.typing_env, ty);
                self.implements_trait(iter, trusted).then_some(EqSide::Known)
            }
        };
        Some([side(this)?, side(other)?])
    }
}
