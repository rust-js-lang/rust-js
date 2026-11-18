//! A `RefCell`'s `borrow_mut()` guard (#1815; ADR 0881's on MIR): its
//! `borrow_mut()` and each `deref` or `deref_mut` of it a copy of its
//! cell, so a length read through the guard is the cell's. A guard is held
//! where only a `deref` or a `deref_mut` reads it, each given `&g` or
//! `&mut g` in a local of its own, and only its drop ends it. What its cell
//! keeps past code run while it's held is the effects query's
//! (`analysis::effects::guards`, #2112 B).

use super::*;
use crate::lower::recognition::{StdItem, deref_call, is_std_def};
use rustc_middle::mir::visit::{PlaceContext, Visitor};

/// Of a body, its held guards, each `&g` or `&mut g` a `deref` of one is
/// given, by its local: the guard; and each `&mut` a held guard's
/// `deref_mut` gives that's only given whole to calls.
#[derive(Default)]
pub(super) struct Guards {
    pub(super) held: IndexSet<Local>,
    holders: IndexMap<Local, Local>,
}

/// Each place a statement or a terminator reads or writes `local` at.
pub(super) fn uses<'tcx>(body: &Body<'tcx>, local: Local) -> Vec<(Location, PlaceContext)> {
    struct Uses(Local, Vec<(Location, PlaceContext)>);
    impl<'tcx> Visitor<'tcx> for Uses {
        fn visit_local(&mut self, local: Local, context: PlaceContext, location: Location) {
            if local == self.0 && !matches!(context, PlaceContext::NonUse(_)) {
                self.1.push((location, context));
            }
        }
    }
    let mut uses = Uses(local, Vec::new());
    uses.visit_body(body);
    uses.1
}

/// The call a terminator at `at` is, if it is one: its function, its
/// arguments and where it writes.
pub(super) fn call_at<'a, 'tcx>(body: &'a Body<'tcx>, at: Location) -> Option<CallAt<'a, 'tcx>> {
    let data = &body.basic_blocks[at.block];
    if at.statement_index != data.statements.len() {
        return None;
    }
    match &data.terminator().kind {
        TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } => Some((func, args, destination)),
        _ => None,
    }
}

/// A call's function, its arguments and where it writes.
pub(super) type CallAt<'a, 'tcx> = (
    &'a Operand<'tcx>,
    &'a [rustc_span::Spanned<Operand<'tcx>>],
    &'a mir::Place<'tcx>,
);

/// Whether a statement is one that does nothing as the program runs, a
/// `let`'s `FakeRead` or a `PlaceMention`, borrowck's.
pub(super) fn inert(kind: &StatementKind<'_>) -> bool {
    matches!(kind, StatementKind::FakeRead(..) | StatementKind::PlaceMention(..))
}

/// Whether `operand` is `local`, whole.
fn is_whole(operand: &Operand<'_>, local: Local) -> bool {
    operand.place().is_some_and(|p| p.as_local() == Some(local))
}

pub(super) fn guards<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> Guards {
    let is_guard =
        |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::RefCellRefMut));
    let mut guards = Guards::default();
    for (guard, decl) in body.local_decls.iter_enumerated() {
        if !is_guard(decl.ty) {
            continue;
        }
        let mut holders = Vec::new();
        let mut made = false;
        // Made by a cell's `borrow_mut()`, lent whole to a local, dropped
        // whole, and read nowhere else.
        let held = uses(body, guard).into_iter().all(|(at, _)| {
            let data = &body.basic_blocks[at.block];
            match data.statements.get(at.statement_index).map(|s| &s.kind) {
                Some(StatementKind::Assign(assign)) => match (&assign.1, assign.0.as_local()) {
                    (Rvalue::Ref(_, _, of), Some(holder)) if of.as_local() == Some(guard) => {
                        holders.push(holder);
                        true
                    }
                    _ => false,
                },
                Some(kind) => inert(kind),
                None => match &data.terminator().kind {
                    TerminatorKind::Drop { place, .. } => place.as_local() == Some(guard),
                    TerminatorKind::Call { func, destination, .. } if destination.as_local() == Some(guard) => {
                        made = true;
                        (func.const_fn_def()).is_some_and(|(callee, _)| refcell_borrow(tcx, callee) == Some(true))
                    }
                    _ => false,
                },
            }
        });
        // Each holder given only to a `deref` or a `deref_mut`, whole, as
        // its receiver.
        let derefed = holders.iter().all(|&holder| {
            uses(body, holder).into_iter().all(|(at, _)| {
                let assigned = (body.basic_blocks[at.block].statements.get(at.statement_index))
                    .is_some_and(|s| matches!(&s.kind, StatementKind::Assign(a) if a.0.as_local() == Some(holder)));
                assigned
                    || call_at(body, at).is_some_and(|(func, args, destination)| {
                        func.const_fn_def().is_some_and(|(callee, _)| deref_call(tcx, callee))
                            && destination.local != holder
                            && args.first().is_some_and(|a| is_whole(&a.node, holder))
                            && args.len() == 1
                    })
            })
        });
        if !(held && made && derefed) {
            continue;
        }
        guards.held.insert(guard);
        guards.holders.extend(holders.into_iter().map(|holder| (holder, guard)));
    }
    guards
}

impl<'tcx> Walk<'_, 'tcx> {
    /// The guard a call's receiver lends, of a `deref` or a `deref_mut`.
    fn derefed_guard(&self, func: &Operand<'tcx>, args: &[rustc_span::Spanned<Operand<'tcx>>]) -> Option<Local> {
        let (callee, _) = func.const_fn_def()?;
        let holder = args.first()?.node.place()?.as_local()?;
        deref_call(self.tcx, callee).then(|| self.guards.holders.get(&holder).copied())?
    }

    /// What a held guard's own call gives, `local`: of its `borrow_mut()`,
    /// a copy of the cell, which it guards; of a `deref` or a `deref_mut`,
    /// a copy of it, whose length a call it's lent to forgets (`guard_lent`).
    pub(super) fn guarded(
        &self,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        local: Local,
        func: &Operand<'tcx>,
        state: &State,
        after: &mut State,
    ) {
        if self.guards.held.contains(&local)
            && let Some(cell) = args.first().and_then(|a| a.node.place()).and_then(|p| p.as_local())
        {
            let cell = state.root(cell);
            after.copies.insert(local, cell);
        }
        if let Some(guard) = self.derefed_guard(func, args) {
            after.copies.insert(local, state.root(guard));
        }
    }
}
