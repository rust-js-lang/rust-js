//! The `const`s MIR's lowering made of a body, which tidy reads as its
//! temporaries.

use rust_js_program::collections::{IndexMap, IndexSet};
use rust_js_program::js::Expr;

/// The `const`s MIR's lowering made of a body, tidy's temporaries, read where used where nothing between tells.
#[derive(Default)]
pub struct Made {
    /// Of what an element made ahead of a branch reads (`capture_jsx`).
    pub captured: IndexSet<rust_js_program::js::Name>,
    /// Of what it held to read in place where that's once (`held_in_place`), a place it fixed,
    /// `const item = $unwrap(resource.state);`, or an `Option` its items read twice.
    pub fixed: IndexSet<rust_js_program::js::Name>,
    /// Of what holds a `&mut`'s handle on a nullable place, the place (#541).
    pub handles: IndexMap<rust_js_program::js::Name, Expr>,
}
