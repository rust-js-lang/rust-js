//! `.await` from MIR (ADRs 0029, 0364). Its desugaring is a call of
//! `IntoFuture::into_future`, then a loop that polls what it made and
//! yields until it's `Ready`, whose value is read:
//!
//! ```text
//! _f = into_future(move x) -> bb1
//! bb1: _a = move _f; loop { _r = poll(pin(&mut _a), cx); if Ready { break } yield }
//! bbReady: _y = move ((_r as Ready).0); ..
//! ```
//!
//! In JS that's one `await x`: the call gives `_y` and goes on to the
//! `Ready` block, and the loop, which nothing reaches then, goes.

use std::collections::HashSet;

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::mir::{
    self, BasicBlock, Operand, Place, PlaceElem, Rvalue, StatementKind, TerminatorKind, UnwindAction,
};
use rustc_middle::ty::{self, TyCtxt};

use super::cfg::successors;

/// Each `.await` in `body` made one call, `into_future`'s, that gives what's
/// awaited: the blocks of those calls.
pub(in crate::lower) fn awaits<'tcx>(tcx: TyCtxt<'tcx>, body: &mut mir::Body<'tcx>) -> HashSet<BasicBlock> {
    let mut awaits = HashSet::new();
    for block in body.basic_blocks.indices() {
        let TerminatorKind::Call {
            func,
            target: Some(after),
            ..
        } = &body.basic_blocks[block].terminator().kind
        else {
            continue;
        };
        let into_future = matches!(*func.ty(&body.local_decls, tcx).kind(),
            ty::FnDef(def_id, _) if tcx.is_lang_item(def_id, LangItem::IntoFutureIntoFuture));
        if !into_future {
            continue;
        }
        let Some((loop_blocks, ready)) = polled(tcx, body, *after) else {
            continue;
        };
        // What's awaited: what `Ready` holds, where it's read, or nothing.
        let read = body.basic_blocks[ready].statements.iter().position(|s| {
            matches!(&s.kind, StatementKind::Assign(assign)
                if matches!(&assign.1, Rvalue::Use(Operand::Copy(p) | Operand::Move(p), _)
                    if matches!(p.projection.as_slice(), [PlaceElem::Downcast(..), PlaceElem::Field(..)])))
        });
        let destination = match read {
            Some(i) => {
                let statement = &mut body.basic_blocks_mut()[ready].statements[i];
                let StatementKind::Assign(assign) = std::mem::replace(&mut statement.kind, StatementKind::Nop) else {
                    unreachable!("matched")
                };
                assign.0
            }
            None => {
                let span = body.basic_blocks[block].terminator().source_info.span;
                Place::from(body.local_decls.push(mir::LocalDecl::new(tcx.types.unit, span)))
            }
        };
        if let TerminatorKind::Call {
            destination: given,
            target,
            ..
        } = &mut body.basic_blocks_mut()[block].terminator_mut().kind
        {
            *given = destination;
            *target = Some(ready);
        }
        for dead in loop_blocks {
            let data = &mut body.basic_blocks_mut()[dead];
            data.statements.clear();
            data.terminator_mut().kind = TerminatorKind::Unreachable;
        }
        awaits.insert(block);
    }
    awaits
}

/// From `start`, the loop that polls what `into_future` made: its blocks,
/// and the block its `Ready` goes to, past the switch on what `poll` gave.
fn polled<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, start: BasicBlock) -> Option<(Vec<BasicBlock>, BasicBlock)> {
    let mut seen = vec![start];
    let mut at = 0;
    while let Some(&block) = seen.get(at) {
        at += 1;
        let data = &body.basic_blocks[block];
        // The switch on what `poll` gave: its `Ready`, variant 0, goes on.
        if let TerminatorKind::SwitchInt { discr, targets } = &data.terminator().kind
            && let Operand::Move(tested) | Operand::Copy(tested) = discr
            && let Some(polled) = data.statements.iter().rev().find_map(|s| match &s.kind {
                StatementKind::Assign(assign) if assign.0 == *tested => match &assign.1 {
                    Rvalue::Discriminant(polled) => Some(*polled),
                    _ => None,
                },
                _ => None,
            })
            && is_poll_of(tcx, body, &seen, polled)
            && let Some(mut ready) = targets
                .iter()
                .find_map(|(value, target)| (value == 0).then_some(target))
        {
            while let TerminatorKind::FalseEdge { real_target, .. } = body.basic_blocks[ready].terminator().kind
                && body.basic_blocks[ready].statements.is_empty()
            {
                seen.push(ready);
                ready = real_target;
            }
            return Some((seen, ready));
        }
        for next in successors(body, block) {
            if !seen.contains(&next) {
                seen.push(next);
            }
        }
        if seen.len() > 64 {
            return None;
        }
    }
    None
}

/// Whether `polled` is what a call of `Future::poll` among `blocks` gave.
fn is_poll_of<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, blocks: &[BasicBlock], polled: Place<'tcx>) -> bool {
    blocks.iter().any(|&b| {
        matches!(&body.basic_blocks[b].terminator().kind, TerminatorKind::Call { func, destination, unwind, .. }
            if *destination == polled
                && !matches!(unwind, UnwindAction::Terminate(_))
                && matches!(*func.ty(&body.local_decls, tcx).kind(),
                    ty::FnDef(def_id, _) if tcx.is_lang_item(def_id, LangItem::FuturePoll)))
    })
}
