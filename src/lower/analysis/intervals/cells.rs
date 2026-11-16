//! What the MIR intervals know of a collection a `RefCell` holds (ADR
//! 1137, #1741): a `Ref` its `borrow()` gives, and what the `Ref`'s `Deref`
//! lends, are copies of the cell, so a length read through any borrow of it
//! is the cell's: `stack.borrow()[stack.borrow().len() - 1]` past
//! `!stack.borrow().is_empty()`. An `Rc`'s or an `Arc`'s `Deref` gives
//! the same cell each time and runs nothing, so is a copy too (#1839):
//! `listeners.borrow()[i]` under `i < listeners.borrow().len()`, while
//! the local isn't reassigned or lent `&mut`. Code anyone can reach the cell from, a
//! shared `&RefCell`, may write it: what's known of a cell's length is
//! forgotten at any call but a `borrow()`, a `Ref`'s or an `Rc`'s `Deref`, a length, an
//! `is_empty()`, an index's read or a `for` loop's step of a range or a
//! slice's iterator; at a `borrow_mut()`; at a drop that may
//! run code; and at a write through a raw pointer, which may reach it with
//! no call.

use super::*;
use crate::lower::analysis::drop_runs_code;
use crate::lower::recognition::{StdItem, is_std_def, rc_pointee};

/// The locals that hold a `RefCell`, or lend one, or point to one
/// through `Rc`s, `Arc`s and `Box`es, whose copies a `borrow()` gives
/// (#1839).
pub(super) fn cells<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> IndexSet<Local> {
    (body.local_decls.iter_enumerated())
        .filter(|(_, decl)| holds_cell(tcx, decl.ty))
        .map(|(local, _)| local)
        .collect()
}

/// Whether `ty` is a `RefCell`, or a reference, an `Rc`, an `Arc` or a
/// `Box` of what holds one.
fn holds_cell<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    let ty = ty.peel_refs();
    match ty.kind() {
        ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::RefCell) => true,
        _ => (rc_pointee(tcx, ty).or_else(|| ty.boxed_ty())).is_some_and(|of| holds_cell(tcx, of)),
    }
}

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

    /// Whether a call of `func` may write a cell: none but a `borrow()`, a
    /// `Ref`'s or an `Rc`'s `Deref`, a length, an `is_empty()`, an index's
    /// read, or a step of a range of integers or a slice's iterator the walk
    /// knows (`range_step`), std's, which runs none of the crate's code:
    /// `for i in (0..batch.borrow().len()).step_by(2)`'s `next()` (ADR 1168,
    /// #1948).
    pub(super) fn may_write_cells(&self, func: &Operand<'tcx>) -> bool {
        let Some((callee, callee_args)) = func.const_fn_def() else {
            return true;
        };
        let reads = refcell_borrow(self.tcx, callee) == Some(false)
            || ref_deref(self.tcx, callee, callee_args)
            || rc_deref(self.tcx, callee, callee_args)
            || range_step(self.tcx, callee, callee_args).is_some()
            || matches!(
                slice_length(self.tcx, callee, callee_args),
                Some(SliceLength::Len | SliceLength::IsEmpty | SliceLength::Index)
            );
        !reads
    }

    /// Whether dropping `place` may run code that writes a cell. Not a
    /// `Ref`'s or a `RefMut`'s, which gives its borrow back and drops
    /// none of what the cell holds (#1815).
    pub(super) fn drop_may_write_cells(&self, place: &mir::Place<'tcx>) -> bool {
        let ty = place.ty(&*self.body, self.tcx).ty;
        let guard = matches!(ty.kind(), ty::Adt(adt, _)
            if is_std_def(self.tcx, adt.did(), StdItem::RefCellRef) || is_std_def(self.tcx, adt.did(), StdItem::RefCellRefMut));
        !guard && drop_runs_code(self.tcx, self.typing_env, ty)
    }

    /// Whether a statement's write to `place` is through a raw pointer.
    pub(super) fn written_through_a_pointer(&self, place: &mir::Place<'tcx>) -> bool {
        place
            .iter_projections()
            .any(|(base, elem)| matches!(elem, mir::PlaceElem::Deref) && base.ty(&*self.body, self.tcx).ty.is_raw_ptr())
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
        self.below_past.retain(|&(_, slice, _)| slice != cell);
        self.keys.retain(|&(map, _)| map != cell);
        self.has_keys.retain(|_, &mut (map, _)| map != cell);
        self.equal_lens.retain(|&(a, b)| a != cell && b != cell);
        self.rows.retain(|&table, &mut row| table != cell && row != cell);
    }
}
