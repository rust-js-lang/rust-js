//! What an iterator owns of what it gives: the items it was given by
//! value, which it drops with it, and which nothing else holds.

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Of a `Vec`'s `Splice` of type `ty`, its items, where its replacement
    /// is an array's, a `Vec`'s or an array's `IntoIter`, a slice's `Iter`
    /// or its `copied()`: stepping it runs nothing and can't panic, so,
    /// where its items have no destructor, its drop runs nothing either,
    /// and it's put in place where it's made, as nothing sees the `Vec` till
    /// it's dropped (`drops`, #1189).
    pub(in crate::lower) fn splice_of_array_items(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(_, args) = ty.kind() else { return None };
        if !self.is_vec_splice(ty) {
            return None;
        }
        let replacement = args.types().next()?;
        let array = |it: Ty<'tcx>| {
            self.is_std_type(it, StdItem::SliceIter)
                || self.is_std_type(it, StdItem::ArrayIntoIter)
                || matches!(it.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::vec::IntoIter")
        };
        let copied = matches!(replacement.kind(), ty::Adt(adt, held) if std_path(self.tcx, adt.did()) == "std::iter::Copied"
            && held.types().next().is_some_and(|it| self.is_std_type(it, StdItem::SliceIter)));
        (array(replacement) || copied)
            .then(|| self.iterator_item(replacement))
            .flatten()
    }

    /// What an iterator that owns its items gives, `vec::IntoIter`'s,
    /// `drain(..)`'s, a map's `(K, V)`, its `into_keys()`'s `K` and its
    /// `into_values()`'s `V`, or `option::IntoIter`'s, each of which it drops
    /// with it.
    pub(in crate::lower) fn owned_items(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else { return None };
        if !(self.array_source(ty) == Some(true) || self.owns_one_item(ty)) {
            return None;
        }
        // A map's `drain()`'s and `into_iter()`'s, its entries, `(K, V)`.
        if matches!(
            std_path(self.tcx, adt.did()),
            "std::collections::hash_map::Drain"
                | "std::collections::hash_map::IntoIter"
                | "std::collections::btree_map::IntoIter"
        ) {
            let mut types = args.types();
            return Some(Ty::new_tup(self.tcx, &[types.next()?, types.next()?]));
        }
        // A map's `into_values()`'s, its values, `V` of `<K, V>` (#432).
        if matches!(
            std_path(self.tcx, adt.did()),
            "std::collections::hash_map::IntoValues" | "std::collections::btree_map::IntoValues"
        ) {
            return args.types().nth(1);
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
            if matches!(std_path(self.tcx, adt.did()), "std::option::IntoIter" | "std::iter::Once"))
    }

    /// Whether `ty` gives what nothing else holds: an owning iterator's
    /// items (`owned_items`), a `Copied`'s or a `Cloned`'s, a copy made as
    /// each is given, or those an `Enumerate`, a `Rev`, a `Skip`, a `Take`,
    /// a `StepBy` or a `Fuse` of one gives on (#141).
    pub(in crate::lower) fn gives_owned_items(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        let ty::Adt(adt, args) = ty.kind() else { return false };
        self.owned_items(ty).is_some()
            || match std_path(self.tcx, adt.did()) {
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
