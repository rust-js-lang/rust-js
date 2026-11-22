//! The drops of a body that drop nothing, by rustc's dataflow of what's
//! there before each (#717), and the body and dataflow `mir::drops`' own
//! elaboration reads: the base's since #2299 moved them from `mir::drops`.
use rust_js_program::collections::IndexSet;
use rustc_middle::mir::{self, BasicBlock, TerminatorKind, UnwindAction};
use rustc_middle::ty::{self, Ty};
use rustc_mir_dataflow::impls::MaybeInitializedPlaces;
use rustc_mir_dataflow::move_paths::{LookupResult, MoveData};
use rustc_mir_dataflow::{Analysis, on_all_children_bits};

/// The blocks whose `Drop` would unwind to a cleanup but drops what JS
/// drops nothing of, so runs nothing and can't panic.
pub fn quiet_drops<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    drops: &impl Fn(Ty<'tcx>) -> bool,
) -> IndexSet<BasicBlock> {
    (body.basic_blocks.iter_enumerated())
        .filter(|(_, data)| {
            matches!(&data.terminator().kind, TerminatorKind::Drop { place, unwind: UnwindAction::Cleanup(_), .. }
                if !drops(place.ty(&body.local_decls, tcx).ty))
        })
        .map(|(block, _)| block)
        .collect()
}

/// rustc's dataflow of what `flow` may have there before each location, of
/// `move_data`'s paths, as its `ElaborateDrops` asks it.
pub fn maybe_inits<'a, 'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    flow: &'a mir::Body<'tcx>,
    move_data: &'a MoveData<'tcx>,
) -> rustc_mir_dataflow::ResultsCursor<'a, 'tcx, MaybeInitializedPlaces<'a, 'tcx>> {
    MaybeInitializedPlaces::new(tcx, flow, move_data)
        .exclude_inactive_in_otherwise()
        .skipping_unreachable_unwind()
        .iterate_to_fixpoint(tcx, flow, None)
        .into_results_cursor(flow)
}

/// The blocks of the crate's `body` whose `Drop` drops nothing: of what JS
/// drops nothing of, `drops` false, or of what's surely moved out by then,
/// a temporary given on, `Style::Dead`'s, as rustc's elaboration drops
/// neither. So it runs nothing and can't panic, which `never_unwinding`
/// asks of each drop it meets (#717). The one answer of both, the
/// dataflow `elaborated` reads.
pub(in crate::lower) fn dead_drops<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    drops: &impl Fn(Ty<'tcx>) -> bool,
) -> IndexSet<BasicBlock> {
    let quiet: IndexSet<BasicBlock> = (body.basic_blocks.iter_enumerated())
        .filter(|(_, data)| {
            matches!(&data.terminator().kind, TerminatorKind::Drop { place, .. }
                if !drops(place.ty(&body.local_decls, tcx).ty))
        })
        .map(|(block, _)| block)
        .collect();
    // Of no other `Drop`, no dataflow to run; nor of a body whose locals'
    // facts aren't the crate's, a `const fn`'s read for a pattern, whose
    // others are taken to drop what's there.
    let others = (body.basic_blocks.iter_enumerated())
        .any(|(block, data)| matches!(data.terminator().kind, TerminatorKind::Drop { .. }) && !quiet.contains(&block));
    let local = (body.local_decls.iter()).all(|decl| matches!(decl.local_info, mir::ClearCrossCrate::Set(_)));
    if !others || !local {
        return quiet;
    }
    let move_data = MoveData::gather_moves(body, tcx, drops);
    let unwinding = quiet_drops(tcx, body, drops);
    let flow = &flowing(body, &unwinding);
    let mut inits = maybe_inits(tcx, flow, &move_data);
    let mut dead = quiet;
    for (block, data) in body.basic_blocks.iter_enumerated() {
        let TerminatorKind::Drop { place, .. } = &data.terminator().kind else {
            continue;
        };
        let LookupResult::Exact(path) = move_data.rev_lookup.find(place.as_ref()) else {
            continue;
        };
        inits.seek_before_primary_effect(body.terminator_loc(block));
        let mut there = false;
        on_all_children_bits(&move_data, path, |child| there |= inits.get().contains(child));
        if !there {
            dead.insert(block);
        }
    }
    dead
}

/// `body` as rustc's `ElaborateDrops` reads it: no unwind from the `Drop`
/// ending each of `quiet`, and none of the edges borrowck alone follows, a
/// `match`'s `falseEdge` to the arm it didn't take and a loop's
/// `falseUnwind`, which `CleanupPostBorrowck` makes the `goto`s they are
/// (#2112's change D). Followed, they asked a flag of what's always there,
/// or never: never taken, they made no other drop, but a flag more.
pub fn flowing<'tcx>(body: &mir::Body<'tcx>, quiet: &IndexSet<BasicBlock>) -> mir::Body<'tcx> {
    let mut body = body.clone();
    for (block, data) in body.basic_blocks.as_mut().iter_enumerated_mut() {
        let kind = &mut data.terminator_mut().kind;
        match kind {
            TerminatorKind::Drop { unwind, .. } if quiet.contains(&block) => *unwind = UnwindAction::Unreachable,
            TerminatorKind::FalseEdge { real_target, .. } | TerminatorKind::FalseUnwind { real_target, .. } => {
                *kind = TerminatorKind::Goto { target: *real_target };
            }
            _ => {}
        }
    }
    body
}
