//! A `Vec` or a `VecDeque` a local holds, lent `&mut` only to the calls
//! it's given to (#1672 M3, step 5): not lent for the whole body, its length
//! known as a shared one's is, and forgotten past each call that may change
//! it, `push` or `remove`. One that keeps it, `index_mut`'s `v[i] = x` or
//! `deref_mut` to a slice, keeps it known. A closure that captures such a
//! `&mut` holds it too, and lends it only to the calls of it it's given
//! to, `take()`'s `call_mut`: past its last, nothing writes the `Vec`
//! (#1946). But through a raw pointer: one given to the crate's code, or
//! to any as a closure that holds it, which may keep a pointer to it, is
//! escaped from then on (`State::escaped`), its length known from a test
//! only until a call that may write, a drop that runs code or a write
//! through a raw pointer, as a cell's is; std's code given the `&mut
//! Vec` itself keeps none (#1966).

use super::*;
use crate::lower::generic_facts::is_type_parameter;
use crate::lower::mut_refs::{holds_a_raw_pointer, lets_pointers_out_of};
use crate::lower::pointers::pointer_may_keep;
use crate::lower::recognition::{
    StdItem, StdPath, deref_call, is_std_crate, is_std_def, is_std_path, keeps_vec_length,
};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_span::def_id::DefId;

