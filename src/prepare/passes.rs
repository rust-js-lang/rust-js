//! Preparation's rewrites, each named, so each can be left out (ADR 1087):
//! `--prepare-without <pass>` prepares a module with every rewrite but that
//! one, and `--prepare-fired <file>` writes which of them changed anything.
//! Each is a readability rewrite of a module already correct, so leaving
//! any out leaves it correct too: a module that does otherwise without one
//! names the rewrite, or the one it fed, that made it so. What a module
//! needs to be printed at all, a `&mut`'s handle written as its object and
//! `this` read apart for it, is no rewrite: it's always made.
//!
//! Which are left out is preparation's alone, a test hook as `--tidy-until`
//! is (ADR 0659): set for one module's preparation (`with`), read by each
//! rewrite where it's about to be made (`applies`, `run`, `rewrite`), and
//! never by a phase before or after.

use std::cell::Cell;

/// One of preparation's rewrites, as `--prepare-without` names it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pass {
    UnboundReads,
    OwnMethodCalls,
    InitializersAtTop,
    InitializersInPlace,
    ForwardedFunctions,
    ForwardedFields,
    ForwardedMembers,
    TryCatches,
    TriedSets,
    LeftOutFields,
    FirstArms,
    Coalescing,
    EmptyElses,
    UndefinedLeftOut,
    Tests,
    FallbacksHeld,
    FallbacksDeclared,
    TailTries,
    LastReturns,
    Shadows,
    UnreadConstants,
    DynamicImports,
    TypeofsImplied,
    NumbersJoined,
    BooleanCallbacks,
    NullOrFalsy,
    AsyncClosures,
    UndefinedAttributes,
    HandlerReturns,
    FallenChains,
    KeptTruths,
    TemplatesSpliced,
    ConditionalsAsLogicals,
    UndefinedArguments,
    EntriesWrittenOut,
    UndefinedKeys,
    SpreadsWrittenOut,
    Methods,
    TruthsJoined,
    UnusedLabels,
    ElsesAfterExits,
    TestedAssignments,
    AssignedBreaks,
    NamesReclaimed,
    CompoundAssignments,
    SteppedWhiles,
    Counters,
    ReadTogether,
    DeclaredFunctions,
    DeclaredModuleFunctions,
    DoWhiles,
}

/// Each rewrite and its name, in the order a module's preparation first
/// makes it: what `--prepare-passes` lists and `--prepare-without` names.
pub const PASSES: &[(Pass, &str)] = &[
    (Pass::UnboundReads, "unbound reads"),
    (Pass::OwnMethodCalls, "own method calls"),
    (Pass::InitializersAtTop, "initializers at top"),
    (Pass::InitializersInPlace, "initializers in place"),
    (Pass::ForwardedFunctions, "forwarded functions"),
    (Pass::ForwardedFields, "forwarded fields"),
    (Pass::ForwardedMembers, "forwarded members"),
    (Pass::TryCatches, "try catches"),
    (Pass::TriedSets, "tried sets"),
    (Pass::LeftOutFields, "left out fields"),
    (Pass::FirstArms, "first arms"),
    (Pass::Coalescing, "coalescing"),
    (Pass::EmptyElses, "empty elses"),
    (Pass::UndefinedLeftOut, "undefined left out"),
    (Pass::Tests, "tests"),
    (Pass::FallbacksHeld, "fallbacks held"),
    (Pass::FallbacksDeclared, "fallbacks declared"),
    (Pass::TailTries, "tail tries"),
    (Pass::LastReturns, "last returns"),
    (Pass::Shadows, "shadows"),
    (Pass::UnreadConstants, "unread constants"),
    (Pass::DynamicImports, "dynamic imports"),
    (Pass::TypeofsImplied, "typeofs implied"),
    (Pass::NumbersJoined, "numbers joined"),
    (Pass::BooleanCallbacks, "boolean callbacks"),
    (Pass::NullOrFalsy, "null or falsy"),
    (Pass::AsyncClosures, "async closures"),
    (Pass::UndefinedAttributes, "undefined attributes"),
    (Pass::HandlerReturns, "handler returns"),
    (Pass::FallenChains, "fallen chains"),
    (Pass::KeptTruths, "kept truths"),
    (Pass::TemplatesSpliced, "templates spliced"),
    (Pass::ConditionalsAsLogicals, "conditionals as logicals"),
    (Pass::UndefinedArguments, "undefined arguments"),
    (Pass::EntriesWrittenOut, "entries written out"),
    (Pass::UndefinedKeys, "undefined keys"),
    (Pass::SpreadsWrittenOut, "spreads written out"),
    (Pass::Methods, "methods"),
    (Pass::TruthsJoined, "truths joined"),
    (Pass::UnusedLabels, "unused labels"),
    (Pass::ElsesAfterExits, "elses after exits"),
    (Pass::TestedAssignments, "tested assignments"),
    (Pass::AssignedBreaks, "assigned breaks"),
    (Pass::NamesReclaimed, "names reclaimed"),
    (Pass::CompoundAssignments, "compound assignments"),
    (Pass::SteppedWhiles, "stepped whiles"),
    (Pass::Counters, "counters"),
    (Pass::ReadTogether, "read together"),
    (Pass::DeclaredFunctions, "declared functions"),
    (Pass::DeclaredModuleFunctions, "declared module functions"),
    (Pass::DoWhiles, "do whiles"),
];

