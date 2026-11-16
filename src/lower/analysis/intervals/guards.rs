//! A `RefCell`'s `borrow_mut()` guard, held while nothing else reaches the
//! cell (#1815; ADR 0881's on MIR): while it lives, any other borrow of the
//! cell panics, so code given no way to it can't change what the cell
//! holds. What's known of the cell's length is kept past a call, or a drop
//! that may run code, until the guard drops (`State::guarded`).
//!
//! A guard is held where only a `deref` or a `deref_mut` reads it, each
//! given `&g` or `&mut g` in a local of its own, and only its drop ends it.
//! What its `deref_mut` lends, a `&mut` of what the cell holds, keeps the
//! length only past a call that keeps it, `index_mut` or a `deref_mut` to
//! a slice (`keeps_vec_length`): given to `push`, `truncate` or code of the
//! crate's, itself or through any number of reborrows, `let all = &mut
//! *items; all.truncate(1)`, the length is forgotten at the call
//! (`lent_through`, #1929); written through, as any write through a `*`
//! forgets its root's length. The `deref_mut` itself is a copy of the cell,
//! as a `deref` is: its own rule that forgot the length there, where the
//! `&mut` went any other way, is retired, the native pass finding no
//! program that differs without it (ADR 0093).

use super::*;
use crate::lower::recognition::{StdItem, deref_call, deref_mut_call, is_std_def, keeps_vec_length};
use rustc_middle::mir::visit::{PlaceContext, Visitor};

/// Of a body, its held guards, each `&g` or `&mut g` a `deref` of one is
/// given, by its local: the guard; and each `&mut` a held guard's
/// `deref_mut` gives that's only given whole to calls.
#[derive(Default)]
pub(super) struct Guards {
    pub(super) held: IndexSet<Local>,
    holders: IndexMap<Local, Local>,
    pub(super) lends: IndexSet<Local>,
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

/// Whether the call at `at` is given `local` whole, or makes it whole.
fn given_at(body: &Body<'_>, at: Location, local: Local) -> bool {
    call_at(body, at).is_some_and(|(_, args, destination)| match destination.local == local {
        true => destination.as_local() == Some(local),
        false => (args.iter()).all(|a| {
            a.node
                .place()
                .is_none_or(|p| p.local != local || p.as_local() == Some(local))
        }),
    })
}

/// Whether `local` is read only as a call's argument whole, or reborrowed,
/// `&mut (*local)`, into a local read only so in turn, however many times,
/// `let all = &mut *items; all.truncate(1)`'s `all` and the `&mut (*all)`
/// its call is given (#1929): each such local pushed to `through`.
fn lent_through(body: &Body<'_>, local: Local, through: &mut Vec<Local>) -> bool {
    uses(body, local).into_iter().all(|(at, _)| {
        match (body.basic_blocks[at.block].statements.get(at.statement_index)).map(|s| &s.kind) {
            Some(StatementKind::Assign(assign)) => match (&assign.1, assign.0.as_local()) {
                (_, Some(made)) if made == local => true,
                (Rvalue::Ref(_, _, of), Some(holder))
                    if of.local == local
                        && matches!(of.projection.as_slice(), [mir::PlaceElem::Deref])
                        && !through.contains(&holder) =>
                {
                    through.push(holder);
                    lent_through(body, holder, through)
                }
                _ => false,
            },
            Some(kind) => inert(kind),
            None => given_at(body, at, local),
        }
    })
}

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
    // What a held guard's `deref_mut` gives, read only as a call's argument
    // whole, or reborrowed, `&mut (*m)`, into a local read only so, however
    // many times (`lent_through`).
    for data in body.basic_blocks.iter() {
        if let TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } = &data.terminator().kind
            && func
                .const_fn_def()
                .is_some_and(|(callee, _)| deref_mut_call(tcx, callee))
            && let Some(holder) = args.first().and_then(|a| a.node.place()).and_then(|p| p.as_local())
            && guards.holders.contains_key(&holder)
            && let Some(lent) = destination.as_local()
        {
            let mut through = Vec::new();
            let only_given = lent_through(body, lent, &mut through);
            if only_given {
                guards.lends.insert(lent);
                guards.lends.extend(through);
            }
        }
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

    /// Whether a call is a held guard's own, none of which writes the cell:
    /// its `borrow_mut()`, or a `deref` or `deref_mut` of it.
    pub(super) fn guard_call(
        &self,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        destination: &mir::Place<'tcx>,
    ) -> bool {
        destination.as_local().is_some_and(|l| self.guards.held.contains(&l))
            || self.derefed_guard(func, args).is_some()
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
            after.guarded.insert(local, cell);
        }
        if let Some(guard) = self.derefed_guard(func, args) {
            after.copies.insert(local, state.root(guard));
        }
    }

    /// A call given what a held guard's `deref_mut` lends: past it, the
    /// cell's length forgotten, but of one that keeps it, `index_mut`.
    pub(super) fn guard_lent(
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
            if let Some(lent) = arg.node.place().and_then(|p| p.as_local())
                && self.guards.lends.contains(&lent)
            {
                after.forget_lengths(state.root(lent));
            }
        }
    }
}
