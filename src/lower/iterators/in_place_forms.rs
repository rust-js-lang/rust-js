//! The forms std's iterator adapters, and the consumers lowering makes in
//! place, are made as, and the std type each stands for: the one owner of
//! which runtime form is a std adapter's (#2112, Change A step 3; ADR
//! 1176). A form answers its std type's row of std's steps catalog
//! (recognition/std_steps.txt) for every source it's chosen for, so no
//! consumer has to choose around it. `in_place_forms.txt` is this owner's
//! table, which test/iterator-steps.test.ts runs each form against as it's
//! emitted, both ways: each pair here run, each form it runs here.
//! `BLESS=1 cargo test in_place_forms` writes it.

use super::*;
use crate::lower::combinators::IterComb;
use crate::lower::recognition::{OwnStep, Reach};

/// A form a std adapter, or a consumer of one, is made as in place, by
/// the runtime's helpers or JS's own iterator methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::lower) enum InPlaceForm {
    /// `src.map(f)`, JS's own.
    StepsMap,
    /// `$mapEnds(src, f)`, of a source stepped from its back too.
    MapEnds,
    /// `$copied(src, copy)`.
    Copied,
    /// `src.filter(p)`, JS's own.
    StepsFilter,
    /// `$filterEnds(src, p)`.
    FilterEnds,
    /// `$filterDropping(src, p, drop)`, each it passes over dropped.
    FilterDropping,
    /// `$somes(src.map(f))`: the `Some`s, whose own `count` is their fold.
    Somes,
    /// `$somes` of a map that knows its back: `$filterEnds`' of it.
    SomesOfEnds,
    /// `src.map(f).filter((item) => item != null)`: the `Some`s as JS
    /// writes them, where asking the source's hint runs nothing.
    SomesFilter,
    /// `$enumerate(src)`.
    Enumerate,
    /// `$skipping(src, n)`.
    Skipping,
    /// `$taking(src, n)`.
    Taking,
    /// `$stepBy(src, n)`, by its source's `nth`, as std's `StepBy` steps.
    StepBy,
    /// `$stepByEnds(src, n)`.
    StepByEnds,
    /// `src.filter((_, i) => i % n === 0)`: an array's items, whose steps
    /// run nothing of the crate's.
    StepByFilter,
    /// `$lazyChain(a, b)`.
    LazyChain,
    /// `$lazyZip(a, b)`.
    LazyZip,
    /// `$zipEnds(a, b)`.
    ZipEnds,
    /// `$rev(src)`.
    Rev,
    /// `$flatMap(src, each)`: each inner iterator's own `count` and `last`.
    FlatMap,
    /// `$flattenEnds(src, each)`.
    FlattenEnds,
    /// `src.flatMap(each)`, JS's own, of inner iterators nothing drops
    /// whose `count` and `last` aren't the crate's.
    StepsFlatMap,
    /// `$lazyTakeWhile(src, p)`.
    LazyTakeWhile,
    /// `$lazySkipWhile(src, p)`.
    LazySkipWhile,
    /// `$lazyMapWhile(src, f, present)`.
    LazyMapWhile,
    /// `$lazyScan(src, init, f)`.
    LazyScan,
    /// `$cycleOf(src, clone, next)`'s `$cycleFrom`.
    CycleFrom,
    /// The consumer `nth(n)` as `$nth(src, n)`, its source's own.
    Nth,
    /// The consumer `nth(n)` as `[...src.drop(n).take(1)][0]`, of a source
    /// whose `nth` isn't the crate's.
    NthDropTake,
    /// The consumer `find_map(f)` as `src.map(f).find((item) => item != null)`.
    FindMapFind,
}

/// When a form is chosen, as the net runs it: a condition each value of
/// the source's type has alike, or MIR's (Campaign 3's classes 1 and 2,
/// #2200), that holds wherever the form is emitted, never a crate-wide
/// fact, so the net's runs over the sources it allows can't hide a form a
/// fact chose, #2197's kind (the lead, 2026-11-20). A choice may also ask
/// what narrows it further; the net runs the form wherever this holds.
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::lower) enum Where {
    /// Of any source.
    Always,
    /// Where asking the source's `size_hint` runs nothing of the crate's
    /// (`hint_reaches_own_where` false).
    HintRunsNothing,
    /// Of an array's items (`bounded_by_items`), whose steps run nothing
    /// of the crate's.
    OfArrayItems,
    /// Of a source whose `nth` isn't the crate's (`reaches_its_own` of
    /// `Nth` false).
    NoOwnNth,
    /// Of inner iterators whose `count` and `last` aren't the crate's
    /// (`reaches_its_own` of `Count` and `Last` false) and whose items
    /// have no destructor (`drops`).
    PlainInners,
}

