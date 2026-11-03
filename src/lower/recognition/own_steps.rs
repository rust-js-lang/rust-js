//! Whether what std asks of an iterator, its `count` or its `last`,
//! reaches the crate's own, which std's adapters pass on (#182, #505).

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether std's `count` of a `ty`, an iterator, reaches the `count` the
    /// impl of a crate's iterator writes, or may, of a type parameter: of
    /// that iterator, and of each adapter whose `count` is its source's (#505).
    pub(in crate::lower) fn counts_its_own(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        if self.is_user_iterator(ty) {
            return !ty.is_ref() && self.own_iterator_method(ty, "count").is_some();
        }
        let ty::Adt(adt, args) = ty.kind() else {
            return matches!(ty.kind(), ty::Param(_));
        };
        let sources = match std_path(self.tcx, adt.did()).strip_prefix("std::iter::") {
            Some("Skip" | "Fuse" | "Peekable" | "Enumerate" | "Copied" | "Cloned") => 1,
            Some("Chain") => 2,
            _ => return false,
        };
        args.types().take(sources).any(|source| self.counts_its_own(source))
    }

    /// Whether std's `last` of a `ty`, an iterator, reaches the `last` the
    /// impl of a crate's iterator writes (#182): of that iterator, and of
    /// each adapter whose `last` is its source's, `skip`'s (after its
    /// `nth(n - 1)`), `fuse`'s, `peekable`'s, `copied`'s and `cloned`'s, and
    /// a chain's, of either side; or may, of a type parameter (#496). Not
    /// through a `&mut`, whose `last` is std's, of `fold`.
    pub(in crate::lower) fn lasts_its_own(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        if self.is_user_iterator(ty) {
            return !ty.is_ref() && self.own_iterator_method(ty, "last").is_some();
        }
        let ty::Adt(adt, args) = ty.kind() else {
            return matches!(ty.kind(), ty::Param(_));
        };
        let sources = match std_path(self.tcx, adt.did()).strip_prefix("std::iter::") {
            Some("Skip" | "Fuse" | "Peekable" | "Copied" | "Cloned") => 1,
            Some("Chain") => 2,
            _ => return false,
        };
        args.types().take(sources).any(|source| self.lasts_its_own(source))
    }
}
