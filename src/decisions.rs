//! Decision coverage (docs/finding-wrong-answers.md): which answer each of
//! the compiler's decisions gives, counted where its owner gives it, so the
//! nets' programs can be asked which answers they never reach. A wrong
//! answer hides where no program reaches a decision's answer: #1349's
//! overlapping swap was one no generated program made.
//!
//! Off unless `RUST_JS_DECISIONS` names a file: then each compile appends
//! to it, as it ends (`counts`, which the driver writes), a line per
//! answer of `CATALOGUE` it gave, by the type's kind of a typed decision
//! and by the body it was given of, the crate's own, another crate's or
//! the crate's analysis, with how many times; the first compile into a
//! file writes every answer, `0` of one never given, which
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
    /// Whether its owner says the type it answers of, each answer then
    /// counted by the type's kind (`KINDS`).
    pub(crate) typed: bool,
}

/// Each decision counted, by its owner, and every answer it can give: one
/// catalogue, which a record of another answer or decision is refused by.
pub(crate) const CATALOGUE: &[Decision] = &[
    Decision {
        name: "transparent-chain",
        owner: "src/lower/iterators/transparent.rs count_chains",
        answers: &[
            "user-iterator",
            "held",
            "source",
            "escapes",
            "drop-glue",
            "unproved",
            "order",
            "capacity",
            "admitted",
        ],
        typed: false,
    },
    Decision {
        name: "refcell-borrow",
        owner: "src/lower/std_types/cell.rs Std::Borrow",
        answers: &["unchecked", "momentary", "counted"],
        typed: false,
    },
    Decision {
        name: "refcell-try-borrow",
        owner: "src/lower/std_types/cell.rs Std::TryBorrow",
        answers: &["flattened", "unchecked", "checked"],
        typed: false,
    },
    Decision {
        name: "generic-form",
        owner: "src/lower/generic_facts.rs form_of",
        answers: &["as-it-is", "handle", "box", "held-box", "tagged"],
        typed: false,
    },
    Decision {
        name: "writer-options",
        owner: "src/lower/display.rs compiled_writer_takes_options",
        answers: &["crate", "shown", "none"],
        typed: false,
    },
    Decision {
        name: "library-writer-options",
        owner: "src/lower/display.rs takes_options",
        answers: &["takes", "plain"],
        typed: false,
    },
    Decision {
        name: "evidence-route",
        owner: "src/lower/traits/routes.rs evidence_for",
        answers: &["given", "given-supertrait", "item", "none"],
        typed: false,
    },
    Decision {
        name: "type-fact-given",
        owner: "src/lower/traits/given.rs given_type_fact",
        answers: &["given", "none"],
        typed: false,
    },
    Decision {
        name: "option-payload",
        owner: "src/lower/representation/options.rs boxed_payload",
        answers: &["boxed", "unboxed"],
        typed: true,
    },
    Decision {
        name: "iterator-given",
        owner: "src/lower/iterators.rs given_as_it_is",
        answers: &["as-it-is", "js-iterator"],
        typed: false,
    },
    Decision {
        name: "rc-counted",
        owner: "src/lower/analysis/rc_counts.rs Counted::counts",
        answers: &["counted", "uncounted"],
        typed: false,
    },
    Decision {
        name: "collect-in-place",
        owner: "src/lower/recognition/specialization.rs collects_in_place",
        answers: &["in-place", "apart", "callers"],
        typed: false,
    },
    Decision {
        name: "vec-collect-way",
        owner: "src/lower/recognition/specialization.rs vec_collect_way",
        answers: &["inPlace", "fold", "next"],
        typed: false,
    },
    Decision {
        name: "copy-if-needed",
        owner: "src/lower/copies.rs copy_if_needed",
        answers: &["fresh", "copied", "shared"],
        typed: true,
    },
    Decision {
        name: "type-drops",
        owner: "src/lower/drops/types.rs DropQuery::drops",
        answers: &["nothing", "runs", "unsupported"],
        typed: true,
    },
    Decision {
        name: "mut-ref-form",
        owner: "src/lower/representation/mut_ref.rs mut_ref_form_in",
        answers: &["box", "generic-box", "object", "guard", "given-as-it-is", "itself"],
        typed: true,
    },
    Decision {
        name: "refcell-field",
        owner: "src/lower/drops/ref_cells.rs holds_value",
        answers: &["holds-value", "cell"],
        typed: false,
    },
    Decision {
        name: "stored-place",
        owner: "src/lower/analysis/stored_places.rs records",
        answers: &["recorded", "unrecorded"],
        typed: false,
    },
    Decision {
        name: "back-stepped",
        owner: "src/lower/iterators/back_facts.rs stepped_from_its_back",
        answers: &["typed", "knowing-in", "made-into", "no"],
        typed: true,
    },
    Decision {
        name: "back-counted",
        owner: "src/lower/iterators/back_facts.rs made_counted_from_its_back",
        answers: &["counted", "uncounted"],
        typed: false,
    },
    Decision {
        name: "stepped-local",
        owner: "src/lower/iterators/back_facts.rs local_stepped_from_its_back",
        answers: &["lent", "given", "none"],
        typed: false,
    },
    Decision {
        name: "hint-seen",
        owner: "src/lower/iterators/hint_asks.rs collect_hint_seen",
        answers: &["collect-asks", "extend-asks", "collect-unseen", "extend-unseen"],
        typed: false,
    },
    Decision {
        name: "extend-reserves",
        owner: "src/lower/iterators/hint_asks.rs extend_reserves",
        answers: &["reserves", "none"],
        typed: false,
    },
];

