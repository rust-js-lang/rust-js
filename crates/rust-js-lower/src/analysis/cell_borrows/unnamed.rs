//! What code that can't be named may borrow of a cell, which the effects
//! query asks of a guard held across such code (#2112 B).

use super::*;

impl<'tcx> CellBorrows<'tcx> {
    /// Whether code that can't be named, a library's or a binding's, or what
    /// a `dyn` or a fn pointer may be, may borrow a cell of `ty`
    /// (`anything_asks`): where it may, a guard of one held across a call
    /// that may run such code excludes no write of it, as a borrow it makes
    /// may not be one this guard is counted for (#2112 B, `analysis::Effects`).
    pub(crate) fn unnamed_may_borrow(&self, tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
        self.anything_asks(tcx, Borrow::of(Cell::Type(ty), true))
    }
}
