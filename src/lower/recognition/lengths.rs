//! What an iterator's type says of how many items it gives, which what
//! collects it asks room for (ADR 0940).

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
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
            std_path(self.tcx, adt.did()).as_str(),
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
        match std_path(self.tcx, adt.did()).as_str() {
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
        match std_path(self.tcx, adt.did()).as_str() {
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
        match std_path(self.tcx, adt.did()).as_str() {
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
