//! Each local's name and how it's used, and the locals `pin!` and `?` bind.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Each local `pin!` pins, and the local its `Pin` is.
    fn pinned_by(&self, body: &mir::Body<'tcx>) -> Vec<(Local, Local)> {
        // The local a `&mut` is to, through its reborrows, `&mut *r`.
        let borrowed = |arg: &Operand<'tcx>, block: &mir::BasicBlockData<'tcx>| {
            let mut local = arg.place()?.as_local()?;
            for statement in block.statements.iter().rev() {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let (to, Rvalue::Ref(_, _, borrowed)) = &**assign
                    && to.as_local() == Some(local)
                {
                    match borrowed.as_local() {
                        Some(to) => return Some(to),
                        None if borrowed.projection.as_slice() == [PlaceElem::Deref] => local = borrowed.local,
                        None => return None,
                    }
                }
            }
            None
        };
        (body.basic_blocks.iter())
            .filter_map(|block| match &block.terminator().kind {
                TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } if fn_def(func.ty(&body.local_decls, self.tcx))
                    .is_some_and(|(id, _)| crate::lower::recognition::is_pin_macro_fn(self.tcx, id)) =>
                {
                    Some((borrowed(&args.first()?.node, block)?, destination.as_local()?))
                }
                _ => None,
            })
            .collect()
    }

    /// Whether `place = rvalue`, the last of `data`'s statements, is `&mut
    /// guard` of a `RefCell`'s guard given at once to `deref_mut` alone,
    /// `guard[i] = x`'s, which writes what the guard guards, never the
    /// guard itself.
    fn derefs_guard(
        &self,
        body: &mir::Body<'tcx>,
        data: &mir::BasicBlockData<'tcx>,
        place: &Place<'tcx>,
        rvalue: &Rvalue<'tcx>,
    ) -> bool {
        let Rvalue::Ref(_, BorrowKind::Mut { .. }, borrowed) = rvalue else {
            return false;
        };
        let guard = borrowed.ty(&body.local_decls, self.tcx).ty;
        let ref_cell_guard = [StdItem::RefCellRef, StdItem::RefCellRefMut]
            .into_iter()
            .any(|item| self.is_std_type(guard, item));
        let TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
            return false;
        };
        ref_cell_guard
            && borrowed.projection.is_empty()
            && matches!(&args[..], [only] if matches!(&only.node, Operand::Move(given) if given == place))
            && fn_def(func.ty(&body.local_decls, self.tcx)).is_some_and(|(id, _)| {
                (self.tcx.trait_of_assoc(id)).is_some_and(|t| self.tcx.is_lang_item(t, LangItem::DerefMut))
            })
    }

    /// Each local's name, the user's where it has one, and how it's used.
    pub(super) fn mir_locals(
        &mut self,
        body: &mir::Body<'tcx>,
        drops: Option<&drops::Elaboration<'tcx>>,
        named: &[(Local, String)],
    ) -> Locals {
        let n = body.local_decls.len();
        let mut locals = Locals {
            names: IndexVec::from_elem_n(String::new(), n),
            reads: IndexVec::from_elem_n(0, n),
            writes: IndexVec::from_elem_n(0, n),
            borrowed: IndexVec::from_elem_n(false, n),
            fields: IndexVec::from_elem_n(Vec::new(), n),
            changed: IndexVec::from_elem_n(false, n),
            declared: IndexVec::from_elem_n(false, n),
            user: IndexVec::from_elem_n(false, n),
            text: IndexVec::from_elem_n(false, n),
        };
        let mut user: IndexVec<Local, Option<String>> = IndexVec::from_elem_n(None, n);
        let sm = self.tcx.sess.source_map();
        // `?`'s own `val`, what its `Continue` holds: a temporary.
        let continued = self.continued(body);
        for info in &body.var_debug_info {
            // A variable a std macro makes, `format_args!`'s `args`, isn't one
            // the program names, nor `.await`'s `result`; a derived impl's
            // parameters, its `self` and its `f`, are.
            if let mir::VarDebugInfoContents::Place(place) = info.value
                && place.projection.is_empty()
                && user[place.local].is_none()
                && (!info.source_info.span.in_external_macro(sm) || body.local_kind(place.local) == LocalKind::Arg)
                && !info.source_info.span.is_desugaring(rustc_span::DesugaringKind::Await)
                && !continued.contains(&place.local)
            {
                // A method's `self`, named after its type, `counter` for a
                // `Counter`.
                let name = match body.local_decls[place.local].ty.peel_refs().kind() {
                    ty::Adt(adt, _) if info.name.as_str() == "self" => {
                        crate::lower::lower_first(self.tcx.item_name(adt.did()).as_str())
                    }
                    _ => info.name.to_string(),
                };
                // A parameter's without the `_` that marks it unused (ADR 0678).
                user[place.local] = Some(match body.local_kind(place.local) {
                    LocalKind::Arg => crate::lower::parameter_name(self.tcx, body.source.def_id(), &name),
                    _ => crate::lower::camel_case(&name),
                });
                locals.user[place.local] = true;
            }
        }
        // What `pin!` pins, held in a variable of std's: named after the
        // binding its `Pin` is, `block` of `let block = pin!(..)`.
        for (pinned, pin) in self.pinned_by(body) {
            if user[pinned].is_none() {
                user[pinned] = user[pin].clone();
            }
        }
        for (block, data) in body.basic_blocks.iter_enumerated() {
            if data.is_cleanup {
                continue;
            }
            let _ = block;
            for (at, statement) in data.statements.iter().enumerate() {
                match &statement.kind {
                    StatementKind::Assign(assign) => {
                        let (place, rvalue) = &**assign;
                        let derefed = at + 1 == data.statements.len() && self.derefs_guard(body, data, place, rvalue);
                        let place = match self.whole(body, place) {
                            true => &Place::from(place.local),
                            false => place,
                        };
                        count_place(&mut locals, place, true);
                        // A shared borrow of a part nothing inside can change, a
                        // match guard's `&(k as Warn).level`: the value it is.
                        let frozen = matches!(rvalue, Rvalue::Ref(_, BorrowKind::Shared | BorrowKind::Fake(_), borrowed)
                            if borrowed.ty(body, self.tcx).ty.is_freeze(self.tcx, self.typing_env));
                        // A `&mut` to a pin is the pin, which only what it
                        // points at is written through (ADR 0329).
                        let itself = derefed
                            || matches!(rvalue, Rvalue::Ref(_, BorrowKind::Mut { .. }, borrowed)
                            if self.mut_is_the_pin(borrowed.ty(&body.local_decls, self.tcx).ty));
                        count_rvalue(&mut locals, rvalue, frozen, itself);
                        // The whole through its box or `MaybeUninit` borrowed mutably,
                        // `Box::new_uninit()`'s written by `write`: as the local.
                        if let Rvalue::Ref(_, BorrowKind::Mut { .. }, borrowed) = rvalue
                            && !borrowed.projection.is_empty()
                            && self.whole(body, borrowed)
                        {
                            locals.writes[borrowed.local] += 1;
                        }
                        // A `&mut` of a value JS can't share is its place, which
                        // a write through it, a closure's say, assigns.
                        let changes = match rvalue {
                            Rvalue::Ref(_, BorrowKind::Mut { .. }, borrowed) => {
                                self.is_cell_pointee(borrowed.ty(&body.local_decls, self.tcx).ty)
                            }
                            Rvalue::RawPtr(kind, _) => kind.to_mutbl_lossy().is_mut(),
                            _ => false,
                        };
                        if changes
                            && let Rvalue::Ref(_, _, borrowed) | Rvalue::RawPtr(_, borrowed) = rvalue
                            && !borrowed.is_indirect()
                        {
                            locals.changed[borrowed.local] = true;
                        }
                    }
                    StatementKind::SetDiscriminant { place, .. } => count_place(&mut locals, place, false),
                    _ => {}
                }
            }
            match &data.terminator().kind {
                TerminatorKind::SwitchInt { discr, .. } => count_operand(&mut locals, discr),
                TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } => {
                    count_operand(&mut locals, func);
                    for arg in args {
                        count_operand(&mut locals, &arg.node);
                    }
                    count_place(&mut locals, destination, true);
                }
                // A check left out, or made where the element is read, reads
                // nothing here.
                TerminatorKind::Assert { .. } if self.assert_check(body, drops, data.terminator()).is_some() => {}
                TerminatorKind::Assert { cond, msg, .. } => {
                    count_operand(&mut locals, cond);
                    for operand in assert_operands(msg) {
                        count_operand(&mut locals, operand);
                    }
                }
                TerminatorKind::Return => locals.reads[mir::RETURN_PLACE] += 1,
                // What a drop reads, its elaboration counts (`mir_function`).
                _ => {}
            }
        }
        for local in body.local_decls.indices() {
            if body.local_decls[local].ty.is_unit() || body.local_decls[local].ty.is_never() {
                continue;
            }
            locals.text[local] = self.is_lang_adt(body.local_decls[local].ty, LangItem::String);
            // Bound by a parameter's pattern: the name THIR's pattern gives it.
            if let Some((_, name)) = named.iter().find(|(l, _)| *l == local) {
                locals.names[local] = name.clone();
                continue;
            }
            let base = match (&user[local], body.local_kind(local)) {
                (Some(name), _) => tidy::user_name(name),
                (None, LocalKind::ReturnPointer) => "result".to_string(),
                (None, _) => tidy::temporary(local.as_usize()),
            };
            locals.names[local] = self.fresh(&base);
        }
        locals
    }

    /// The locals `?` binds what a `ControlFlow::Continue` holds to, `val`.
    fn continued(&self, body: &mir::Body<'tcx>) -> Vec<Local> {
        let mut continued = Vec::new();
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let (place, Rvalue::Use(Operand::Copy(from) | Operand::Move(from), ..)) = &**assign
                    && let [PlaceElem::Downcast(_, variant), PlaceElem::Field(..)] = from.projection.as_slice()
                    && let ty::Adt(adt, _) = body.local_decls[from.local].ty.kind()
                    && self
                        .tcx
                        .is_lang_item(adt.variant(*variant).def_id, LangItem::ControlFlowContinue)
                {
                    continued.push(place.local);
                }
            }
        }
        continued
    }

    /// Whether `local` is a temporary written where it's read: made once,
    /// read once, never borrowed. The return place too, read by `return`.
    pub(super) fn foldable(&self, state: &State<'_, 'tcx>, local: Local) -> bool {
        let kind = state.body.local_kind(local);
        matches!(kind, LocalKind::Temp | LocalKind::ReturnPointer)
            && !state.locals.user[local]
            && state.locals.writes[local] == 1
            && (state.locals.reads[local] == 1 || split(&state.locals, local))
            && !state.locals.borrowed[local]
            && !state.facts.patterned().contains(&local)
    }

    // ── Structure ───────────────────────────────────────────────────────
}

/// Whether `local` is what a guarded arm's pattern binds for the arm's
/// body, `first` of `Kind::Array(&[first, _]) if f(first) => ..`, which the
/// guard reads through a reference of its own (ADR 0751).
pub(super) fn bound_for_a_guarded_arm(body: &mir::Body<'_>, local: Local) -> bool {
    (body.local_decls.iter()).any(|decl| {
        matches!(&decl.local_info, mir::ClearCrossCrate::Set(info)
            if matches!(**info, mir::LocalInfo::User(mir::BindingForm::RefForGuard(arm)) if arm == local))
    })
}
