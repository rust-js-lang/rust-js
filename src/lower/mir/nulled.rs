//! An `Option` already `null` or a value, which a `#[rust_js::nullable]`
//! place stores as it is: what's given or kept in it, each write of a
//! variable so, and what a function of the crate's returns found so
//! (ADRs 0275, 0841, 0861).

use super::*;
use crate::lower::aggregates::Given;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `operand`, an `Option`, is `null` or a value already, as a
    /// `#[rust_js::nullable]` field or binding parameter takes it as it is
    /// (ADR 0275): such a field, moved where it's read, or bound to a
    /// variable, or a `Some(..)`, or a `#[rust_js::nullable]` function's
    /// result (ADR 0841, amended).
    pub(super) fn nulled(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> bool {
        self.stored_given(state, operand) == Given::Nulled
    }

    /// Whether field `i` of `ty`, a struct, is `null` or a value wherever
    /// it's read, its `None` never `undefined`, so it's tested `=== null`
    /// (`bindings::null_or_value`, rd647, #519).
    pub(super) fn null_or_value_field(&self, ty: Ty<'tcx>, i: usize) -> bool {
        let ty::Adt(adt, _) = ty.kind() else { return false };
        let Some(field) = self.struct_field_def(ty, i) else {
            return false;
        };
        let constructed = self.krate.constructed.contains(&field.did);
        let only_the_crate_makes = self.krate.cell_borrows.pointers().only_the_crate_makes(adt.did());
        bindings::null_or_value(self.tcx, field, constructed, only_the_crate_makes)
    }

    /// Whether `place`, read in `body`, is such a field
    /// (`null_or_value_field`) or reads one: through a shared reference
    /// made once, a copy of one, or a `RefCell` field's guard, which is the
    /// field's value, `state.clones.borrow()`, or a `Deref` of it (ADR 0863,
    /// rd647). The `Option` a `let` holds of it then says its `None` is
    /// `null` (`mir_nones`).
    pub(super) fn reads_null_or_value(&self, body: &mir::Body<'tcx>, place: Place<'tcx>, depth: usize) -> bool {
        if depth > 8 {
            return false;
        }
        if let Some((base, PlaceElem::Field(field, _))) = place.as_ref().last_projection() {
            let base = base.ty(body, self.tcx);
            return base.variant_index.is_none() && self.null_or_value_field(base.ty, field.as_usize());
        }
        if !matches!(place.projection.as_slice(), [] | [PlaceElem::Deref]) {
            return false;
        }
        let local = place.local;
        let mut writes = (body.basic_blocks.iter()).flat_map(|data| {
            let statements = (data.statements.iter()).filter_map(move |statement| match &statement.kind {
                mir::StatementKind::Assign(assign) if assign.0.local == local => Some(Ok(&assign.1)),
                _ => None,
            });
            let call = match &data.terminator().kind {
                mir::TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } if destination.local == local => Some(Err((func, args))),
                _ => None,
            };
            statements.chain(call)
        });
        let (Some(write), None) = (writes.next(), writes.next()) else {
            return false;
        };
        match write {
            // `&place`, read through, and a copy of a reference.
            Ok(Rvalue::Ref(_, BorrowKind::Shared, of)) if !place.projection.is_empty() => {
                self.reads_null_or_value(body, *of, depth + 1)
            }
            Ok(Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _)) if of.projection.is_empty() => self
                .reads_null_or_value(
                    body,
                    Place::from(of.local).project_deeper(place.projection, self.tcx),
                    depth + 1,
                ),
            // A guard of a shared `borrow()`, and a `Deref` of one: what the
            // `RefCell` it's given holds.
            Err((func, args)) => {
                let (Some((def_id, generic_args)), [arg]) = (func.const_fn_def(), &args[..]) else {
                    return false;
                };
                let deref = (self.tcx.trait_of_assoc(def_id))
                    .is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Deref))
                    && matches!(generic_args.type_at(0).kind(), ty::Adt(guard, _)
                        if crate::lower::recognition::is_std_def(self.tcx, guard.did(), StdItem::RefCellRef));
                let through = match self.recognition().classify(def_id, generic_args) {
                    Some(Std::Borrow {
                        mutable: false,
                        lock: false,
                    }) => place.projection.is_empty(),
                    _ => deref && !place.projection.is_empty(),
                };
                let given = arg.node.place().filter(|given| given.projection.is_empty());
                through
                    && given.is_some_and(|given| {
                        self.reads_null_or_value(body, given.project_deeper(&[PlaceElem::Deref], self.tcx), depth + 1)
                    })
            }
            _ => false,
        }
    }

    /// `is_some()` or `is_none()` of a field that's `null` or a value, read
    /// through a reference (`reads_null_or_value`), `state.clones.borrow()
    /// .is_none()`: `=== null` (rd647). `None` of any other call.
    pub(super) fn null_or_value_tested(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        operands: &[&Operand<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let some = match self.recognition().classify(def_id, generic_args) {
            Some(Std::IsSome) => true,
            Some(Std::IsNone) => false,
            _ => return Ok(None),
        };
        let [given] = operands else { return Ok(None) };
        let Some(place) = given.place() else { return Ok(None) };
        if !self.reads_null_or_value(state.body, place.project_deeper(&[PlaceElem::Deref], self.tcx), 0) {
            return Ok(None);
        }
        let value = (self.take_operands(state, operands, out)?.into_iter().next()).expect("one operand");
        let value = self.value_expr(value, span)?;
        let value = Expr {
            facts: value.facts.with(js::Known::NoneNull),
            ..value
        };
        Ok(Some(Expr::presence(value, some)))
    }

    /// Of a call of `func` with `args`, an `as_ref()` or `as_deref()`, or a `Cell`'s `get()`, of a
    /// shared reference to a field that's `null` or a value
    /// (`reads_null_or_value`): its `None`'s, `null` (rd647).
    pub(super) fn as_ref_of_null_or_value(
        &self,
        body: &mir::Body<'tcx>,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
    ) -> Option<js::Known> {
        let (def_id, generic_args) = func.const_fn_def()?;
        let [arg] = args else { return None };
        let given = arg.node.place()?;
        let shared = matches!(given.ty(body, self.tcx).ty.kind(), ty::Ref(_, _, ty::Mutability::Not));
        (shared
            && matches!(
                self.recognition().classify(def_id, generic_args),
                Some(Std::Pointee | Std::CellGet)
            )
            && self.reads_null_or_value(body, given.project_deeper(&[PlaceElem::Deref], self.tcx), 0))
        .then_some(js::Known::NoneNull)
    }

    /// What `operand`, an `Option`, is to a `#[rust_js::nullable]` place
    /// it's stored in: `null` or a value already (`nulled`), what an
    /// optional nullable field holds, read, which another such takes as it
    /// is (ADR 0810, amended), or what may be `undefined`. Not a
    /// `WrittenNone`, which `written_none` tells.
    pub(super) fn stored_given(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> Given {
        self.stored_from(state, operand).0
    }

    /// The field a call of `func` with `args` reads whole, a `Cell` field's
    /// `get()`, `return_fiber.debug_task.get()`, of the field the `get`
    /// is lent (`borrowed`).
    pub(super) fn cell_field_got(
        &self,
        state: &State<'_, 'tcx>,
        (func, args): (&Operand<'tcx>, &[rustc_span::Spanned<Operand<'tcx>>]),
    ) -> Option<&'tcx ty::FieldDef> {
        let (def_id, generic_args) = fn_def(func.ty(state.body, self.tcx))?;
        if self.recognition().classify(def_id, generic_args) != Some(crate::lower::Std::CellGet) {
            return None;
        }
        let [arg] = args else { return None };
        self.place_field(state, self.borrowed(state, &arg.node)?)
    }

    /// Of what `stored_given` says is `null` or a value, the function's own
    /// nullable parameter it's that parameter given on, by index: `null` or
    /// a value only where that one is, each call of the function giving it
    /// so (`NulledParams`, rd624).
    pub(super) fn given_on_param(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> Option<usize> {
        self.stored_from(state, operand).1
    }

    /// `stored_given`, and the own nullable parameter it's of, given on.
    fn stored_from(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> (Given, Option<usize>) {
        self.stored_in(state, operand, &mut Vec::new())
    }

    /// `stored_from`, of the variables it's followed through, `seen`, each
    /// once.
    fn stored_in(
        &self,
        state: &State<'_, 'tcx>,
        operand: &Operand<'tcx>,
        seen: &mut Vec<Local>,
    ) -> (Given, Option<usize>) {
        let Some(place) = operand.place() else {
            return (Given::Unknown, None);
        };
        // A `#[rust_js::nullable(..)]` parameter of the function's own, `T |
        // null` as declared: what each caller gives it and the body sets it
        // to, `null` for `None` (ADR 0861, amended), the user's decision
        // (2026-10-27); a closure's binding its parameter names so too,
        // `function::with_this`'s tuple's, read where it's the tuple's item.
        if self.own_nullable_param(state, place) {
            let param = (place.projection.is_empty() && !self.tcx.is_closure_like(state.body.source.def_id()))
                .then(|| place.local.as_usize() - 1);
            return (Given::Nulled, param);
        }
        if let Some((PlaceElem::Field(..), _)) = place.projection.split_last() {
            let given = match self.place_field(state, place) {
                Some(field) if bindings::null_where_made(self.tcx, field) => Given::Nulled,
                Some(field) if bindings::is_optional(self.tcx, field) => Given::Optional,
                _ => Given::Unknown,
            };
            return (given, None);
        }
        // What a shared reference refers to, `&*instance`'s: the reference's.
        if let [PlaceElem::Deref] = place.projection.as_slice()
            && let ty::Ref(_, _, ty::Mutability::Not) = state.body.local_decls[place.local].ty.kind()
        {
            return self.stored_in(state, &Operand::Copy(Place::from(place.local)), seen);
        }
        if !place.projection.is_empty() {
            return (Given::Unknown, None);
        }
        if state.facts.options().null_nones.contains(&place.local) {
            return (Given::Nulled, None);
        }
        // Any other parameter holds what its caller gave as well as what
        // the body assigns it, and an assignment of it may read itself,
        // `x = x;`, which is followed once (`seen`).
        let local = place.local;
        if local.as_usize() <= state.body.arg_count || seen.contains(&local) {
            return (Given::Unknown, None);
        }
        seen.push(local);
        // What it's written once: another such, or `Some(..)`.
        if state.locals.writes[local] == 1 {
            let assigned = (state.body.basic_blocks.iter().flat_map(|b| &b.statements)).find_map(|s| match &s.kind {
                mir::StatementKind::Assign(assign) if assign.0 == local.into() => Some(Write::Assign(&assign.1)),
                _ => None,
            });
            let call = (state.body.basic_blocks.iter_enumerated()).find_map(|(block, b)| match &b.terminator().kind {
                mir::TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } if *destination == local.into() => Some(Write::Call(block, func, args)),
                _ => None,
            });
            return match call.or(assigned) {
                Some(write) => self.written_given(state, local, &write, seen),
                None => (Given::Unknown, None),
            };
        }
        // Written more than once, each write whole and so, never lent as a
        // `&mut`: `null` or a value too, as `if profiling { reason() } else
        // { js::null() }`'s temporary is (rd632).
        let nulled = self.whole_writes(state, local).is_some_and(|writes| {
            writes.len() == state.locals.writes[local]
                && (writes.iter())
                    .all(|write| self.written_given(state, local, write, &mut seen.clone()) == (Given::Nulled, None))
        });
        match nulled {
            true => (Given::Nulled, None),
            false => (Given::Unknown, None),
        }
    }

    /// Each write of `local`, if each is whole, an assignment or a call's
    /// result, and nothing lends it as a `&mut` or sets its variant.
    fn whole_writes<'b>(&self, state: &'b State<'_, 'tcx>, local: Local) -> Option<Vec<Write<'b, 'tcx>>> {
        let mut writes = Vec::new();
        for (block, data) in state.body.basic_blocks.iter_enumerated() {
            for statement in &data.statements {
                match &statement.kind {
                    mir::StatementKind::Assign(assign) => {
                        if assign.0.local == local {
                            if !assign.0.projection.is_empty() {
                                return None;
                            }
                            writes.push(Write::Assign(&assign.1));
                        }
                        if let Rvalue::Ref(_, BorrowKind::Mut { .. }, lent) | Rvalue::RawPtr(mir::RawPtrKind::Mut, lent) =
                            &assign.1
                            && lent.local == local
                        {
                            return None;
                        }
                    }
                    mir::StatementKind::SetDiscriminant { place, .. } if place.local == local => return None,
                    _ => {}
                }
            }
            if let mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
                && destination.local == local
            {
                if !destination.projection.is_empty() {
                    return None;
                }
                writes.push(Write::Call(block, func, args));
            }
        }
        Some(writes)
    }

    /// Whether the function lowered, `self.item`'s own, returns `null` or a
    /// value at each return, each write of its result so, which its callers
    /// then give as it is (`NulledParams::returns_nulled`, rd632): React
    /// DOM's `getEventTarget`, `target.parentNode` or `target`. Not a copy
    /// of it, an `async` one's, or a writer's.
    pub(super) fn note_nulled_returns(&self, state: &State<'_, 'tcx>) {
        let id = state.body.source.def_id();
        if id != self.item
            || self.copy.is_some()
            || state.body.coroutine.is_some()
            || state.writer.is_some()
            || !crate::lower::analysis::nulled_params::may_return_nulled(self.tcx, id)
        {
            return;
        }
        let nulled = self.whole_writes(state, mir::RETURN_PLACE).is_some_and(|writes| {
            !writes.is_empty()
                && (writes.iter()).all(|write| {
                    self.written_given(state, mir::RETURN_PLACE, write, &mut Vec::new()) == (Given::Nulled, None)
                })
        });
        if nulled {
            self.krate.nulled_params.returned_nulled(id);
        }
    }

    /// Whether the call of `func` with `args` the block lowered ends with,
    /// written to `local`, gives `null` or a value (`stored_given`).
    pub(super) fn call_nulled(
        &self,
        state: &State<'_, 'tcx>,
        local: Local,
        (func, args): (&Operand<'tcx>, &[rustc_span::Spanned<Operand<'tcx>>]),
    ) -> bool {
        let write = Write::Call(state.at, func, args);
        self.written_given(state, local, &write, &mut Vec::new()) == (Given::Nulled, None)
    }

    /// What `write` of `local` gives a nullable place (`stored_given`).
    fn written_given(
        &self,
        state: &State<'_, 'tcx>,
        local: Local,
        write: &Write<'_, 'tcx>,
        seen: &mut Vec<Local>,
    ) -> (Given, Option<usize>) {
        let (block, func, args) = match *write {
            Write::Call(block, func, args) => (block, func, args),
            Write::Assign(rvalue) => {
                return match rvalue {
                    Rvalue::Use(operand, _) => self.stored_in(state, operand, seen),
                    // A shared reference, what it refers to, a nullable
                    // field's `instance` a pattern binds by reference (ADR
                    // 1017).
                    Rvalue::Ref(_, BorrowKind::Shared, of) => self.stored_in(state, &Operand::Copy(*of), seen),
                    Rvalue::Aggregate(kind, fields)
                        if fields.len() == 1
                            && matches!(**kind, AggregateKind::Adt(did, variant, ..)
                            if variant.as_u32() == 1 && self.tcx.is_lang_item(did, LangItem::Option)) =>
                    {
                        (Given::Nulled, None)
                    }
                    _ => (Given::Unknown, None),
                };
            }
        };
        // What a `#[rust_js::nullable]` thread-local gives, read whole,
        // `KEY.with_borrow(Clone::clone)`, or taken, `KEY.take()`: what it
        // holds, `null` or a value (ADR 0949), a guard's field made of it
        // restored as it is (rd523).
        if state.facts.quiet_cells().nullable.contains(&block) {
            return (Given::Nulled, None);
        }
        let Some((def_id, generic_args)) = fn_def(func.ty(state.body, self.tcx)) else {
            return (Given::Unknown, None);
        };
        // What a `#[rust_js::nullable]` function gives, a binding's too, its
        // `None` `null` (ADRs 0578, 0638): `ReactSharedInternals.T` kept in
        // `prevTransition` (ADR 0841, amended). And what a function of the
        // crate's gives that returns so at each return (rd632).
        if crate::lower::calls::gives_null_for_none(self.tcx, def_id)
            || (crate::lower::analysis::nulled_params::may_return_nulled(self.tcx, def_id)
                && self.krate.nulled_params.returns_nulled(self.item, def_id))
        {
            return (Given::Nulled, None);
        }
        // A binding that's its argument in JS, `js::cast(Some(js::unknown(options)))`
        // of an `Option`: what it's given (rd632).
        if bindings::is_binding(self.tcx, def_id)
            && matches!(bindings::js_form(self.tcx, def_id), bindings::JsForm::This)
            && self.option_of(state.body.local_decls[local].ty).is_some()
            && let [arg] = args
            && self.option_of(arg.node.ty(state.body, self.tcx)).is_some()
        {
            return self.stored_in(state, &arg.node, seen);
        }
        let known = self.recognition().classify(def_id, generic_args);
        // An `Option::map` that's the option itself in JS is what it maps:
        // `native_event_target.map(js::unknown)` (the port's gap rd94).
        if known == Some(crate::lower::Std::OptionMap)
            && let [option, f] = args
            && self.maps_to_itself(
                f.node.ty(state.body, self.tcx),
                (generic_args.type_at(0), generic_args.type_at(1)),
            )
        {
            return self.stored_in(state, &option.node, seen);
        }
        // A shared `as_ref()` or `as_deref()` of such, the option itself in
        // JS (ADR 0023): `parent_fragment_instances.as_deref()` (rd632).
        if known == Some(crate::lower::Std::Pointee)
            && let [option] = args
            && matches!(
                option.node.ty(state.body, self.tcx).kind(),
                ty::Ref(_, _, ty::Mutability::Not)
            )
            && let Some(borrowed) = self.borrowed(state, &option.node)
        {
            return self.stored_in(state, &Operand::Copy(borrowed), seen);
        }
        // A `Cell` made of such, `Cell::new(Some(reducer))`, a nullable
        // field's, is what it's made of (ADR 0841, amended).
        if known == Some(crate::lower::Std::CellNew)
            && let [arg] = args
        {
            return self.stored_in(state, &arg.node, seen);
        }
        // A clone of such, `m.rule_id.clone()`, of an item a clone gives as
        // it is, a string's, a number's or a reference's: its JS value.
        let itself = self
            .option_of(state.body.local_decls[local].ty)
            .is_some_and(|item| item.is_ref() || item.is_primitive() || self.is_string_like(item));
        if itself
            && (self.tcx.trait_of_assoc(def_id)).is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Clone))
            && let [arg] = args
            && let Some(borrowed) = self.borrowed(state, &arg.node)
        {
            return self.stored_in(state, &Operand::Copy(borrowed), seen);
        }
        // A `Cell` field's `get()`, `return_fiber.debug_task.get()`: what the
        // field holds, as a read of a plain one (ADR 0810, amended).
        // One each constructor of its struct sets first is `null` or a value,
        // present from where it's made on (rd628).
        if let Some(field) = self.cell_field_got(state, (func, args)) {
            let given = match bindings::is_optional(self.tcx, field) {
                true => Given::Optional,
                false if self.krate.constructed.contains(&field.did) => Given::Nulled,
                false => Given::Unknown,
            };
            return (given, None);
        }
        (Given::Unknown, None)
    }

    /// Of a call of `callee` with `operands`, a `function::bind` of the
    /// crate's function with a nullable parameter, each bound, its tuple
    /// made where it's given: a call of it, given each item (rd632),
    /// `bind(complete_root, None, (root, suspended_commit_reason))`.
    pub(super) fn record_bound(
        &self,
        state: &State<'_, 'tcx>,
        callee: (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        operands: &[&Operand<'tcx>],
    ) {
        if !crate::lower::recognition::is_function_bind(self.tcx, callee.0) {
            return;
        }
        let Some(f) = crate::lower::analysis::nulled_params::bound_all(self.tcx, callee.1) else {
            return;
        };
        if !self.krate.nulled_params.may_have(f) {
            return;
        }
        let made = (operands.get(2).and_then(|tuple| tuple.place()))
            .filter(|tuple| tuple.projection.is_empty() && state.locals.writes[tuple.local] == 1)
            .and_then(|tuple| {
                (state.body.basic_blocks.iter().flat_map(|b| &b.statements)).find_map(|s| match &s.kind {
                    mir::StatementKind::Assign(assign) if assign.0 == tuple.local.into() => match &assign.1 {
                        Rvalue::Aggregate(kind, items) if matches!(**kind, AggregateKind::Tuple) => Some(items),
                        _ => None,
                    },
                    _ => None,
                })
            });
        let inputs = self.tcx.fn_sig(f).skip_binder().skip_binder().inputs().len();
        let (nulled, given_on): (Vec<bool>, Vec<Option<usize>>) = match made {
            Some(items) => (items.iter())
                .map(|item| match self.stored_from(state, item) {
                    (Given::Nulled, param) => (true, param),
                    _ => (false, None),
                })
                .unzip(),
            None => (vec![false; inputs], vec![None; inputs]),
        };
        let caller = (self.item, (state.body.source.def_id(), state.at));
        self.krate.nulled_params.record(caller, f, &nulled, &given_on);
    }

    /// Each read of a local, an `Option`, that a nullable parameter takes or
    /// that reads `null` and `undefined` alike, besides those `given` counts,
    /// and whether it's given a nullable parameter of the crate's own
    /// (rd632): a shared reference to it that a shared `as_ref()` or
    /// `as_deref()` alone reads, whose result each read is so, `given` and
    /// `toward`; its test, `is_some()`, `is_none()`, a `match`'s or an `if
    /// let`'s, a `Some`'s value read, or `map_or`'s. Not a `&mut` of it,
    /// nor any other read, which keeps it as it is.
    pub(super) fn alike_reads(
        &self,
        body: &mir::Body<'tcx>,
        locals: &Locals,
        given: &std::collections::HashMap<Local, usize>,
        toward: &std::collections::HashSet<Local>,
    ) -> Vec<(Local, bool)> {
        let option = |local: Local| self.option_of(body.local_decls[local].ty).is_some();
        let mut reads = Vec::new();
        // Each shared reference to a whole `Option`, made once.
        let mut lent: std::collections::HashMap<Local, Local> = Default::default();
        for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
            let mir::StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            match &assign.1 {
                Rvalue::Ref(_, BorrowKind::Shared, of)
                    if of.projection.is_empty() && assign.0.projection.is_empty() && option(of.local) =>
                {
                    if locals.writes[assign.0.local] == 1 {
                        lent.insert(assign.0.local, of.local);
                    }
                }
                Rvalue::Discriminant(of) if of.projection.is_empty() && option(of.local) => {
                    reads.push((of.local, false))
                }
                // A `Some`'s value, `if let Some(v) = x`'s `v`.
                Rvalue::Use(mir::Operand::Copy(of) | mir::Operand::Move(of), _)
                    if matches!(of.projection.as_slice(), [PlaceElem::Downcast(_, variant), PlaceElem::Field(..)] if variant.as_u32() == 1)
                        && option(of.local) =>
                {
                    reads.push((of.local, false));
                }
                _ => {}
            }
        }
        for block in body.basic_blocks.iter() {
            let mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &block.terminator().kind
            else {
                continue;
            };
            let Some((def_id, generic_args)) = func.const_fn_def() else {
                continue;
            };
            let Some(arg) = args
                .first()
                .and_then(|arg| arg.node.place())
                .filter(|arg| arg.projection.is_empty())
            else {
                continue;
            };
            match self.recognition().classify(def_id, generic_args) {
                // A shared `as_ref()` or `as_deref()`, the option itself in
                // JS, its result read so (ADR 0023).
                Some(Std::Pointee)
                    if destination.projection.is_empty()
                        && locals.writes[destination.local] == 1
                        && given.get(&destination.local) == Some(&locals.reads[destination.local])
                        && locals.reads[arg.local] == 1
                        && let Some(&of) = lent.get(&arg.local) =>
                {
                    reads.push((of, toward.contains(&destination.local)));
                }
                Some(Std::IsSome | Std::IsNone) if locals.reads[arg.local] == 1 && lent.contains_key(&arg.local) => {
                    reads.push((lent[&arg.local], false));
                }
                Some(Std::Comb(crate::lower::combinators::Comb::MapOr)) if option(arg.local) => {
                    reads.push((arg.local, false));
                }
                _ => {}
            }
        }
        reads
    }
}

/// A write of a local: an assignment's, or the result of the call its
/// block ends with.
enum Write<'b, 'tcx> {
    Assign(&'b Rvalue<'tcx>),
    Call(BasicBlock, &'b Operand<'tcx>, &'b [rustc_span::Spanned<Operand<'tcx>>]),
}
