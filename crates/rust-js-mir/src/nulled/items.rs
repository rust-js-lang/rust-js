//! What a closure reads of the thread-local a `with` gives it, where that
//! thread-local is `Some` wherever the closure runs (`NulledParams::
//! given_present`, #1933 step 4 (G)): React DOM's
//! `SCHEDULE_TIMEOUT.with(|s| s.unwrap())`, in a closure made only where
//! `SCHEDULE_TIMEOUT.with(Option::is_some)` held. Split by concept from
//! `nulled`; `mir_operand` asks it, beside `proved_present`.

use super::*;

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// Whether `place`, read in the closure lowered now, is what its one
    /// parameter, the `&` a `with` lends it, refers to, `*_2`, or a local
    /// that holds it, written once, `_3 = copy (*_2)`, of a thread-local
    /// that's `Some` wherever the closure runs: the parameter written
    /// nowhere. A copy, so of an `Option` that's `Copy`; an `as_ref()` of
    /// one that isn't, `name.as_ref().unwrap()`, isn't yet (#1933).
    pub(crate) fn present_item_read(&self, state: &State<'_, 'tcx>, place: mir::Place<'tcx>) -> bool {
        let closure = state.body.source.def_id();
        let param = Local::from_u32(2);
        if !self.tcx.is_closure_like(closure)
            || state.body.arg_count != 2
            || state.locals.writes[param] != 0
            || !self.krate.nulled_params.given_present(closure)
        {
            return false;
        }
        let referred = |of: &mir::Place<'tcx>| of.local == param && of.projection[..] == [mir::ProjectionElem::Deref];
        referred(&place)
            || place.projection.is_empty()
                && matches!(self.whole_writes(state, place.local).as_deref(),
                    Some([Write::Assign(Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _))]) if referred(of))
    }
}