/// What a typed decision's type is, each counted apart: a number, a
/// `bool` or a `char`; an array or a slice; a tuple; a struct; an enum; a
/// type parameter or a projection; a reference or a pointer; anything else.
/// The type's owner, `lower`, says which a type is (`lower::kind_of`).
#[derive(Clone, Copy)]
pub(crate) enum Kind {
    Number,
    Array,
    Tuple,
    Struct,
    Enum,
    Param,
    Ref,
    Other,
}

/// Each kind's name, in `Kind`'s order.
pub(crate) const KINDS: &[&str] = &["number", "array", "tuple", "struct", "enum", "param", "ref", "other"];

/// Where a decision was given: of a body of the crate's own, `own`; of
/// another crate's it lowers, a library's or std's, `other`; of the whole
/// crate, its analysis, `crate` (`in_body`).
pub(crate) const BODIES: &[&str] = &["own", "other", "crate"];

/// The file the counts go to, where asked: an empty setting asks none, as
/// a workflow gives of a run not the nightly.
static PATH: LazyLock<Option<std::ffi::OsString>> =
    LazyLock::new(|| std::env::var_os("RUST_JS_DECISIONS").filter(|path| !path.is_empty()));

/// How many times each decision gave each answer, of each kind and body,
/// this compile.
type Key = (&'static str, &'static str, &'static str, &'static str);
static GIVEN: Mutex<BTreeMap<Key, u64>> = Mutex::new(BTreeMap::new());

thread_local! {
    /// Of which body the decisions now given are (`BODIES`).
    static BODY: std::cell::Cell<&'static str> = const { std::cell::Cell::new("crate") };
}

/// The file the counts go to, where asked.
pub(crate) fn path() -> Option<&'static std::ffi::OsStr> {
    PATH.as_deref()
}

/// `decision` gave `answer`, counted where decisions are asked.
#[inline]
pub(crate) fn record(decision: &'static str, answer: &'static str) {
    if PATH.is_some() {
        count(decision, answer, "-");
    }
}

/// `decision` gave `answer` of a type of `kind`, counted by it.
#[inline]
pub(crate) fn record_typed(decision: &'static str, answer: &'static str, kind: Kind) {
    if PATH.is_some() {
        count(decision, answer, KINDS[kind as usize]);
    }
}

/// `run`, its decisions of a body, the crate's own or another crate's.
pub(crate) fn in_body<T>(own: bool, run: impl FnOnce() -> T) -> T {
    let outer = BODY.with(|body| body.replace(if own { "own" } else { "other" }));
    let done = run();
    BODY.with(|body| body.set(outer));
    done
}

#[cold]
fn count(decision: &'static str, answer: &'static str, kind: &'static str) {
    let known = CATALOGUE
        .iter()
        .any(|d| d.name == decision && d.answers.contains(&answer) && (kind == "-") != d.typed);
    assert!(
        known,
        "rust-js: decision `{decision}`'s answer `{answer}` of kind `{kind}` isn't in src/decisions.rs' catalogue"
    );
    let body = BODY.with(|body| body.get());
    let mut given = GIVEN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    *given.entry((decision, answer, kind, body)).or_default() += 1;
}

/// This compile's counts, where asked, and the file they go to: a
/// `decision\tanswer\tkind\tbody\tcount\tbuild\tshape\towner` line of each
/// answer it gave, its kind `-` of a decision not typed; and, where the
/// file has none yet, `whole`, every decision's every answer of every kind
/// and body, `0` of one never given, so `scripts/decision-coverage.ts`
/// knows each answer there is, the file kept to a line an answer given. Its
/// build is `std` of `--std-iterators` or `plain`, its shape `library` of
/// `--library` or `program`, by its arguments. The driver appends them
/// (`main.rs`), as only it writes files.
pub(crate) fn counts(whole: bool) -> Option<(&'static std::ffi::OsStr, String)> {
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
        let kinds: &[&str] = if decision.typed { KINDS } else { &["-"] };
        for &answer in decision.answers {
            for &kind in kinds {
                for &body in BODIES {
                    let n = given.get(&(decision.name, answer, kind, body)).copied().unwrap_or(0);
                    if n > 0 || whole {
                        lines.push_str(&format!(
                            "{}\t{answer}\t{kind}\t{body}\t{n}\t{build}\t{shape}\t{}\n",
                            decision.name, decision.owner
                        ));
                    }
                }
            }
        }
    }
    Some((path, lines))
}
