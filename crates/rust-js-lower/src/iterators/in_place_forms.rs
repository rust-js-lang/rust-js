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
use crate::combinators::IterComb;
use crate::recognition::{OwnStep, Reach};

/// A form a std adapter, or a consumer of one, is made as in place, by
/// the runtime's helpers or JS's own iterator methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InPlaceForm {
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
    /// writes them, where asking the source's hint runs nothing, and of a
    /// flatten, where its steps reach nothing of the crate's by `try_fold`
    /// that they wouldn't by `next`.
    SomesFilter,
    /// `$someItems(src.map(f))`: a flatten's items of its `Option`s,
    /// std's `FlattenCompat` of one-shot inner iterators, which steps its
    /// source by `next` (#2205).
    SomeItems,
    /// `$someItems` of a map that knows its back: `$somes` of ends', its
    /// steps from its back its source's `next_back`.
    SomeItemsOfEnds,
    /// `$enumerate(src)`, which passes its source's `nth` on.
    Enumerate,
    /// `src.map((x, i) => [i, x])`, JS's own, which passes nothing on: of
    /// a source whose `nth` no adapter between may ask in place of `next`.
    StepsEnumerate,
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
    /// whose `count`, `last` and hint aren't the crate's.
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
pub(crate) enum Where {
    /// Of any source.
    Always,
    /// Where asking the source's `size_hint` runs nothing of the crate's
    /// (`hint_reaches_own_where` false).
    HintRunsNothing,
    /// Where what std's adapter asks of its source by `next` its form may
    /// ask by `try_fold`: no crate's iterator within it writes `nth` or
    /// `nth_back`, nor is a type within unseen (`own_nth_within` false),
    /// and asking its hint runs nothing of the crate's
    /// (`hint_reaches_own_where` false).
    StepsRunNothing,
    /// Of an array's items (`bounded_by_items`), whose steps run nothing
    /// of the crate's.
    OfArrayItems,
    /// Of a source whose `nth` isn't the crate's (`reaches_its_own` of
    /// `Nth` false).
    NoOwnNth,
    /// Where no crate's iterator within the source writes `nth` or
    /// `nth_back`, nor is a type within unseen (`own_nth_within` false): an
    /// adapter between that steps it by `next` asks what std's would.
    NoOwnNthWithin,
    /// Of inner iterators whose `count` and `last` aren't the crate's
    /// (`reaches_its_own` of `Count` and `Last` false), whose hint runs
    /// nothing of the crate's (`hint_reaches_own_where` false) and whose
    /// items have no destructor (`drops`).
    PlainInners,
}

#[cfg(test)]
impl Where {
    pub(crate) const ALL: [Where; 7] = [
        Where::Always,
        Where::HintRunsNothing,
        Where::StepsRunNothing,
        Where::OfArrayItems,
        Where::NoOwnNth,
        Where::NoOwnNthWithin,
        Where::PlainInners,
    ];

    /// The APIs that answer it, each an owner's of docs/architecture.md,
    /// which a test holds to a row of class 1 or 2.
    pub(crate) fn asks(self) -> &'static [&'static str] {
        match self {
            Where::Always => &[],
            Where::HintRunsNothing => &["hint_reaches_own_where"],
            Where::StepsRunNothing => &["own_nth_within", "hint_reaches_own_where"],
            Where::OfArrayItems => &["bounded_by_items"],
            Where::NoOwnNth => &["reaches_its_own"],
            Where::NoOwnNthWithin => &["own_nth_within"],
            Where::PlainInners => &["reaches_its_own", "hint_reaches_own_where", "drops"],
        }
    }
}