impl Pass {
    /// The pass `name` names, if any.
    pub fn named(name: &str) -> Option<Pass> {
        PASSES.iter().find(|(_, n)| *n == name).map(|(pass, _)| *pass)
    }

    fn bit(self) -> u64 {
        1 << PASSES
            .iter()
            .position(|(pass, _)| *pass == self)
            .expect("each pass is listed")
    }
}

/// The passes a preparation leaves out, and whether it records which of
/// the others changed anything. By default, none left out, none recorded:
/// what a build prepares.
#[derive(Clone, Copy, Default)]
pub struct Passes {
    without: u64,
    recording: bool,
}

impl Passes {
    /// Every pass but `without`, recording which fired where `recording`.
    pub fn without(without: &[Pass], recording: bool) -> Passes {
        Passes {
            without: without.iter().fold(0, |bits, pass| bits | pass.bit()),
            recording,
        }
    }
}

/// The passes a preparation's rewrites changed something by, of those it
/// ran: none, where it recorded none.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Fired(u64);

impl Fired {
    /// Those of `self` and of `other`.
    pub fn and(self, other: Fired) -> Fired {
        Fired(self.0 | other.0)
    }

    /// Their names, in `PASSES`' order.
    pub fn names(self) -> Vec<&'static str> {
        (PASSES.iter())
            .filter(|(pass, _)| self.0 & pass.bit() != 0)
            .map(|(_, name)| *name)
            .collect()
    }
}

thread_local! {
    /// The passes of the preparation running, and which fired so far.
    static RUNNING: Cell<(Passes, Fired)> = Cell::new((Passes::default(), Fired::default()));
}

/// `prepare` run with `passes`, and which of them fired.
pub(super) fn with(passes: Passes, prepare: impl FnOnce()) -> Fired {
    let outer = RUNNING.replace((passes, Fired::default()));
    prepare();
    RUNNING.replace(outer).1
}

fn on(pass: Pass) -> bool {
    RUNNING.get().0.without & pass.bit() == 0
}

fn fire(pass: Pass) {
    let (passes, fired) = RUNNING.get();
    if passes.recording {
        RUNNING.set((passes, Fired(fired.0 | pass.bit())));
    }
}

fn recording() -> bool {
    RUNNING.get().0.recording
}

/// Whether `pass`'s rewrite is made, asked last of its conditions, where
/// it would be: it fired, where it's on.
pub(super) fn applies(pass: Pass) -> bool {
    let on = on(pass);
    if on {
        fire(pass);
    }
    on
}

/// `pass`, a rewrite of `target` in place, where it's on: it fired where it
/// changed it.
pub(super) fn run<T: Clone + PartialEq>(pass: Pass, target: &mut T, rewrite: impl FnOnce(&mut T)) {
    if !on(pass) {
        return;
    }
    if !recording() {
        return rewrite(target);
    }
    let before = target.clone();
    rewrite(target);
    if *target != before {
        fire(pass);
    }
}

/// `pass`, a rewrite of `target` as another, where it's on: it fired where
/// that's another.
pub(super) fn rewrite<T: PartialEq>(pass: Pass, target: &mut T, rewritten: impl FnOnce(&T) -> T) {
    if !on(pass) {
        return;
    }
    let new = rewritten(target);
    if new != *target {
        fire(pass);
    }
    *target = new;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Each pass is listed once, by a name of its own, and fits the set.
    #[test]
    fn each_pass_is_named_once() {
        assert!(PASSES.len() <= 64);
        for (i, (pass, name)) in PASSES.iter().enumerate() {
            assert_eq!(Pass::named(name), Some(*pass));
            assert!(PASSES[i + 1..].iter().all(|(other, n)| other != pass && n != name));
        }
    }

    // A pass left out isn't made, and only a rewrite made of one recorded
    // is said to have fired.
    #[test]
    fn a_pass_left_out_is_not_made() {
        let fired = with(Passes::without(&[Pass::Tests], true), || {
            assert!(!applies(Pass::Tests));
            assert!(applies(Pass::Methods));
            let mut n = 1;
            run(Pass::Counters, &mut n, |n| *n += 0);
            rewrite(Pass::DoWhiles, &mut n, |n| n + 1);
            assert_eq!(n, 2);
        });
        assert_eq!(fired.names(), ["methods", "do whiles"]);
        let unrecorded = with(Passes::default(), || assert!(applies(Pass::Tests)));
        assert_eq!(unrecorded, Fired::default());
    }
}
