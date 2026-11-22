//! The crate's closures given, as their one parameter, a thread-local a test
//! proved `Some` where they're made, and that never changes (#1933 step 4
//! (G)): React DOM's `localRequestAnimationFrame`, made only where
//! `SCHEDULE_TIMEOUT.with(Option::is_some)` held, whose
//! `SCHEDULE_TIMEOUT.with(|s| s.unwrap())` reads it `Some` whenever it
//! runs, so `scheduleTimeout(..)`, no `$unwrap`.
//!
//! Sound only within these limits (the lead, for the user, 2026-11-16):
//! the thread-local's value is its initializer's alone, every read seeing
//! it, as its type has no interior mutability (`is_freeze`, rustc's
//! answer): no `Cell` or `RefCell` in it, which a `set` changes; a
//! `static mut`, which any code writes, is no thread-local; and the closure
//! reads it through `with`'s `&` parameter, of `KEY.with(closure)`, whose
//! test's edge every way to where the closure, or the closure it's made in,
//! is made takes. One program has one thread (`thread::spawn` is refused),
//! so the value tested is the value read.

use crate::Body;
use crate::analysis::Mir;
use crate::fn_def;
use crate::recognition::{Recognition, is_local_with, option_presence_test};
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::mir::visit::Visitor;
use rustc_middle::mir::{self, BasicBlock, Location, Operand, Rvalue, TerminatorKind};
use rustc_middle::ty;
use rustc_span::def_id::DefId;

/// The closures of `bodies`, the crate's every body and initializer, each
/// given by a `with` a thread-local that's `Some` wherever it runs.
pub(super) fn found<'tcx>(recognition: &Recognition<'_, 'tcx>, bodies: &[&Body<'tcx>]) -> IndexSet<DefId> {
    let by_id: IndexMap<DefId, &Mir<'tcx>> = (bodies.iter())
        .filter_map(|body| Some((body.def_id, body.mir.as_ref()?)))
        .collect();
    // Where each closure is made: the body and block, of each body that
    // makes it.
    let mut made: IndexMap<DefId, Vec<(DefId, BasicBlock)>> = IndexMap::default();
    for (&id, mir) in &by_id {
        let mut making = Making(Vec::new());
        making.visit_body(&mir.body);
        for (closure, block) in making.0 {
            made.entry(closure).or_default().push((id, block));
        }
    }
    let scan = Scan {
        recognition,
        by_id: &by_id,
        made: &made,
    };
    let mut given = IndexSet::default();
    for (&id, mir) in &by_id {
        for (block, data) in mir.body.basic_blocks.iter_enumerated() {
            let TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
                continue;
            };
            let Some((callee, _)) = fn_def(func.ty(&mir.body, recognition.tcx)) else {
                continue;
            };
            let [key, f] = args.as_ref() else { continue };
            if !is_local_with(recognition.tcx, callee) {
                continue;
            }
            let ty::Closure(closure, _) = *f.node.ty(&mir.body, recognition.tcx).kind() else {
                continue;
            };
            let Some(key) = scan.key_of(mir, &key.node) else {
                continue;
            };
            if scan.present_at(id, block, 0).contains(&key) {
                given.insert(closure);
            }
        }
    }
    given
}

/// Each closure a body makes, and the block: an aggregate of its captures,
/// or the constant of one that captures nothing.
struct Making(Vec<(DefId, BasicBlock)>);

impl<'tcx> Visitor<'tcx> for Making {
    fn visit_rvalue(&mut self, rvalue: &Rvalue<'tcx>, location: Location) {
        if let Rvalue::Aggregate(kind, _) = rvalue
            && let mir::AggregateKind::Closure(closure, _) = **kind
        {
            self.0.push((closure, location.block));
        }
        self.super_rvalue(rvalue, location);
    }

    fn visit_operand(&mut self, operand: &Operand<'tcx>, location: Location) {
        if let Operand::Constant(c) = operand
            && let ty::Closure(closure, _) = *c.ty().kind()
        {
            self.0.push((closure, location.block));
        }
        self.super_operand(operand, location);
    }
}

