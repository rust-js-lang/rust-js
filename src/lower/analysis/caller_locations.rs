//! Which of the crate's functions take their caller's location (ADR 0427):
//! one that tracks its caller, `#[track_caller]` or its trait's, and reads
//! where it was called, `Location::caller()` in its own body, not a closure
//! in it, or passes it on, calling one that takes it, the crate's or a
//! library's. Another that tracks its caller takes none: nothing it does
//! reads it, so its callers give it none, `v[i]` of an `Index` among them.
//! A trait's method is called through a dictionary, which knows no impl: if
//! one of its impls, or its default, takes one, its declaration does, and
//! each entry of it is given one, which it passes on where its impl takes it.

use crate::lower::library::Foreign;
use crate::lower::recognition::{location_of, tracks_caller};
use crate::lower::std_types::location::LocationOp;
use crate::lower::{Body, fn_def};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

pub(super) fn caller_location_fns<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    foreign: &Foreign<'_, 'tcx>,
) -> HashSet<DefId> {
    let mut taking: HashSet<DefId> = HashSet::new();
    // Each call a tracking function makes, of what it resolves to.
    let mut calls: Vec<(DefId, DefId)> = Vec::new();
    // Each trait method's declaration, and its default's and impls' bodies
    // that track their caller: if one takes its caller's location, each
    // entry of it does.
    let mut families: HashMap<DefId, Vec<DefId>> = HashMap::new();
    for body in all_bodies {
        let owner = body.def_id.to_def_id();
        if !tracks_caller(tcx, owner) {
            continue;
        }
        if let Some(declared) = tcx.trait_item_of(owner).or(tcx.trait_of_assoc(owner).map(|_| owner)) {
            families.entry(declared).or_default().push(owner);
        }
        let typing_env = ty::TypingEnv::post_analysis(tcx, owner);
        for expr in body.thir.exprs.iter() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            if location_of(tcx, callee) == Some(LocationOp::Caller) {
                taking.insert(owner);
                continue;
            }
            // What it resolves to, or through a dictionary, generic code's or
            // a `dyn`'s, its declaration.
            match crate::lower::items::resolve_instance_in(tcx, typing_env, callee, args) {
                Ok(Some(instance)) if !matches!(instance.def, ty::InstanceKind::Virtual(..)) => {
                    calls.push((owner, instance.def_id()));
                }
                _ => calls.push((owner, callee)),
            }
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for &(caller, callee) in &calls {
            if (taking.contains(&callee) || foreign.takes_location(callee)) && taking.insert(caller) {
                changed = true;
            }
        }
        for (&declared, members) in &families {
            let takes = taking.contains(&declared)
                || foreign.takes_location(declared)
                || members.iter().any(|member| taking.contains(member));
            if takes {
                changed |= taking.insert(declared);
            }
        }
    }
    taking
}
