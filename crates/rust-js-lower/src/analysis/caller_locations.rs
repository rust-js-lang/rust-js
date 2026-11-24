//! Which of the crate's functions take their caller's location (ADR 0427):
//! one that tracks its caller, `#[track_caller]` or its trait's, and reads
//! where it was called, `Location::caller()` in its own body, not a closure
//! in it, or passes it on, calling one that takes it, the crate's or a
//! library's, or raises a panic of its own, an `Assert`'s, overflow's or a
//! bound's, or std's that tracks its caller, `panic!`'s, `unwrap`'s, which
//! rustc places where it was called (#2350). Another that tracks its caller
//! raises nothing and takes none, so its callers give it none.
//! A trait's method is called through a dictionary, which knows no impl: if
//! one of its impls, or its default, takes one, its declaration does, and
//! each entry of it is given one, which it passes on where its impl takes it.

use crate::library::Foreign;
use crate::recognition::{location_of, tracks_caller};
use crate::std_types::location::{LocationOp, takes_location};
use crate::{Body, fn_def};
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::mir::TerminatorKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;

pub(super) fn caller_location_fns<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    foreign: &Foreign<'_, 'tcx>,
) -> IndexSet<DefId> {
    let mut taking: IndexSet<DefId> = IndexSet::default();
    // Each call a tracking function makes, of what it resolves to.
    let mut calls: Vec<(DefId, DefId)> = Vec::new();
    // Each trait method's declaration, and its default's and impls' bodies
    // that track their caller: if one takes its caller's location, each
    // entry of it does.
    let mut families: IndexMap<DefId, Vec<DefId>> = IndexMap::default();
    // The functions whose bodies are compiled here, whose taking one is
    // found as theirs is.
    let compiled: IndexSet<DefId> = all_bodies.iter().map(|body| body.def_id).collect();
    for body in all_bodies {
        let owner = body.def_id;
        if !tracks_caller(tcx, owner) {
            continue;
        }
        if let Some(declared) = tcx.trait_item_of(owner).or(tcx.trait_of_assoc(owner).map(|_| owner)) {
            families.entry(declared).or_default().push(owner);
        }
        let typing_env = ty::TypingEnv::post_analysis(tcx, owner);
        // What it calls, by its MIR, which a std body alone has (ADR 0561),
        // or else its THIR; and whether it raises a panic of its own, an
        // `Assert`'s, overflow's or a bound's, which rustc places where it
        // was called, as each of its panics (#2350).
        let mut called = Vec::new();
        match &body.mir {
            Some(mir) => {
                for block in mir.body.basic_blocks.iter() {
                    match &block.terminator().kind {
                        TerminatorKind::Assert { .. } => {
                            taking.insert(owner);
                        }
                        TerminatorKind::Call { func, .. } => called.extend(func.const_fn_def()),
                        _ => {}
                    }
                }
            }
            None => called.extend(body.thir.exprs.iter().filter_map(|expr| match expr.kind {
                ExprKind::ZstLiteral { .. } => fn_def(expr.ty),
                _ => None,
            })),
        }
        for (callee, args) in called {
            if location_of(tcx, callee) == Some(LocationOp::Caller) {
                taking.insert(owner);
                continue;
            }
            // What it resolves to, or through a dictionary, generic code's or
            // a `dyn`'s, its declaration.
            let resolved = match crate::items::resolve_instance_in(tcx, typing_env, callee, args) {
                Ok(Some(instance)) if !matches!(instance.def, ty::InstanceKind::Virtual(..)) => instance.def_id(),
                _ => callee,
            };
            // Std's that tracks its caller, `panic!`'s `panic_fmt`, `unwrap`,
            // a slice's `index`, raises its panic where this one was called:
            // it takes the location. Of the crate's or a library's, whether
            // it takes one is asked below.
            if tracks_caller(tcx, resolved) && !compiled.contains(&resolved) && foreign.item(resolved).is_none() {
                taking.insert(owner);
            }
            // A std method whose model raises at its caller, called through a
            // dictionary, `a / b` of a `T: Div`, whose impl may track its
            // caller: where this one was called, as rustc's instance of it
            // does (#2457).
            if crate::recognition::model_raises_at_caller(tcx, resolved) {
                taking.insert(owner);
            }
            calls.push((owner, resolved));
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for &(caller, callee) in &calls {
            if takes_location(tcx, &taking, foreign, callee) && taking.insert(caller) {
                changed = true;
            }
        }
        for (&declared, members) in &families {
            let takes =
                takes_location(tcx, &taking, foreign, declared) || members.iter().any(|member| taking.contains(member));
            if takes {
                changed |= taking.insert(declared);
            }
        }
    }
    taking
}
