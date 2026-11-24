//! std's steps: the one catalog of what each of std's iterator adapters
//! and wrappers writes of `Iterator`, `DoubleEndedIterator` and
//! `ExactSizeIterator`, and what each of those steps asks of what it holds
//! (#2000, #2112). It's std_steps.txt, which test/std-steps.test.ts makes
//! from std alone: what each impl writes, read from rust-src, and what each
//! step asks, run natively over sources that print each step they're asked
//! (test/std-steps.rs). The lowering asks it here, `Wrapper::passes` and
//! `asks`, of the runs over sources that write every step: a run over
//! another kind, `Take#bare0`, shows the traits' defaults over the steps
//! its sources write, not what std's step asks, and is the runtime net's
//! alone (test/iterator-steps.test.ts).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use super::*;

/// The catalog, as test/std-steps.test.ts writes it.
const CATALOG: &str = include_str!("std_steps.txt");

/// One step a source was asked: which, `src`, `other` or `inner`, and the
/// step, std's name, without its argument.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Asked {
    pub(crate) of: &'static str,
    pub(crate) step: &'static str,
}

/// Of each adapter or wrapper, by its catalog name, `Skip` or `&mut I`:
/// the steps std's impls of it write, and of each step, what it asked in
/// each of the probe's runs.
#[derive(Default)]
struct Catalogued {
    writes: Vec<&'static str>,
    runs: BTreeMap<&'static str, Vec<Vec<Asked>>>,
    /// The steps of its `DoubleEndedIterator`, the probe's line of them,
    /// the one its `next_back` begins.
    back: Vec<&'static str>,
}

fn catalog() -> &'static BTreeMap<&'static str, Catalogued> {
    static PARSED: OnceLock<BTreeMap<&'static str, Catalogued>> = OnceLock::new();
    PARSED.get_or_init(|| {
        let mut all: BTreeMap<&'static str, Catalogued> = BTreeMap::new();
        for line in CATALOG.lines().filter(|line| !line.starts_with('#')) {
            if let Some((name, written)) = line.split_once(" writes:") {
                all.entry(name).or_default().writes = written.split_whitespace().collect();
                continue;
            }
            let (head, cells) = line
                .split_once(" |")
                .expect("a catalog line is `<name>#<run> |<step>: ..`");
            let (name, run) = head.rsplit_once('#').expect("a run's line is `<name>#<kind><run>`");
            if !run.bytes().all(|b| b.is_ascii_digit()) {
                continue;
            }
            let steps = all.entry(name).or_default();
            let back = cells.starts_with("next_back:");
            for cell in cells.split(" |") {
                let (step, asked) = cell.split_once(':').expect("a cell is `<step>: <asked>`");
                // What it answered, `size_hint`'s and `len`'s, after ` = `:
                // the runtime net's alone (#2205).
                let asked = asked.split_once(" = ").map_or(asked, |(asked, _)| asked);
                if back && !steps.back.contains(&step) {
                    steps.back.push(step);
                }
                let asked = (asked.split_whitespace())
                    .map(|call| {
                        let (of, step) = call.split_once('.').expect("a call is `<source>.<step>`");
                        Asked {
                            of,
                            step: step.split_once('(').map_or(step, |(step, _)| step),
                        }
                    })
                    .collect();
                steps.runs.entry(step).or_default().push(asked);
            }
        }
        all
    })
}

/// Whether std's impls of `name`, a catalog name, write `step` themselves,
/// not the trait's default; false of a name the catalog hasn't.
pub(crate) fn writes(name: &str, step: &str) -> bool {
    catalog().get(name).is_some_and(|steps| steps.writes.contains(&step))
}

