//! What a closure is told where it's made (rd423): what's known there of
//! what it captures that nothing changes while it lives, so its body's
//! walk starts from it, `OBJECT_IS.with(|is| is(&next[i], &prev[i]))` of
//! an `i` below both lengths reading each in bounds.
//!
//! Rust's borrows say what's unchanged: a variable it captures by shared
//! reference nothing writes while the closure lives, nor does the closure,
//! and one it takes by value it has as it was, where it never writes it,
//! which rustc's capture says. A slice in a `RefCell` any code may change
//! (ADR 0881), and a `bool`'s test, a cell still empty and what reaches
//! cells are of the body's own code, so none is told.

use super::known::{CELL, Known, either};
use super::{Slice, Walk, Walked};
use rustc_middle::thir::{ExprKind, LocalVarId, Thir};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::HashSet;

/// Each closure a body of `bodies` makes that's walked here: told what's
/// known where it's made, walked from that alone.
pub(super) fn told_closures(bodies: &[Walked<'_, '_>]) -> HashSet<DefId> {
    let walked: HashSet<DefId> = bodies.iter().map(|&(def_id, ..)| def_id).collect();
    (bodies.iter())
        .flat_map(|&(_, thir, _)| made(thir))
        .filter(|closure| walked.contains(closure))
        .collect()
}

impl Walk<'_, '_> {
    /// `closure`, made where `known` is, is told what it captures that
    /// nothing changes while it lives, joined with what it's told where it's
    /// made again; made where nothing goes on, nothing.
    pub(super) fn tell_closure(&mut self, closure: DefId, known: &Option<Known>) {
        if let Some(k) = known
            && self.closures.contains(&closure)
        {
            let told = told(self.tcx, closure, k);
            let entry = self.told.entry(closure).or_insert(None);
            *entry = either(entry.take(), Some(told));
        }
    }
}

/// Each closure a body makes, not a coroutine's: where it's made is the
/// one place it's told from.
pub(super) fn made<'a>(thir: &'a Thir<'_>) -> impl Iterator<Item = DefId> + 'a {
    (thir.exprs.iter()).filter_map(|e| match &e.kind {
        ExprKind::Closure(closure) if matches!(closure.args, ty::UpvarArgs::Closure(_)) => {
            Some(closure.closure_id.to_def_id())
        }
        _ => None,
    })
}

/// What `closure`, made where `known` is, is told: each integer it
/// captures unchanged and its bounds by what it captures unchanged, and
/// each such slice's least length and what it's as long as.
fn told(tcx: TyCtxt<'_>, closure: DefId, known: &Known) -> Known {
    let unchanged = unchanged(tcx, closure);
    let held = |(path, var): &Slice| unchanged.contains(var) && !path.contains(&CELL);
    let mut told = Known::default();
    for (var, range) in &known.vars {
        if unchanged.contains(var) {
            let mut range = range.clone();
            range.below.retain(|(s, _)| held(s));
            told.vars.insert(*var, range);
        }
    }
    told.lengths = (known.lengths.iter())
        .filter(|(s, _)| held(s))
        .map(|(s, &least)| (s.clone(), least))
        .collect();
    told.same = (known.same.iter())
        .filter(|(s, base)| held(s) && held(base))
        .map(|(s, base)| (s.clone(), base.clone()))
        .collect();
    told
}

/// The variables `closure` captures that nothing changes while it lives:
/// each of whose places it captures by shared reference, or by value
/// without writing it.
fn unchanged(tcx: TyCtxt<'_>, closure: DefId) -> HashSet<LocalVarId> {
    let Some(local) = closure.as_local() else {
        return HashSet::new();
    };
    let captures = tcx.closure_captures(local);
    let kept = |captured: &&&ty::CapturedPlace<'_>| match captured.info.capture_kind {
        ty::UpvarCapture::ByRef(kind) => kind == ty::BorrowKind::Immutable,
        ty::UpvarCapture::ByValue => captured.mutability == ty::Mutability::Not,
        ty::UpvarCapture::ByUse => false,
    };
    let all: HashSet<LocalVarId> = (captures.iter()).map(|c| LocalVarId(c.get_root_variable())).collect();
    let changed: HashSet<LocalVarId> = (captures.iter())
        .filter(|c| !kept(c))
        .map(|c| LocalVarId(c.get_root_variable()))
        .collect();
    all.difference(&changed).copied().collect()
}
