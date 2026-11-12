//! The `const`s MIR's lowering made of a body, which tidy reads as its
//! temporaries.

use crate::collections::{IndexMap, IndexSet};
use crate::js::Expr;

/// The `const`s MIR's lowering made of a body, tidy's temporaries, read where used where nothing between tells.
#[derive(Default)]
pub(in crate::lower) struct Made {
    /// Of what an element made ahead of a branch reads (`capture_jsx`).
    pub(in crate::lower) captured: IndexSet<String>,
    /// Of what it held to read in place where that's once (`held_in_place`), a place it fixed,
    /// `const item = $unwrap(resource.state);`, or an `Option` its items read twice.
    pub(in crate::lower) fixed: IndexSet<String>,
    /// Of what holds a `&mut`'s handle on a nullable place, the place (#541).
    pub(in crate::lower) handles: IndexMap<String, Expr>,
    /// The body's naming table (#890): each name `spill` gave a value of a
    /// role at once, `value$3`, a name valid wherever tidy stops, with that
    /// role (`note_spilled`), which tidy's one pick names it by again once
    /// the code is placed.
    pub(in crate::lower) spilled: IndexMap<String, crate::lower::temporary_names::Role>,
}
