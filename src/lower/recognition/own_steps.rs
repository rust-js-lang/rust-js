//! What std asks of an iterator that reaches the crate's own: each of
//! std's steps the crate's iterator may write and std's adapters pass on,
//! `OwnStep`, and the one answer of whether a step of a type reaches the one
//! the crate's iterator writes (#162, #182, #220, #505, #521, ADR 1089).

use super::*;

/// A step of std's `Iterator` or `DoubleEndedIterator`, beside `next` and
/// `next_back`, that the crate's iterator may write and std's adapters
/// pass on to what they hold; and `try_fold`, which no crate's iterator
/// writes (its `Try` is unstable), but through which std's adapters reach
/// what it does: `Take::fold` is its source's `try_fold`, and
/// `Skip::try_fold` asks its source's `nth(n - 1)` (#2195).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum OwnStep {
    Fold,
    TryFold,
    Rfold,
    TryRfold,
    Nth,
    NthBack,
    Last,
    Count,
}

/// Of what an adapter asks a step of (`OwnStep::asked_of`), not one of its
/// type arguments but each inner iterator it makes of its items, a
/// flatten's (`Recognition::flattened_inner`).
const INNERS: usize = usize::MAX;

/// What a type parameter's answer is, whose caller's type may write the
/// step: `Sure`, of what drops nothing after, or gives a compiled body
/// nothing, as the crate's own took it, which says no but of a step whose
/// helper asks the iterator itself, `nth`'s, `last`'s, `count`'s and a
/// double-ended one's `nth_back`'s (#496, #505, #2134); `May`, of what asks the iterator itself at run time, `$foldOf`
/// and `$foldBack`, which says yes of each (#521, #649).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Reach {
    Sure,
    May,
}

impl OwnStep {
    /// Each of them.
    const ALL: [OwnStep; 8] = [
        OwnStep::Fold,
        OwnStep::TryFold,
        OwnStep::Rfold,
        OwnStep::TryRfold,
        OwnStep::Nth,
        OwnStep::NthBack,
        OwnStep::Last,
        OwnStep::Count,
    ];

    /// Its method's name in std's trait.
    pub(in crate::lower) fn method(self) -> &'static str {
        match self {
            OwnStep::Fold => "fold",
            OwnStep::TryFold => "try_fold",
            OwnStep::Rfold => "rfold",
            OwnStep::TryRfold => "try_rfold",
            OwnStep::Nth => "nth",
            OwnStep::NthBack => "nth_back",
            OwnStep::Last => "last",
            OwnStep::Count => "count",
        }
    }

    /// Whether it folds what it's taken from its front, `fold` or
    /// `try_fold`: an `nth` either asks is `nth_folded_sources`' answer.
    fn folds(self) -> bool {
        matches!(self, OwnStep::Fold | OwnStep::TryFold)
    }

    /// Whether it's `DoubleEndedIterator`'s, not `Iterator`'s.
    fn is_double_ended(self) -> bool {
        matches!(self, OwnStep::Rfold | OwnStep::TryRfold | OwnStep::NthBack)
    }

    /// Whether std's impl for a `&mut I` passes it on to `I`'s own, of a
    /// sized `I` or not (`Wrapper::passes`): `nth` and `nth_back`; its
    /// `fold` and `rfold` are `I`'s `try_fold`s, its `last` and `count`
    /// std's defaults, of `next`.
    fn through_a_mut_ref(self) -> bool {
        use super::std_steps::{Passed, Wrapper};
        [Wrapper::MutRefSized, Wrapper::MutRefUnsized]
            .iter()
            .all(|w| w.passes(self.method()) == Passed::Own)
    }

    /// Whether the helper that takes it, `$nth`, `$last`, `$count` or
    /// `$nthBack`, asks the iterator itself, so a type parameter may be
    /// said to have it wherever it's asked (#2134).
    fn asked_at_run_time(self) -> bool {
        matches!(self, OwnStep::Nth | OwnStep::Last | OwnStep::Count | OwnStep::NthBack)
    }

    /// The step of std's name `method`, of those the crate's iterator may
    /// write.
    fn of_method(method: &str) -> Option<OwnStep> {
        OwnStep::ALL.into_iter().find(|step| step.method() == method)
    }

    /// What std's adapter `adapter`, a name of `std::iter`'s, asks of what it
    /// holds as it takes this step, std's catalog's (`std_steps::asks`): each
    /// source, by its index among the adapter's type arguments, its source
    /// and a chain's or a zip's second, or each inner iterator a flatten
    /// makes of its items, `INNERS`; and each step it asks of it that the
    /// crate's iterator may write. None where std's adapter doesn't write
    /// the step, the trait's default, which asks the adapter's own steps.
    fn asked_of(self, adapter: &str) -> Vec<(usize, OwnStep)> {
        (super::std_steps::asks(adapter, self.method()).into_iter())
            .filter_map(|asked| {
                let at = match asked.of {
                    "src" => 0,
                    "other" => 1,
                    "inner" => INNERS,
                    of => unreachable!("std_steps.txt asks no source `{of}`"),
                };
                Some((at, OwnStep::of_method(asked.step)?))
            })
            .collect()
    }
}

