//! An `Option` a closure captures by a shared reference, tested and read
//! in the closure: `run(&|| if value.is_some() { value.unwrap_or(d) } else
//! { .. })`, `value` (#1933's removal). What a shared reference refers to
//! can't change while it's lent: an `Option` holds no cell of its own, so
//! nothing the closure runs, nor its maker, which lent it, writes it. Split
//! by concept from `nulled`; `mir_operand` asks it, beside
//! `proved_present`.

use super::*;

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// Whether `place`, read in the closure lowered now, is what its
    /// capture `k`, a shared reference to an `Option`, refers to,
    /// `*((*_1).k)`, or `*_7` of `_7 = copy ((*_1).k)`, where a test of the
    /// same capture proved it: `is_some()`'s true edge or `is_none()`'s
    /// false one, entered from the switch alone and on every way to the
    /// block lowered now.
    pub(in crate::lower::mir) fn capture_read_proved(&self, state: &State<'_, 'tcx>, place: Place<'tcx>) -> bool {
        if !self.tcx.is_closure_like(state.body.source.def_id()) {
            return false;
        }
        let Some((PlaceElem::Deref, reference)) = place.projection.split_last() else {
            return false;
        };
        let Some(capture) = self.shared_capture(state, place.local, reference) else {
            return false;
        };
        let blocks = &state.body.basic_blocks;
        let dominators = blocks.dominators();
        blocks.iter().any(|data| {
            let TerminatorKind::SwitchInt { discr, targets } = &data.terminator().kind else {
                return false;
            };
            let Some(flag) = discr.place().filter(|p| p.projection.is_empty()) else {
                return false;
            };
            let writes = self.whole_writes(state, flag.local);
            let Some([Write::Call(_, func, [given])]) = writes.as_deref() else {
                return false;
            };
            let Some((callee, _)) = fn_def(func.ty(state.body, self.tcx)) else {
                return false;
            };
            let Some(some) = crate::lower::recognition::option_presence_test(self.tcx, callee) else {
                return false;
            };
            let [(0, zero)] = targets.iter().collect::<Vec<_>>()[..] else {
                return false;
            };
            let edge = match some {
                true => targets.otherwise(),
                false => zero,
            };
            let tested = (given.node.place()).and_then(|p| self.shared_capture(state, p.local, &p.projection[..]));
            tested == Some(capture) && blocks.predecessors()[edge].len() == 1 && dominators.dominates(edge, state.at)
        })
    }

    /// The capture a place of `local` and `projection` is, by its index,
    /// where it's a shared reference to an `Option` that holds no cell of
    /// its own (rustc's `is_freeze`): the capture itself, `(*_1).k` of a
    /// `Fn`'s or an `FnMut`'s environment, `_1.k` of an `FnOnce`'s, or a
    /// local written once by a copy of it, `_7 = copy ((*_1).k)`, or a
    /// reborrow, `_4 = &(*((*_1).k))`.
    fn shared_capture(&self, state: &State<'_, 'tcx>, local: Local, projection: &[PlaceElem<'tcx>]) -> Option<usize> {
        let (local, projection) = match projection {
            [] if local != Local::from_u32(1) => {
                let writes = self.whole_writes(state, local);
                match writes.as_deref() {
                    Some([Write::Assign(Rvalue::Use(Operand::Copy(of), _))]) => (of.local, of.projection.to_vec()),
                    // A reborrow of what it refers to, `&(*((*_1).k))`.
                    Some([Write::Assign(Rvalue::Ref(_, BorrowKind::Shared, of))])
                        if of.projection.last() == Some(&PlaceElem::Deref) =>
                    {
                        (of.local, of.projection[..of.projection.len() - 1].to_vec())
                    }
                    _ => return None,
                }
            }
            projection => (local, projection.to_vec()),
        };
        let ([PlaceElem::Deref, PlaceElem::Field(capture, ty)] | [PlaceElem::Field(capture, ty)]) = projection[..]
        else {
            return None;
        };
        let ty::Ref(_, referred, ty::Mutability::Not) = *ty.kind() else {
            return None;
        };
        (local == Local::from_u32(1)
            && self.option_of(referred).is_some()
            && referred.is_freeze(self.tcx, self.typing_env))
        .then_some(capture.as_usize())
    }
}