impl InPlaceForm {
    pub(crate) const ALL: [InPlaceForm; 33] = [
        InPlaceForm::StepsMap,
        InPlaceForm::MapEnds,
        InPlaceForm::Copied,
        InPlaceForm::StepsFilter,
        InPlaceForm::FilterEnds,
        InPlaceForm::FilterDropping,
        InPlaceForm::Somes,
        InPlaceForm::SomesOfEnds,
        InPlaceForm::SomesFilter,
        InPlaceForm::SomeItems,
        InPlaceForm::SomeItemsOfEnds,
        InPlaceForm::Enumerate,
        InPlaceForm::StepsEnumerate,
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
    /// of a consumer, std's method, `nth`; and of each, where it's chosen
    /// for it, as the net runs it (`Where`): a form may stand for one std
    /// type wherever it's made and for another only where its steps agree.
    #[cfg(test)]
    pub(crate) fn stands_for(self) -> &'static [(&'static str, Where)] {
        use Where::*;
        match self {
            InPlaceForm::StepsMap | InPlaceForm::MapEnds => &[("Map", Always), ("Cloned", Always), ("Inspect", Always)],
            InPlaceForm::Copied => &[("Copied", Always)],
            InPlaceForm::StepsFilter | InPlaceForm::FilterEnds | InPlaceForm::FilterDropping => &[("Filter", Always)],
            InPlaceForm::Somes | InPlaceForm::SomesOfEnds => &[("FilterMap", Always)],
            // A filter's `next` its source's `try_fold`, as `FilterMap`'s is;
            // a flatten's its source's `next`, which asks the same of it
            // only where no `nth` of the crate's is between (#2205).
            InPlaceForm::SomesFilter => &[
                ("FilterMap", HintRunsNothing),
                ("Flatten<Option>", StepsRunNothing),
                ("FlatMap<Option>", StepsRunNothing),
            ],
            InPlaceForm::SomeItems | InPlaceForm::SomeItemsOfEnds => {
                &[("Flatten<Option>", Always), ("FlatMap<Option>", Always)]
            }
            InPlaceForm::Enumerate => &[("Enumerate", Always)],
            // Std's `Enumerate::nth` asks its source's, which JS's map
            // can't: only where nothing between may ask it (#2238).
            InPlaceForm::StepsEnumerate => &[("Enumerate", NoOwnNthWithin)],
            InPlaceForm::Skipping => &[("Skip", Always)],
            InPlaceForm::Taking => &[("Take", Always)],
            InPlaceForm::StepBy | InPlaceForm::StepByEnds => &[("StepBy", Always)],
            InPlaceForm::StepByFilter => &[("StepBy", OfArrayItems)],
            InPlaceForm::LazyChain => &[("Chain", Always)],
            InPlaceForm::LazyZip | InPlaceForm::ZipEnds => &[("Zip", Always)],
            InPlaceForm::Rev => &[("Rev", Always)],
            InPlaceForm::FlatMap | InPlaceForm::FlattenEnds => &[("Flatten", Always), ("FlatMap", Always)],
            InPlaceForm::StepsFlatMap => &[("Flatten", PlainInners), ("FlatMap", PlainInners)],
            InPlaceForm::LazyTakeWhile => &[("TakeWhile", Always)],
            InPlaceForm::LazySkipWhile => &[("SkipWhile", Always)],
            InPlaceForm::LazyMapWhile => &[("MapWhile", Always)],
            InPlaceForm::LazyScan => &[("Scan", Always)],
            InPlaceForm::CycleFrom => &[("Cycle", Always)],
            InPlaceForm::Nth => &[("nth", Always)],
            InPlaceForm::NthDropTake => &[("nth", NoOwnNth)],
            InPlaceForm::FindMapFind => &[("find_map", Always)],
        }
    }

    /// Its name as the net runs it.
    #[cfg(test)]
    pub(crate) fn name(self) -> &'static str {
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
            InPlaceForm::SomeItems => "$someItems",
            InPlaceForm::SomeItemsOfEnds => "$someItems of ends",
            InPlaceForm::Enumerate => "$enumerate",
            InPlaceForm::StepsEnumerate => "$Steps.map enumerating",
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
}

/// The std type, as std_steps.txt names its row, of an adapter of type
/// `ty` a maker makes knowing its hint alone (`hint_knowing_of`): a
/// flatten's `Flatten` or `FlatMap` whatever its items, which
/// `hint_knowing_form_of` tells apart. `None` of another.
pub(crate) fn hint_knowing_std<'tcx>(tcx: ty::TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<&'static str> {
    use crate::recognition as r;
    let ty = ty.peel_refs();
    Some(match () {
        _ if r::mapped_source(tcx, ty).is_some() => "Map",
        _ if r::cloned_source(tcx, ty).is_some() => "Cloned",
        _ if r::inspected_source(tcx, ty).is_some() => "Inspect",
        _ if r::copied_source(tcx, ty).is_some() => "Copied",
        _ if r::filtered_source(tcx, ty).is_some() => "Filter",
        _ if r::filter_mapped_source(tcx, ty).is_some() => "FilterMap",
        _ if r::enumerated_source(tcx, ty).is_some() => "Enumerate",
        _ if r::chained_sources(tcx, ty).is_some() => "Chain",
        _ if r::reversed_source(tcx, ty).is_some() => "Rev",
        _ => match r::flattened_source(tcx, ty)? {
            (_, None) => "Flatten",
            (_, Some(_)) => "FlatMap",
        },
    })
    .filter(|&std| hint_knowing_of(std).is_some())
}

