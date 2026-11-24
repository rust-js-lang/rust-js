//! The lowering session's own state: what it finds once and asks again,
//! keyed by rustc's ids, and the flags its run is given (ADR 1111). A
//! `DefId`, a `CrateNum` or a `Ty` names an item of one session alone: a
//! second session in the process numbers its crates as it loads them, so a
//! cache of the first's answers would answer for another item. So each
//! lives in the session's `Caches`, made as the session starts and dropped
//! as it ends (`start`'s guard), as rustc carries its own implicit context:
//! one scoped slot, which only the guard fills and empties, and every read
//! of it checked against the `tcx` it's asked with (`caches`). No
//! `thread_local!` or `static` of the compiler's holds such a cache
//! (test/architecture.test.ts).

use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

use rust_js_program::collections::{IndexSet, OnceMap};

use crate::mir_hooks::MirHooks;

// The compile's reports, which `mir` records into as it lowers: the labels
// each body keeps (`--label-census`) and the tidy passes that fired
// (`--tidy-fired`), the session's since #2299 moved them from `mir`.
pub mod label_census;
pub mod tidy_fired;

/// What a lowering session finds once, and the flags of its run.
pub struct Caches {
    /// The session's, by its rustc session's address: a read with another
    /// `tcx` is a bug, which `caches` says.
    gcx: usize,
    /// MIR's lowering, as the base asks it, which the driver gives the
    /// session (`MirHooks`, #2299).
    pub(crate) hooks: &'static dyn MirHooks,
    /// `std_path`'s answers, each of std's items printed once.
    pub std_paths: OnceMap<DefId, &'static str>,
    /// Each impl's dictionary's name, and its type's and trait's words
    /// alone, which every other impl's name is compared with: found once
    /// each, not of every impl for each name asked, zerocopy's many (#1298).
    pub impl_names: OnceMap<DefId, String>,
    pub impl_short_names: OnceMap<DefId, String>,
    /// `overridden_in_reach`'s, of each provided method asked (#2414).
    pub overridden: OnceMap<DefId, bool>,
    /// `tracking_impls`'s, of each method whose model raises at its caller
    /// asked (#2457).
    pub tracking_impls: OnceMap<DefId, std::rc::Rc<Vec<DefId>>>,
    /// The std types whose bodies are compiled into `@rust-js/std`.
    pub compiled_types: OnceCell<IndexSet<DefId>>,
    /// std's bodies compiled into `@rust-js/std`, each found in std.
    pub compiled: OnceCell<Rc<IndexSet<DefId>>>,
    /// The crate's types JS may hold a value of and give back
    /// (`declarations::js_holds`, #2024), found once.
    pub js_held: OnceCell<Rc<IndexSet<DefId>>>,
    /// The crate's classes whose prototype the program replaces, each a
    /// constructor function (`classes::forms`, #2495), found once.
    pub prototype_replaced: OnceCell<Rc<IndexSet<DefId>>>,
    /// The functions only calls reach, by their calls, found of the crate
    /// graph's names (ADRs 0577, 1148): the own callbacks' to ask.
    pub only_called:
        OnceCell<Rc<rust_js_program::collections::IndexMap<DefId, Vec<crate::analysis::only_called::Call>>>>,
    /// std's items a recognizer knows by their path, each resolved (#1681).
    pub std_path_ids: OnceCell<Rc<crate::recognition::StdPaths>>,
    /// Each of std's modules a recognizer asks what's under, of each of std's
    /// crates that has it (#1681).
    pub std_module_ids: OnceCell<Rc<crate::recognition::StdModules>>,
    /// Each of std's items' visible parent, of std's crates' modules alone
    /// (`std_paths::std_visible_parents`, #2314).
    pub std_visible_parents: OnceCell<Rc<rustc_hash::FxHashMap<rustc_span::def_id::DefId, rustc_span::def_id::DefId>>>,
    /// std's catalog of compiled bodies, each row resolved (#1681).
    pub rows: OnceCell<Rc<crate::recognition::Rows>>,
    /// That this session builds std's structs as their meaning (ADR 0932).
    pub meaning: Cell<bool>,
    /// That every struct outside the crate is one JS may make (ADR 1105).
    pub every_outside_struct: Cell<bool>,
    /// The types of std's adapters compiled as their meaning alone whose
    /// `{:?}` the crate asks, each by its address interned in this session,
    /// regions erased, and their names: std's struct where they're held
    /// (#1401). A type's address names it in this session's `TyCtxt` alone,
    /// so it lives no longer than the session, never written to disk.
    pub debug_shown: OnceCell<(IndexSet<usize>, IndexSet<crate::recognition::StdPath>)>,
    /// The std functions whose recognisers this session leaves out, by
    /// their std paths, `--without-recognizer`'s (ADR 1128).
    pub without_recognizers: IndexSet<String>,
    /// The types this session forgets of what's changed in place, as
    /// `{ty}` prints them, `--without-mutated`'s (#1665).
    pub without_mutated: IndexSet<String>,
    /// That this session takes nothing MIR's intervals prove,
    /// `--without-intervals`'s (#2112 B).
    pub without_intervals: bool,
    /// That this session's drop elaboration is blind to moves,
    /// `--without-moves`'s (#2112's change D).
    pub without_moves: bool,
    /// That what MIR's intervals keep is asked of the effects query too,
    /// `--intervals-by-effects`'s (#2112 B step 3a).
    pub intervals_by_effects: bool,
    /// Each check THIR's facts leave out that MIR doesn't prove, where the
    /// session records them, `--intervals-shadow`'s (#1672).
    pub intervals_shadow: Option<RefCell<super::analysis::Shadow>>,
    /// What std's own code reaches of the crate's impls that take an
    /// iterator that `crate_impl_called_by` doesn't answer, found as the
    /// crate is analyzed (`instances::check_std_reach`), each said where the
    /// compile ends without an error, as one a lowering refuses is no
    /// program (#2376).
    #[cfg(debug_assertions)]
    pub(crate) std_reach_unanswered: RefCell<Vec<String>>,
    /// Of each `Drop` of the crate's bodies, its comparison with rustc's
    /// elaboration, a line each, where the session records them,
    /// `--drops-shadow`'s (#2112's change D).
    pub drops_shadow: Option<RefCell<Vec<String>>>,
    /// What MIR proves of the bodies being lowered, and what each function
    /// only calls reach is given (#1672, ADR 1137).
    pub intervals: RefCell<super::analysis::Intervals>,
    /// The std functions a recogniser took, each once, in the order it
    /// first did, where the session records them (`--recognizers-found`).
    pub recognized: Option<RefCell<IndexSet<&'static str>>>,
    /// Of a library being lowered, each function's copies its copy policy
    /// left out, by the shape of the type each was of: what a type nothing
    /// in its crate changes in place is copied by nowhere (ADR 0914). Its
    /// contract says them, so a consumer that changes one compiles the
    /// function itself (#2112's change D).
    pub elided: RefCell<
        rust_js_program::collections::IndexMap<DefId, std::collections::BTreeSet<rust_js_program::library::RcShape>>,
    >,
}

