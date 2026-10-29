//! What an iterator owns of what it gives: the items it was given by
//! value, which it drops with it, and which nothing else holds.

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// What an iterator that owns its items gives, `vec::IntoIter`'s,
    /// `drain(..)`'s, a map's `(K, V)`, or `option::IntoIter`'s, each of which
    /// it drops with it.
    pub(in crate::lower) fn owned_items(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else { return None };
        if !(self.array_source(ty) == Some(true) || self.owns_one_item(ty)) {
            return None;
        }
        // A map's `drain()`'s and `into_iter()`'s, its entries, `(K, V)`.
        if matches!(
            std_path(self.tcx, adt.did()).as_str(),
            "std::collections::hash_map::Drain"
                | "std::collections::hash_map::IntoIter"
                | "std::collections::btree_map::IntoIter"
        ) {
            let mut types = args.types();
            return Some(Ty::new_tup(self.tcx, &[types.next()?, types.next()?]));
        }
        // A `splice`'s, what it took out, its replacing iterator's items
        // (ADR 0501).
        if std_path(self.tcx, adt.did()) == "std::vec::Splice" {
            return self.iterator_item(args.types().next()?);
        }
        args.types().next()
    }

    /// What a value given as an iterable owns and drops with what it hasn't
    /// given, `zip(v)`'s `Vec`: an owning iterator's items, or a `Vec`'s, a
    /// `VecDeque`'s or an array's, given by value.
    pub(in crate::lower) fn owned_into_items(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        if let Some(item) = self.owned_items(ty) {
            return Some(item);
        }
        match ty.kind() {
            ty::Array(item, _) => Some(*item),
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::Vec) || self.is_std_type(ty, StdItem::VecDeque) => {
                args.types().next()
            }
            _ => None,
        }
    }

    /// An iterator that owns the one item it may give, `Some(x).into_iter()`
    /// or `iter::once(x)`: the array of none or one (ADR 0128).
    pub(in crate::lower) fn owns_one_item(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _)
            if matches!(std_path(self.tcx, adt.did()).as_str(), "std::option::IntoIter" | "std::iter::Once"))
    }

    /// Whether `ty` gives what nothing else holds: an owning iterator's
    /// items (`owned_items`), a `Copied`'s or a `Cloned`'s, a copy made as
    /// each is given, or those an `Enumerate`, a `Rev`, a `Skip`, a `Take`,
    /// a `StepBy` or a `Fuse` of one gives on (#141).
    pub(in crate::lower) fn gives_owned_items(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        let ty::Adt(adt, args) = ty.kind() else { return false };
        self.owned_items(ty).is_some()
            || match std_path(self.tcx, adt.did()).as_str() {
                "std::iter::Copied" | "std::iter::Cloned" => true,
                "std::iter::Enumerate"
                | "std::iter::Rev"
                | "std::iter::Skip"
                | "std::iter::Take"
                | "std::iter::StepBy"
                | "std::iter::Fuse" => self.gives_owned_items(args.type_at(0)),
                _ => false,
            }
    }
}
