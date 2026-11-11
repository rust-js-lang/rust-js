//! Decision coverage (docs/finding-wrong-answers.md): which answer each of
//! the compiler's decisions gives, counted where its owner gives it, so the
//! nets' programs can be asked which answers they never reach. A wrong
//! answer hides where no program reaches a decision's answer: #1349's
//! overlapping swap was one no generated program made.
//!
//! Off unless `RUST_JS_DECISIONS` names a file: then each compile appends
//! to it, as it ends (`counts`, which the driver writes), a line per decision and answer of `CATALOGUE`, with how many
//! times it was given, `0` of one never given, which
//! `scripts/decision-coverage.ts` merges across the nets. Off, a record is
//! one test of a flag read once.

use std::collections::BTreeMap;
use std::sync::{LazyLock, Mutex};

/// A decision and every answer its owner gives.
pub(crate) struct Decision {
    pub(crate) name: &'static str,
    /// Where it's decided, `file function`.
    pub(crate) owner: &'static str,
    pub(crate) answers: &'static [&'static str],
}

/// Each decision counted, by its owner, and every answer it can give: one
/// catalogue, which a record of another answer or decision is refused by.
pub(crate) const CATALOGUE: &[Decision] = &[
    Decision {
        name: "refcell-borrow",
        owner: "src/lower/std_types/cell.rs Std::Borrow",
        answers: &["unchecked", "momentary", "counted"],
    },
    Decision {
        name: "refcell-try-borrow",
        owner: "src/lower/std_types/cell.rs Std::TryBorrow",
        answers: &["flattened", "unchecked", "checked"],
    },
    Decision {
        name: "generic-form",
        owner: "src/lower/generic_facts.rs form_of",
        answers: &["as-it-is", "handle", "box", "held-box"],
    },
    Decision {
        name: "writer-options",
        owner: "src/lower/display.rs compiled_writer_takes_options",
        answers: &["crate", "shown", "none"],
    },
    Decision {
        name: "library-writer-options",
        owner: "src/lower/display.rs takes_options",
        answers: &["takes", "plain"],
    },
    Decision {
        name: "evidence-route",
        owner: "src/lower/traits/routes.rs evidence_for",
        answers: &["given", "given-supertrait", "item", "none"],
    },
    Decision {
        name: "type-fact-given",
        owner: "src/lower/traits/given.rs given_type_fact",
        answers: &["given", "none"],
    },
    Decision {
        name: "option-payload",
        owner: "src/lower/representation/options.rs boxed_payload",
        answers: &["boxed", "unboxed"],
    },
    Decision {
        name: "iterator-given",
        owner: "src/lower/iterators.rs given_as_it_is",
        answers: &["as-it-is", "js-iterator"],
    },
    Decision {
        name: "rc-counted",
        owner: "src/lower/analysis/rc_counts.rs Counted::counts",
        answers: &["counted", "uncounted"],
    },
    Decision {
        name: "collect-in-place",
        owner: "src/lower/recognition/specialization.rs collects_in_place",
        answers: &["in-place", "apart", "callers"],
    },
    Decision {
        name: "vec-collect-way",
        owner: "src/lower/recognition/specialization.rs vec_collect_way",
        answers: &["inPlace", "fold", "next"],
    },
    Decision {
        name: "copy-if-needed",
        owner: "src/lower/copies.rs copy_if_needed",
        answers: &["fresh", "copied", "shared"],
    },
    Decision {
        name: "type-drops",
        owner: "src/lower/drops/types.rs DropQuery::drops",
        answers: &["nothing", "runs", "unsupported"],
    },
    Decision {
        name: "mut-ref-form",
        owner: "src/lower/representation/mut_ref.rs mut_ref_form_in",
        answers: &["box", "generic-box", "object", "guard", "given-as-it-is", "itself"],
    },
    Decision {
        name: "refcell-field",
        owner: "src/lower/drops/ref_cells.rs holds_value",
        answers: &["holds-value", "cell"],
    },
    Decision {
        name: "stored-place",
        owner: "src/lower/analysis/stored_places.rs records",
        answers: &["recorded", "unrecorded"],
    },
];

/// The file the counts go to, where asked.
static PATH: LazyLock<Option<std::ffi::OsString>> = LazyLock::new(|| std::env::var_os("RUST_JS_DECISIONS"));

/// How many times each decision gave each answer, this compile.
static GIVEN: Mutex<BTreeMap<(&'static str, &'static str), u64>> = Mutex::new(BTreeMap::new());

/// `decision` gave `answer`, counted where decisions are asked.
#[inline]
pub(crate) fn record(decision: &'static str, answer: &'static str) {
    if PATH.is_some() {
        count(decision, answer);
    }
}

#[cold]
fn count(decision: &'static str, answer: &'static str) {
    let known = CATALOGUE
        .iter()
        .any(|d| d.name == decision && d.answers.contains(&answer));
    assert!(
        known,
        "rust-js: decision `{decision}`'s answer `{answer}` isn't in src/decisions.rs' catalogue"
    );
    let mut given = GIVEN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *given.entry((decision, answer)).or_default() += 1;
}

/// This compile's counts, where asked, and the file they go to: a
/// `decision\tanswer\tcount\tbuild\tshape\towner` line of every decision's
/// every answer, its build `std` of `--std-iterators` or `plain`, and its
/// shape `library` of `--library` or `program`, by its arguments. The
/// driver appends them (`main.rs`), as only it writes files.
pub(crate) fn counts() -> Option<(&'static std::ffi::OsStr, String)> {
    let path = PATH.as_deref()?;
    let given = GIVEN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let args: Vec<String> = std::env::args().collect();
    let build = if args.iter().any(|a| a == "--std-iterators") {
        "std"
    } else {
        "plain"
    };
    let shape = if args.iter().any(|a| a == "--library") {
        "library"
    } else {
        "program"
    };
    let mut lines = String::new();
    for decision in CATALOGUE {
        for &answer in decision.answers {
            let n = given.get(&(decision.name, answer)).copied().unwrap_or(0);
            lines.push_str(&format!(
                "{}\t{answer}\t{n}\t{build}\t{shape}\t{}\n",
                decision.name, decision.owner
            ));
        }
    }
    Some((path, lines))
}
