//! A `Vec` a local holds, lent `&mut` only to the calls it's given to
//! (#1672 M3, step 5): not lent for the whole body, its length known as a
//! shared one's is, and forgotten past each call that may change it,
//! `push` or `remove`. One that keeps it, `index_mut`'s `v[i] = x` or
//! `deref_mut` to a slice, keeps it known.

use super::*;
use crate::lower::recognition::{StdItem, deref_call, is_std_def, keeps_vec_length};

/// Of `body`, each `&mut` of a `Vec` a local holds that's given only to a
/// call, by the local holding it: the `Vec`'s local. A local is only one
/// whose every `&mut` is so; none kept that's lent any other way, its
/// `&mut` stored or given on. What a call does with one it's given, kept
/// in what it gives back or another argument, is done before the `Vec` is
/// used again, as borrowck holds, and a call that may change its length
/// forgets it (`called_on_mut`).
pub(super) fn called_on<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> IndexMap<Local, Local> {
    let is_vec = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Vec));
    // A `&mut Vec` local, a parameter's `targets: &mut Vec<u32>`: of the
    // `Vec` it points to, the one way to it while it lives.
    let is_mut_of_vec = |ty: Ty<'tcx>| matches!(*ty.kind(), ty::Ref(_, to, ty::Mutability::Mut) if is_vec(to));
    // Each `&mut` of a `Vec` local, its own place whole, `&mut v`, or of
    // what a `&mut Vec` local points to, `&mut (*targets)`, or a reborrow
    // of one, `&mut (*t)`: by the local holding it.
    let mut borrows: IndexMap<Local, Local> = IndexMap::default();
    let mut lent_otherwise: IndexSet<Local> = IndexSet::default();
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            match &assign.1 {
                Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of)
                    if matches!(of.projection.as_slice(), [mir::PlaceElem::Deref])
                        && borrows.contains_key(&of.local) =>
                {
                    match assign.0.as_local() {
                        Some(holder) => {
                            borrows.insert(holder, borrows[&of.local]);
                        }
                        None => {
                            lent_otherwise.insert(borrows[&of.local]);
                        }
                    }
                }
                Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of) if is_mut_of_vec(body.local_decls[of.local].ty) => {
                    match (assign.0.as_local(), of.projection.as_slice()) {
                        (Some(holder), [mir::PlaceElem::Deref]) => {
                            borrows.insert(holder, of.local);
                        }
                        _ => {
                            lent_otherwise.insert(of.local);
                        }
                    }
                }
                Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of) if is_vec(body.local_decls[of.local].ty) => {
                    match (assign.0.as_local(), of.projection.as_slice()) {
                        (Some(holder), []) => {
                            borrows.insert(holder, of.local);
                        }
                        _ => {
                            lent_otherwise.insert(of.local);
                        }
                    }
                }
                Rvalue::RawPtr(_, of) => {
                    lent_otherwise.insert(of.local);
                }
                _ => {}
            }
        }
    }
    // Where each `&mut` is read: given a call, as itself, and reborrowed,
    // and nowhere else; and a `&mut Vec` local only borrowed through,
    // `&(*targets)` or `&mut (*targets)`, never itself copied or moved.
    let pointers: IndexSet<Local> = (borrows.values().copied())
        .filter(|&of| is_mut_of_vec(body.local_decls[of].ty))
        .collect();
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            for (holder, of) in borrows.iter() {
                if mentions(statement, *holder) && !reborrows(statement, *holder) {
                    lent_otherwise.insert(*of);
                }
            }
            for &pointer in &pointers {
                if mentions(statement, pointer) && !borrowed_through(statement, pointer) {
                    lent_otherwise.insert(pointer);
                }
            }
        }
        for &pointer in &pointers {
            if terminator_mentions(data.terminator(), pointer) {
                lent_otherwise.insert(pointer);
            }
        }
        // A call given one whole, as itself; not one it writes, nor a place
        // in one. Any other terminator reading one lends it.
        let terminator = data.terminator();
        for (&holder, &of) in &borrows {
            if !terminator_mentions(terminator, holder) {
                continue;
            }
            let given = match &terminator.kind {
                TerminatorKind::Call { args, destination, .. } => {
                    destination.local != holder
                        && (args.iter()).all(|arg| {
                            arg.node
                                .place()
                                .is_none_or(|p| p.local != holder || p.projection.is_empty())
                        })
                }
                _ => false,
            };
            if !given {
                lent_otherwise.insert(of);
            }
        }
    }
    borrows.retain(|_, of| !lent_otherwise.contains(of));
    borrows
}