/// The form a std adapter, by the std type it is, `std`, is made as where
/// it's made knowing its hint alone, as `hint_asked.rs` says of its type
/// (`made_knowing_its_hint`): one that knows std's hint of what it holds
/// and asks no back of it, chosen wherever it's made (`Where::Always`), as
/// a fact of the crate keyed by type chooses it, which a net over the
/// sources a narrower `Where` allows couldn't hold (#2209). Never a form
/// that steps what it holds from its back, which a hint asks none of: a
/// flatten's is `$flatMap`, whose `$flatHint` asks each inner's hint, not
/// `$flattenEnds`, which steps each inner by its back. `None` of another:
/// a zip's, whose JS knows its hint as it's made, or a `take(n)`'s, a
/// `skip(n)`'s or a `step_by(n)`'s, made knowing it by `counted_in`'s own
/// makers (`$taking`, `$skipping`, `$stepBy`).
pub(crate) fn hint_knowing_of(std: &str) -> Option<InPlaceForm> {
    Some(match std {
        "Map" | "Cloned" | "Inspect" => InPlaceForm::MapEnds,
        "Copied" => InPlaceForm::Copied,
        "Filter" => InPlaceForm::FilterEnds,
        "FilterMap" => InPlaceForm::SomesOfEnds,
        "Flatten<Option>" | "FlatMap<Option>" => InPlaceForm::SomeItemsOfEnds,
        "Enumerate" => InPlaceForm::Enumerate,
        "Chain" => InPlaceForm::LazyChain,
        "Rev" => InPlaceForm::Rev,
        "Flatten" | "FlatMap" => InPlaceForm::FlatMap,
        _ => return None,
    })
}

/// The owner's table, `in_place_forms.txt`: each form's std types, its
/// name and where it's chosen, then each `Where`'s APIs, then the form of
/// each std type made knowing its hint alone (`hint_knowing_of`).
#[cfg(test)]
pub(crate) fn table() -> String {
    let mut out = String::from(
        "# The forms lowering makes std's adapters and consumers as in place, and the\n\
         # std type each stands for (crates/rust-js-lower/src/iterators/in_place_forms.rs; never by\n\
         # hand: `BLESS=1 cargo test in_place_forms` writes it). Each line,\n\
         # `<std type> | <form> | <where>`; each `where <name>: <APIs>`, what\n\
         # answers it, each an owner of class 1 or 2 (#2112, #2200); each\n\
         # `hint <std type> | <form> | <where>`, the form one made knowing its\n\
         # hint alone is made as, chosen by a fact keyed by type (#2209).\n",
    );
    for form in InPlaceForm::ALL {
        for (std, at) in form.stands_for() {
            out.push_str(&format!("{std} | {} | {at:?}\n", form.name()));
        }
    }
    for at in Where::ALL {
        let asks: String = at.asks().iter().map(|api| format!(" {api}")).collect();
        out.push_str(&format!("where {at:?}:{asks}\n"));
    }
    let mut seen = Vec::new();
    for std in (InPlaceForm::ALL.iter()).flat_map(|form| form.stands_for().iter().map(|&(std, _)| std)) {
        if let Some(form) = hint_knowing_of(std).filter(|_| !seen.contains(&std)) {
            seen.push(std);
            let at = (form.stands_for().iter())
                .find(|&&(of, _)| of == std)
                .map(|&(_, at)| at);
            let at = at.map_or_else(|| "not chosen".to_string(), |at| format!("{at:?}"));
            out.push_str(&format!("hint {std} | {} | {at}\n", form.name()));
        }
    }
    out
}