thread_local! {
    /// The session's caches while it runs: filled by `start`, emptied by
    /// its guard's drop alone.
    static SESSION: RefCell<Option<Rc<Caches>>> = const { RefCell::new(None) };
}

/// The std functions a recogniser took in the session `tcx` is, a line
/// each, in the order each first was: what `--recognizers-found` writes
/// (ADR 1128); nothing of a session that records none.
pub fn recognized(tcx: TyCtxt<'_>) -> String {
    let caches = caches(tcx);
    let Some(recognized) = &caches.recognized else {
        return String::new();
    };
    recognized.borrow().iter().map(|path| format!("{path}\n")).collect()
}

/// Each check THIR's ranges leave out that MIR's intervals don't prove, a
/// line each: what `--intervals-shadow` writes (#1672).
pub fn intervals_shadow(tcx: TyCtxt<'_>) -> String {
    super::analysis::intervals_shadow_report(tcx)
}

/// Each `Drop` of the crate's bodies compared with rustc's elaboration, a
/// line each: what `--drops-shadow` writes (#2112's change D).
pub fn drops_shadow(tcx: TyCtxt<'_>) -> String {
    let caches = caches(tcx);
    let Some(lines) = &caches.drops_shadow else {
        return String::new();
    };
    let lines = lines.borrow();
    let unique: IndexSet<&String> = lines.iter().collect();
    unique.into_iter().map(|line| format!("{line}\n")).collect()
}

/// Records which of tidy's passes change a body, for the rest of the
/// compile: `--tidy-fired`'s (#1461).
pub fn record_tidy_fired() {
    tidy_fired::record();
}

/// Each of tidy's passes that changed a body, and how many it changed, a
/// line each, `stage/pass<TAB>bodies`, in the order they run: what
/// `--tidy-fired` writes; nothing of a compile that records none. Of
/// `names`, tidy's passes in the order they run, which the driver gives
/// (`lower::tidy_pass_names`), tidy being `mir`'s.
pub fn tidy_fired(names: &[String]) -> String {
    tidy_fired::report(names)
}

