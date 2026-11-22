//! A table's lengths, on MIR (ADR 0576, amended; #1672 M3, step 5): what
//! `vec![item; n]` is as long as, `n`, a slice's length, or another
//! table's, `vec![0.0; terraces.len()]` as long as `terraces`
//! (`State::equal_lens`); and of a table of rows, `vec![vec![1; width];
//! height]`, each row as long as the one it was made of (`State::rows`),
//! so `board[r][c]` under `c < width` is proved. Only of a row type no
//! `&mut` in the body may resize (`rows_kept`): a row's `&mut` given to
//! anything but its `index_mut` or `deref_mut`, or a row written whole,
//! and its tables' rows aren't known.
//! A call given a table's `&mut` forgets them, as it does its length.

use super::guards::{call_at, inert, uses};
use super::*;
use crate::analysis::keeps_vec_length;
use crate::pointers::raw_pointee;
use crate::recognition::{StdItem, is_std_def, is_vec_from_elem};

/// The rows of `body`'s tables, `vec![row; n]`'s, whose type no `&mut`
/// in the body may resize: each local that's a `&mut` of one read only
/// as an `index_mut`'s or a `deref_mut`'s receiver, reborrowed shared or
/// so. A closure or a function reaches a row only through such a local,
/// or a call given the table's `&mut`, which forgets its rows (`tabled`).
pub(super) fn rows_kept<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> IndexSet<Ty<'tcx>> {
    let is_vec = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Vec));
    let rows: IndexSet<Ty<'tcx>> = (body.basic_blocks.iter())
        .filter_map(|data| match &data.terminator().kind {
            TerminatorKind::Call { func, destination, .. }
                if func
                    .const_fn_def()
                    .is_some_and(|(callee, _)| is_vec_from_elem(tcx, callee)) =>
            {
                match destination.ty(&body.local_decls, tcx).ty.kind() {
                    ty::Adt(_, args) => args.types().next().filter(|&row| is_vec(row)),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect();
    rows.into_iter()
        .filter(|&row| {
            (body.local_decls.iter_enumerated()).all(|(local, decl)| match *decl.ty.kind() {
                ty::Ref(_, of, ty::Mutability::Mut) if of == row => row_lent_only_kept(tcx, body, local),
                _ => true,
            })
        })
        .collect()
}

/// Whether a `&mut` of a row, `local`, is read only as an `index_mut`'s or
/// a `deref_mut`'s receiver, or reborrowed shared, or reborrowed `&mut`
/// into a local read so, and made only by a call or a statement of its own.
fn row_lent_only_kept<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>, local: Local) -> bool {
    uses(body, local).into_iter().all(|(at, _)| {
        match (body.basic_blocks[at.block].statements.get(at.statement_index)).map(|s| &s.kind) {
            Some(StatementKind::Assign(assign)) => match (&assign.1, assign.0.as_local()) {
                _ if assign.0.as_local() == Some(local) => true,
                (Rvalue::Ref(_, mir::BorrowKind::Shared, _), Some(_)) => true,
                (Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of), Some(holder))
                    if of.local == local && matches!(of.projection.as_slice(), [mir::PlaceElem::Deref]) =>
                {
                    row_lent_only_kept(tcx, body, holder)
                }
                _ => false,
            },
            Some(kind) => inert(kind),
            None => call_at(body, at).is_some_and(|(func, args, destination)| {
                destination.as_local() == Some(local)
                    || func
                        .const_fn_def()
                        .is_some_and(|(callee, _)| keeps_vec_length(tcx, callee))
                        && args
                            .first()
                            .is_some_and(|a| a.node.place().is_some_and(|p| p.as_local() == Some(local)))
                        && args
                            .iter()
                            .skip(1)
                            .all(|a| a.node.place().is_none_or(|p| p.local != local))
            }),
        }
    })
}

impl<'tcx> Walk<'_, 'tcx> {
    /// What a call that makes a table or reads a row gives, `local`: of
    /// `vec![item; n]`, a table as long as `n`, a slice's length or an
    /// integer's, or a constant, and of a row kept (`rows_kept`), its rows
    /// each as long as `item`; of `board[r]`, a row as long as those.
    pub(super) fn tabled(
        &self,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        local: Local,
        state: &State,
        after: &mut State,
    ) {
        let Some((callee, callee_args)) = func.const_fn_def() else {
            return;
        };
        let root = |at: usize| {
            (args.get(at))
                .and_then(|a| a.node.place())
                .and_then(|p| p.as_local())
                .filter(|l| !self.lent.contains(l))
                .map(|l| state.root(l))
        };
        // A call given a table's `&mut`, or its slice's, may write a row
        // whole, `board.fill(vec![])`: its rows forgotten, but of one that
        // keeps them, an index's, a length's, `index_mut` or `deref_mut`.
        let keeps = keeps_vec_length(self.tcx, callee)
            || matches!(
                slice_length(self.tcx, callee, callee_args),
                Some(SliceLength::Index | SliceLength::Len | SliceLength::IsEmpty)
            );
        if !keeps {
            for (at, arg) in args.iter().enumerate() {
                let given = arg.node.ty(&*self.body, self.tcx);
                let mutable = matches!(given.kind(), ty::Ref(_, _, ty::Mutability::Mut))
                    || raw_pointee(self.tcx, given).is_some_and(|(_, m)| m.is_mut());
                if let Some(table) = root(at).filter(|_| mutable) {
                    after.rows.shift_remove(&table);
                }
            }
        }
        if is_vec_from_elem(self.tcx, callee) {
            match (root(1), args.get(1).and_then(|a| self.operand(&a.node, state))) {
                (Some(n), _) => match state.length_of(n) {
                    Some(of) => after.same_length(local, of),
                    None => {
                        after.lens.insert(n, local);
                    }
                },
                (None, Some(n)) if n.lo == n.hi => {
                    after.least.insert(local, n.lo);
                }
                _ => {}
            }
            let row = self.body.local_decls[local].ty;
            if let ty::Adt(_, a) = row.kind()
                && a.types().next().is_some_and(|r| self.kept_rows.contains(&r))
                && let Some(item) = root(0)
            {
                after.rows.insert(local, item);
            }
        }
        if let Some(SliceLength::Index) = slice_length(self.tcx, callee, callee_args)
            && let Some(table) = root(0)
            && let Some(&item) = state.rows.get(&table)
        {
            after.same_length(local, item);
        }
    }
}

impl State {
    /// `a` as long as `b`, two slices' lengths one (`equal_lens`).
    pub(super) fn same_length(&mut self, a: Local, b: Local) {
        if a != b {
            self.equal_lens.insert((a.min(b), a.max(b)));
        }
    }

    /// Each slice as long as `slice`, itself among them.
    pub(super) fn as_long_as(&self, slice: Local) -> Vec<Local> {
        let others = (self.equal_lens.iter()).filter_map(|&(a, b)| match (a == slice, b == slice) {
            (true, _) => Some(b),
            (_, true) => Some(a),
            _ => None,
        });
        std::iter::once(slice).chain(others).collect()
    }
}
