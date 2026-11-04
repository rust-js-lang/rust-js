//! Which of std's steps of an iterator reach a source that drops what it
//! passes over as one slice, a `Vec`'s `IntoIter`'s `advance_by` and
//! `advance_back_by`, front to back and past a panicking item, where a step
//! by `next` drops each in turn (#234, #668, #670).

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether `ty`'s `nth` drops what it passes over as one slice: a `Rev`
    /// of one that does so from its back, a `Vec`'s, or a `StepBy`, a
    /// `Skip`, a `Take`, a `Fuse`, a `Peekable` or an `Enumerate` of one whose `nth` does,
    /// which ask it, or a `Chain` with such a side (#234, #261).
    pub(in crate::lower) fn nths_a_slice(&self, ty: Ty<'tcx>) -> bool {
        self.nth_drops_a_slice(ty, false)
    }

    /// Whether `ty`'s `nth` drops what it passes over as one slice, of a
    /// `Vec`'s from its front too, whose `advance_by` drops them in the
    /// order it steps them: only a drop that panics tells that slice from
    /// each dropped as it's passed, as a slice's drop goes on past it, the
    /// rest of it dropped before what unwinds (#668).
    pub(in crate::lower) fn nths_a_slice_in_order(&self, ty: Ty<'tcx>) -> bool {
        self.nth_drops_a_slice(ty, true)
    }

    /// `nths_a_slice`'s, and of a `Vec`'s from its front, `in_order`.
    fn nth_drops_a_slice(&self, ty: Ty<'tcx>, in_order: bool) -> bool {
        let ty = self.reveal(ty.peel_refs());
        if let Some(source) = self.reversed(ty) {
            return self.drops_passed_as_slice(source);
        }
        if in_order && self.drops_passed_as_slice(ty) {
            return true;
        }
        match ty.kind() {
            ty::Adt(adt, args)
                if matches!(
                    std_path(self.tcx, adt.did()).as_str(),
                    "std::iter::StepBy"
                        | "std::iter::Skip"
                        | "std::iter::Take"
                        | "std::iter::Fuse"
                        | "std::iter::Peekable"
                        | "std::iter::Enumerate"
                ) =>
            {
                self.nth_drops_a_slice(args.type_at(0), in_order)
            }
            // A `Chain`'s, each side's `advance_by` (#261).
            ty::Adt(adt, args) if std_path(self.tcx, adt.did()) == "std::iter::Chain" => {
                self.nth_drops_a_slice(args.type_at(0), in_order) || self.nth_drops_a_slice(args.type_at(1), in_order)
            }
            _ => false,
        }
    }

    /// What std's `fold` of a `ty`, an iterator, asks an `nth_back` of: of a
    /// `rev()`, what its source's `rfold` asks one of; and those of each
    /// adapter's whose `fold` is its source's. A `take(n).rev()`'s fold asks
    /// its source's `nth_back(len - n - 1)`, then drops what that gives,
    /// where its `next` asks `nth_back(len - n)`: of a `Vec`'s, which drops
    /// what it passes as one slice, front to back, the two drop in another
    /// order (#670).
    pub(in crate::lower) fn nth_back_folded_sources(&self, ty: Ty<'tcx>) -> Vec<Ty<'tcx>> {
        let ty = self.reveal(ty);
        if let Some(reversed) = self.reversed_source(ty) {
            return self.nth_back_rfolded_sources(reversed);
        }
        (self.folded_sources(ty))
            .flat_map(|source| self.nth_back_folded_sources(source))
            .collect()
    }

    /// What std's `rfold` of a `ty`, a double-ended iterator, asks an
    /// `nth_back` of: a `take`'s source, past what it won't give; and those
    /// of each adapter's whose `rfold` is its source's, as `rfolds_its_own`
    /// follows them.
    fn nth_back_rfolded_sources(&self, ty: Ty<'tcx>) -> Vec<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else {
            return Vec::new();
        };
        let (asked, sources) = match std_path(self.tcx, adt.did()).strip_prefix("std::iter::") {
            Some("Take") => (true, 1),
            Some("Map" | "Filter" | "Enumerate" | "Fuse" | "Skip") => (false, 1),
            Some("Chain") => (false, 2),
            _ => (false, 0),
        };
        let sources: Vec<Ty<'tcx>> = args.types().take(sources).collect();
        let mut found = if asked { sources.clone() } else { Vec::new() };
        for source in sources {
            found.extend(self.nth_back_rfolded_sources(source));
        }
        found
    }
}
