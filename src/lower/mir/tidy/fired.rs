//! Which of tidy's passes changed a body, and how many, where a compile
//! records it, `--tidy-fired <file>` (#1461): what a pass's deletion is
//! measured by, a pass that changed nothing of the corpus, the ports and
//! the generated programs. Recorded by statics, as `known.rs`' facts
//! found are, tidy's hints being JS's alone, with no session of their own.
//! "Fired" is "changed the body", not "was needed": a pass may fire and a
//! later one undo it. A body is compared by `Stmt`'s `==`, which compares
//! facts as their readers see them and leaves a value's held name out, so
//! a pass that only renames one held doesn't fire.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::js::Stmt;

/// Whether this compile records which passes fired.
static RECORDS: AtomicBool = AtomicBool::new(false);
/// Each pass that fired, by its name, `stage/pass`, and how many bodies it
/// changed.
static FIRED: Mutex<BTreeMap<String, usize>> = Mutex::new(BTreeMap::new());

/// Records which passes fire, for the rest of the compile.
pub(in crate::lower) fn record() {
    RECORDS.store(true, Ordering::Relaxed);
}

/// `body` as it is before a pass, where this compile records: a build that
/// doesn't records nothing, at the cost of one read a pass.
pub(super) fn before(body: &[Stmt]) -> Option<Vec<Stmt>> {
    RECORDS.load(Ordering::Relaxed).then(|| body.to_vec())
}

/// The pass `name` names changed `body`, which `was` before it, where it
/// did.
pub(super) fn after(was: Option<Vec<Stmt>>, body: &[Stmt], name: impl FnOnce() -> String) {
    if was.is_some_and(|was| was != body) {
        *FIRED
            .lock()
            .expect("no pass panics holding it")
            .entry(name())
            .or_default() += 1;
    }
}

/// Each pass that fired and how many bodies it changed, a line each,
/// `stage/pass<TAB>bodies`, in `names`' order, tidy's run's.
pub(in crate::lower) fn report(names: &[String]) -> String {
    let fired = FIRED.lock().expect("no pass panics holding it");
    (names.iter())
        .filter_map(|name| fired.get(name).map(|count| format!("{name}\t{count}\n")))
        .collect()
}
