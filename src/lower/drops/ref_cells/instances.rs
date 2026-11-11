//! The `RefCell` borrows of a body the crate compiles from another crate's
//! MIR, a library's function a consumer gives more (ADR 0914) or std's
//! (ADR 0561): it has no THIR to say a guard is held only while nothing can
//! ask of its cell, so each borrow it makes is counted (#1325).

use super::Found;
use crate::lower::Std;
use crate::lower::recognition::{Recognition, StdItem};
use rustc_middle::mir::{self, TerminatorKind};

/// Each borrow `body`'s MIR makes of a `RefCell`, counted: a borrow's, a
/// `try_borrow`'s, a `replace_with`'s, held while its function runs, and a
/// `swap`'s of two that may be one, as `counted_in` has a body's THIR's
/// where nothing says they're momentary.
pub(super) fn mir_borrows<'tcx>(recognition: &Recognition<'_, 'tcx>, body: &mir::Body<'tcx>) -> Vec<Found<'tcx>> {
    let tcx = recognition.tcx;
    let mut found = Vec::new();
    for data in body.basic_blocks.iter() {
        let TerminatorKind::Call {
            func, args, fn_span, ..
        } = &data.terminator().kind
        else {
            continue;
        };
        let Some((def_id, generic_args)) = func.const_fn_def() else {
            continue;
        };
        let held = matches!(
            recognition.classify(def_id, generic_args),
            Some(
                Std::Borrow { lock: false, .. }
                    | Std::TryBorrow { lock: false, .. }
                    | Std::CellReplaceWith
                    | Std::CellSwap
            )
        );
        let Some(cell) = args.first().map(|arg| arg.node.ty(body, tcx).peel_refs()) else {
            continue;
        };
        if held && recognition.is_std_type(cell, StdItem::RefCell) {
            found.push(Found {
                cell: tcx.erase_and_anonymize_regions(cell),
                callee: Some(def_id),
                key: None,
                fields: Vec::new(),
                counted: true,
                // Its library's own build checked what its code can't see.
                seen: true,
                span: *fn_span,
            });
        }
    }
    found
}