/// Forgets, for the rest of the compile, where the program writes its
/// constructs, which tidy reads its JS by: `--no-thir-hints` (ADR 1144).
pub fn forget_thir_hints() {
    NO_THIR_HINTS.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// Whether this compile forgets where the program writes its constructs,
/// `--no-thir-hints`: tidy is told nothing of them, its JS reads worse, and
/// must run the same, as a readability hint carries no meaning (ADR 1144,
/// amended: a net of `test/tidy-stages.test.ts`).
static NO_THIR_HINTS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether tidy is told where the program writes its constructs.
// helper: the compile's `--no-thir-hints` switch, asked by `mir::hints`, the session's since #2299 moved it there with the other switches
pub fn thir_hints() -> bool {
    !NO_THIR_HINTS.load(std::sync::atomic::Ordering::Relaxed)
}

/// Resolves std's paths before any body is lowered, for the rest of the
/// compile: `--std-paths-first`'s (ADR 0879, amended).
pub fn ask_std_paths_first() {
    super::recognition::ask_std_paths_first();
}

/// Records which compiler-made labels each body keeps after tidy, by the
/// MIR shape that left it: `--label-census`'s (#1461).
pub fn record_label_census() {
    label_census::record();
}

/// Each shape of a label kept and how many, a line each, `count<TAB>shape`,
/// the most first: what `--label-census` writes.
pub fn label_census() -> String {
    label_census::report()
}

/// The address of `tcx`'s rustc session, one each session.
fn gcx_of(tcx: TyCtxt<'_>) -> usize {
    std::ptr::from_ref(tcx.sess) as usize
}

/// The flags a session's run is given as it starts.
#[derive(Clone, Default)]
pub struct Flags {
    pub meaning: bool,
    pub every_outside_struct: bool,
    /// The std paths of the functions whose recognisers are left out.
    pub without_recognizers: Vec<String>,
    /// The types forgotten of what's changed in place.
    pub without_mutated: Vec<String>,
    /// Whether nothing MIR's intervals prove is taken.
    pub without_intervals: bool,
    /// Whether the drop elaboration is blind to moves.
    pub without_moves: bool,
    /// Whether what they keep is asked of the effects query too.
    pub intervals_by_effects: bool,
    /// Whether the functions a recogniser takes are recorded.
    pub records_recognized: bool,
    /// Whether the checks THIR leaves out that MIR doesn't prove are.
    pub records_intervals_shadow: bool,
    /// Whether each `Drop`'s comparison with rustc's elaboration is.
    pub records_drops_shadow: bool,
}

/// Holds a session's caches in place; dropped, it takes them away, as the
/// session ends.
pub struct Session(());

impl Drop for Session {
    fn drop(&mut self) {
        SESSION.with(|slot| slot.borrow_mut().take());
    }
}

/// A lowering session of `tcx` begun, its caches made empty and its flags
/// `flags`, until the guard it gives is dropped. One at a time: a session
/// started inside another is a bug, which it says.
#[must_use = "the session ends when its guard is dropped"]
pub fn start(tcx: TyCtxt<'_>, flags: Flags, hooks: &'static dyn MirHooks) -> Session {
    start_keyed(gcx_of(tcx), flags, hooks)
}

/// `start`, of the session whose rustc session is at `gcx`.
fn start_keyed(gcx: usize, flags: Flags, hooks: &'static dyn MirHooks) -> Session {
    SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        assert!(
            slot.is_none(),
            "rust-js: a lowering session started inside another (lower::session)"
        );
        *slot = Some(Rc::new(Caches {
            gcx,
            hooks,
            std_paths: OnceMap::default(),
            impl_names: OnceMap::default(),
            impl_short_names: OnceMap::default(),
            overridden: OnceMap::default(),
            tracking_impls: OnceMap::default(),
            compiled_types: OnceCell::new(),
            compiled: OnceCell::new(),
            js_held: OnceCell::new(),
            prototype_replaced: OnceCell::new(),
            only_called: OnceCell::new(),
            std_path_ids: OnceCell::new(),
            std_module_ids: OnceCell::new(),
            std_visible_parents: OnceCell::new(),
            rows: OnceCell::new(),
            meaning: Cell::new(flags.meaning),
            every_outside_struct: Cell::new(flags.every_outside_struct),
            debug_shown: OnceCell::new(),
            without_recognizers: flags.without_recognizers.into_iter().collect(),
            without_mutated: flags.without_mutated.into_iter().collect(),
            without_intervals: flags.without_intervals,
            without_moves: flags.without_moves,
            intervals_by_effects: flags.intervals_by_effects,
            intervals_shadow: flags.records_intervals_shadow.then(RefCell::default),
            #[cfg(debug_assertions)]
            std_reach_unanswered: RefCell::default(),
            drops_shadow: flags.records_drops_shadow.then(RefCell::default),
            intervals: RefCell::default(),
            recognized: flags.records_recognized.then(RefCell::default),
            elided: RefCell::default(),
        }));
    });
    Session(())
}

