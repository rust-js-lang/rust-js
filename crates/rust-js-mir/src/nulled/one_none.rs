//! Rust's one `None` where it leaves for JS as `undefined` alone, which JS
//! tells from `null`: what may hold a `None` JS gave given `?? undefined` at
//! each exit, a binding's optional argument, what a function JS calls
//! returns, a struct's plain field, a JS callback's argument (#1840, #1843,
//! ADR 0030 amended); and what's made `Some(..)` or `None` where it's
//! given, which needn't be (`none_trusted`, `made_some`). Split by concept
//! from `nulled`, whose nullable places store `null` (ADRs 0275, 0841).

use super::*;

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// Whether `operand`, an `Option`, holds Rust's one `None`, `undefined`,
    /// where it's `None`, the one answer of it: not where it may hold a
    /// `None` JS gave, `null` (`nulled_params::JsNones`, #1840). A
    /// constant's is rustc's.
    /// And what's made of one, `Some(..)` or `None`, or a constant, whatever
    /// its `Some` holds, which a `None` JS gave may be inside of but isn't
    /// (`made_some_or_none`).
    pub(crate) fn none_trusted(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> bool {
        let Some(place) = operand.place() else { return true };
        self.made_some_or_none(state, place, (0, false))
            || !(self.krate.nulled_params.js_nones()).may_hold_null(self.tcx, state.body, place)
    }

    /// Whether `local` is what `jsx!` gives a prop it isn't given, each of
    /// its writes a call of react's `__omitted` (`bindings::is_omitted`, ADR
    /// 0213): Rust's `None`, never a `null` JS gave (#2112).
    pub(crate) fn omitted_prop(&self, state: &State<'_, 'tcx>, local: Local) -> bool {
        self.whole_writes(state, local).is_some_and(|writes| {
            !writes.is_empty()
                && writes.iter().all(|write| {
                    matches!(write, Write::Call(_, func, _)
                        if func.const_fn_def().is_some_and(|(callee, _)| rust_js_lower::bindings::is_omitted(self.tcx, callee)))
                })
        })
    }

    /// Whether `operand`, an `Option`, is a `Some` where it's read: a
    /// variable each write of which makes one, `Some(view.byte_length())`,
    /// or moves or copies what does, or a field so (`made_some_or_none`).
    /// Not a constant, whose JS is its value, nor a parameter.
    pub(crate) fn made_some(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> bool {
        operand
            .place()
            .is_some_and(|place| self.made_some_or_none(state, place, (0, true)))
    }

    /// Whether `place` holds what's made `Some(..)` or `None`, or a
    /// constant: a variable each write of which makes one, or moves or
    /// copies what does; and a field of a struct or a tuple a variable is
    /// made once of, `jsx!`'s props' tuple's, a field of its field too, `props.anchor.title` of `LinkProps {
    /// anchor: Anchor { title: Some("Home"), .. }, .. }`, what it's made of.
    /// Not a parameter, which holds what its caller gave before any write.
    /// `some`, only what's made `Some(..)` (`made_some`).
    fn made_some_or_none(&self, state: &State<'_, 'tcx>, place: Place<'tcx>, (depth, some): (usize, bool)) -> bool {
        let param = (1..=state.body.arg_count).contains(&place.local.as_usize());
        if depth > 8 || param || !place.projection.iter().all(|elem| matches!(elem, PlaceElem::Field(..))) {
            return false;
        }
        let field = match place.projection.first() {
            Some(PlaceElem::Field(field, _)) => Some(*field),
            _ => None,
        };
        let Some(writes) = self.whole_writes(state, place.local) else {
            return false;
        };
        !writes.is_empty() && writes.iter().all(|write| match (field, write) {
            // A `Some` of what may itself be JS's `null`, a caught one, which
            // is a `None` JS gave (ADR 0030, #1939), isn't.
            (None, Write::Assign(Rvalue::Aggregate(kind, operands))) => {
                matches!(**kind, AggregateKind::Adt(did, variant, ..) if self.tcx.is_lang_item(did, LangItem::Option)
                        && (!some || variant.as_u32() == 1))
                    && (operands.iter().filter_map(Operand::place))
                        .all(|of| !(self.krate.nulled_params.js_nones()).payload_may_be_null(self.tcx, state.body, of))
            }
            (None, Write::Assign(Rvalue::Use(Operand::Constant(c), _))) => {
                !some && c.check_static_ptr(self.tcx).is_none()
            }
            (None, Write::Assign(Rvalue::Use(Operand::Move(read) | Operand::Copy(read), _))) => {
                self.made_some_or_none(state, *read, (depth + 1, some))
            }
            // Not a nullable field's, which holds `None` as `null`.
            (Some(field), Write::Assign(Rvalue::Aggregate(kind, operands)))
                if matches!(**kind, AggregateKind::Tuple)
                    || matches!(**kind, AggregateKind::Adt(did, _, _, _, None) if self.tcx.adt_def(did).is_struct()
                        && self.plain_option_field(did, field)) =>
            {
                match &operands[field] {
                    Operand::Constant(_) => !some,
                    Operand::Move(read) | Operand::Copy(read) => {
                        let read = read.project_deeper(&place.projection[1..], self.tcx);
                        self.made_some_or_none(state, read, (depth + 1, some))
                    }
                    _ => false,
                }
            }
            _ => false,
        })
    }

    /// Whether field `field` of `adt`, a struct, holds `None` as `undefined`,
    /// as it's written, not a nullable one's `null`.
    fn plain_option_field(&self, adt: rustc_span::def_id::DefId, field: rustc_abi::FieldIdx) -> bool {
        use rust_js_lower::representation::{NonePlace, none_form};
        let field = &self.tcx.adt_def(adt).non_enum_variant().fields[field];
        none_form(self.tcx, NonePlace::Field(field, None)) == NoneForm::Undefined
    }

    /// `value`, what `operand` gives where a `None` leaves for JS as
    /// `undefined` alone, which JS tells from `null`: of an `Option` of one
    /// `None` (`nulled_params::one_none`) that may hold one JS gave
    /// (`none_trusted`), `value ?? undefined`, Rust's; else as it is
    /// (#1840, #1843).
    /// What gives no `null` as it is, by MIR (`gives_no_null`, #2112), is as
    /// it is.
    pub(crate) fn one_none_given(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>, value: Expr) -> Expr {
        let ty = operand.ty(state.body, self.tcx);
        match !self.gives_no_null(state, operand)
            && rust_js_lower::analysis::nulled_params::one_none(self.tcx, ty)
            && !self.none_trusted(state, operand)
        {
            true => Expr::bin(js::Op::Coalesce, value, Expr::js_undefined()),
            false => value,
        }
    }

    /// `value`, written to `place`, of `operand` where it's one: of the
    /// result of a function JS calls (`returns_one_none`), Rust's `None`
    /// where what's written may hold one JS gave (`one_none_given`), each
    /// write so, which the return that reads it gives as it is (#1840).
    /// What else writes it, a call's result, may where the result does.
    pub(crate) fn one_none_returned(
        &mut self,
        state: &State<'_, 'tcx>,
        place: Place<'tcx>,
        (operand, called): (Option<&Operand<'tcx>>, Option<rustc_span::def_id::DefId>),
        value: Value<'tcx>,
        span: Span,
    ) -> R<Value<'tcx>> {
        // `js::null()`'s `null`, JS's on purpose (ADR 0935), and `?`'s
        // `None`, Rust's own (`FromResidual::from_residual`), as they are.
        if place != Place::from(mir::RETURN_PLACE)
            || called.is_some_and(|called| {
                rust_js_lower::bindings::is_js_null(self.tcx, called)
                    || self.tcx.is_lang_item(called, LangItem::TryTraitFromResidual)
            })
            || state.writer.is_some()
            || !rust_js_lower::analysis::nulled_params::returns_one_none(
                self.tcx,
                self.krate.foreign,
                state.body.source.def_id(),
            )
        {
            return Ok(value);
        }
        let read = Operand::Copy(place);
        let operand = operand.unwrap_or(&read);
        if operand.place().is_some_and(|read| {
            self.krate
                .nulled_params
                .js_nones()
                .holds_written_null(self.tcx, state.body, read)
                || self.local_has_stated_option(state, read, 0)
        }) {
            return Ok(value);
        }
        let value = self.value_expr(value, span)?;
        Ok(Value::Expr(self.one_none_given(state, operand, value)))
    }

    /// Each whole write has a known outer representation. A binding may
    /// give an object with optional fields even when its own outer option
    /// has only an undefined missing value. Preserve that distinction at
    /// the return without weakening the analysis of the object's contents.
    fn local_has_stated_option(&self, state: &State<'_, 'tcx>, place: Place<'tcx>, depth: usize) -> bool {
        if depth > 8
            || (1..=state.body.arg_count).contains(&place.local.as_usize())
            || !place.projection.is_empty()
            || state.locals.changed[place.local]
        {
            return false;
        }
        if self.made_some_or_none(state, place, (0, false)) {
            return true;
        }
        self.whole_writes(state, place.local).is_some_and(|writes| {
            !writes.is_empty()
                && writes.iter().all(|write| match write {
                    Write::Call(_, function, _) => function.const_fn_def().is_some_and(|(id, _)| {
                        bindings::is_js_null(self.tcx, id) || bindings::none_undefined_result(self.tcx, id)
                    }),
                    Write::Assign(Rvalue::Use(Operand::Copy(read) | Operand::Move(read), _)) => {
                        self.local_has_stated_option(state, *read, depth + 1)
                    }
                    _ => false,
                })
        })
    }

    /// `value`, what `operand` gives `place`, a struct's plain `Option`
    /// field (`nulled_params::field_takes_one_none`), or a field
    /// `field`'s of one made: Rust's `None` where it may hold one JS gave
    /// (`one_none_given`), so what JS sees of the object and each read of
    /// the field holds Rust's (#1840). Of any other place, as it is.
    pub(crate) fn one_none_stored(
        &mut self,
        state: &State<'_, 'tcx>,
        place: Place<'tcx>,
        operand: &Operand<'tcx>,
        value: Value<'tcx>,
        span: Span,
    ) -> R<Value<'tcx>> {
        let Some((base, PlaceElem::Field(field, _))) = place.as_ref().last_projection() else {
            return Ok(value);
        };
        let base = base.ty(state.body, self.tcx);
        let ty::Adt(adt, _) = base.ty.kind() else {
            return Ok(value);
        };
        if !adt.is_struct() || base.variant_index.is_some() {
            return Ok(value);
        }
        let field = &adt.non_enum_variant().fields[field];
        let value = self.value_expr(value, span)?;
        Ok(Value::Expr(self.one_none_field(state, field, operand, value)))
    }

    /// `value`, what `operand` gives `field`, a struct's: Rust's `None`
    /// where it's a plain `Option` field (`field_takes_one_none`) and it may
    /// hold one JS gave (`one_none_given`, #1840).
    pub(crate) fn one_none_field(
        &self,
        state: &State<'_, 'tcx>,
        field: &ty::FieldDef,
        operand: &Operand<'tcx>,
        value: Expr,
    ) -> Expr {
        let ty = operand.ty(state.body, self.tcx);
        match (self.krate.nulled_params.js_nones()).field_takes_one_none(self.tcx, field, ty)
            && !self.read_of_a_shown_field(state, operand, 0)
        {
            true => self.one_none_given(state, operand, value),
            false => value,
        }
    }

    /// Whether `operand` is what a field JS sees already holds, read: of a
    /// `pub` or another crate's struct or enum, moved, copied or cloned,
    /// through variables each written once so, a reference a pattern binds
    /// to it too. Given another such field, it's as JS saw it, an untagged
    /// enum's variant made again in place of its own fields, `node[0]` kept
    /// (ADR 0262's case "none").
    fn read_of_a_shown_field(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>, depth: usize) -> bool {
        let Some(place) = operand.place() else { return false };
        if let Some((base, PlaceElem::Field(..))) = place.as_ref().last_projection() {
            let ty::Adt(adt, _) = base.ty(state.body, self.tcx).ty.kind() else {
                return false;
            };
            return !adt.did().is_local() || self.tcx.visibility(adt.did()).is_public();
        }
        let param = (1..=state.body.arg_count).contains(&place.local.as_usize());
        if depth > 4 || param || !matches!(place.projection.as_slice(), [] | [PlaceElem::Deref]) {
            return false;
        }
        let Some(writes) = self.whole_writes(state, place.local) else {
            return false;
        };
        let [write] = &writes[..] else { return false };
        match *write {
            Write::Assign(Rvalue::Use(read @ (Operand::Move(_) | Operand::Copy(_)), _))
                if place.projection.is_empty() =>
            {
                self.read_of_a_shown_field(state, read, depth + 1)
            }
            Write::Assign(Rvalue::Ref(_, BorrowKind::Shared, of)) if !place.projection.is_empty() => {
                self.read_of_a_shown_field(state, &Operand::Copy(*of), depth + 1)
            }
            Write::Call(_, func, [given])
                if place.projection.is_empty()
                    && let Some((def_id, _)) = func.const_fn_def()
                    && (self.tcx.trait_of_assoc(def_id)).is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Clone)) =>
            {
                let pointee = given
                    .node
                    .place()
                    .map(|p| p.project_deeper(&[PlaceElem::Deref], self.tcx));
                pointee.is_some_and(|pointee| self.read_of_a_shown_field(state, &Operand::Copy(pointee), depth + 1))
            }
            _ => false,
        }
    }

    /// Of a call through `fn_ptr`, a pointer that may be a JS function,
    /// given `operands` as `exprs`: each Rust's `None` (`one_none_given`)
    /// but where each function the crate makes the pointer of takes it as
    /// it is, a Rust function, which gives Rust's at its own exits, or a
    /// binding that takes it nullable, `?? null` (#1840).
    pub(crate) fn ones_given_to_a_pointer(
        &self,
        state: &State<'_, 'tcx>,
        fn_ptr: Ty<'tcx>,
        operands: &[&Operand<'tcx>],
        exprs: &mut [Expr],
    ) {
        use rust_js_lower::representation::{NonePlace, none_form};
        let candidates = self.krate.cell_borrows.pointers().may_be(self.tcx, fn_ptr);
        for (i, (e, operand)) in exprs.iter_mut().zip(operands).enumerate() {
            let taken = candidates.is_none_or(|made| {
                made.iter().any(|&(f, _)| {
                    bindings::is_binding(self.tcx, f)
                        && rust_js_lower::bindings::is_optional_param(self.tcx, f, i)
                        && (self.option_of(operand.ty(state.body, self.tcx))).is_some_and(|payload| {
                            none_form(self.tcx, NonePlace::BindingArg(f, i, Some(payload))) == NoneForm::Undefined
                        })
                })
            });
            if taken {
                *e = self.one_none_given(state, operand, std::mem::replace(e, Expr::taken()));
            }
        }
    }

    /// Whether the closure the call `state.at` ends with calls, `callee`,
    /// called with `args` and giving `output`, may be a JS function: one a
    /// pointer may be (`may_be_js`), a `dyn Fn`'s, a generic one's, whose
    /// value JS may have given, or a binding's (`JsNones`); not a value the
    /// crate's code made of its own, its closure or function, Rust's. The
    /// one answer of it, `ones_given_to_a_callback`'s and `mir_closure_call`'s
    /// call of each count (#1876).
    pub(crate) fn callback_may_be_js(
        &self,
        state: &State<'_, 'tcx>,
        (callee, args, output): (Ty<'tcx>, Ty<'tcx>, Ty<'tcx>),
    ) -> bool {
        if !(self.krate.cell_borrows.pointers()).may_be_js(self.tcx, callee, args, output) {
            return false;
        }
        let mir::TerminatorKind::Call { args: given, .. } = &state.body[state.at].terminator().kind else {
            return false;
        };
        (given.first().and_then(|called| called.node.place()))
            .is_none_or(|called| (self.krate.nulled_params.js_nones()).may_hold_null(self.tcx, state.body, called))
    }

    /// Of the call `state.at` ends with, of a closure that may be JS's,
    /// `callee`, called with `args` and giving `output` (`may_be_js`), a
    /// `dyn Fn`'s, a generic one's or a pointer's: each of `exprs`, of
    /// `tys`, an `Option` of one `None`, Rust's where the tuple they're
    /// given in may hold one JS gave (`one_none_given`, #1840).
    pub(crate) fn ones_given_to_a_callback(
        &self,
        state: &State<'_, 'tcx>,
        (callee, args, output): (Ty<'tcx>, Ty<'tcx>, Ty<'tcx>),
        (tys, present): (&[Ty<'tcx>], &[Presence]),
        exprs: &mut [Expr],
    ) {
        if !self.callback_may_be_js(state, (callee, args, output)) {
            return;
        }
        let mir::TerminatorKind::Call { args: given, .. } = &state.body[state.at].terminator().kind else {
            return;
        };
        if given.get(1).is_none_or(|tuple| self.none_trusted(state, &tuple.node)) {
            return;
        }
        for (i, (e, &ty)) in exprs.iter_mut().zip(tys).enumerate() {
            if rust_js_lower::analysis::nulled_params::one_none(self.tcx, ty)
                && !never_null(e)
                && present.get(i) != Some(&Presence::Present)
            {
                let given = std::mem::replace(e, Expr::taken());
                *e = Expr::bin(js::Op::Coalesce, given, Expr::js_undefined());
            }
        }
    }

    /// Of a call of `bind`, `function::bind`, of a binding, given `operands`
    /// as `values`: each item of the tuple it binds that the binding takes
    /// as `undefined` alone for `None`, given Rust's (`one_none_given`), as
    /// a call of it is, its `.bind(..)` kept (#1840). The tuple's items, each
    /// of its own where it's made, and else read of it.
    fn ones_bound(
        &mut self,
        state: &State<'_, 'tcx>,
        (bind, operands): (rustc_span::def_id::DefId, &[&Operand<'tcx>]),
        values: &mut [Value<'tcx>],
        span: Span,
    ) -> R<()> {
        use rust_js_lower::representation::{NonePlace, none_form};
        let Some(&ty::FnDef(f, _)) = operands.first().map(|f| f.ty(state.body, self.tcx).peel_refs().kind()) else {
            return Ok(());
        };
        let (Some(tuple), Some(given)) = (operands.get(2), values.get_mut(2)) else {
            return Ok(());
        };
        let ty::Tuple(tys) = *tuple.ty(state.body, self.tcx).kind() else {
            return Ok(());
        };
        let normalized: Vec<usize> = (0..tys.len())
            .filter(|&i| {
                let ty = tys[i];
                rust_js_lower::analysis::nulled_params::one_none(self.tcx, ty)
                    && rust_js_lower::bindings::is_optional_param(self.tcx, f, i)
                    && self.option_of(ty).is_some_and(|payload| {
                        none_form(self.tcx, NonePlace::BindingArg(f, i, Some(payload))) == NoneForm::Undefined
                    })
            })
            .collect();
        if normalized.is_empty() || !bindings::is_binding(self.tcx, f) {
            return Ok(());
        }
        let _ = bind;
        // Each item where the tuple's made, `(label, start, end)`.
        let made = (tuple.place())
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
        let items: Vec<Value<'tcx>> = match std::mem::replace(given, Value::Expr(Expr::taken())) {
            Value::List(items, _) if items.len() == tys.len() => items,
            other => {
                let tuple = self.value_expr(other, span)?;
                (0..tys.len())
                    .map(|i| Value::Expr(Expr::index(tuple.clone(), Expr::int(i as i128))))
                    .collect()
            }
        };
        let mut out = Vec::with_capacity(items.len());
        for (i, item) in items.into_iter().enumerate() {
            if !normalized.contains(&i) {
                out.push(item);
                continue;
            }
            let item = self.value_expr(item, span)?;
            let trusted = made.is_some_and(|made| self.none_trusted(state, &made[rustc_abi::FieldIdx::from_usize(i)]));
            out.push(Value::Expr(match trusted || never_null(&item) {
                true => item,
                false => Expr::bin(js::Op::Coalesce, item, Expr::js_undefined()),
            }));
        }
        *given = Value::List(out, js::Span::NONE);
        Ok(())
    }

    /// Of a call of `callee`, a trait's method, through a `dyn` JS may have
    /// made, its pair with an impl of its own: one JS gave, as what may hold
    /// a `None` JS gave may be (`JsNones`), of a trait a declaration names,
    /// `pub` or another crate's. Each argument past the receiver its method
    /// declares an `Option` is given Rust's `None` (`one_none_given`,
    /// #1840). Of a `dyn` the crate made, each impl is Rust's, which holds
    /// what it's given as it may be.
    pub(crate) fn ones_given_to_a_dyn(
        &mut self,
        state: &State<'_, 'tcx>,
        (callee, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        operands: &[&Operand<'tcx>],
        values: &mut [Value<'tcx>],
        span: Span,
    ) -> R<()> {
        let Some(t) = self.tcx.trait_of_assoc(callee) else {
            return Ok(());
        };
        let dyn_self = (generic_args.types().next()).is_some_and(|ty| matches!(ty.peel_refs().kind(), ty::Dynamic(..)));
        let js_made = (operands.first().and_then(|receiver| receiver.place()))
            .is_none_or(|receiver| (self.krate.nulled_params.js_nones()).may_hold_null(self.tcx, state.body, receiver));
        if !dyn_self
            || !js_made
            || (t.is_local() && !self.tcx.visibility(t).is_public())
            || self.tcx.fn_trait_kind_from_def_id(t).is_some()
            || self.tcx.async_fn_trait_kind_from_def_id(t).is_some()
        {
            return Ok(());
        }
        for (i, (operand, value)) in operands.iter().zip(values.iter_mut()).enumerate().skip(1) {
            if !rust_js_lower::bindings::is_optional_param(self.tcx, callee, i) {
                continue;
            }
            let given = self.value_expr(std::mem::replace(value, Value::Expr(Expr::taken())), span)?;
            *value = Value::Expr(self.one_none_given(state, operand, given));
        }
        Ok(())
    }

    /// Of a call of `callee`, std's, given a binding's function, `operands`
    /// as `values`: that function's arrow, which gives each argument it takes
    /// as `undefined` alone Rust's `None`, as std's code calls it with what
    /// it's given, `values.map(text)` of a nullable place's `None` (#1840,
    /// #1843). The function itself elsewhere, whose callers in the crate's
    /// code give Rust's (`ones_given_to_a_callback`).
    pub(crate) fn ones_given_by_std(
        &mut self,
        state: &State<'_, 'tcx>,
        callee: rustc_span::def_id::DefId,
        operands: &[&Operand<'tcx>],
        values: &mut [Value<'tcx>],
        span: Span,
    ) -> R<()> {
        if !self.recognition().in_sysroot(callee) {
            return Ok(());
        }
        for (operand, value) in operands.iter().zip(values.iter_mut()) {
            let ty::FnDef(f, args) = *operand.ty(state.body, self.tcx).peel_refs().kind() else {
                continue;
            };
            // Of one that takes an `Option` of one `None`, which its arrow
            // gives Rust's; any other, as std's lowering takes it.
            let sig = self
                .tcx
                .fn_sig(f)
                .instantiate(self.tcx, args.skip_binder())
                .skip_normalization();
            let inputs = sig.skip_binder().inputs();
            let takes = (inputs.iter().enumerate()).any(|(i, &ty)| {
                rust_js_lower::analysis::nulled_params::one_none(self.tcx, ty)
                    && rust_js_lower::bindings::is_optional_param(self.tcx, f, i)
            });
            if bindings::is_binding(self.tcx, f) && takes {
                *value = Value::Expr(self.binding_fn_pointer(f, args.skip_binder(), (span, true), span)?);
            }
        }
        Ok(())
    }

    /// Of a call of `binding`, given `operands` as `values`: each it takes
    /// as `undefined` alone for `None`, a native optional argument's, not a
    /// nullable one's, given Rust's `None` (`one_none_given`), whatever
    /// it's of, a nullable parameter's `null` too (#1843). A `Some` where
    /// it's given, `Some(view.byte_length())`'s value, is the call of each
    /// count's to know (`presence`, #1842, #2178).
    pub(crate) fn ones_given_to_a_binding(
        &mut self,
        state: &State<'_, 'tcx>,
        binding: rustc_span::def_id::DefId,
        operands: &[&Operand<'tcx>],
        values: &mut [Value<'tcx>],
        span: Span,
    ) -> R<()> {
        use rust_js_lower::representation::{NonePlace, none_form};
        if rust_js_lower::recognition::is_function_bind(self.tcx, binding) {
            return self.ones_bound(state, (binding, operands), values, span);
        }
        for (i, (operand, value)) in operands.iter().zip(values.iter_mut()).enumerate() {
            let ty = operand.ty(state.body, self.tcx);
            let Some(payload) = self.option_of(ty) else { continue };
            // A pair's, `{ value, impl }`, is given its value, `q?.value`,
            // `undefined` of either `None` already (ADR 0331).
            let pair = matches!(payload.peel_refs().kind(), ty::Dynamic(traits, ..)
                if traits.principal_def_id().is_some_and(|t| self.recognition().is_pair_trait(t)));
            if pair
                || none_form(self.tcx, NonePlace::BindingArg(binding, i, Some(payload))) != NoneForm::Undefined
                || !rust_js_lower::bindings::is_optional_param(self.tcx, binding, i)
                || self.none_trusted(state, operand)
                || !rust_js_lower::analysis::nulled_params::one_none(self.tcx, ty)
            {
                continue;
            }
            let given = self.value_expr(std::mem::replace(value, Value::Expr(Expr::taken())), span)?;
            *value = Value::Expr(self.one_none_given(state, operand, given));
        }
        Ok(())
    }
}

/// Whether `value` is given as it's written: its JS is never `null`, a
/// literal's, `undefined`'s, an object's, an array's or a function's, what's
/// given `?? undefined` already, or one known never `None`; or it's
/// `js::null()`'s `null`, JS's on purpose (ADR 0935).
pub(super) fn never_null(value: &Expr) -> bool {
    use js::ExprKind as K;
    matches!(
        value.kind,
        K::Null
            | K::Num(_)
            | K::BigInt(_)
            | K::BigUint(_)
            | K::Bool(_)
            | K::Str(_)
            | K::Written(_)
            | K::Undefined
            | K::GivenUndefined
            | K::Array(..)
            | K::Object(..)
            | K::Arrow(..)
            | K::AsyncArrow(..)
            | K::New(..)
    ) || value.has_null_fallback()
        || matches!(&value.kind, K::Binary(js::Op::Coalesce, _, given) if matches!(given.kind, K::Undefined))
}