/// Whether `statement` reads or writes `local` at all.
fn mentions(statement: &mir::Statement<'_>, local: Local) -> bool {
    use rustc_middle::mir::visit::{PlaceContext, Visitor};
    struct Mention(Local, bool);
    impl<'tcx> Visitor<'tcx> for Mention {
        fn visit_local(&mut self, local: Local, context: PlaceContext, _: Location) {
            if local == self.0 && !matches!(context, PlaceContext::NonUse(_)) {
                self.1 = true;
            }
        }
    }
    let mut mention = Mention(local, false);
    mention.visit_statement(statement, Location::START);
    mention.1
}

/// Whether `statement` makes `local` or reborrows it, `&mut (*local)`, the
/// one place it's allowed in a statement.
fn reborrows(statement: &mir::Statement<'_>, local: Local) -> bool {
    matches!(&statement.kind, StatementKind::Assign(assign)
        if assign.0.as_local() == Some(local)
            || matches!(&assign.1, Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of)
                if of.local == local && matches!(of.projection.as_slice(), [mir::PlaceElem::Deref])))
}

/// Whether `statement` borrows what `pointer` points to, `&(*pointer)`
/// or a place in it, the one place it's allowed in a statement.
fn borrowed_through(statement: &mir::Statement<'_>, pointer: Local) -> bool {
    matches!(&statement.kind, StatementKind::Assign(assign)
        if assign.0.local != pointer
            && matches!(&assign.1, Rvalue::Ref(_, _, of)
                if of.local == pointer && matches!(of.projection.first(), Some(mir::PlaceElem::Deref))))
}

/// Whether a terminator not a call reads `local`.
fn terminator_mentions(terminator: &mir::Terminator<'_>, local: Local) -> bool {
    use rustc_middle::mir::visit::{PlaceContext, Visitor};
    struct Mention(Local, bool);
    impl<'tcx> Visitor<'tcx> for Mention {
        fn visit_local(&mut self, local: Local, context: PlaceContext, _: Location) {
            if local == self.0 && !matches!(context, PlaceContext::NonUse(_)) {
                self.1 = true;
            }
        }
    }
    let mut mention = Mention(local, false);
    mention.visit_terminator(terminator, Location::START);
    mention.1
}

impl<'tcx> Walk<'_, 'tcx> {
    /// What a call given `&mut`s of `Vec`s only lent to calls does to their
    /// lengths (`called_on`): one that keeps them, `index_mut` and
    /// `deref_mut`, keeps them known; any other, `push` or `remove`, forgets
    /// them past it.
    pub(super) fn called_on_mut(
        &self,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        state: &State,
        after: &mut State,
    ) {
        if func
            .const_fn_def()
            .is_some_and(|(callee, _)| keeps_vec_length(self.tcx, callee))
        {
            return;
        }
        for arg in args {
            if let Some(local) = arg.node.place().and_then(|p| p.as_local())
                && let Some(&of) = self.called_on.get(&local)
            {
                after.forget_lengths(state.root(of));
            }
        }
    }

    /// A `Vec`'s `deref` or `deref_mut`, a slice of its items: of its
    /// length, a copy of it, so `swap(i, j)` of the slice is of the `Vec`.
    pub(super) fn derefed(
        &self,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        local: Local,
        state: &State,
        after: &mut State,
    ) {
        if func
            .const_fn_def()
            .is_some_and(|(callee, _)| deref_call(self.tcx, callee))
            && let Some(receiver) = args.first()
            && (receiver.node.ty(&*self.body, self.tcx).builtin_deref(true))
                .is_some_and(|of| matches!(of.kind(), ty::Adt(adt, _) if is_std_def(self.tcx, adt.did(), StdItem::Vec)))
            && let Some(of) = receiver
                .node
                .place()
                .and_then(|p| p.as_local())
                .filter(|of| !self.lent.contains(of))
        {
            after.copies.insert(local, state.root(of));
        }
    }
}
