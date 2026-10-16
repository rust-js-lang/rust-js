//! Which of the crate's functions take their caller's location (ADR 0427):
//! one that tracks its caller, `#[track_caller]` or its trait's, and reads
//! where it was called, `Location::caller()` in its own body, not a closure
//! in it, or passes it on, calling one that takes it, the crate's or a
//! library's. Another that tracks its caller takes none: nothing it does
//! reads it, so its callers give it none, `v[i]` of an `Index` among them.

use crate::lower::library::Foreign;
use crate::lower::recognition::{location_of, tracks_caller};
use crate::lower::std_types::location::LocationOp;
use crate::lower::{Body, fn_def};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::HashSet;

pub(super) fn caller_location_fns<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    foreign: &Foreign<'_, 'tcx>,
) -> HashSet<DefId> {
    let mut taking: HashSet<DefId> = HashSet::new();
    // Each call a tracking function makes, of what it resolves to.
    let mut calls: Vec<(DefId, DefId)> = Vec::new();
    for body in all_bodies {
        let owner = body.def_id.to_def_id();
        if !tracks_caller(tcx, owner) {
            continue;
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
            if let Ok(Some(instance)) = crate::lower::items::resolve_instance_in(tcx, typing_env, callee, args)
                && !matches!(instance.def, ty::InstanceKind::Virtual(..))
            {
                calls.push((owner, instance.def_id()));
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
    }
    taking
}
