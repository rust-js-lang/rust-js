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

use crate::collections::{IndexSet, OnceMap};

/// What a lowering session finds once, and the flags of its run.
pub(crate) struct Caches {
    /// The session's, by its rustc session's address: a read with another
    /// `tcx` is a bug, which `caches` says.
    gcx: usize,
    /// `std_path`'s answers, each of std's items printed once.
    pub std_paths: OnceMap<DefId, &'static str>,
    /// The std types whose bodies are compiled into `@rust-js/std`.
    pub compiled_types: OnceCell<IndexSet<DefId>>,
    /// std's bodies compiled into `@rust-js/std`, each found in std.
    pub compiled: OnceCell<Rc<Vec<DefId>>>,
    /// A trait's provided method of std's, by the trait's and its name.
    pub provided: OnceMap<(&'static str, &'static str), Option<DefId>>,
    /// That this session builds std's structs as their meaning (ADR 0932).
    pub meaning: Cell<bool>,
    /// That every struct outside the crate is one JS may make (ADR 1105).
    pub every_outside_struct: Cell<bool>,
}

thread_local! {
    /// The session's caches while it runs: filled by `start`, emptied by
    /// its guard's drop alone.
    static SESSION: RefCell<Option<Rc<Caches>>> = const { RefCell::new(None) };
}

/// The address of `tcx`'s rustc session, one each session.
fn gcx_of(tcx: TyCtxt<'_>) -> usize {
    std::ptr::from_ref(tcx.sess) as usize
}

/// The flags a session's run is given as it starts.
#[derive(Clone, Copy, Default)]
pub(crate) struct Flags {
    pub meaning: bool,
    pub every_outside_struct: bool,
}

/// Holds a session's caches in place; dropped, it takes them away, as the
/// session ends.
pub(crate) struct Session(());

impl Drop for Session {
    fn drop(&mut self) {
        SESSION.with(|slot| slot.borrow_mut().take());
    }
}

/// A lowering session of `tcx` begun, its caches made empty and its flags
/// `flags`, until the guard it gives is dropped. One at a time: a session
/// started inside another is a bug, which it says.
#[must_use = "the session ends when its guard is dropped"]
pub(crate) fn start(tcx: TyCtxt<'_>, flags: Flags) -> Session {
    start_keyed(gcx_of(tcx), flags)
}

/// `start`, of the session whose rustc session is at `gcx`.
fn start_keyed(gcx: usize, flags: Flags) -> Session {
    SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        assert!(
            slot.is_none(),
            "rust-js: a lowering session started inside another (lower::session)"
        );
        *slot = Some(Rc::new(Caches {
            gcx,
            std_paths: OnceMap::default(),
            compiled_types: OnceCell::new(),
            compiled: OnceCell::new(),
            provided: OnceMap::default(),
            meaning: Cell::new(flags.meaning),
            every_outside_struct: Cell::new(flags.every_outside_struct),
        }));
    });
    Session(())
}

/// The caches of the session `tcx` is, which must be the one running: asked
/// outside a session, or with another session's `tcx`, it's a bug, which it
/// says.
pub(crate) fn caches(tcx: TyCtxt<'_>) -> Rc<Caches> {
    caches_keyed(gcx_of(tcx))
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
    use rustc_span::def_id::{CrateNum, DefId, DefIndex};

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
                every_outside_struct: false,
            },
        );
        caches_keyed(1).std_paths.keep(item(7), "std::first::Item");
        assert_eq!(caches_keyed(1).std_paths.get(&item(7)), Some("std::first::Item"));
        assert!(caches_keyed(1).meaning.get());
        drop(first);
        let second = start_keyed(2, Flags::default());
        assert_eq!(caches_keyed(2).std_paths.get(&item(7)), None);
        assert!(!caches_keyed(2).meaning.get());
        drop(second);
    }

    #[test]
    #[should_panic(expected = "asked outside a lowering session")]
    fn a_cache_asked_after_its_session_ends_is_a_bug() {
        drop(start_keyed(3, Flags::default()));
        caches_keyed(3);
    }

    #[test]
    #[should_panic(expected = "asked with another session's tcx")]
    fn a_cache_asked_with_another_sessions_tcx_is_a_bug() {
        let _session = start_keyed(4, Flags::default());
        caches_keyed(5);
    }

    #[test]
    #[should_panic(expected = "started inside another")]
    fn a_session_started_inside_another_is_a_bug() {
        let _outer = start_keyed(6, Flags::default());
        let _inner = start_keyed(7, Flags::default());
    }
}