/// The steps a std source, of std's `path`, writes itself, not std's
/// defaults of its `next`, which a JS form of it gives too, so what asks
/// them reaches them: `Repeat`'s `nth`, of its `advance_by`, which makes no
/// item, so clones none (repeat.rs:92), and its `last` and `count`, which
/// panic, as it never ends (repeat.rs:110, #1138); `RepeatN`'s `nth`, `last`, the
/// value itself, and `count`, its length, none of which clones what it
/// passes (repeat_n.rs:140, #1124).
fn own_source_steps(item: Option<StdPath>) -> Option<&'static [OwnStep]> {
    match item {
        Some(StdPath::IterRepeat) => Some(&[OwnStep::Nth, OwnStep::Last, OwnStep::Count]),
        Some(StdPath::IterRepeatN) => Some(&[OwnStep::Nth, OwnStep::Last, OwnStep::Count]),
        _ => None,
    }
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether std's `step` of a `ty`, an iterator, reaches the one the impl
    /// of a crate's iterator writes: of that iterator, and through a `&mut`
    /// of it where std's impl for it passes the step on; of each std
    /// adapter, through each step it asks of what it holds, std's catalog's
    /// (`OwnStep::asked_of`), its `try_fold` too, but a fold's `nth`, from
    /// its `fold` or its `try_fold`: a `step_by`'s or a `skip`'s reaching the
    /// crate's own `nth` is `nth_folded_sources`' answer, which folds it by
    /// its own;
    /// of a type generic code doesn't know, a type parameter or an
    /// associated type, `T::IntoIter` (`is_unknown`, #1595), as `reach`
    /// says. The one answer, each step's (ADR 1089).
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
            return self.is_unknown(ty)
                && (reach == Reach::May || step.asked_at_run_time() && self.may_write(ty, step));
        };
        if self.source_writes(ty, step) {
            return true;
        }
        let Some(named) = std_item_in(self.tcx, adt.did(), StdPath::IterMod) else {
            return false;
        };
        let adapter = named.as_str();
        (step.asked_of(adapter).into_iter())
            .filter(|&(_, asked)| !(step.folds() && matches!(asked, OwnStep::Nth)))
            .any(|(at, asked)| {
                self.asked_source(ty, args, at)
                    .is_some_and(|source| self.reaches_its_own(source, asked, reach))
            })
    }

    /// Whether a type generic code doesn't know, `ty`, may be given an
    /// iterator that writes `step`: a `DoubleEndedIterator`'s step only of
    /// one its bounds say is double-ended, as no other is stepped from its
    /// back.
    fn may_write(&self, ty: Ty<'tcx>, step: OwnStep) -> bool {
        !step.is_double_ended()
            || (self.declared_bounds(ty, self.typing_env.param_env).into_iter())
                .any(|bound| super::is_double_ended(self.tcx, bound))
    }

    /// Whether std's adapter of type `ty` reaches each of the crate's own
    /// steps `source`, what it holds, reaches: so a JS form of it may be its
    /// source itself, a `copied()` of references whose copy is the item, as
    /// std's `Copied` passes each step on; not a `cloned()` of one that
    /// writes `count`, whose `count` std's `Cloned` leaves to its default,
    /// its `fold` (#2122).
    pub(in crate::lower) fn reaches_as_its_source(&self, ty: Ty<'tcx>, source: Ty<'tcx>) -> bool {
        OwnStep::ALL
            .into_iter()
            .all(|step| !self.reaches_its_own(source, step, Reach::Sure) || self.reaches_its_own(ty, step, Reach::Sure))
    }

    /// Whether `ty` is a std source that writes `step` itself, not std's
    /// default of its `next` (`own_source_steps`): `repeat_n`'s `count`; and
    /// a slice's `chunk_by` by a predicate that runs code, `$lazyChunkBy`,
    /// whose `last` is its `next_back`, as std's `ChunkBy::last` is (#1501).
    fn source_writes(&self, ty: Ty<'tcx>, step: OwnStep) -> bool {
        let ty = self.reveal(ty);
        let ty::Adt(adt, _) = ty.kind() else {
            return false;
        };
        own_source_steps(std_path_item(self.tcx, adt.did())).is_some_and(|own| own.contains(&step))
            || step == OwnStep::Last
                && self.search_runs_code(ty)
                && matches!(self.tcx.item_name(adt.did()).as_str(), "ChunkBy" | "ChunkByMut")
    }

    /// The source `at` of an adapter of type `ty`, of type arguments
    /// `args`, that `OwnStep::asked_of` names: its type argument, or each
    /// inner iterator it makes, `INNERS`'s.
    fn asked_source(&self, ty: Ty<'tcx>, args: ty::GenericArgsRef<'tcx>, at: usize) -> Option<Ty<'tcx>> {
        match at {
            INNERS => self.flattened_inner(ty),
            at => args.types().nth(at),
        }
    }

    /// What std's `step` of a `ty`, one of std's adapters, asks of what it
    /// holds, `OwnStep::asked_of`'s: each source and the step it asks of it;
    /// none of another.
    pub(in crate::lower) fn asked_sources(&self, ty: Ty<'tcx>, step: OwnStep) -> Vec<(Ty<'tcx>, OwnStep)> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else {
            return Vec::new();
        };
        let Some(named) = std_item_in(self.tcx, adt.did(), StdPath::IterMod) else {
            return Vec::new();
        };
        let adapter = named.as_str();
        (step.asked_of(adapter).iter())
            .filter_map(|&(at, asked)| Some((self.asked_source(ty, args, at)?, asked)))
            .collect()
    }

    /// Whether what std asks of an iterator of type `ty` reaches the crate's
    /// own `nth`, `last`, `nth_back` or `count`, each of which an adapter
    /// that passes it on gives, a `fuse()`'s `count` its source's, and a
    /// `map` doesn't, its `count` its `fold`'s (#235, #1240).
    pub(in crate::lower) fn reaches_own_steps(&self, ty: Ty<'tcx>) -> bool {
        [OwnStep::Nth, OwnStep::Last, OwnStep::NthBack, OwnStep::Count]
            .into_iter()
            .any(|step| self.reaches_its_own(ty, step, Reach::Sure))
    }

    /// Whether an iterator of type `ty`, or one it holds through std's
    /// adapters, a `Box` or a reference, is the crate's that writes `nth`
    /// or `nth_back`, or of a type whose steps aren't seen: one generic
    /// code doesn't know (`is_unknown`), a `dyn`'s or an opaque type's. What
    /// an adapter between asks in place of `next`, and asks otherwise of
    /// what it holds by `next` than by `try_fold`: `Skip::next` asks
    /// `nth(n)` where its `try_fold` asks `nth(n - 1)` and folds, and
    /// `StepBy`'s first `next` asks `nth(0)` where its `try_fold` asks
    /// `next`; of the crate's iterator that writes neither, each asks its
    /// `next` alike. So a form that steps it by `try_fold` stands for std's
    /// adapter that steps it by `next` only where this is false, a filter
    /// of the `Some`s for a flatten of `Option`s (#2205). Of the type
    /// alone, each value of it alike, as its impls say.
    pub(in crate::lower) fn own_nth_within(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(super::held::iterator_held(ty));
        if self.is_user_iterator(ty) {
            return self.own_iterator_method(ty, "nth").is_some() || self.own_back_method(ty, "nth_back").is_some();
        }
        if matches!(
            ty.kind(),
            ty::Dynamic(..)
                | ty::Alias(
                    _,
                    ty::AliasTy {
                        kind: ty::Opaque { .. },
                        ..
                    }
                )
        ) {
            return true;
        }
        // Of a type generic code doesn't know, what its caller's may write,
        // `reaches_its_own`'s answer of what it may be given.
        if !matches!(ty.kind(), ty::Adt(..)) {
            return self.reaches_its_own(ty, OwnStep::Nth, Reach::May);
        }
        adapted_iterators(self.tcx, self.typing_env, ty)
            .is_some_and(|held| held.into_iter().any(|held| self.own_nth_within(held)))
    }

    /// Whether std's `fold` of a `ty`, an iterator, asks an `nth` that
    /// reaches the crate's own (#182), or may, of a type parameter.
    pub(in crate::lower) fn folds_by_its_own_nth(&self, ty: Ty<'tcx>) -> bool {
        (self.nth_folded_sources(ty).into_iter()).any(|source| self.reaches_its_own(source, OwnStep::Nth, Reach::Sure))
    }

    /// What std's `fold` of a `ty`, an iterator, asks an `nth` of, as std's
    /// catalog says (`asked_of`): a `skip`'s source, its `nth(n - 1)`, and a
    /// `step_by`'s, each step its `nth`; and those of each adapter's whose
    /// `fold` is its source's `fold` or `try_fold`, a `take`'s (#2195).
    pub(in crate::lower) fn nth_folded_sources(&self, ty: Ty<'tcx>) -> Vec<Ty<'tcx>> {
        self.nth_folded_by(ty, OwnStep::Fold)
    }

    /// What std's `step` of a `ty`, its `fold` or its `try_fold`, asks an
    /// `nth` of, `nth_folded_sources`' answer.
    fn nth_folded_by(&self, ty: Ty<'tcx>, step: OwnStep) -> Vec<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else {
            return Vec::new();
        };
        let Some(named) = std_item_in(self.tcx, adt.did(), StdPath::IterMod) else {
            return Vec::new();
        };
        let adapter = named.as_str();
        let mut sources = Vec::new();
        for (at, asked) in step.asked_of(adapter) {
            let Some(source) = args.types().nth(at) else {
                continue;
            };
            match asked {
                OwnStep::Nth => sources.push(source),
                asked if asked.folds() => sources.extend(self.nth_folded_by(source, asked)),
                _ => {}
            }
        }
        sources
    }
}
