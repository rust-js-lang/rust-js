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
}
