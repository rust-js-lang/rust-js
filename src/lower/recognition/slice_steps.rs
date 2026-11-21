//! Which of std's steps of an iterator reach a source that drops what it
//! passes over as one slice, a `Vec`'s `IntoIter`'s `advance_by` and
//! `advance_back_by`, front to back and past a panicking item, where a step
//! by `next` drops each in turn (#234, #668, #670).

use super::*;
use crate::lower::recognition::OwnStep;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether `ty`'s `nth` drops what it passes over as one slice: one
    /// whose `nth` asks, through the `OwnStep` catalog's adapters, the
    /// `nth_back` of a `Vec`'s, which drops them front to back where each
    /// passed from its back drops back to front: a `Rev`'s, a `Rev` of a
    /// `take(n)`'s, a `skip(k)`'s, an `enumerate()`'s or a `fuse()`'s, each
    /// of whose `nth_back` asks its source's (#234, #261, #697, #2239).
    pub(in crate::lower) fn nths_a_slice(&self, ty: Ty<'tcx>) -> bool {
        self.passes_a_slice(ty, OwnStep::Nth, false)
    }

    /// Whether `ty`'s `nth` drops what it passes over as one slice, of a
    /// `Vec`'s from its front too, whose `advance_by` drops them in the
    /// order it steps them: only a drop that panics tells that slice from
    /// each dropped as it's passed, as a slice's drop goes on past it, the
    /// rest of it dropped before what unwinds (#668).
    pub(in crate::lower) fn nths_a_slice_in_order(&self, ty: Ty<'tcx>) -> bool {
        self.passes_a_slice(ty, OwnStep::Nth, true)
    }

    /// Whether stepping `ty` by its `next` first asks an `nth` that drops
    /// what it passes as one slice, a `Skip`'s of a reversed `Vec`: of an
    /// adapter whose `next` asks it, or asks its source's `next` that does,
    /// the catalog's (`asked_by_next`), a map's, a fuse's, a take's, a
    /// take-while's. Its `fold` asks `nth(n - 1)` then the next, which drop
    /// otherwise, so what's stepped as std's `StepBy` steps it, by `next`,
    /// isn't folded (#294, #2239).
    pub(in crate::lower) fn steps_past_a_slice(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        (self.asked_by_next(ty).into_iter()).any(|(source, asked)| match asked {
            "nth" => self.nths_a_slice(source),
            "next" => self.steps_past_a_slice(source),
            _ => false,
        })
    }

    /// Whether `ty`'s `step`, its `nth` or its `nth_back`, drops what it
    /// passes over as one slice: a `Vec`'s `nth_back`; its `nth`, `in_order`;
    /// or what the `OwnStep` catalog says it asks of what it holds, an `nth`
    /// or an `nth_back`, does: a `Rev`'s `nth` its source's `nth_back`, a
    /// `take`'s `nth_back` its source's.
    fn passes_a_slice(&self, ty: Ty<'tcx>, step: OwnStep, in_order: bool) -> bool {
        let ty = self.reveal(ty.peel_refs());
        if (step == OwnStep::NthBack || in_order) && self.drops_passed_as_slice(ty) {
            return true;
        }
        (self.asked_sources(ty, step).into_iter())
            .filter(|&(_, asked)| matches!(asked, OwnStep::Nth | OwnStep::NthBack))
            .any(|(source, asked)| self.passes_a_slice(source, asked, in_order))
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
                OwnStep::Fold | OwnStep::TryFold | OwnStep::Rfold | OwnStep::TryRfold => {
                    found.extend(self.nth_back_sources(source, asked))
                }
                OwnStep::Nth | OwnStep::Last | OwnStep::Count => {}
            }
        }
        found
    }
}
