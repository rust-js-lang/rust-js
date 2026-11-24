//! What a call in a body's MIR runs that `CellForm`'s proof alone, without
//! the crate's bodies' summaries, can't pass while a guard is held (#2490,
//! Campaign 4 phase 1), of `called`'s answers.

use super::*;

/// What a call runs that `CellForm`'s proof alone can't pass while a guard
/// is live (`CellBorrows::unproven_call`, #2490): the crate's function or
/// closure, std's code given the crate's, what may be any code, code
/// rust-js can't see, or another borrow of a cell the guard's may be.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Unproven {
    CrateFn,
    Closure,
    StdRunsCrate,
    Generic,
    Unseen,
    SecondBorrow,
}

impl<'tcx> CellBorrows<'tcx> {
    /// What a call in a body's MIR, of `def_id` and `args`, runs that
    /// `CellForm`'s proof alone, without the bodies' summaries, can't pass
    /// while a guard of `held` is live (#2490): `None` of one that runs no
    /// code of the crate's, by std's or a binding's, and borrows no cell
    /// `held`'s may be. With no receiver known, a borrow's call is what may
    /// be any (`called`'s `Any`); the proof reads a `RefCell`'s own borrows
    /// itself.
    pub(crate) fn unproven_call(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        (owner, def_id, args): (DefId, DefId, ty::GenericArgsRef<'tcx>),
        held: Borrow<'tcx>,
    ) -> Option<Unproven> {
        let tcx = recognition.tcx;
        let itself = |ty| self.clones_itself(recognition, owner, ty);
        match called(recognition, def_id, args, None, &itself) {
            Called::Nothing => None,
            Called::Cell(theirs) => asks(tcx, &self.closed, theirs, held).then_some(Unproven::SecondBorrow),
            Called::Body(body, _) if tcx.is_closure_like(body) => Some(Unproven::Closure),
            Called::Body(..) | Called::Bodies(_) => Some(Unproven::CrateFn),
            Called::Std(id, args) => {
                let given = std_given(recognition, &self.impls, id, args);
                if given.any {
                    Some(Unproven::Generic)
                } else if !given.bodies.is_empty() {
                    Some(Unproven::StdRunsCrate)
                } else {
                    (given.cells.iter().any(|&theirs| asks(tcx, &self.closed, theirs, held)))
                        .then_some(Unproven::SecondBorrow)
                }
            }
            Called::Read(_) | Called::Any => Some(Unproven::Generic),
            Called::Unseen => Some(Unproven::Unseen),
        }
    }

    /// Whether a borrow `theirs` may meet `held`, one of a cell it may be,
    /// one of them mutable (`asks`).
    pub(crate) fn borrows_meet(&self, tcx: TyCtxt<'tcx>, theirs: Borrow<'tcx>, held: Borrow<'tcx>) -> bool {
        asks(tcx, &self.closed, theirs, held)
    }
}
