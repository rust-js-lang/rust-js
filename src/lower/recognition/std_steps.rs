//! std's steps: the one catalog of what each of std's iterator adapters
//! and wrappers writes of `Iterator`, `DoubleEndedIterator` and
//! `ExactSizeIterator`, and what each of those steps asks of what it holds
//! (#2000, #2112). It's std_steps.txt, which test/std-steps.test.ts makes
//! from std alone: what each impl writes, read from rust-src, and what each
//! step asks, run natively over sources that print each step they're asked
//! (test/std-steps.rs). The lowering asks it here, `Wrapper::passes`.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use super::*;

/// The catalog, as test/std-steps.test.ts writes it.
const CATALOG: &str = include_str!("std_steps.txt");

/// One step a source was asked: which, `src`, `other` or `inner`, and the
/// step, std's name, without its argument.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) struct Asked {
    pub(in crate::lower) of: &'static str,
    pub(in crate::lower) step: &'static str,
}

/// Of each adapter or wrapper, by its catalog name, `Skip` or `&mut I`:
/// the steps std's impls of it write, and of each step, what it asked in
/// each of the probe's runs.
#[derive(Default)]
struct Steps {
    writes: Vec<&'static str>,
    runs: BTreeMap<&'static str, Vec<Vec<Asked>>>,
    /// The steps of its `DoubleEndedIterator`, the probe's line of them,
    /// the one its `next_back` begins.
    back: Vec<&'static str>,
}

fn catalog() -> &'static BTreeMap<&'static str, Steps> {
    static PARSED: OnceLock<BTreeMap<&'static str, Steps>> = OnceLock::new();
    PARSED.get_or_init(|| {
        let mut all: BTreeMap<&'static str, Steps> = BTreeMap::new();
        for line in CATALOG.lines().filter(|line| !line.starts_with('#')) {
            if let Some((name, written)) = line.split_once(" writes:") {
                all.entry(name).or_default().writes = written.split_whitespace().collect();
                continue;
            }
            let (head, cells) = line
                .split_once(" |")
                .expect("a catalog line is `<name>#<run> |<step>: ..`");
            let (name, _run) = head.rsplit_once('#').expect("a run's line is `<name>#<run>`");
            let steps = all.entry(name).or_default();
            let back = cells.starts_with("next_back:");
            for cell in cells.split(" |") {
                let (step, asked) = cell.split_once(':').expect("a cell is `<step>: <asked>`");
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
pub(in crate::lower) fn writes(name: &str, step: &str) -> bool {
    catalog().get(name).is_some_and(|steps| steps.writes.contains(&step))
}

/// The steps of `DoubleEndedIterator` past its `next_back`, the entry of
/// its dictionary (`extension_entry`), that std's impls of an adapter or a
/// wrapper write: what a dictionary of it carries beside its entry, and
/// what a compiled std struct made knowing its back is given, in that
/// order, `WRAPPER_STEPS`'. `rfold`, `nth_back`, `try_rfold` and
/// `advance_back_by`.
pub(in crate::lower) fn back_steps() -> Vec<&'static str> {
    (WRAPPER_STEPS.into_iter())
        .filter(|&step| step != "next_back")
        .filter(|step| (catalog().values()).any(|steps| steps.back.contains(step) && steps.writes.contains(step)))
        .collect()
}

/// The steps std's impls of `name`, a catalog name, write themselves, in
/// the catalog's order; none of a name it hasn't.
#[cfg(test)]
pub(in crate::lower) fn written(name: &str) -> &'static [&'static str] {
    catalog().get(name).map_or(&[], |steps| steps.writes.as_slice())
}

/// A wrapper std's impls step the iterator it holds through: a `&mut I` of
/// a sized `I` or of an unsized one, a `dyn`'s, and a `Box<I>` of each.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Wrapper {
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
pub(in crate::lower) enum Passed {
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
    pub(in crate::lower) fn of<'tcx>(
        tcx: TyCtxt<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
        ty: Ty<'tcx>,
    ) -> Option<Wrapper> {
        Self::holding(tcx, typing_env, ty).map(|(wrapper, _)| wrapper)
    }

    /// `of`'s, and the iterator it holds.
    pub(in crate::lower) fn holding<'tcx>(
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
    pub(in crate::lower) fn passes(self, method: &str) -> Passed {
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

    /// Whether it's a `Box`'s, which owns what it holds; a `&mut`'s lends it.
    pub(in crate::lower) fn owns(self) -> bool {
        matches!(self, Wrapper::BoxSized | Wrapper::BoxUnsized)
    }

    /// The methods it passes on to its iterator's own, std's names.
    pub(in crate::lower) fn own_methods(self) -> impl Iterator<Item = &'static str> {
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

    /// src/runtime/lent.js's `$passedOn` lists what the catalog says each
    /// wrapper passes on, own or by its `try_` fold, by `$forward`'s names.
    #[test]
    fn lent_passes_on_what_std_does() {
        let lent = include_str!("../../runtime/lent.js");
        for (wrapper, kind) in [
            (Wrapper::MutRefSized, "lent"),
            (Wrapper::MutRefUnsized, "lentUnsized"),
            (Wrapper::BoxSized, "boxed"),
            (Wrapper::BoxUnsized, "boxedUnsized"),
        ] {
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
    /// is the catalog's, and src/runtime/std_iterator.js's `knowing` takes
    /// each, compiled, in that order, after its back and its length: the
    /// mirror of a JS signature, run on both sides (#2125).
    #[test]
    fn back_steps_are_the_catalogs_and_knowings() {
        assert_eq!(back_steps(), ["rfold", "nth_back", "try_rfold", "advance_back_by"]);
        let runtime = include_str!("../../runtime/std_iterator.js");
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