/// The caches of the session `tcx` is, which must be the one running: asked
/// outside a session, or with another session's `tcx`, it's a bug, which it
/// says.
pub fn caches(tcx: TyCtxt<'_>) -> Rc<Caches> {
    caches_keyed(gcx_of(tcx))
}

/// The hooks of the session running (`Caches::hooks`), asked where no `tcx`
/// is at hand: one for every session of a compile, the driver's, so no
/// session's ids are in them to check `tcx` against, as `caches` does.
/// Asked outside a session, it's a bug, which it says.
pub(crate) fn hooks() -> &'static dyn MirHooks {
    let caches = SESSION.with(|slot| slot.borrow().clone());
    caches
        .expect("rust-js: MIR's hooks asked outside a lowering session (lower::start_session)")
        .hooks
}

/// `caches`, of the session whose rustc session is at `gcx`.
fn caches_keyed(gcx: usize) -> Rc<Caches> {
    let caches = SESSION.with(|slot| slot.borrow().clone());
    let caches = caches.expect("rust-js: a session's cache asked outside a lowering session (lower::session::start)");
    assert_eq!(
        caches.gcx, gcx,
        "rust-js: a session's cache asked with another session's tcx (lower::session)"
    );
    caches
}

#[cfg(test)]
mod tests {
    use super::{Flags, caches_keyed, start_keyed};
    use crate::analysis::Mir;
    use crate::mir_hooks::MirHooks;
    use crate::{Body, FnCx, R};
    use rust_js_program::js::{self, Expr, Stmt};
    use rustc_middle::ty;
    use rustc_span::def_id::{CrateNum, DefId, DefIndex};

    /// No MIR lowering: a session's caches asked, never its hooks.
    struct NoHooks;

    impl MirHooks for NoHooks {
        fn mir_signature<'a, 'tcx>(
            &self,
            _: &mut FnCx<'a, 'tcx>,
            _: &Body<'tcx>,
            _: &Mir<'tcx>,
            _: &mut Vec<Stmt>,
        ) -> R<(Vec<js::Pattern>, bool)> {
            unreachable!("a session's test lowers nothing")
        }

        fn crate_const_as_code<'a, 'tcx>(
            &self,
            _: &mut FnCx<'a, 'tcx>,
            _: DefId,
            _: ty::GenericArgsRef<'tcx>,
        ) -> R<Expr> {
            unreachable!("a session's test lowers nothing")
        }

        fn settle_unthrown_catches(&self, _: &mut Vec<Stmt>) {
            unreachable!("a session's test lowers nothing")
        }
    }

    fn item(n: u32) -> DefId {
        DefId {
            krate: CrateNum::from_u32(1),
            index: DefIndex::from_u32(n),
        }
    }

    // A second session in the process is given caches of its own: what the
    // first found of an item isn't its answer, as its crates are numbered
    // anew.
    #[test]
    fn a_second_session_starts_with_caches_of_its_own() {
        let first = start_keyed(
            1,
            Flags {
                meaning: true,
                ..Flags::default()
            },
            &NoHooks,
        );
        caches_keyed(1).std_paths.keep(item(7), "std::first::Item");
        assert_eq!(caches_keyed(1).std_paths.get(&item(7)), Some("std::first::Item"));
        assert!(caches_keyed(1).meaning.get());
        drop(first);
        let second = start_keyed(2, Flags::default(), &NoHooks);
        assert_eq!(caches_keyed(2).std_paths.get(&item(7)), None);
        assert!(!caches_keyed(2).meaning.get());
        drop(second);
    }

    #[test]
    #[should_panic(expected = "asked outside a lowering session")]
    fn a_cache_asked_after_its_session_ends_is_a_bug() {
        drop(start_keyed(3, Flags::default(), &NoHooks));
        caches_keyed(3);
    }

    #[test]
    #[should_panic(expected = "asked with another session's tcx")]
    fn a_cache_asked_with_another_sessions_tcx_is_a_bug() {
        let _session = start_keyed(4, Flags::default(), &NoHooks);
        caches_keyed(5);
    }

    #[test]
    #[should_panic(expected = "started inside another")]
    fn a_session_started_inside_another_is_a_bug() {
        let _outer = start_keyed(6, Flags::default(), &NoHooks);
        let _inner = start_keyed(7, Flags::default(), &NoHooks);
    }
}
