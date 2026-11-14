//! An `Option` already `null` or a value, which a `#[rust_js::nullable]`
//! place stores as it is: what's given or kept in it, each write of a
//! variable so, and what a function of the crate's returns found so
//! (ADRs 0275, 0841, 0861).

use super::*;
use crate::collections::{IndexMap, IndexSet};
use crate::lower::aggregates::Given;
use crate::lower::representation::NoneForm;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `operand`, an `Option`, holds Rust's one `None`, `undefined`,
    /// where it's `None`, the one answer of it: not where it may hold a
    /// `None` JS gave, `null` (`nulled_params::JsNones`, #1840). A
    /// constant's is rustc's.
    /// And what's made of one, `Some(..)` or `None`, or a constant, whatever
    /// its `Some` holds, which a `None` JS gave may be inside of but isn't
    /// (`made_some_or_none`).
    pub(super) fn none_trusted(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> bool {
        let Some(place) = operand.place() else { return true };
        self.made_some_or_none(state, place, 0)
            || !(self.krate.nulled_params.js_nones()).may_hold_null(self.tcx, state.body, place)
    }

    /// Whether `place` holds what's made `Some(..)` or `None`, or a
    /// constant: a variable each write of which makes one, or moves or
    /// copies what does; and a field of a struct or a tuple a variable is
    /// made once of, `jsx!`'s props' tuple's, a field of its field too, `props.anchor.title` of `LinkProps {
    /// anchor: Anchor { title: Some("Home"), .. }, .. }`, what it's made of.
    /// Not a parameter, which holds what its caller gave before any write.
    fn made_some_or_none(&self, state: &State<'_, 'tcx>, place: Place<'tcx>, depth: usize) -> bool {
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
        !writes.is_empty()
            && writes.iter().all(|write| match (field, write) {
                (None, Write::Assign(Rvalue::Aggregate(kind, _))) => {
                    matches!(**kind, AggregateKind::Adt(did, ..) if self.tcx.is_lang_item(did, LangItem::Option))
                }
                (None, Write::Assign(Rvalue::Use(Operand::Constant(c), _))) => c.check_static_ptr(self.tcx).is_none(),
                (None, Write::Assign(Rvalue::Use(Operand::Move(read) | Operand::Copy(read), _))) => {
                    self.made_some_or_none(state, *read, depth + 1)
                }
                // Not a nullable field's, which holds `None` as `null`.
                (Some(field), Write::Assign(Rvalue::Aggregate(kind, operands)))
                    if matches!(**kind, AggregateKind::Tuple)
                        || matches!(**kind, AggregateKind::Adt(did, _, _, _, None) if self.tcx.adt_def(did).is_struct()
                        && self.plain_option_field(did, field)) =>
                {
                    match &operands[field] {
                        Operand::Constant(_) => true,
                        Operand::Move(read) | Operand::Copy(read) => {
                            let read = read.project_deeper(&place.projection[1..], self.tcx);
                            self.made_some_or_none(state, read, depth + 1)
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
        use crate::lower::representation::{NonePlace, none_form};
        let field = &self.tcx.adt_def(adt).non_enum_variant().fields[field];
        none_form(self.tcx, NonePlace::Field(field, None)) == NoneForm::Undefined
    }

    /// `value`, what `operand` gives where a `None` leaves for JS as
    /// `undefined` alone, which JS tells from `null`: of an `Option` of one
    /// `None` (`nulled_params::one_none`) that may hold one JS gave
    /// (`none_trusted`), `value ?? undefined`, Rust's; else as it is
    /// (#1840, #1843).
    /// A value whose JS is never `null` (`never_null`) is as it is.
    pub(super) fn one_none_given(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>, value: Expr) -> Expr {
        let ty = operand.ty(state.body, self.tcx);
        match !never_null(&value)
            && crate::lower::analysis::nulled_params::one_none(self.tcx, ty)
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
    pub(super) fn one_none_returned(
        &mut self,
        state: &State<'_, 'tcx>,
        place: Place<'tcx>,
        operand: Option<&Operand<'tcx>>,
        value: Value<'tcx>,
        span: Span,
    ) -> R<Value<'tcx>> {
        if place != Place::from(mir::RETURN_PLACE)
            || state.writer.is_some()
            || !crate::lower::analysis::nulled_params::returns_one_none(
                self.tcx,
                self.krate.foreign,
                state.body.source.def_id(),
            )
        {
            return Ok(value);
        }
        let read = Operand::Copy(place);
        let operand = operand.unwrap_or(&read);
        let value = self.value_expr(value, span)?;
        Ok(Value::Expr(self.one_none_given(state, operand, value)))
    }

    /// `value`, what `operand` gives `place`, a struct's plain `Option`
    /// field (`nulled_params::field_takes_one_none`), or a field
    /// `field`'s of one made: Rust's `None` where it may hold one JS gave
    /// (`one_none_given`), so what JS sees of the object and each read of
    /// the field holds Rust's (#1840). Of any other place, as it is.
    pub(super) fn one_none_stored(
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
    pub(super) fn one_none_field(
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
    pub(super) fn ones_given_to_a_pointer(
        &self,
        state: &State<'_, 'tcx>,
        fn_ptr: Ty<'tcx>,
        operands: &[&Operand<'tcx>],
        exprs: &mut [Expr],
    ) {
        use crate::lower::representation::{NonePlace, none_form};
        let candidates = self.krate.cell_borrows.pointers().may_be(self.tcx, fn_ptr);
        for (i, (e, operand)) in exprs.iter_mut().zip(operands).enumerate() {
            let taken = candidates.is_none_or(|made| {
                made.iter().any(|&(f, _)| {
                    bindings::is_binding(self.tcx, f)
                        && crate::lower::analysis::nulled_params::optional_argument(self.tcx, f, i)
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

    /// Of the call `state.at` ends with, of a closure that may be JS's,
    /// `callee`, called with `args` and giving `output` (`may_be_js`), a
    /// `dyn Fn`'s, a generic one's or a pointer's: each of `exprs`, of
    /// `tys`, an `Option` of one `None`, Rust's where the tuple they're
    /// given in may hold one JS gave (`one_none_given`, #1840).
    pub(super) fn ones_given_to_a_callback(
        &self,
        state: &State<'_, 'tcx>,
        (callee, args, output): (Ty<'tcx>, Ty<'tcx>, Ty<'tcx>),
        tys: &[Ty<'tcx>],
        exprs: &mut [Expr],
    ) {
        if !(self.krate.cell_borrows.pointers()).may_be_js(self.tcx, callee, args, output) {
            return;
        }
        let mir::TerminatorKind::Call { args: given, .. } = &state.body[state.at].terminator().kind else {
            return;
        };
        // What's called, a value the crate's code made of its own, its
        // closure or function, is Rust's, which holds what it's given as it
        // may be; one JS may have given, or a binding's, JS's (`JsNones`).
        let js = (given.first().and_then(|called| called.node.place()))
            .is_none_or(|called| (self.krate.nulled_params.js_nones()).may_hold_null(self.tcx, state.body, called));
        if !js {
            return;
        }
        if given.get(1).is_none_or(|tuple| self.none_trusted(state, &tuple.node)) {
            return;
        }
        for (e, &ty) in exprs.iter_mut().zip(tys) {
            if crate::lower::analysis::nulled_params::one_none(self.tcx, ty) && !never_null(e) {
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
        use crate::lower::representation::{NonePlace, none_form};
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
                crate::lower::analysis::nulled_params::one_none(self.tcx, ty)
                    && crate::lower::analysis::nulled_params::optional_argument(self.tcx, f, i)
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
    pub(super) fn ones_given_to_a_dyn(
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
            if !crate::lower::analysis::nulled_params::optional_argument(self.tcx, callee, i) {
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
    pub(super) fn ones_given_by_std(
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
                crate::lower::analysis::nulled_params::one_none(self.tcx, ty)
                    && crate::lower::analysis::nulled_params::optional_argument(self.tcx, f, i)
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
    /// it's of, a nullable parameter's `null` too (#1843).
    pub(super) fn ones_given_to_a_binding(
        &mut self,
        state: &State<'_, 'tcx>,
        binding: rustc_span::def_id::DefId,
        operands: &[&Operand<'tcx>],
        values: &mut [Value<'tcx>],
        span: Span,
    ) -> R<()> {
        use crate::lower::representation::{NonePlace, none_form};
        if crate::lower::recognition::is_function_bind(self.tcx, binding) {
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
                || !crate::lower::analysis::nulled_params::optional_argument(self.tcx, binding, i)
                || self.none_trusted(state, operand)
                || !crate::lower::analysis::nulled_params::one_none(self.tcx, ty)
            {
                continue;
            }
            let given = self.value_expr(std::mem::replace(value, Value::Expr(Expr::taken())), span)?;
            *value = Value::Expr(self.one_none_given(state, operand, given));
        }
        Ok(())
    }

    /// Whether `operand`, an `Option`, is `null` or a value already, as a
    /// `#[rust_js::nullable]` field or binding parameter takes it as it is
    /// (ADR 0275): such a field, moved where it's read, or bound to a
    /// variable, or a `Some(..)`, or a `#[rust_js::nullable]` function's
    /// result (ADR 0841, amended).
    pub(super) fn nulled(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> bool {
        self.stored_given(state, operand) == Given::Nulled
    }

    /// What a read of field `i` of `ty`, a struct, may trust its `None`
    /// is, the owner's answer (`NoneForm::trust`, #819): of a nullable
    /// field declared an `Option` or a cell of one, `Known::NoneNull`,
    /// tested `=== null`, where the crate alone writes it, of a struct only
    /// it makes (`only_the_crate_makes`), or that each constructor of it
    /// sets first (`constructed`, rd628); JS that makes its struct may
    /// leave it out, so it's tested `== null` (rd647, #519, ADR 1072).
    pub(super) fn field_none_trust(&self, ty: Ty<'tcx>, i: usize) -> Option<js::Known> {
        use crate::lower::representation::{NonePlace, Writers, none_form};
        let ty::Adt(adt, _) = ty.kind() else { return None };
        let field = self.struct_field_def(ty, i)?;
        let declared = self.tcx.type_of(field.did).instantiate_identity().skip_normalization();
        let writers = match self.krate.constructed.contains(&field.did)
            || self.krate.cell_borrows.pointers().only_the_crate_makes(adt.did())
        {
            true => Writers::Crate,
            false => Writers::Loose,
        };
        none_form(self.tcx, NonePlace::Field(field, Some(declared))).trust(writers)
    }

    /// What `place`, read in `body`, may trust its `None` is, of a field
    /// it is (`field_none_trust`) or reads: through a shared reference made
    /// once, a copy of one, or a `RefCell` field's guard, which is the
    /// field's value, `state.clones.borrow()`, or a `Deref` of it (ADR 0863,
    /// rd647). The `Option` a `let` holds of it then says so
    /// (`mir_nones`).
    pub(super) fn read_none_trust(
        &self,
        body: &mir::Body<'tcx>,
        place: Place<'tcx>,
        depth: usize,
    ) -> Option<js::Known> {
        if depth > 8 {
            return None;
        }
        if let Some((base, PlaceElem::Field(field, _))) = place.as_ref().last_projection() {
            let base = base.ty(body, self.tcx);
            return base
                .variant_index
                .is_none()
                .then(|| self.field_none_trust(base.ty, field.as_usize()))
                .flatten();
        }
        if !matches!(place.projection.as_slice(), [] | [PlaceElem::Deref]) {
            return None;
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
            return None;
        };
        match write {
            // `&place`, read through, and a copy of a reference.
            Ok(Rvalue::Ref(_, BorrowKind::Shared, of)) if !place.projection.is_empty() => {
                self.read_none_trust(body, *of, depth + 1)
            }
            Ok(Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _)) if of.projection.is_empty() => self
                .read_none_trust(
                    body,
                    Place::from(of.local).project_deeper(place.projection, self.tcx),
                    depth + 1,
                ),
            // A guard of a shared `borrow()`, and a `Deref` of one: what the
            // `RefCell` it's given holds.
            Err((func, args)) => {
                let (Some((def_id, generic_args)), [arg]) = (func.const_fn_def(), &args[..]) else {
                    return None;
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
                let given = arg
                    .node
                    .place()
                    .filter(|given| through && given.projection.is_empty())?;
                self.read_none_trust(body, given.project_deeper(&[PlaceElem::Deref], self.tcx), depth + 1)
            }
            _ => None,
        }
    }

    /// `is_some()` or `is_none()` of a field that's `null` or a value, read
    /// through a reference (`read_none_trust`), `state.clones.borrow()
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
        let Some(none) = self.read_none_trust(state.body, place.project_deeper(&[PlaceElem::Deref], self.tcx), 0)
        else {
            return Ok(None);
        };
        let value = (self.take_operands(state, operands, out)?.into_iter().next()).expect("one operand");
        let value = self.value_expr(value, span)?;
        let value = Expr {
            facts: value.facts.with(none),
            ..value
        };
        Ok(Some(Expr::presence(value, some)))
    }

    /// Of a call of `func` with `args`, an `as_ref()` or `as_deref()`, or a `Cell`'s `get()`, of a
    /// shared reference to a field that's `null` or a value
    /// (`read_none_trust`): what its `None` may be trusted to be (rd647).
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
        let read = shared
            && matches!(
                self.recognition().classify(def_id, generic_args),
                Some(Std::Pointee | Std::CellGet)
            );
        read.then(|| self.read_none_trust(body, given.project_deeper(&[PlaceElem::Deref], self.tcx), 0))
            .flatten()
    }

    /// How `place` holds a `None`, however it's written: a
    /// `#[rust_js::nullable]` field, one a `RefCell`'s `get_mut` lent,
    /// `*t.types.get_mut()` (ADR 0863), an item of one's items, `v[i]` (ADR
    /// 0586, amended), or a parameter its function names nullable (ADR
    /// 0861), the owner's answer of each (`representation::NoneForm`,
    /// #819). The one answer of it: a write of it stores so
    /// (`NoneForm::storage`), and so does a `&mut` to it, its handle,
    /// wherever that's given (#541).
    pub(super) fn place_none_form(&self, state: &State<'_, 'tcx>, place: Place<'tcx>) -> NoneForm {
        let field = (self.place_field(state, place))
            .or_else(|| self.field_got_mut(state, place))
            .or_else(|| self.field_item(state, place));
        match field {
            Some(field) => crate::lower::representation::none_form(
                self.tcx,
                crate::lower::representation::NonePlace::Field(field, None),
            ),
            None => self.own_param_none_form(state, place),
        }
    }

    /// A `&mut` to `place`, `target` its JS: of one that stores `null` for
    /// `None` (`place_none_form`), a handle on it that stores so,
    /// held, its `value` the `&mut`'s place, whatever's written through
    /// it, a callee's box copied back too, `fiber.owner = slot.value ??
    /// null` (#541). Of any other, `target`.
    pub(super) fn nullable_ref(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        target: Expr,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let form = self.place_none_form(state, place);
        if self.option_of(place.ty(state.body, self.tcx).ty).is_none() || !form.stores_null() {
            return Ok(Value::Ref(target));
        }
        self.flush(state, out)?;
        let handle = Expr::stored_handle(target, form.storage());
        Ok(Value::Ref(self.through_handle(handle, out)))
    }

    /// What `operand`, an `Option`, is to a `#[rust_js::nullable]` place
    /// it's stored in: `null` or a value already (`nulled`), what an
    /// optional nullable field holds, read, which another such takes as it
    /// is (ADR 0810, amended), or what may be `undefined`. Not a
    /// `WrittenNone`, which `written_none` tells.
    /// The function's own nullable parameter, given on, is `null` or a value
    /// only where each call of the function gives it so (`NulledParams`,
    /// `is_nulled`): a call of the crate's gives what it's given as it is,
    /// `undefined` too (ADR 0861), so a place that stores `null` for
    /// `None`, and another crate's function that tests it `!== null`, is
    /// given `?? null` until it's found so, and lowered again once it is.
    /// A call of the crate's function with such a parameter is told it as
    /// it's given on (`given_on_param`, rd624).
    pub(super) fn stored_given(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> Given {
        match self.stored_from(state, operand) {
            (Given::Nulled, Some(param)) if !self.settled.nulled.is_nulled(state.body.source.def_id(), param) => {
                Given::Unknown
            }
            (given, _) => given,
        }
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

    /// `stored_given` as each call gave the function's own nullable
    /// parameters what's `null` or a value, and the one it's of, given on.
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
        // The `Ok` of what `catch_unwind(f)` gave, what `f` returned: `null`
        // or a value where each of `f`'s returns is, React DOM's
        // getActiveElement's `doc.activeElement ?? doc.body` (rd16). Its
        // `Err`'s, a `Box<dyn Any>`, is never an `Option` a nullable place
        // stores.
        if let [PlaceElem::Downcast(..), PlaceElem::Field(..)] = place.projection.as_slice()
            && let Some(caught) = self.caught_function(state, place.local)
        {
            return match self.returns_null_or_value(caught) {
                true => (Given::Nulled, None),
                false => (Given::Unknown, None),
            };
        }
        if let Some((PlaceElem::Field(..), _)) = place.projection.split_last() {
            let given = match self.place_field(state, place) {
                Some(field)
                    if crate::lower::representation::none_form(
                        self.tcx,
                        crate::lower::representation::NonePlace::Field(field, None),
                    ) == NoneForm::Null =>
                {
                    Given::Nulled
                }
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
    /// DOM's `getEventTarget`, `target.parentNode` or `target`. Or a
    /// closure it holds, which `catch_unwind` calls (rd16). Not a copy of
    /// it, an `async` one's, or a writer's.
    pub(super) fn note_nulled_returns(&self, state: &State<'_, 'tcx>) {
        let id = state.body.source.def_id();
        if (id != self.item && !self.tcx.is_closure_like(id))
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
            self.nulled_seen.borrow_mut().returned_nulled(id);
        }
    }

    /// Whether what a call of `def_id` gives is `null` or a value: a
    /// `#[rust_js::nullable]` function's, a binding's too, its `None` `null`
    /// (ADRs 0578, 0638), or one of the crate's, or a closure, that returns
    /// so at each return (rd632, rd16).
    fn returns_null_or_value(&self, def_id: rustc_span::def_id::DefId) -> bool {
        crate::lower::calls::gives_null_for_none(self.tcx, def_id)
            || (crate::lower::analysis::nulled_params::may_return_nulled(self.tcx, def_id)
                && (self.settled.nulled).returns_nulled(&mut self.nulled_seen.borrow_mut(), self.item, def_id))
    }

    /// The function or closure that `catch_unwind` calls whose result
    /// `local` is, written once: `catch_unwind(f)`'s `f`, in an
    /// `AssertUnwindSafe` or not (ADR 0660).
    fn caught_function(&self, state: &State<'_, 'tcx>, local: Local) -> Option<rustc_span::def_id::DefId> {
        if state.locals.writes[local] != 1 {
            return None;
        }
        let (func, args) = (state.body.basic_blocks.iter()).find_map(|b| match &b.terminator().kind {
            mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } if *destination == local.into() => Some((func, args)),
            _ => None,
        })?;
        let (def_id, _) = fn_def(func.ty(state.body, self.tcx))?;
        if crate::lower::recognition::unwind_op(self.tcx, def_id)
            != Some(crate::lower::std_types::unwind::UnwindOp::Catch)
        {
            return None;
        }
        let [f] = &args[..] else { return None };
        let f = match f.node.ty(state.body, self.tcx).kind() {
            ty::Adt(adt, args) if crate::lower::recognition::is_assert_unwind_safe(self.tcx, adt.did()) => {
                args.type_at(0)
            }
            _ => f.node.ty(state.body, self.tcx),
        };
        match f.kind() {
            ty::Closure(def_id, _) | ty::FnDef(def_id, _) => Some(*def_id),
            _ => None,
        }
    }

    /// Whether the call `block` ends with, written to `local`, an
    /// `Option`, reads a `#[rust_js::nullable]` thread-local whole: its
    /// `get()`, `take()`, `replace(v)` or `with_borrow(Clone::clone)`,
    /// which give what it holds, `null` or a value, as each write of it
    /// stores (ADRs 0586, 0861, 0949). The one answer of it: `stored_given`
    /// reads it, and the call's value is marked so (`Known::Defined`)
    /// wherever it goes, never `Known::Present`, as it may be `None`
    /// (#539).
    pub(super) fn reads_nullable_local(&self, state: &State<'_, 'tcx>, block: BasicBlock, local: Local) -> bool {
        state.facts.quiet_cells().nullable.contains_key(&block)
            && self.option_of(state.body.local_decls[local].ty).is_some()
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
        // A guard's field made of a nullable thread-local's read restored
        // as it is (rd523).
        if self.reads_nullable_local(state, block, local) {
            return (Given::Nulled, None);
        }
        let Some((def_id, generic_args)) = fn_def(func.ty(state.body, self.tcx)) else {
            return (Given::Unknown, None);
        };
        // What a `#[rust_js::nullable]` function gives, a binding's too, its
        // `None` `null` (ADRs 0578, 0638): `ReactSharedInternals.T` kept in
        // `prevTransition` (ADR 0841, amended). And what a function of the
        // crate's gives that returns so at each return (rd632).
        if self.returns_null_or_value(def_id) {
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
        // `a.or(b)` and `a.or_else(f)`, `a ?? b`: `a` where it's `Some`, a
        // value, else `b`, or what `f` returns, so `null` or a value where
        // that is, whatever `a`'s `None` is: React DOM's
        // `doc.active_element().or_else(|| doc.body())` (rd16), and
        // ReactFiberBeginWork's `component_stack.or(js::null())` of a field
        // JS may leave out (#689). Not `and`, `xor` or `filter`, whose `None`
        // is `undefined`.
        if let Some(crate::lower::Std::Comb(
            comb @ (crate::lower::combinators::Comb::Or | crate::lower::combinators::Comb::OrElse),
        )) = known
            && let [_, fallback] = args
        {
            let fallback = match comb {
                crate::lower::combinators::Comb::Or => self.stored_in(state, &fallback.node, seen).0 == Given::Nulled,
                _ => match fallback.node.ty(state.body, self.tcx).kind() {
                    ty::Closure(f, _) | ty::FnDef(f, _) => self.returns_null_or_value(*f),
                    _ => false,
                },
            };
            if fallback {
                return (Given::Nulled, None);
            }
        }
        // An `Option`'s `take()` and `replace(v)`, what its place held: `null`
        // or a value where that's a field that stores `null` for `None`,
        // `root.pending.take()` (#689), which a take sets `null` (ADR 0996).
        if let Some(crate::lower::Std::OptionTake | crate::lower::Std::OptionReplace) = known
            && let Some(place) = args.first().and_then(|taken| self.borrowed(state, &taken.node))
            && matches!(place.projection.last(), Some(PlaceElem::Field(..)))
            && self.stored_in(state, &Operand::Copy(place), seen).0 == Given::Nulled
        {
            return (Given::Nulled, None);
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
        self.nulled_call_seen(caller, f, (&nulled, &given_on));
    }

    /// The call of `callee` in `caller`'s block, giving each parameter
    /// what's `null` or a value where `nulled` says, and each the caller's
    /// own given on, as `given_on` says: what this body saw, given back
    /// (`NulledParams::record`, ADR 1148).
    pub(super) fn nulled_call_seen(
        &self,
        caller: (rustc_span::def_id::DefId, (rustc_span::def_id::DefId, mir::BasicBlock)),
        callee: rustc_span::def_id::DefId,
        (nulled, given_on): (&[bool], &[Option<usize>]),
    ) {
        let seen = &mut self.nulled_seen.borrow_mut();
        self.krate.nulled_params.record(seen, caller, callee, nulled, given_on);
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
        given: &IndexMap<Local, usize>,
        toward: &IndexSet<Local>,
    ) -> Vec<(Local, bool)> {
        let option = |local: Local| self.option_of(body.local_decls[local].ty).is_some();
        let mut reads = Vec::new();
        // Each shared reference to a whole `Option`, made once.
        let mut lent: IndexMap<Local, Local> = Default::default();
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

/// Whether `value` is given as it's written: its JS is never `null`, a
/// literal's, `undefined`'s, an object's, an array's or a function's, what's
/// given `?? undefined` already, or one known `undefined` for `None` or
/// never `None`; or it's `js::null()`'s `null`, JS's on purpose (ADR 0935).
fn never_null(value: &Expr) -> bool {
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
            | K::Array(_)
            | K::Object(..)
            | K::Arrow(..)
            | K::AsyncArrow(..)
            | K::New(..)
    ) || matches!(&value.kind, K::Binary(js::Op::Coalesce, _, given) if matches!(given.kind, K::Undefined))
        || value.facts.has(js::Known::NoneUndefined)
        || value.facts.has(js::Known::Present)
}

/// A write of a local: an assignment's, or the result of the call its
/// block ends with.
enum Write<'b, 'tcx> {
    Assign(&'b Rvalue<'tcx>),
    Call(BasicBlock, &'b Operand<'tcx>, &'b [rustc_span::Spanned<Operand<'tcx>>]),
}
