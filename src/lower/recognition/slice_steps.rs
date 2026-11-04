//! Which of std's steps of an iterator reach a source that drops what it
//! passes over as one slice, a `Vec`'s `IntoIter`'s `advance_by` and
//! `advance_back_by`, front to back and past a panicking item, where a step
//! by `next` drops each in turn (#234, #668, #670).

use super::*;
use crate::lower::recognition::OwnStep;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether `ty`'s `nth` drops what it passes over as one slice: a `Rev`
    /// of one that does so from its back, a `Vec`'s, or an adapter whose
    /// `nth` asks one that does, the `OwnStep` catalog's, or a `Chain` with
    /// such a side (#234, #261, #697).
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
        // What its `nth` asks an `nth` or an `advance_by` of, the `OwnStep`
        // catalog's.
        let asked = (self.asked_sources(ty, OwnStep::Nth).into_iter())
            .filter(|&(_, asked)| asked == OwnStep::Nth)
            .map(|(source, _)| source);
        let mut sources = asked.chain(self.nth_advanced_source(ty));
        sources.any(|source| self.nth_drops_a_slice(source, in_order))
    }

    /// What std's `fold` of a `ty`, an iterator, asks an `nth_back` of:
    /// the `OwnStep` catalog's answer, through each adapter whose fold, from
    /// either end, is its source's. A `take(n).rev()`'s fold asks its
    /// source's `nth_back(len - n - 1)`, then drops what that gives, where
    /// its `next` asks `nth_back(len - n)`: of a `Vec`'s, which drops what it
    /// passes as one slice, front to back, the two drop in another order
    /// (#670); so does one with a `peekable()` or a `skip(k)` of a `fuse()`
    /// between them, whose `rfold` or `try_rfold` is its source's (#697).
    pub(in crate::lower) fn nth_back_folded_sources(&self, ty: Ty<'tcx>) -> Vec<Ty<'tcx>> {
        self.nth_back_sources(ty, OwnStep::Fold)
    }

    /// What std's `step` of a `ty`, a fold from either end, asks an
    /// `nth_back` of (`asked_sources`): each source asked one, and those of
    /// each asked a fold.
    fn nth_back_sources(&self, ty: Ty<'tcx>, step: OwnStep) -> Vec<Ty<'tcx>> {
        let mut found = Vec::new();
        for (source, asked) in self.asked_sources(ty, step) {
            match asked {
                OwnStep::NthBack => found.push(source),
                OwnStep::Fold | OwnStep::Rfold | OwnStep::TryRfold => {
                    found.extend(self.nth_back_sources(source, asked))
                }
                OwnStep::Nth | OwnStep::Last | OwnStep::Count => {}
            }
        }
        found
    }
}
