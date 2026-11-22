//! What the MIR intervals keep through a statement or a terminator: what
//! the forgetting rules keep, and, where the session asks the effects
//! query too (`--intervals-by-effects`, #2112 B step 3a), nothing of a
//! local it says may be written there (`analysis::Effects::stale`). Step 3b
//! makes the query the transfer's alone and deletes the rules.

use super::*;
use crate::analysis::Written;

impl<'tcx> Walk<'_, 'tcx> {
    /// `statement`'s transfer: the rules', then, of the query, each local it
    /// may write forgotten but the one it assigns, which the rules told
    /// again. Not of a local dying or living, which is read again only once
    /// it's assigned.
    pub(super) fn statement(&self, statement: &mir::Statement<'tcx>, state: &mut State) {
        self.statement_rules(statement, state);
        if matches!(
            statement.kind,
            StatementKind::StorageDead(_) | StatementKind::StorageLive(_)
        ) {
            return;
        }
        let Some(effects) = self.by_effects() else {
            return;
        };
        let Some(&location) = self.located.get(&std::ptr::from_ref(statement).addr()) else {
            return;
        };
        let assigned = match &statement.kind {
            StatementKind::Assign(assign) => assign.0.as_local(),
            _ => None,
        };
        forget_stale(effects.stale(location), assigned, state);
    }

    /// `block`'s terminator's transfer: the rules' each way out, then, of
    /// the query, at a call, a drop, a yield or inline assembly, each local
    /// it may write forgotten but a call's destination, which the rules told
    /// again.
    pub(super) fn edges(&self, block: BasicBlock, state: &State) -> Vec<(BasicBlock, State)> {
        let mut outs = self.edges_rules(block, state);
        let Some(effects) = self.by_effects() else {
            return outs;
        };
        let terminator = self.body.basic_blocks[block].terminator();
        let assigned = match &terminator.kind {
            TerminatorKind::Call { destination, .. } => destination.as_local(),
            TerminatorKind::Drop { .. } | TerminatorKind::InlineAsm { .. } | TerminatorKind::Yield { .. } => None,
            _ => return outs,
        };
        let stale = effects.stale(self.body.terminator_loc(block));
        for (_, out) in &mut outs {
            forget_stale(stale.clone(), assigned, out);
        }
        outs
    }

    /// The effects query, where the session asks it of the transfer.
    fn by_effects(&self) -> Option<&crate::analysis::Effects> {
        (self.effects.as_ref()).filter(|_| crate::session::caches(self.tcx).intervals_by_effects)
    }
}

/// What the query says may be written of each of `stale` but `assigned`,
/// forgotten of `state` (`State::written`).
pub(super) fn forget_stale(stale: IndexMap<Local, Written>, assigned: Option<Local>, state: &mut State) {
    for (local, written) in stale {
        if Some(local) != assigned {
            state.written(local, written);
        }
    }
}

impl State {
    /// `local` written, as `written` says: replaced, nothing of it known;
    /// grown, a `push`, what's known of its length but that it's at least
    /// as long, `x < len` and its least holding; its items written, of a
    /// table, how long its rows are.
    pub(super) fn written(&mut self, local: Local, written: Written) {
        match written {
            Written::Whole => self.forget(local),
            Written::Grows => {
                self.lens.retain(|_, &of| of != local);
                self.empties.retain(|_, &of| of != local);
                self.equal_lens.retain(|&(a, b)| a != local && b != local);
                self.rows.retain(|&table, _| table != local);
            }
            Written::Items => self.rows.retain(|&table, _| table != local),
        }
    }
}