/// Whether std's `step` of `name`, a catalog name, is observably its own,
/// not what its `fold` asks: in each of the probe's runs, what the step
/// asked of what it holds against what `fold` did. `Never` where every run
/// asks the same, so a form's own of it, which a form may mark where the
/// two are alike (the runtime net's rule), is never what tells them apart;
/// `Always` where every run differs, `Filter::count`'s hint first; and
/// `Depends` where it differs only as what it holds writes it, a pass-on's,
/// `Copied::count`'s, which `reaches_its_own` answers of the source. `None`
/// of a name or a step the catalog hasn't run. The one answer of whether a
/// consumer of std's adapter may take its own step for its default (#2112,
/// Change A step 3a), derived from the committed runs, which
/// test/iterator-steps.test.ts reads the same (`asked !== folded`).
pub(crate) fn observably_owns(name: &str, step: &str) -> Option<Observed> {
    let steps = catalog().get(name)?;
    let (asked, folded) = (steps.runs.get(step)?, steps.runs.get("fold")?);
    let differing = asked
        .iter()
        .zip(folded)
        .filter(|(asked, folded)| asked != folded)
        .count();
    Some(match differing {
        0 => Observed::Never,
        n if n == asked.len() => Observed::Always,
        _ => Observed::Depends,
    })
}

/// `observably_owns`' answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Observed {
    Always,
    Never,
    Depends,
}

/// The steps of `DoubleEndedIterator` past its `next_back`, the entry of
/// its dictionary (`extension_entry`), that std's impls of an adapter or a
/// wrapper write: what a dictionary of it carries beside its entry, and
/// what a compiled std struct made knowing its back is given, in that
/// order, `WRAPPER_STEPS`'. `rfold`, `nth_back`, `try_rfold` and
/// `advance_back_by`.
pub(crate) fn back_steps() -> Vec<&'static str> {
    (WRAPPER_STEPS.into_iter())
        .filter(|&step| step != "next_back")
        .filter(|step| (catalog().values()).any(|steps| steps.back.contains(step) && steps.writes.contains(step)))
        .collect()
}

/// The steps std's impls of `name`, a catalog name, write themselves, in
/// the catalog's order; none of a name it hasn't.
#[cfg(test)]
pub(crate) fn written(name: &str) -> &'static [&'static str] {
    catalog().get(name).map_or(&[], |steps| steps.writes.as_slice())
}

/// What std's `step` of `name` asks of what it holds, in any of the
/// probe's runs, each once; none where std's impls don't write `step`, the
/// trait's default, which asks the adapter's own steps, not its sources'.
pub(crate) fn asks(name: &str, step: &str) -> Vec<Asked> {
    let mut asked: Vec<Asked> = Vec::new();
    if !writes(name, step) {
        return asked;
    }
    for run in catalog()
        .get(name)
        .and_then(|steps| steps.runs.get(step))
        .into_iter()
        .flatten()
    {
        for &one in run {
            if !asked.contains(&one) {
                asked.push(one);
            }
        }
    }
    asked
}

/// Each step the catalog runs, std's name: what a value's later uses may
/// ask of it, one bit each of `Steps` (#1458).
const STEP_NAMES: [&str; 25] = [
    "next",
    "size_hint",
    "size_hint_after_next",
    "nth",
    "advance_by",
    "fold",
    "try_fold",
    "last",
    "count",
    "position",
    "find",
    "find_map",
    "any",
    "all",
    "max_by",
    "min_by",
    "next_back",
    "nth_back",
    "advance_back_by",
    "rfold",
    "try_rfold",
    "rfind",
    "rposition",
    "len",
    "is_empty",
];

/// The steps of std's `Iterator`, `DoubleEndedIterator` and
/// `ExactSizeIterator` a value's later uses ask of it, by the catalog's
/// names (`STEP_NAMES`): what's asked of a value, never what its type may
/// be asked (#1458, #2474's item 6). `ALL` of one whose uses aren't all
/// seen, or that asks one the catalog doesn't run, which every step may
/// then be.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub struct Steps(u32);

impl Steps {
    /// Every step, the answer where what's asked isn't known.
    pub const ALL: Steps = Steps(u32::MAX);
    /// None.
    pub const NONE: Steps = Steps(0);

    /// The step std's method `name` is, or `ALL` of a method the catalog
    /// doesn't run, which may ask any.
    pub fn of(name: &str) -> Steps {
        match STEP_NAMES.iter().position(|&step| step == name) {
            Some(at) => Steps(1 << at),
            None => Steps::ALL,
        }
    }

    /// Whether `name`, a step, is among them: of `ALL`, every step.
    pub fn asks(self, name: &str) -> bool {
        self == Steps::ALL
            || STEP_NAMES
                .iter()
                .position(|&step| step == name)
                .is_some_and(|at| self.0 & (1 << at) != 0)
    }

