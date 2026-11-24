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
    /// Of what holds a `&mut`'s handle on a place that stores its own way,
    /// a nullable place's (#541) or a flattened state's (#2429), the place
    /// and how it stores (`note_handle`, `FnCx::held_handle`).
    pub handles: IndexMap<rust_js_program::js::Name, Handled>,
    /// Of what holds an `Ok` made of such a handle, a flattened state's
    /// `try_borrow_mut()`'s, that handle's variable, which its payload read
    /// is (`held_ok`, #2429).
    oks: IndexMap<rust_js_program::js::Name, Expr>,
}

impl Made {
    /// `name`, a `const` MIR's lowering declares once of `made`, noted
    /// where `made` is a handle on a place (`Handled::of`), so what's lent
    /// of its `value` is that handle,
    /// `mut_refs/handles.rs`' (ADR 0791, #2429).
    /// And of `made`, a variable holding one, that handle's: `name` is
    /// another name of it, `const state$1 = state`; and of an `Ok` made of
    /// one, its payload's (`held_ok`, #2429).
    pub fn note_handle(&mut self, name: &rust_js_program::js::Name, made: &Expr) {
        let handled = match &made.kind {
            rust_js_program::js::ExprKind::Var(held) => self.handles.get(held).cloned(),
            _ => Handled::of(made),
        };
        if let Some(handled) = handled {
            self.handles.insert(name.clone(), handled);
        }
        if let Some(Some(payload)) = rust_js_program::js::result::made_ok_payload(made)
            && let rust_js_program::js::ExprKind::Var(held) = &payload.kind
            && self.handles.contains_key(held)
        {
            self.oks.insert(name.clone(), payload.clone());
        }
    }

    /// Of `name`, an `Ok` `note_handle` noted made of a held handle, that
    /// handle's variable, which its payload read is.
    pub fn held_ok(&self, name: &rust_js_program::js::Name) -> Option<&Expr> {
        self.oks.get(name)
    }
}

/// A handle a variable holds of a `&mut` to a place that stores its own
/// way: its place, and how what's written to it is stored.
#[derive(Clone)]
pub struct Handled {
    pub on: Expr,
    pub storage: rust_js_program::js::Storage,
}

impl Handled {
    /// Of `made`, a handle on a place, its place and storage, which stores
    /// what's written its own way, `null` for a `None` or a flattened
    /// state's keys set on its object (ADRs 0791, 1044), or as it's given;
    /// `None` of anything else. One stored as it's given is recorded too:
    /// a pair or a handle on its place reads and writes as one on its
    /// `value` does (#2433).
    fn of(made: &Expr) -> Option<Handled> {
        match &made.kind {
            rust_js_program::js::ExprKind::Handle(on, storage) => Some(Handled {
                on: (**on).clone(),
                storage: *storage,
            }),
            _ => None,
        }
    }

    /// The place a write through it assigns, its own, of a storage that
    /// writes the value it's given (`Storage::written`), a nullable
    /// place's; `None` of one written in place, a flattened state's keys
    /// or an object `$assign`ed (`Storage::written_in_place`), whose
    /// place a write would rebind.
    pub fn assigned_place(&self) -> Option<&Expr> {
        self.storage.assigns_its_place().then_some(&self.on)
    }
}
