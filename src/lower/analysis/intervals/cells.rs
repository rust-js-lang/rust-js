//! What the MIR intervals know of a collection a `RefCell` holds (ADR
//! 1137, #1741): a `Ref` its `borrow()` gives, and what the `Ref`'s `Deref`
//! lends, are copies of the cell, so a length read through any borrow of it
//! is the cell's: `stack.borrow()[stack.borrow().len() - 1]` past
//! `!stack.borrow().is_empty()`. Code anyone can reach the cell from, a
//! shared `&RefCell`, may write it: what's known of a cell's length is
//! forgotten at any call but a `borrow()`, a `Ref`'s `Deref`, a length, an
//! `is_empty()` or an index's read; at a `borrow_mut()`; at a drop that may
//! run code; and at a write through a raw pointer, which may reach it with
//! no call.

use super::*;
use crate::lower::analysis::drop_runs_code;
use crate::lower::recognition::{StdItem, is_std_def};

/// The locals that hold a `RefCell`, or lend one.
pub(super) fn cells<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> IndexSet<Local> {
    (body.local_decls.iter_enumerated())
        .filter(|(_, decl)| matches!(decl.ty.peel_refs().kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::RefCell)))
        .map(|(local, _)| local)
        .collect()
}

impl<'tcx> Walk<'_, 'tcx> {
    /// A cell's `borrow()` or a `Ref`'s `Deref` of `of`, the root of what
    /// it's given: what it gives, `local`, a copy of it. A `borrow_mut()`
    /// is a call that may write it (`may_write_cells`).
    pub(super) fn borrowed_cell(&self, func: &Operand<'tcx>, of: Option<Local>, local: Local, after: &mut State) {
        if let (Some((callee, callee_args)), Some(of)) = (func.const_fn_def(), of)
            && (refcell_borrow(self.tcx, callee) == Some(false) || ref_deref(self.tcx, callee, callee_args))
        {
            after.copies.insert(local, of);
        }
    }

    /// Whether a call of `func` may write a cell: none but a `borrow()`, a
    /// `Ref`'s `Deref`, a length, an `is_empty()` or an index's read.
    pub(super) fn may_write_cells(&self, func: &Operand<'tcx>) -> bool {
        let Some((callee, callee_args)) = func.const_fn_def() else {
            return true;
        };
        let reads = refcell_borrow(self.tcx, callee) == Some(false)
            || ref_deref(self.tcx, callee, callee_args)
            || matches!(
                slice_length(self.tcx, callee, callee_args),
                Some(SliceLength::Len | SliceLength::IsEmpty | SliceLength::Index)
            );
        !reads
    }

    /// Whether dropping `place` may run code that writes a cell.
    pub(super) fn drop_may_write_cells(&self, place: &mir::Place<'tcx>) -> bool {
        drop_runs_code(self.tcx, self.typing_env, place.ty(self.body, self.tcx).ty)
    }

    /// Whether a statement's write to `place` is through a raw pointer.
    pub(super) fn written_through_a_pointer(&self, place: &mir::Place<'tcx>) -> bool {
        place
            .iter_projections()
            .any(|(base, elem)| matches!(elem, mir::PlaceElem::Deref) && base.ty(self.body, self.tcx).ty.is_raw_ptr())
    }
}

impl State {
    /// What's known of `cell`'s length, a slice its copies lend, forgotten:
    /// the locals that hold it, its least, its `is_empty()`s and what's
    /// below it.
    pub(super) fn forget_lengths(&mut self, cell: Local) {
        self.lens.retain(|_, of| *of != cell);
        self.least.shift_remove(&cell);
        self.empties.retain(|_, of| *of != cell);
        self.below.retain(|&(_, y, _)| y != Term::Len(cell));
    }
}
