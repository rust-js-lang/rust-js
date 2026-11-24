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
    /// itself. Its `receiver`, in its body's THIR where the proof finds the
    /// call there, says what a binding's method runs, as `call_asks`'s does.
    pub(crate) fn unproven_call(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        (owner, def_id, args): (DefId, DefId, ty::GenericArgsRef<'tcx>),
        receiver: Option<(&Thir<'tcx>, ExprId)>,
        held: Borrow<'tcx>,
    ) -> Option<Unproven> {
        let tcx = recognition.tcx;
        let itself = |ty| self.clones_itself(recognition, owner, ty);
        let one = receiver.map(|(_, r)| [r]);
        let given = (receiver.zip(one.as_ref())).map(|((thir, _), one)| CallArgs {
            thir,
            args: &one[..],
            at_hand: &self.at_hand,
            pointers: Some(&self.pointers),
        });
        match called(recognition, def_id, args, given, &itself) {
            Called::Nothing => None,
            // A cell's own call: its borrow, and what it runs by its bounds
            // while it holds it, a `with_borrow_mut`'s closure.
            Called::Cell(theirs) if asks(tcx, &self.closed, theirs, held) => Some(Unproven::SecondBorrow),
            Called::Cell(_) => self.std_runs_unproven(recognition, (def_id, args), held),
            Called::Body(body, _) if tcx.is_closure_like(body) => Some(Unproven::Closure),
            Called::Body(..) | Called::Bodies(_) => Some(Unproven::CrateFn),
            Called::Std(id, args) => self.std_runs_unproven(recognition, (id, args), held),
            Called::Read(_) | Called::Any => Some(Unproven::Generic),
            Called::Unseen => Some(Unproven::Unseen),
        }
    }

    /// What std's call of `id`, of `args`, runs by its bounds that the
    /// proof alone can't pass (`std_given`): what may be any code, the
    /// crate's code, or a borrow of a cell `held`'s may be.
    fn std_runs_unproven(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        (id, args): (DefId, ty::GenericArgsRef<'tcx>),
        held: Borrow<'tcx>,
    ) -> Option<Unproven> {
        let given = std_given(recognition, &self.impls, id, args);
        if given.any {
            Some(Unproven::Generic)
        } else if !given.bodies.is_empty() {
            Some(Unproven::StdRunsCrate)
        } else {
            (given
                .cells
                .iter()
                .any(|&theirs| asks(recognition.tcx, &self.closed, theirs, held)))
            .then_some(Unproven::SecondBorrow)
        }
    }

    /// Whether a borrow `theirs` may meet `held`, one of a cell it may be,
    /// one of them mutable (`asks`).
    pub(crate) fn borrows_meet(&self, tcx: TyCtxt<'tcx>, theirs: Borrow<'tcx>, held: Borrow<'tcx>) -> bool {
        asks(tcx, &self.closed, theirs, held)
    }
}
