//! Which of the crate's functions are throwing functions (ADR 0644): one
//! whose result is a throwing JS call's `Result` as it is, each return of
//! it, which its JS returns as the call's value, its throw going on; and
//! whether a call is a throwing call, of a binding or of such a function,
//! the one answer every path asks.

use crate::lower::bindings::is_binding;
use crate::lower::recognition::{Catching, Recognition, StdItem};
use crate::lower::{Body, fn_def};
use rustc_hir::def::DefKind;
use rustc_middle::mir::{self, RETURN_PLACE, StatementKind, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::HashSet;

/// How a call of `id`, given `args`, delivers its result: a binding's by
/// the result it's declared with (ADR 0588), a throwing function's as a
/// `Result` of what it threw.
pub(in crate::lower) fn call_throws<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    throwing: &HashSet<DefId>,
    id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Catching {
    if is_binding(recognition.tcx, id) {
        recognition.catching(id, args)
    } else if throwing.contains(&id) {
        Catching::Result
    } else {
        Catching::Direct
    }
}

/// The crate's throwing functions: each whose every write of its result is
/// a throwing call's, a binding's or another's of these, given its result
/// as it is. Not one JS may call, a `pub` one, whose JS callers read its
/// `Result`; nor a trait's method, called through a dictionary; nor a
/// generic one, nor a closure. Found from the least: a function that
/// returns its own call is none, its `$try` kept.
pub(super) fn throwing_fns<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: &Recognition<'_, 'tcx>,
    bodies: &[&Body<'tcx>],
) -> HashSet<DefId> {
    let candidates: Vec<(DefId, &mir::Body<'tcx>)> = (bodies.iter())
        .filter(|body| candidate(tcx, recognition, body.def_id))
        .filter_map(|body| Some((body.def_id, &body.mir.as_ref()?.body)))
        .collect();
    let mut throwing = HashSet::new();
    loop {
        let found: Vec<DefId> = (candidates.iter())
            .filter(|(id, mir)| !throwing.contains(id) && returns_throws(tcx, recognition, &throwing, mir))
            .map(|&(id, _)| id)
            .collect();
        if found.is_empty() {
            return throwing;
        }
        throwing.extend(found);
    }
}

/// Whether `id` may be a throwing function: the crate's own, of no type
/// parameters, no trait's, none JS may call, giving a `Result`.
fn candidate<'tcx>(tcx: TyCtxt<'tcx>, recognition: &Recognition<'_, 'tcx>, id: DefId) -> bool {
    let Some(local) = id.as_local() else {
        return false;
    };
    matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn)
        && !is_binding(tcx, id)
        && tcx.trait_of_assoc(id).is_none()
        && tcx.trait_impl_of_assoc(id).is_none()
        && !tcx.visibility(local).is_public()
        && !tcx.generics_of(id).requires_monomorphization(tcx)
        && recognition.is_std_type(
            tcx.fn_sig(id)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .output(),
            StdItem::Result,
        )
}

/// Whether each write of `mir`'s result is a throwing call's, into it as
/// it is.
fn returns_throws<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: &Recognition<'_, 'tcx>,
    throwing: &HashSet<DefId>,
    mir: &mir::Body<'tcx>,
) -> bool {
    let mut calls = 0;
    for block in mir.basic_blocks.iter() {
        let written = |place: &mir::Place<'tcx>| place.local == RETURN_PLACE;
        if (block.statements.iter()).any(|s| matches!(&s.kind, StatementKind::Assign(assign) if written(&assign.0))) {
            return false;
        }
        let Some(terminator) = &block.terminator else { continue };
        if let TerminatorKind::Call { func, destination, .. } = &terminator.kind
            && written(destination)
        {
            let thrown = destination.projection.is_empty()
                && fn_def(func.ty(mir, tcx))
                    .is_some_and(|(id, args)| matches!(call_throws(recognition, throwing, id, args), Catching::Result));
            if !thrown {
                return false;
            }
            calls += 1;
        }
    }
    calls > 0
}