/// Of `body`, each `&mut` of a `Vec` a local holds that's given only to a
/// call, by the local holding it: the `Vec`'s local. A local is only one
/// whose every `&mut` is so; none kept that's lent any other way, its
/// `&mut` stored or given on. What a call does with one it's given, kept
/// in what it gives back or another argument, is done before the `Vec` is
/// used again, as borrowck holds, and a call that may change its length
/// forgets it (`called_on_mut`).
pub(super) fn called_on<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> IndexMap<Local, Local> {
    let is_vec = |ty: Ty<'tcx>| {
        matches!(ty.kind(), ty::Adt(adt, _)
            if is_std_def(tcx, adt.did(), StdItem::Vec) || is_std_def(tcx, adt.did(), StdItem::VecDeque))
    };
    // A `&mut Vec` local, a parameter's `targets: &mut Vec<u32>`: of the
    // `Vec` it points to, the one way to it while it lives.
    let is_mut_of_vec = |ty: Ty<'tcx>| matches!(*ty.kind(), ty::Ref(_, to, ty::Mutability::Mut) if is_vec(to));
    // Each `&mut` of a `Vec` local, its own place whole, `&mut v`, or of
    // what a `&mut Vec` local points to, `&mut (*targets)`, or a reborrow
    // of one, `&mut (*t)`: by the local holding it.
    let mut borrows: IndexMap<Local, Local> = IndexMap::default();
    let mut lent_otherwise: IndexSet<Local> = IndexSet::default();
    // The closures that hold one, `{closure}(move _t)` of a `&mut` held,
    // each a holder of its own (#1946).
    let mut closures: IndexSet<Local> = IndexSet::default();
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            match &assign.1 {
                Rvalue::Aggregate(kind, operands) if matches!(**kind, mir::AggregateKind::Closure(..)) => {
                    for operand in operands {
                        if let Operand::Move(moved) = operand
                            && let Some(held) = moved.as_local()
                            && let Some(&of) = borrows.get(&held)
                        {
                            match assign.0.as_local() {
                                Some(closure) => {
                                    borrows.insert(closure, of);
                                    closures.insert(closure);
                                }
                                None => {
                                    lent_otherwise.insert(of);
                                }
                            }
                        }
                    }
                }
                // `&mut take` of a closure that holds one, given its
                // `call_mut`: a holder of the same.
                Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of)
                    if of.projection.is_empty() && closures.contains(&of.local) =>
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
                // One cast to what it's given as, `&mut take as &mut dyn
                // FnMut()` of a closure that holds one, or `&mut v as &mut
                // [u32]`: a holder of the same, a call given it one of its
                // `&mut` (#1947).
                Rvalue::Cast(
                    mir::CastKind::PointerCoercion(PointerCoercion::Unsize, _),
                    Operand::Copy(of) | Operand::Move(of),
                    _,
                ) if of.projection.is_empty() && borrows.contains_key(&of.local) => match assign.0.as_local() {
                    Some(holder) => {
                        borrows.insert(holder, borrows[&of.local]);
                    }
                    None => {
                        lent_otherwise.insert(borrows[&of.local]);
                    }
                },
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
                if mentions(statement, *holder) && !reborrows(statement, *holder, &closures, &borrows) {
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

/// Whether `statement` reads or writes `local` at all. Not borrowck's
/// `FakeRead` of a `let`, `let r = &mut v;`'s or `let mut take = || ..;`'s,
/// which neither writes it nor copies it anywhere (#1946).
fn mentions(statement: &mir::Statement<'_>, local: Local) -> bool {
    use rustc_middle::mir::visit::{PlaceContext, Visitor};
    if matches!(statement.kind, StatementKind::FakeRead(..)) {
        return false;
    }
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
/// one place it's allowed in a statement; or moves it into a closure that
/// holds it, or borrows such a closure `&mut` whole (#1946).
fn reborrows(
    statement: &mir::Statement<'_>,
    local: Local,
    closures: &IndexSet<Local>,
    borrows: &IndexMap<Local, Local>,
) -> bool {
    matches!(&statement.kind, StatementKind::Assign(assign)
        if assign.0.as_local() == Some(local)
            || matches!(&assign.1, Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of)
                if of.local == local && matches!(of.projection.as_slice(), [mir::PlaceElem::Deref]))
            || matches!(&assign.1, Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of)
                if of.local == local && of.projection.is_empty() && closures.contains(&local)
                    && assign.0.as_local().is_some_and(|holder| borrows.contains_key(&holder)))
            || matches!(&assign.1, Rvalue::Aggregate(kind, _) if matches!(**kind, mir::AggregateKind::Closure(..))
                && assign.0.as_local().is_some_and(|closure| closures.contains(&closure)))
            || matches!(&assign.1, Rvalue::Cast(mir::CastKind::PointerCoercion(PointerCoercion::Unsize, _), Operand::Copy(of) | Operand::Move(of), _)
                if of.local == local && of.projection.is_empty()
                    && assign.0.as_local().is_some_and(|holder| borrows.contains_key(&holder))))
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
        // Std's code given the `&mut Vec` itself keeps no pointer to it,
        // but where its signature lets one out, `ptr::from_mut(&mut v)`'s
        // result (`lets_pointers_out_of`); any given a closure that holds
        // it may (#1966). The crate's and a library's, where the session
        // knows its libraries, as `pointer_may_keep` says, which walks what
        // it runs (#1947); else, may.
        let kept = |given: Ty<'tcx>| match (func.const_fn_def(), self.libraries) {
            (Some(call), Some(libraries)) => pointer_may_keep(self.tcx, self.typing_env, &libraries, call, given),
            _ => true,
        };
        let std = func
            .const_fn_def()
            .is_some_and(|(callee, _)| is_std_crate(self.tcx, callee));
        let sig = (self.tcx).instantiate_bound_regions_with_erased(func.ty(&*self.body, self.tcx).fn_sig(self.tcx));
        for (i, arg) in args.iter().enumerate() {
            if let Some(local) = arg.node.place().and_then(|p| p.as_local())
                && let Some(&of) = self.called_on.get(&local)
            {
                after.forget_lengths(state.root(of));
                let vec = matches!(*self.body.local_decls[local].ty.kind(), ty::Ref(_, to, ty::Mutability::Mut)
                    if matches!(to.kind(), ty::Adt(adt, _)
                        if is_std_def(self.tcx, adt.did(), StdItem::Vec) || is_std_def(self.tcx, adt.did(), StdItem::VecDeque)));
                let given = self.body.local_decls[local].ty;
                let unseen = match std {
                    true => !vec,
                    false => kept(given),
                };
                if unseen || lets_pointers_out_of(self.tcx, (sig.inputs(), sig.output()), i) {
                    after.escaped.insert(state.root(of));
                }
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

impl State {
    /// What's known of each escaped `Vec`'s length forgotten (#1966). Not
    /// that a `push` made it hold an item: a push lends it `&mut`, which
    /// leaves no pointer kept before able to write it (Miri's Stacked
    /// Borrows), so none is known of an escaped one.
    pub(super) fn forget_escaped(&mut self) {
        for root in self.escaped.clone() {
            self.forget_lengths(root);
        }
    }
}

/// Whether `ty` carries code a std function given it may run, which may
/// write through a pointer kept of an escaped `Vec` (#1966): a closure, a
/// `dyn`, a function pointer, a function or a type not std's, a type
/// parameter; and, where the body's formatting may run code
/// (`formats_code`), std's `fmt::Arguments` and `Argument`, which hide
/// the `Display` impls they're made of.
fn carries_code<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, formats_code: bool) -> bool {
    ty.walk().any(|part| {
        part.as_type().is_some_and(|t| match *t.kind() {
            _ if is_type_parameter(t) => true,
            ty::Adt(adt, _) if formats_code && is_formatting(tcx, adt.did()) => true,
            ty::Adt(adt, _) => !is_std_crate(tcx, adt.did()),
            ty::FnDef(def, _) => !is_std_crate(tcx, def),
            ty::Closure(..)
            | ty::CoroutineClosure(..)
            | ty::Coroutine(..)
            | ty::Dynamic(..)
            | ty::FnPtr(..)
            | ty::Alias(..)
            | ty::Foreign(_) => true,
            _ => false,
        })
    })
}

/// Whether `id` is std's `fmt::Arguments` or `fmt::rt::Argument`.
fn is_formatting(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_std_path(tcx, id, StdPath::FmtArguments) || is_std_path(tcx, id, StdPath::RtArgument)
}

/// Whether `body`'s formatting may run code: an `Argument` made of a
/// type that carries code, `Argument::new_display::<T>`'s `T`; or an
/// `Arguments` or `Argument` made elsewhere, a parameter's or a
/// capture's, which no statement or call here makes, or a call's of code
/// not std's, or given what carries code (#1966).
pub(super) fn formats_code<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> bool {
    let formatting = |ty: Ty<'tcx>| {
        (ty.walk()).any(|part| {
            part.as_type()
                .is_some_and(|t| matches!(*t.kind(), ty::Adt(adt, _) if is_formatting(tcx, adt.did())))
        })
    };
    let mut made: IndexSet<Local> = IndexSet::default();
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            if let StatementKind::Assign(assign) = &statement.kind {
                made.insert(assign.0.local);
            }
        }
        if let TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } = &data.terminator().kind
        {
            made.insert(destination.local);
            let returns = formatting(destination.ty(body, tcx).ty);
            let code = match func.const_fn_def() {
                Some((callee, callee_args)) => {
                    !is_std_crate(tcx, callee)
                        || callee_args.types().any(|t| carries_code(tcx, t, false))
                        || (args.iter()).any(|a| carries_code(tcx, a.node.ty(body, tcx), false))
                }
                None => true,
            };
            if returns && code {
                return true;
            }
        }
    }
    (body.local_decls.iter_enumerated()).any(|(local, decl)| !made.contains(&local) && formatting(decl.ty))
}

impl<'tcx> Walk<'_, 'tcx> {
    /// Whether a call of `func` may write an escaped `Vec` through a pointer
    /// kept of it (#1966): one that may run the crate's code, a call of a
    /// `dyn`, a function pointer or code not std's, or std's given what
    /// carries code (`carries_code`); or one given what may hold a raw
    /// pointer, std's `ptr::write(p, ..)` or `NonNull::as_ptr`, which
    /// writes through it running none (`holds_a_raw_pointer`). Not a global
    /// allocator's, which JS never runs.
    pub(super) fn may_write_escaped(&self, func: &Operand<'tcx>, args: &[rustc_span::Spanned<Operand<'tcx>>]) -> bool {
        let Some((callee, callee_args)) = func.const_fn_def() else {
            return true;
        };
        let given = |a: &rustc_span::Spanned<Operand<'tcx>>| a.node.ty(&*self.body, self.tcx);
        !is_std_crate(self.tcx, callee)
            || callee_args
                .types()
                .any(|t| carries_code(self.tcx, t, self.formats_code))
            || (args.iter()).any(|a| carries_code(self.tcx, given(a), self.formats_code))
            || (args.iter()).any(|a| holds_a_raw_pointer(self.tcx, given(a), &mut Vec::new()))
    }
}
