//! What the MIR intervals know of a collection a `RefCell` holds (ADR
//! 1137, #1741): a `Ref` its `borrow()` gives, and what the `Ref`'s `Deref`
//! lends, are copies of the cell, so a length read through any borrow of it
//! is the cell's: `stack.borrow()[stack.borrow().len() - 1]` past
//! `!stack.borrow().is_empty()`. An `Rc`'s or an `Arc`'s `Deref` gives
//! the same cell each time and runs nothing, so is a copy too (#1839):
//! `listeners.borrow()[i]` under `i < listeners.borrow().len()`. What may
//! write the cell is the effects query's (`analysis::Effects`, #2112 B).

use super::*;

impl<'tcx> Walk<'_, 'tcx> {
    /// A cell's `borrow()`, or a `Ref`'s or an `Rc`'s `Deref`, of `of`,
    /// the root of what it's given: what it gives, `local`, a copy of it.
    /// A `borrow_mut()` is a call that may write it (`may_write_cells`).
    pub(super) fn borrowed_cell(&self, func: &Operand<'tcx>, of: Option<Local>, local: Local, after: &mut State) {
        if let (Some((callee, callee_args)), Some(of)) = (func.const_fn_def(), of)
            && (refcell_borrow(self.tcx, callee) == Some(false)
                || ref_deref(self.tcx, callee, callee_args)
                || rc_deref(self.tcx, callee, callee_args))
        {
            after.copies.insert(local, of);
        }
    }
}
