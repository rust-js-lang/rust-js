//! What a `Vec`'s `push` shows of its length (ADR 1137, #1700): past it,
//! the `Vec` holds an item at least, until it's lent `&mut` again or
//! written, so its `len()` is at least one and `len() - 1` never wraps,
//! `properties[prev_length - 1]` of `prev_length = properties.len()` right
//! past a push. Of a `Vec` a local holds, or one a `&mut` local points to;
//! never of a local a raw pointer is made of anywhere in the body, which
//! may write it after.

use super::*;

impl<'tcx> Walk<'_, 'tcx> {
    /// What `statement` does to what's known pushed (`State::pushed`): a
    /// local written forgets it; a `&mut` of one, or a raw pointer, forgets
    /// what it lends, which may be written through it, and keeps what it
    /// lends (`State::lending`) for a `push` of it; and a shared reborrow of
    /// one pushed, `&(*_1)`, a copy of it, or a move, is one too: a `Vec`
    /// lent shared is written by nothing while the borrow lives.
    pub(super) fn pushes(&self, statement: &mir::Statement<'tcx>, state: &mut State) {
        let written = match &statement.kind {
            StatementKind::Assign(assign) => Some(assign.0.local),
            StatementKind::StorageLive(local) | StatementKind::StorageDead(local) => Some(*local),
            StatementKind::SetDiscriminant { place, .. } => Some(place.local),
            _ => None,
        };
        if let Some(local) = written {
            state.unpush(local);
        }
        let StatementKind::Assign(assign) = &statement.kind else {
            return;
        };
        let (place, rvalue) = &**assign;
        let whole = |of: &mir::Place<'tcx>| matches!(of.projection.as_slice(), [] | [mir::PlaceElem::Deref]);
        let lent = |state: &mut State, of: &mir::Place<'tcx>| {
            let root = state.lending.get(&of.local).copied().unwrap_or(of.local);
            state.pushed.shift_remove(&of.local);
            state.pushed.shift_remove(&root);
            root
        };
        match rvalue {
            Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of) => {
                let root = lent(state, of);
                if let Some(local) = place.as_local()
                    && whole(of)
                    && !self.pointed.contains(&root)
                {
                    state.lending.insert(local, root);
                }
            }
            Rvalue::RawPtr(_, of) => {
                lent(state, of);
            }
            Rvalue::Ref(_, _, of) | Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _)
                if whole(of) && state.pushed.contains(&of.local) =>
            {
                if let Some(local) = place.as_local() {
                    state.pushed.insert(local);
                }
            }
            _ => {}
        }
    }
}

impl State {
    /// `local` written: neither pushed, nor lending, nor lent.
    pub(super) fn unpush(&mut self, local: Local) {
        self.pushed.shift_remove(&local);
        self.lending.retain(|&lend, &mut of| lend != local && of != local);
    }
}
