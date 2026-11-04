//! What std asks of an iterator that reaches the crate's own: each of
//! std's steps the crate's iterator may write and std's adapters pass on,
//! `OwnStep`, and the one answer of whether a step of a type reaches the one
//! the crate's iterator writes (#162, #182, #220, #505, #521, ADR 1089).

use super::*;

/// A step of std's `Iterator` or `DoubleEndedIterator`, beside `next` and
/// `next_back`, that the crate's iterator may write and std's adapters
/// pass on to what they hold.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum OwnStep {
    Fold,
    Rfold,
    TryRfold,
    Nth,
    NthBack,
    Last,
    Count,
}

/// What a type parameter's answer is, whose caller's type may write the
/// step: `Sure`, of what drops nothing after, or gives a compiled body
/// nothing, as the crate's own took it, which says no but of a step whose
/// helper asks the iterator itself, `nth`'s, `last`'s and `count`'s (#496,
/// #505); `May`, of what asks the iterator itself at run time, `$foldOf`
/// and `$foldBack`, which says yes of each (#521, #649).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Reach {
    Sure,
    May,
}

impl OwnStep {
    /// Its method's name in std's trait.
    pub(in crate::lower) fn method(self) -> &'static str {
        match self {
            OwnStep::Fold => "fold",
            OwnStep::Rfold => "rfold",
            OwnStep::TryRfold => "try_rfold",
            OwnStep::Nth => "nth",
            OwnStep::NthBack => "nth_back",
            OwnStep::Last => "last",
            OwnStep::Count => "count",
        }
    }

    /// Whether it's `DoubleEndedIterator`'s, not `Iterator`'s.
    fn is_double_ended(self) -> bool {
        matches!(self, OwnStep::Rfold | OwnStep::TryRfold | OwnStep::NthBack)
    }

    /// Whether std's impl for a `&mut I` passes it on to `I`'s: `nth` and
    /// `nth_back`; its `fold` and `rfold` are `I`'s `try_fold`s, its `last`
    /// and `count` std's defaults, of `next`.
    fn through_a_mut_ref(self) -> bool {
        matches!(self, OwnStep::Nth | OwnStep::NthBack)
    }

    /// Whether the helper that takes it, `$nth`, `$last` or `$count`, asks
    /// the iterator itself, so a type parameter may be said to have it
    /// wherever it's asked.
    fn asked_at_run_time(self) -> bool {
        matches!(self, OwnStep::Nth | OwnStep::Last | OwnStep::Count)
    }

    /// What std's adapter `adapter`, a name of `std::iter`'s, asks of what it
    /// holds as it takes this step: each source, by its index among the
    /// adapter's type arguments, and the step it asks of it. A `rev()` asks
    /// each from the other end (#217, #220); a chain both sides, but for
    /// `nth`, its second's once its first runs out (#214), and for
    /// `nth_back`, its first's, past its second (#220); a `take` from its
    /// back its source's `nth_back` past what it won't give, then its
    /// `rfold` or `try_rfold`; a `skip`'s `rfold` its `try_rfold`; a
    /// peekable's `rfold` its source's, after what it peeked at (#649).
    fn asked_of(self, adapter: &str) -> &'static [(usize, OwnStep)] {
        use OwnStep::*;
        let own: &'static [(usize, OwnStep)] = match self {
            Fold => &[(0, Fold)],
            Rfold => &[(0, Rfold)],
            TryRfold => &[(0, TryRfold)],
            Nth => &[(0, Nth)],
            NthBack => &[(0, NthBack)],
            Last => &[(0, Last)],
            Count => &[(0, Count)],
        };
        match (self, adapter) {
            (Fold, "Rev") => &[(0, Rfold)],
            (Rfold, "Rev") => &[(0, Fold)],
            (Nth, "Rev") => &[(0, NthBack)],
            (NthBack, "Rev") => &[(0, Nth)],
            (Fold, "Chain") => &[(0, Fold), (1, Fold)],
            (Rfold, "Chain") => &[(0, Rfold), (1, Rfold)],
            (TryRfold, "Chain") => &[(0, TryRfold), (1, TryRfold)],
            (Last, "Chain") => &[(0, Last), (1, Last)],
            (Count, "Chain") => &[(0, Count), (1, Count)],
            (Nth, "Chain") => &[(1, Nth)],
            (NthBack, "Chain") => &[(0, NthBack)],
            (
                Fold,
                "Map" | "Filter" | "FilterMap" | "Enumerate" | "Inspect" | "Skip" | "SkipWhile" | "Copied" | "Cloned"
                | "Fuse" | "Peekable" | "FlatMap" | "Flatten" | "Take" | "TakeWhile" | "Scan" | "MapWhile",
            ) => own,
            (Rfold, "Map" | "Filter" | "Enumerate" | "Fuse" | "Peekable") => own,
            (Rfold, "Take") => &[(0, Rfold), (0, NthBack)],
            (Rfold, "Skip") => &[(0, TryRfold)],
            (TryRfold, "Take") => &[(0, NthBack), (0, TryRfold)],
            (TryRfold, "Map" | "Filter" | "Enumerate" | "Skip") => own,
            (Nth, "Skip" | "Take" | "StepBy" | "Enumerate" | "Fuse" | "Peekable") => own,
            (NthBack, "Skip" | "Take" | "Enumerate" | "Fuse") => own,
            (Last, "Skip" | "Fuse" | "Peekable" | "Copied" | "Cloned") => own,
            (Count, "Skip" | "Fuse" | "Peekable" | "Enumerate" | "Copied" | "Cloned") => own,
            _ => &[],
        }
    }
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether std's `step` of a `ty`, an iterator, reaches the one the impl
    /// of a crate's iterator writes: of that iterator, and through a `&mut`
    /// of it where std's impl for it passes the step on; of each std
    /// adapter, through what it asks of what it holds (`OwnStep::asked_of`);
    /// of a type parameter, as `reach` says. The one
    /// answer, each step's (ADR 1089).
    pub(in crate::lower) fn reaches_its_own(&self, ty: Ty<'tcx>, step: OwnStep, reach: Reach) -> bool {
        let ty = self.reveal(ty);
        if let ty::Ref(_, inner, ty::Mutability::Mut) = ty.kind() {
            return step.through_a_mut_ref() && self.reaches_its_own(*inner, step, reach);
        }
        if self.is_user_iterator(ty) {
            let own = match step.is_double_ended() {
                true => self.own_back_method(ty, step.method()),
                false => self.own_iterator_method(ty, step.method()),
            };
            return own.is_some();
        }
        let ty::Adt(adt, args) = ty.kind() else {
            return matches!(ty.kind(), ty::Param(_)) && (reach == Reach::May || step.asked_at_run_time());
        };
        let path = std_path(self.tcx, adt.did());
        let Some(adapter) = path.strip_prefix("std::iter::") else {
            return false;
        };
        (step.asked_of(adapter).iter()).any(|&(at, asked)| {
            args.types()
                .nth(at)
                .is_some_and(|source| self.reaches_its_own(source, asked, reach))
        })
    }

    /// What std's `step` of a `ty`, one of std's adapters, asks of what it
    /// holds, `OwnStep::asked_of`'s: each source and the step it asks of it;
    /// none of another.
    pub(in crate::lower) fn asked_sources(&self, ty: Ty<'tcx>, step: OwnStep) -> Vec<(Ty<'tcx>, OwnStep)> {
        let ty::Adt(adt, args) = self.reveal(ty).kind() else {
            return Vec::new();
        };
        let path = std_path(self.tcx, adt.did());
        let Some(adapter) = path.strip_prefix("std::iter::") else {
            return Vec::new();
        };
        (step.asked_of(adapter).iter())
            .filter_map(|&(at, asked)| Some((args.types().nth(at)?, asked)))
            .collect()
    }

    /// Whether what std asks of an iterator of type `ty` reaches the crate's
    /// own `nth`, `last` or `nth_back`, each of which an adapter that
    /// passes it on gives, and a `map` doesn't (#235).
    pub(in crate::lower) fn reaches_own_steps(&self, ty: Ty<'tcx>) -> bool {
        [OwnStep::Nth, OwnStep::Last, OwnStep::NthBack]
            .into_iter()
            .any(|step| self.reaches_its_own(ty, step, Reach::Sure))
    }

    /// Whether std's `fold` of a `ty`, an iterator, asks an `nth` that
    /// reaches the crate's own (#182), or may, of a type parameter.
    pub(in crate::lower) fn folds_by_its_own_nth(&self, ty: Ty<'tcx>) -> bool {
        (self.nth_folded_sources(ty).into_iter()).any(|source| self.reaches_its_own(source, OwnStep::Nth, Reach::Sure))
    }

    /// What std's `fold` of a `ty`, an iterator, asks an `nth` of: a
    /// `skip`'s source, its `nth(n - 1)`, and a `step_by`'s, each step its
    /// `nth`; and those of each adapter's whose `fold` is its source's.
    pub(in crate::lower) fn nth_folded_sources(&self, ty: Ty<'tcx>) -> Vec<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else {
            return Vec::new();
        };
        let path = std_path(self.tcx, adt.did());
        let Some(adapter) = path.strip_prefix("std::iter::") else {
            return Vec::new();
        };
        let mut sources = Vec::new();
        if matches!(adapter, "Skip" | "StepBy") {
            sources.extend(args.types().next());
        }
        for &(at, asked) in OwnStep::Fold.asked_of(adapter) {
            if asked == OwnStep::Fold
                && let Some(source) = args.types().nth(at)
            {
                sources.extend(self.nth_folded_sources(source));
            }
        }
        sources
    }
}