#[cfg(test)]
impl Where {
    pub(in crate::lower) const ALL: [Where; 5] = [
        Where::Always,
        Where::HintRunsNothing,
        Where::OfArrayItems,
        Where::NoOwnNth,
        Where::PlainInners,
    ];

    /// The APIs that answer it, each an owner's of docs/architecture.md,
    /// which a test holds to a row of class 1 or 2.
    pub(in crate::lower) fn asks(self) -> &'static [&'static str] {
        match self {
            Where::Always => &[],
            Where::HintRunsNothing => &["hint_reaches_own_where"],
            Where::OfArrayItems => &["bounded_by_items"],
            Where::NoOwnNth => &["reaches_its_own"],
            Where::PlainInners => &["reaches_its_own", "drops"],
        }
    }
}

impl InPlaceForm {
    pub(in crate::lower) const ALL: [InPlaceForm; 30] = [
        InPlaceForm::StepsMap,
        InPlaceForm::MapEnds,
        InPlaceForm::Copied,
        InPlaceForm::StepsFilter,
        InPlaceForm::FilterEnds,
        InPlaceForm::FilterDropping,
        InPlaceForm::Somes,
        InPlaceForm::SomesOfEnds,
        InPlaceForm::SomesFilter,
        InPlaceForm::Enumerate,
        InPlaceForm::Skipping,
        InPlaceForm::Taking,
        InPlaceForm::StepBy,
        InPlaceForm::StepByEnds,
        InPlaceForm::StepByFilter,
        InPlaceForm::LazyChain,
        InPlaceForm::LazyZip,
        InPlaceForm::ZipEnds,
        InPlaceForm::Rev,
        InPlaceForm::FlatMap,
        InPlaceForm::FlattenEnds,
        InPlaceForm::StepsFlatMap,
        InPlaceForm::LazyTakeWhile,
        InPlaceForm::LazySkipWhile,
        InPlaceForm::LazyMapWhile,
        InPlaceForm::LazyScan,
        InPlaceForm::CycleFrom,
        InPlaceForm::Nth,
        InPlaceForm::NthDropTake,
        InPlaceForm::FindMapFind,
    ];

    /// The std types it stands for, as std_steps.txt names its rows, or
    /// of a consumer, std's method, `nth`.
    #[cfg(test)]
    pub(in crate::lower) fn stands_for(self) -> &'static [&'static str] {
        match self {
            InPlaceForm::StepsMap | InPlaceForm::MapEnds => &["Map", "Cloned", "Inspect"],
            InPlaceForm::Copied => &["Copied"],
            InPlaceForm::StepsFilter | InPlaceForm::FilterEnds | InPlaceForm::FilterDropping => &["Filter"],
            InPlaceForm::Somes | InPlaceForm::SomesOfEnds | InPlaceForm::SomesFilter => {
                &["FilterMap", "Flatten<Option>", "FlatMap<Option>"]
            }
            InPlaceForm::Enumerate => &["Enumerate"],
            InPlaceForm::Skipping => &["Skip"],
            InPlaceForm::Taking => &["Take"],
            InPlaceForm::StepBy | InPlaceForm::StepByEnds | InPlaceForm::StepByFilter => &["StepBy"],
            InPlaceForm::LazyChain => &["Chain"],
            InPlaceForm::LazyZip | InPlaceForm::ZipEnds => &["Zip"],
            InPlaceForm::Rev => &["Rev"],
            InPlaceForm::FlatMap | InPlaceForm::FlattenEnds | InPlaceForm::StepsFlatMap => &["Flatten", "FlatMap"],
            InPlaceForm::LazyTakeWhile => &["TakeWhile"],
            InPlaceForm::LazySkipWhile => &["SkipWhile"],
            InPlaceForm::LazyMapWhile => &["MapWhile"],
            InPlaceForm::LazyScan => &["Scan"],
            InPlaceForm::CycleFrom => &["Cycle"],
            InPlaceForm::Nth | InPlaceForm::NthDropTake => &["nth"],
            InPlaceForm::FindMapFind => &["find_map"],
        }
    }

    /// Its name as the net runs it.
    #[cfg(test)]
    pub(in crate::lower) fn name(self) -> &'static str {
        match self {
            InPlaceForm::StepsMap => "$Steps.map",
            InPlaceForm::MapEnds => "$mapEnds",
            InPlaceForm::Copied => "$copied",
            InPlaceForm::StepsFilter => "$Steps.filter",
            InPlaceForm::FilterEnds => "$filterEnds",
            InPlaceForm::FilterDropping => "$filterDropping",
            InPlaceForm::Somes => "$somes",
            InPlaceForm::SomesOfEnds => "$somes of ends",
            InPlaceForm::SomesFilter => "$Steps.filter of somes",
            InPlaceForm::Enumerate => "$enumerate",
            InPlaceForm::Skipping => "$skipping",
            InPlaceForm::Taking => "$taking",
            InPlaceForm::StepBy => "$stepBy",
            InPlaceForm::StepByEnds => "$stepByEnds",
            InPlaceForm::StepByFilter => "$Steps.filter by index",
            InPlaceForm::LazyChain => "$lazyChain",
            InPlaceForm::LazyZip => "$lazyZip",
            InPlaceForm::ZipEnds => "$zipEnds",
            InPlaceForm::Rev => "$rev",
            InPlaceForm::FlatMap => "$flatMap",
            InPlaceForm::FlattenEnds => "$flattenEnds",
            InPlaceForm::StepsFlatMap => "$Steps.flatMap",
            InPlaceForm::LazyTakeWhile => "$lazyTakeWhile",
            InPlaceForm::LazySkipWhile => "$lazySkipWhile",
            InPlaceForm::LazyMapWhile => "$lazyMapWhile",
            InPlaceForm::LazyScan => "$lazyScan",
            InPlaceForm::CycleFrom => "$cycleFrom",
            InPlaceForm::Nth => "$nth",
            InPlaceForm::NthDropTake => "$Steps.drop.take",
            InPlaceForm::FindMapFind => "$Steps.map.find",
        }
    }

    /// Where it's chosen, as the net runs it (`Where`).
    #[cfg(test)]
    pub(in crate::lower) fn chosen(self) -> Where {
        match self {
            InPlaceForm::SomesFilter => Where::HintRunsNothing,
            InPlaceForm::StepByFilter => Where::OfArrayItems,
            InPlaceForm::NthDropTake => Where::NoOwnNth,
            InPlaceForm::StepsFlatMap => Where::PlainInners,
            _ => Where::Always,
        }
    }
}