/// The form an adapter made in place, of `form`, is stepped as where only
/// its `next` is asked, a loop's: of an enumeration, JS's own map, which
/// stands for `Enumerate` wherever nothing asks its `nth` (#2238); of
/// another, itself.
pub fn stepped_by_next_alone(form: InPlaceForm) -> InPlaceForm {
    match form {
        InPlaceForm::Enumerate => InPlaceForm::StepsEnumerate,
        form => form,
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The form an `enumerate()` of a `source` made in place is made as:
    /// JS's own map, where no adapter between may ask the source's own
    /// `nth` in place of `next` (`own_nth_within`), else `$enumerate`, as
    /// std's `Enumerate::nth` asks its source's whatever it knows of its
    /// back or its length (#2238). Of the source's type alone, each value
    /// of it alike: no fact chooses it.
    pub fn enumeration_form(&self, source: Ty<'tcx>) -> InPlaceForm {
        let form = match self.recognition().own_nth_within(source) {
            true => InPlaceForm::Enumerate,
            false => InPlaceForm::StepsEnumerate,
        };
        debug_assert!(InPlaceForm::ALL.contains(&form));
        form
    }

    /// The form `comb`, one of the in-place adapters and consumers whose
    /// form is chosen between several, is made as, of a `receiver_ty`
    /// giving an `output`: the one decision of it. `None` of another,
    /// whose form is one alone, or which its arm decides by what it's
    /// given.
    pub(crate) fn in_place_form(&self, comb: IterComb, receiver_ty: Ty<'tcx>, output: Ty<'tcx>) -> Option<InPlaceForm> {
        let form = match comb {
            // The `Some`s of a `filter_map`: `$somes` where the source's
            // `size_hint` may run the crate's code, whose own `count` is
            // std's `FilterMap`'s, its fold, not `Filter::count`'s, which
            // asks that hint first (#2199). A flatten of `Option`s, or a
            // `flat_map` to them (`flattened_options`), std's
            // `FlattenCompat`, steps its source by `next` where a filter
            // steps it by `try_fold`: `$someItems` where the source's steps
            // may reach a crate's `nth` or `nth_back` between, which asks it
            // otherwise, or its hint the crate's code (#2205). Its type says
            // each, of every value of it alike, whatever form a fact made it
            // (`hint_reaches_own_where`, `own_nth_within`, of a type
            // parameter's and what a `dyn` holds too): no fact chooses it.
            IterComb::FilterMap | IterComb::FlatMap | IterComb::Flatten => {
                let recognition = self.recognition();
                let hint_runs = recognition.hint_reaches_own_where(receiver_ty, true);
                let mapped = recognition.flattened_options(output).map(|(_, _, mapped)| mapped);
                let items = match hint_runs || recognition.own_nth_within(receiver_ty) {
                    true => InPlaceForm::SomeItems,
                    false => InPlaceForm::SomesFilter,
                };
                match comb {
                    IterComb::FilterMap => Some(match hint_runs {
                        true => InPlaceForm::Somes,
                        false => InPlaceForm::SomesFilter,
                    }),
                    IterComb::FlatMap => (mapped == Some(true)).then_some(items),
                    _ => (mapped == Some(false)).then_some(items),
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

    /// The form a std adapter of type `ty` is made as where it's made
    /// knowing its hint alone (`hint_knowing_of`), a flatten's of `Option`s
    /// told apart from another (`flattened_options`). `None` of another.
    pub fn hint_knowing_form_of(&self, ty: Ty<'tcx>) -> Option<InPlaceForm> {
        let std = match hint_knowing_std(self.tcx, ty)? {
            "Flatten" if self.recognition().flattened_options(ty.peel_refs()).is_some() => "Flatten<Option>",
            "FlatMap" if self.recognition().flattened_options(ty.peel_refs()).is_some() => "FlatMap<Option>",
            std => std,
        };
        let form = hint_knowing_of(std);
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
    /// iterators whose `count` or `last` is the crate's (#736), of items
    /// with a destructor (ADR 0510), or of inner iterators whose hint may
    /// run the crate's code, which std's `FlattenCompat::size_hint` asks
    /// and `$flatMap` asks as it does, where JS's own `flatMap` of a JS
    /// iterator has none (`hint_reaches_own_where`, of what a `dyn` holds
    /// too, #2207); else JS's own `flatMap`.
    pub(crate) fn flattened_form(&self, inner: Ty<'tcx>, own: bool) -> InPlaceForm {
        let gives = self.iterable_item(inner);
        let made = crate::recognition::iterator_type_of(self.tcx, self.typing_env, inner);
        let hint_runs = self.recognition().hint_reaches_own_where(made, true);
        let form = match own || hint_runs || self.drops(gives) != crate::drops::Drops::Nothing {
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
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("crates/rust-js-lower/src/iterators/in_place_forms.txt");
            std::fs::write(path, &made).expect("the table written");
        } else {
            assert_eq!(
                include_str!("in_place_forms.txt"),
                made,
                "crates/rust-js-lower/src/iterators/in_place_forms.txt: BLESS=1 cargo test in_place_forms"
            );
        }
    }

    /// Each form a std type made knowing its hint alone is made as stands
    /// for it wherever it's chosen, `Always`, as a fact of the crate keyed
    /// by type chooses it (#2209); and none steps what it holds from its
    /// back, a flatten's `$flattenEnds`.
    #[test]
    fn hint_knowing_forms_stand_for_every_source() {
        for form in InPlaceForm::ALL {
            for &(std, _) in form.stands_for() {
                let Some(hint) = hint_knowing_of(std) else { continue };
                assert_ne!(hint, InPlaceForm::FlattenEnds, "{std}");
                assert!(
                    hint.stands_for().contains(&(std, Where::Always)),
                    "{std}: {} isn't std's wherever it's made",
                    hint.name()
                );
            }
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
            assert!(
                at == Where::Always
                    || (InPlaceForm::ALL.iter()).any(|form| form.stands_for().iter().any(|&(_, chosen)| chosen == at))
            );
        }
    }
}
