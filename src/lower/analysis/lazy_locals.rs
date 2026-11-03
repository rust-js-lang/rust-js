//! What each thread-local's initializer may run and read as its module
//! loads, where it's made, as JS's `const x = f();` and `export const`
//! are, whatever it runs (the user's decisions, ADR 1006, amended): React
//! DOM's `const isScrollEndEventSupported = isEventSupported("scrollend")`.
//! The pipeline asks of the modules' imports where its module's load can't
//! make it first (`load_cycles::out_of_order`): one that runs what rust-js
//! can't follow, reads itself, or runs a module in an import cycle with its
//! own. A private one of those is made where it's first used, as Rust makes
//! each (ADR 0037, amended), the crate lowered again so (`made_where_used`);
//! a `pub` one is refused.

use std::collections::{HashMap, HashSet};

use rustc_span::def_id::{DefId, LocalDefId};

use super::may_run::Runs;

/// The thread-locals made where they're first used, and each other one,
/// made as its module loads, by its key.
pub(in crate::lower) struct LazyLocals {
    pub(in crate::lower) lazy: HashSet<LocalDefId>,
    /// Each made as its module loads, as JS's `const x = f();` is, and what
    /// its initializer may run and read there, all `may_run` finds, the
    /// one rule; `None` where it may run what rust-js can't follow. The pipeline asks, of the
    /// modules' imports, where its module's load can't make it
    /// (`load_cycles::out_of_order`, ADR 1006, amended).
    pub(in crate::lower) at_load: HashMap<LocalDefId, Option<Vec<DefId>>>,
}

/// The thread-locals, by their keys, of `inits`, each initializer's key:
/// those made where they're first used already, `made_where_used`, which a
/// lowering of the crate found their module's load can't make; and each
/// other, made as its module loads whatever its initializer runs (the
/// user's decision, ADR 1006, amended), and what it may run and read there,
/// as `runs` finds.
pub(super) fn lazy_locals(
    (inits, made_where_used): (&HashMap<LocalDefId, LocalDefId>, &HashSet<LocalDefId>),
    runs: &Runs<'_, '_>,
) -> LazyLocals {
    let mut found = LazyLocals {
        lazy: made_where_used.clone(),
        at_load: HashMap::new(),
    };
    // What each initializer may run and read as its module loads, all
    // `may_run` finds, which follows each thread-local it reads to its
    // own initializer: the one rule (the user's decision, rd120).
    for (&init, &key) in inits {
        if made_where_used.contains(&key) || runs.body(init.to_def_id()).is_none() {
            continue;
        }
        found.at_load.insert(key, runs.of(init.to_def_id()));
    }
    found
}