struct Scan<'s, 'tcx> {
    recognition: &'s Recognition<'s, 'tcx>,
    by_id: &'s IndexMap<DefId, &'s Mir<'tcx>>,
    made: &'s IndexMap<DefId, Vec<(DefId, BasicBlock)>>,
}

impl<'tcx> Scan<'_, 'tcx> {
    /// The thread-locals `Some` wherever `block` of `id`'s body runs: each
    /// that a test of its body proves there, and, of a closure's body, each
    /// that's so where every body that makes it makes it. `depth` bounds
    /// the closures followed out.
    fn present_at(&self, id: DefId, block: BasicBlock, depth: usize) -> IndexSet<DefId> {
        let Some(mir) = self.by_id.get(&id) else {
            return IndexSet::default();
        };
        let mut present = self.tested_at(mir, block);
        if depth < 8
            && self.recognition.tcx.is_closure_like(id)
            && let Some(makers) = self.made.get(&id)
        {
            let mut outer = makers.iter().map(|&(maker, at)| self.present_at(maker, at, depth + 1));
            if let Some(first) = outer.next() {
                let common = outer.fold(first, |kept, other| kept.intersection(&other).copied().collect());
                present.extend(common);
            }
        }
        present
    }

    /// The thread-locals of `mir` a test proves `Some` where `block` runs:
    /// a switch on `KEY.with(Option::is_some)`, its true edge, or on
    /// `KEY.with(Option::is_none)`, its false one, entered from the switch
    /// alone and on every way to `block`; of one whose value never changes
    /// (`unchanging`).
    fn tested_at(&self, mir: &Mir<'tcx>, block: BasicBlock) -> IndexSet<DefId> {
        let tcx = self.recognition.tcx;
        let blocks = &mir.body.basic_blocks;
        let dominators = blocks.dominators();
        let mut present = IndexSet::default();
        for data in blocks.iter() {
            let TerminatorKind::SwitchInt { discr, targets } = &data.terminator().kind else {
                continue;
            };
            let Some(flag) = discr.place().filter(|p| p.projection.is_empty()) else {
                continue;
            };
            let [(0, no)] = targets.iter().collect::<Vec<_>>()[..] else {
                continue;
            };
            // The flag's one write, the call of `with` that gives it.
            let mut writes = (blocks.iter()).filter(|b| {
                b.statements.iter().any(|s| matches!(&s.kind, mir::StatementKind::Assign(a) if a.0.local == flag.local))
                    || matches!(&b.terminator().kind, TerminatorKind::Call { destination, .. } if destination.local == flag.local)
            });
            let (Some(write), None) = (writes.next(), writes.next()) else {
                continue;
            };
            let TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &write.terminator().kind
            else {
                continue;
            };
            let Some((callee, _)) = fn_def(func.ty(&mir.body, tcx)) else {
                continue;
            };
            let [key, test] = args.as_ref() else { continue };
            if !destination.projection.is_empty() || !is_local_with(tcx, callee) {
                continue;
            }
            let Some((test, _)) = fn_def(test.node.ty(&mir.body, tcx)) else {
                continue;
            };
            let edge = match option_presence_test(tcx, test) {
                Some(true) => targets.otherwise(),
                Some(false) => no,
                None => continue,
            };
            if blocks.predecessors()[edge].len() == 1
                && dominators.dominates(edge, block)
                && let Some(key) = self.key_of(mir, &key.node)
            {
                present.insert(key);
            }
        }
        present
    }

    /// The thread-local `operand`, a `with`'s key, refers to, where its
    /// value never changes: its type has no interior mutability, which
    /// rustc's `is_freeze` answers, so nothing sets it once its
    /// initializer has.
    fn key_of(&self, mir: &Mir<'tcx>, operand: &Operand<'tcx>) -> Option<DefId> {
        let assigned = crate::body_queries::assigned_once(&mir.body);
        let key = crate::analysis::mir_reads::referred::referred_key(
            &mir.promoted,
            |local| assigned.get(&local).copied().flatten(),
            operand,
        )?;
        let held = crate::bindings::thread_local_value(self.recognition.tcx, key)?;
        held.is_freeze(self.recognition.tcx, self.recognition.typing_env)
            .then_some(key)
    }
}