    /// Both's steps.
    // helper: a set's union, which decides nothing.
    #[must_use]
    pub fn and(self, other: Steps) -> Steps {
        Steps(self.0 | other.0)
    }

    /// Each step's name, of `ALL` each the catalog runs.
    pub fn names(self) -> impl Iterator<Item = &'static str> {
        STEP_NAMES.into_iter().filter(move |&step| self.asks(step))
    }
}

/// What `steps` of std's adapter `name`, a catalog name, ask of what it
/// holds at `of`, `src` or `other`: each step any run of each asked of it,
/// whether std's impls write the step or the trait's default asks it, as
/// the catalog ran them. `ALL` of `ALL`, as what's unseen of the adapter
/// may be a method the catalog doesn't run, a `Peekable`'s `peek`; of a
/// name the catalog hasn't; and of a step it doesn't run of it, which may
/// ask any.
pub(crate) fn asked_of_held(name: &str, steps: Steps, of: &str) -> Steps {
    let Some(entry) = catalog().get(name).filter(|_| steps != Steps::ALL) else {
        return Steps::ALL;
    };
    let mut asked = Steps::NONE;
    for step in steps.names() {
        let Some(runs) = entry.runs.get(step) else {
            return Steps::ALL;
        };
        for call in runs.iter().flatten().filter(|call| call.of == of) {
            asked = asked.and(Steps::of(call.step));
        }
    }
    asked
}

/// A wrapper std's impls step the iterator it holds through: a `&mut I` of
/// a sized `I` or of an unsized one, a `dyn`'s, and a `Box<I>` of each.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wrapper {
    MutRefSized,
    MutRefUnsized,
    BoxSized,
    BoxUnsized,
}

/// How a wrapper's method reaches its iterator's: as the iterator's own
/// method; by its `try_fold` or `try_rfold`, never stopped, a sized `&mut
/// I`'s `fold` and `rfold` (`IteratorRefSpec`); or not at all, the trait's
/// default over `next` and `next_back`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Passed {
    Own,
    ByTry,
    Default,
}

/// The steps a wrapper may pass on, std's names, in the order a
/// dictionary carries them.
const WRAPPER_STEPS: [&str; 15] = [
    "next",
    "size_hint",
    "advance_by",
    "nth",
    "fold",
    "try_fold",
    "last",
    "count",
    "next_back",
    "rfold",
    "nth_back",
    "try_rfold",
    "advance_back_by",
    "len",
    "is_empty",
];

impl Wrapper {
    /// The wrapper `ty` is of the iterator it holds, a `&mut` or a `Box`, or
    /// none.
    pub fn of<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> Option<Wrapper> {
        Self::holding(tcx, typing_env, ty).map(|(wrapper, _)| wrapper)
    }