/// The owner's table, `in_place_forms.txt`: each form's std types, its
/// name and where it's chosen, then each `Where`'s APIs.
#[cfg(test)]
pub(in crate::lower) fn table() -> String {
    let mut out = String::from(
        "# The forms lowering makes std's adapters and consumers as in place, and the\n\
         # std type each stands for (src/lower/iterators/in_place_forms.rs; never by\n\
         # hand: `BLESS=1 cargo test in_place_forms` writes it). Each line,\n\
         # `<std type> | <form> | <where>`; each `where <name>: <APIs>`, what\n\
         # answers it, each an owner of class 1 or 2 (#2112, #2200).\n",
    );
    for form in InPlaceForm::ALL {
        for std in form.stands_for() {
            out.push_str(&format!("{std} | {} | {:?}\n", form.name(), form.chosen()));
        }
    }
    for at in Where::ALL {
        let asks: String = at.asks().iter().map(|api| format!(" {api}")).collect();
        out.push_str(&format!("where {at:?}:{asks}\n"));
    }
    out
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The form `comb`, one of the in-place adapters and consumers whose
    /// form is chosen between several, is made as, of a `receiver_ty`
    /// giving an `output`: the one decision of it. `None` of another,
    /// whose form is one alone, or which its arm decides by what it's
    /// given.
    pub(in crate::lower) fn in_place_form(
        &self,
        comb: IterComb,
        receiver_ty: Ty<'tcx>,
        output: Ty<'tcx>,
    ) -> Option<InPlaceForm> {
        let form = match comb {
            // The `Some`s: `$somes` where the source's `size_hint` may run
            // the crate's code, whose own `count` is std's `FilterMap`'s and
            // `Flatten`'s, not `Filter::count`'s, which asks that hint first
            // (#2199). Its type says, of every value of it alike, whatever
            // form a fact made it (`hint_reaches_own_where`, of a type
            // parameter's and what a `dyn` holds too): no fact chooses it.
            // Of a `filter_map`, a `flat_map` to `Option`s and a flatten of
            // them (`flattened_options`).
            IterComb::FilterMap | IterComb::FlatMap | IterComb::Flatten => {
                let somes = match self.recognition().hint_reaches_own_where(receiver_ty, true) {
                    true => InPlaceForm::Somes,
                    false => InPlaceForm::SomesFilter,
                };
                let mapped = self
                    .recognition()
                    .flattened_options(output)
                    .map(|(_, _, mapped)| mapped);
                match comb {
                    IterComb::FilterMap => Some(somes),
                    IterComb::FlatMap => (mapped == Some(true)).then_some(somes),
                    _ => (mapped == Some(false)).then_some(somes),
                }
            }
            IterComb::Nth => Some(
                match self
                    .recognition()
                    .reaches_its_own(receiver_ty, OwnStep::Nth, Reach::Sure)
                {
                    true => InPlaceForm::Nth,
                    false => InPlaceForm::NthDropTake,
                },
            ),
            IterComb::StepBy => Some(match self.steps_by_index(receiver_ty, output) {
                true => InPlaceForm::StepByFilter,
                false => InPlaceForm::StepBy,
            }),
            _ => None,
        };
        // A form the table lists, which the net runs as it's emitted.
        debug_assert!(form.is_none_or(|form| InPlaceForm::ALL.contains(&form)));
        form
    }

    /// Whether a `step_by(n)` of a `receiver_ty` giving an `output` is a
    /// filter of its items by index: of an array's items alone
    /// (`bounded_by_items`), whose steps run nothing of the crate's, and
    /// none of what std's `StepBy`, which steps its source by its `nth`,
    /// does otherwise: one that may give more after a `None`, which a
    /// `None` stops (#161); one whose `nth` reaches the crate's own, which
    /// std asks (#214); a range's numbers, whose `nth` is of its bounds,
    /// which tidy makes `$lazyRange(a, b, n)` where it's made there (#231);
    /// a zip's, whose fold, std's `SpecFold`, stops where the shorter ends
    /// (#654); a flatten's, what its inner iterator hasn't given dropped
    /// after its source as a panic unwinds (#668); and its hint std's, its
    /// source's count `n` apart, which a collect asks its room by, where a
    /// filter's is none at least (#1046).
    fn steps_by_index(&self, receiver_ty: Ty<'tcx>, output: Ty<'tcx>) -> bool {
        let recognition = self.recognition();
        recognition.bounded_by_items(receiver_ty)
            // Not of a `Vec`'s reversed, whose `nth` drops what it passes
            // over as one slice (#234), or of one whose first step asks a
            // `Skip`'s `nth` of one (#294).
            && !recognition.nths_a_slice(receiver_ty)
            && !recognition.steps_past_a_slice(receiver_ty)
            // Nor of one whose fold moves a function an adapter holds with
            // a destructor, which a filter's fold would drop before what's
            // left, where std's `StepBy` steps it by `nth`, so it drops
            // after (#710).
            && !self.folds_held_function(receiver_ty.peel_refs())
            // Nor of a `Vec`'s, whose `nth` drops them as one slice in
            // order, of items whose drop may panic, past which the slice's
            // goes on (#668).
            && !((self.iterator_item(receiver_ty.peel_refs())).is_some_and(|item| self.drop_may_panic(item))
                && recognition.nths_a_slice_in_order(receiver_ty))
            // Nor of a source whose steps may panic: std's `StepBy` takes
            // its first `next` while its fold's frame holds only its
            // parameters, so what drops with the consumer's `f` drops
            // before the accumulator, `$stepBy`'s `$stepHolding`, where a
            // filter's first step is inside its source's fold, which drops
            // the accumulator first (#773).
            && !self.steps_may_panic(receiver_ty.peel_refs())
            // Nor of one generic code asks the hint of, given it, whose
            // hint `$stepBy` knows as std's (#1236).
            && !self.made_knowing_its_hint(output)
    }

    /// The form a flatten of inner iterators of type `inner`, each giving
    /// what it gives, is made as: `$flatMap` where `own` says, of inner
    /// iterators whose `count` or `last` is the crate's (#736), or of items
    /// with a destructor (ADR 0510); else JS's own `flatMap`.
    pub(in crate::lower) fn flattened_form(&self, inner: Ty<'tcx>, own: bool) -> InPlaceForm {
        let gives = self.iterable_item(inner);
        let form = match own || self.drops(gives) != crate::lower::drops::Drops::Nothing {
            true => InPlaceForm::FlatMap,
            false => InPlaceForm::StepsFlatMap,
        };
        debug_assert!(InPlaceForm::ALL.contains(&form));
        form
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `in_place_forms.txt` is the one `table` makes:
    /// `BLESS=1 cargo test in_place_forms` writes it.
    #[test]
    fn in_place_forms() {
        let made = table();
        if std::env::var_os("BLESS").is_some() {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lower/iterators/in_place_forms.txt");
            std::fs::write(path, &made).expect("the table written");
        } else {
            assert_eq!(
                include_str!("in_place_forms.txt"),
                made,
                "src/lower/iterators/in_place_forms.txt: BLESS=1 cargo test in_place_forms"
            );
        }
    }

    /// Each form's name is its own, and each `Where` one a form is chosen
    /// by, or `Always`.
    #[test]
    fn each_form_named_once() {
        let mut names: Vec<_> = InPlaceForm::ALL.iter().map(|form| form.name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), InPlaceForm::ALL.len());
        for at in Where::ALL {
            assert!(at == Where::Always || InPlaceForm::ALL.iter().any(|form| form.chosen() == at));
        }
    }
}