    /// `of`'s, and the iterator it holds.
    pub(crate) fn holding<'tcx>(
        tcx: TyCtxt<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
        ty: Ty<'tcx>,
    ) -> Option<(Wrapper, Ty<'tcx>)> {
        let (held, boxed) = match *ty.kind() {
            ty::Ref(_, to, ty::Mutability::Mut) => (to, false),
            _ => (ty.boxed_ty()?, true),
        };
        let wrapper = match (boxed, held.is_sized(tcx, typing_env)) {
            (false, true) => Wrapper::MutRefSized,
            (false, false) => Wrapper::MutRefUnsized,
            (true, true) => Wrapper::BoxSized,
            (true, false) => Wrapper::BoxUnsized,
        };
        Some((wrapper, held))
    }

    /// How a `&mut` of type `by` lends the iterator it holds: of a sized
    /// one or an unsized one, a `dyn`'s. Of a `&mut` of a `&mut`,
    /// `(&mut d).find(..)` of a `d: &mut dyn Iterator`, the innermost's, as
    /// a sized `&mut I` passes each of its steps on to its `I`'s (#2180).
    pub(crate) fn lending<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, by: Ty<'tcx>) -> Wrapper {
        let mut held = by;
        while let ty::Ref(_, to, _) = *held.kind() {
            held = to;
        }
        match held.is_sized(tcx, typing_env) {
            true => Wrapper::MutRefSized,
            false => Wrapper::MutRefUnsized,
        }
    }

    /// What std steps by `step`, std's name, of a `ty`: through each
    /// wrapper, from the outside in, that passes `step` on to what it holds
    /// as that's own (`passes`), what it holds; `ty` itself where none
    /// does. A `len()` of a `&mut Box<I>` is `I`'s, as std's `&mut I` and
    /// `Box<I>` each pass `len` on (#2189).
    pub fn passed_through<'tcx>(
        tcx: TyCtxt<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
        ty: Ty<'tcx>,
        step: &str,
    ) -> Ty<'tcx> {
        let mut held = ty;
        while let Some((wrapper, inner)) = Self::holding(tcx, typing_env, held)
            && wrapper.passes(step) == Passed::Own
        {
            held = inner;
        }
        held
    }

    /// Its catalog name.
    fn name(self) -> &'static str {
        match self {
            Wrapper::MutRefSized => "&mut I",
            Wrapper::MutRefUnsized => "&mut dyn I",
            Wrapper::BoxSized => "Box<I>",
            Wrapper::BoxUnsized => "Box<dyn I>",
        }
    }

    /// How its `method`, std's name, reaches its iterator's: its own where
    /// std's impl writes it and asks nothing else of it in every run, the
    /// same step; by its `try_` fold where it asks that alone; else the
    /// trait's default.
    pub(crate) fn passes(self, method: &str) -> Passed {
        let name = self.name();
        if !writes(name, method) {
            return Passed::Default;
        }
        let runs = catalog().get(name).and_then(|steps| steps.runs.get(method));
        let each = |step: &str| {
            runs.is_some_and(|runs| {
                (runs.iter()).all(|run| matches!(run.as_slice(), [Asked { of: "src", step: asked }] if *asked == step))
            })
        };
        if each(method) {
            Passed::Own
        } else if each(&format!("try_{method}")) {
            Passed::ByTry
        } else {
            Passed::Default
        }
    }

    /// `$lent`'s kind of it (crates/rust-js-program/src/runtime/lent.js's `$passedOn`): what a JS
    /// iterator stepped through it is lent as.
    pub(crate) fn lend_kind(self) -> &'static str {
        match self {
            Wrapper::MutRefSized => "lent",
            Wrapper::MutRefUnsized => "lentUnsized",
            Wrapper::BoxSized => "boxed",
            Wrapper::BoxUnsized => "boxedUnsized",
        }
    }

    /// Whether it's a `Box`'s, which owns what it holds; a `&mut`'s lends it.
    pub(crate) fn owns(self) -> bool {
        matches!(self, Wrapper::BoxSized | Wrapper::BoxUnsized)
    }

    /// The methods it passes on to its iterator's own, std's names.
    pub(crate) fn own_methods(self) -> impl Iterator<Item = &'static str> {
        WRAPPER_STEPS
            .into_iter()
            .filter(move |method| self.passes(method) == Passed::Own)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `$forward`'s name of each of std's steps a wrapper passes on.
    fn js(step: &str) -> Option<&'static str> {
        Some(match step {
            "next" => "next",
            "size_hint" => "sizeHint",
            "advance_by" => "advanceBy",
            "nth" => "nth",
            "fold" => "fold",
            "try_fold" => "tryFold",
            "last" => "last",
            "next_back" => "back",
            "advance_back_by" => "advanceBackBy",
            "nth_back" => "nthBack",
            "rfold" => "rfold",
            "try_rfold" => "tryRfold",
            "len" => "len",
            // `count` no wrapper passes on; `is_empty`, `len`'s.
            _ => return None,
        })
    }

    /// crates/rust-js-program/src/runtime/lent.js's `$passedOn` lists what the catalog says each
    /// wrapper passes on, own or by its `try_` fold, by `$forward`'s names.
    #[test]
    fn lent_passes_on_what_std_does() {
        let lent = include_str!("../../../../crates/rust-js-program/src/runtime/lent.js");
        for wrapper in [
            Wrapper::MutRefSized,
            Wrapper::MutRefUnsized,
            Wrapper::BoxSized,
            Wrapper::BoxUnsized,
        ] {
            let kind = wrapper.lend_kind();
            let line = (lent.lines())
                .find(|line| line.starts_with(&format!("  {kind}: [")))
                .unwrap_or_else(|| panic!("lent.js's $passedOn has no {kind}"));
            let mut listed: Vec<&str> = line.split('"').skip(1).step_by(2).collect();
            listed.sort_unstable();
            let mut passed: Vec<&str> = (WRAPPER_STEPS.iter())
                .filter(|step| wrapper.passes(step) != Passed::Default)
                .filter_map(|step| js(step))
                .collect();
            passed.sort_unstable();
            assert_eq!((kind, listed), (kind, passed));
        }
    }

    /// What a dictionary of `DoubleEndedIterator` carries beside its entry
    /// is the catalog's, and crates/rust-js-program/src/runtime/std_iterator.js's `knowing` takes
    /// each, compiled, in that order, after its back and its length: the
    /// mirror of a JS signature, run on both sides (#2125).
    #[test]
    fn back_steps_are_the_catalogs_and_knowings() {
        assert_eq!(back_steps(), ["rfold", "nth_back", "try_rfold", "advance_back_by"]);
        let runtime = include_str!("../../../../crates/rust-js-program/src/runtime/std_iterator.js");
        let params = (runtime.lines())
            .find_map(|line| line.trim().strip_prefix("it.knowing = ("))
            .and_then(|rest| rest.split_once(')'))
            .map(|(params, _)| params.split(", ").collect::<Vec<_>>())
            .expect("std_iterator.js's `it.knowing = (..) =>`");
        let mut steps = vec!["back", "len"];
        steps.extend(
            back_steps()
                .into_iter()
                .map(|step| js(step).expect("a back step's `$forward` name")),
        );
        assert_eq!(params, steps);
    }

    /// What std's `&mut I` and `Box<I>` pass on, as the catalog says: of a
    /// sized `&mut I`, its folds by `try_fold`s; of an unsized one, neither;
    /// of a `Box<I>`, its `last` only of a sized one (#1601, #1986, #1995).
    #[test]
    fn wrappers_pass_on_as_std_does() {
        use Passed::{ByTry, Default, Own};
        let of = |w: Wrapper| WRAPPER_STEPS.map(|step| w.passes(step));
        assert_eq!(
            of(Wrapper::MutRefSized),
            [
                Own, Own, Own, Own, ByTry, Own, Default, Default, Own, ByTry, Own, Own, Own, Own, Own
            ]
        );
        assert_eq!(
            of(Wrapper::MutRefUnsized),
            [
                Own, Own, Own, Own, Default, Default, Default, Default, Own, Default, Own, Default, Own, Own, Own
            ]
        );
        assert_eq!(
            of(Wrapper::BoxSized),
            [
                Own, Own, Default, Own, Default, Default, Own, Default, Own, Default, Own, Default, Default, Own, Own
            ]
        );
        assert_eq!(
            of(Wrapper::BoxUnsized),
            [
                Own, Own, Default, Own, Default, Default, Default, Default, Own, Default, Own, Default, Default, Own,
                Own
            ]
        );
    }
}

#[cfg(test)]
mod observed_tests {
    use super::{Observed, observably_owns};

    /// Each std type's `count` and `last` against its `fold`, as std's own
    /// runs ask them: pinned, as the consumers' shortcut reads them (#2112,
    /// Change A step 3a).
    #[test]
    fn observably_owns_reads_the_runs() {
        use Observed::*;
        for (name, step, wanted) in [
            ("Filter", "count", Always),
            ("Filter", "last", Never),
            ("FilterMap", "count", Never),
            ("Flatten<Option>", "count", Never),
            ("Flatten<Option>", "last", Never),
            ("Map", "count", Never),
            ("Map", "last", Never),
            ("Copied", "count", Always),
            ("Skip", "count", Depends),
            ("Flatten", "last", Depends),
            ("Take", "last", Never),
            ("Chain", "last", Always),
        ] {
            assert_eq!(observably_owns(name, step), Some(wanted), "{name}'s {step}");
        }
        assert_eq!(observably_owns("Nonesuch", "count"), None);
    }
}
