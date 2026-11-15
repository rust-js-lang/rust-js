//! An `Option` already `null` or a value, which a `#[rust_js::nullable]`
//! place stores as it is: what's given or kept in it, each write of a
//! variable so, and what a function of the crate's returns found so
//! (ADRs 0275, 0841, 0861).

use super::*;
use crate::collections::{IndexMap, IndexSet};
use crate::lower::aggregates::Given;
use crate::lower::representation::NoneForm;

mod one_none;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
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

    /// What `cell`, a struct's `Cell` or `RefCell` field, holds, read
    /// whole, gives a nullable place: its `get()`, `take()` or
    /// `replace(v)`'s old value, or its guard's deref, `task.key_path
    /// .borrow()`. `null` or a value where a read of the field is never
    /// `undefined`, the representation's answer (`cell_field_read_defined`),
    /// or where each constructor of a struct JS makes sets it first
    /// (`krate.constructed`, rd628); maybe absent of an optional one (ADR
    /// 0810); unknown of any other. `None` of what isn't a struct's field
    /// (#1933).
    fn cell_field_held(&self, state: &State<'_, 'tcx>, cell: Place<'tcx>) -> Option<Given> {
        let (base, PlaceElem::Field(i, _)) = cell.as_ref().last_projection()? else {
            return None;
        };
        let base = base.ty(state.body, self.tcx);
        let field = self.place_field(state, cell)?;
        if base.variant_index.is_some() {
            return None;
        }
        Some(match bindings::is_optional(self.tcx, field) {
            true => Given::Optional,
            false
                if self.cell_field_read_defined(base.ty, i.as_usize())
                    || self.krate.constructed.contains(&field.did) =>
            {
                Given::Nulled
            }
            false => Given::Unknown,
        })
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

    /// Whether `local`, an `Option`, is proved there where the block
    /// lowered now runs: every path to it takes the edge of a test of it
    /// that `None` never takes, and `local` is a parameter nothing writes or
    /// has one whole write, which comes before the test, and is never lent
    /// as a `&mut` (#1902, #1933). The tests: `js::truthy(local)`, JS's `if
    /// (local)`, and `local.is_some()`, their true edge; `local.is_none()`,
    /// its false one; a `match` or `if let` of it, `Some`'s arm. A read of
    /// it there is `Known::Present`, so a nullable result or place stores it
    /// as it is, no `?? null`, and an exit gives it as it is, no `??
    /// undefined`.
    pub(super) fn proved_present(&self, state: &State<'_, 'tcx>, local: Local) -> bool {
        let parameter = (1..=state.body.arg_count).contains(&local.as_usize());
        if self
            .whole_writes(state, local)
            .is_none_or(|writes| writes.len() != usize::from(!parameter))
        {
            return false;
        }
        let blocks = &state.body.basic_blocks;
        let dominators = blocks.dominators();
        blocks.iter_enumerated().any(|(_, data)| {
            let TerminatorKind::SwitchInt { discr, targets } = &data.terminator().kind else {
                return false;
            };
            let Some(flag) = discr.place().filter(|p| p.projection.is_empty()) else {
                return false;
            };
            // Entered from the test alone: a `match`'s false edges, which
            // MIR's borrow check reads and no run takes, aside.
            let entered = |edge: BasicBlock| {
                blocks.predecessors()[edge]
                    .iter()
                    .filter(|&&from| {
                        !matches!(blocks[from].terminator().kind,
                            TerminatorKind::FalseEdge { imaginary_target, real_target } if imaginary_target == edge && real_target != edge)
                    })
                    .count()
                    == 1
            };
            self.present_edge(state, flag.local, targets, local)
                .is_some_and(|edge| entered(edge) && dominators.dominates(edge, state.at))
        })
    }

    /// The edge of a switch on `flag` that `local` is `Some` on: of a test
    /// `js::truthy(local)` or `local.is_some()`, its true edge; of
    /// `local.is_none()`, its false one; of `discriminant(local)`, `Some`'s.
    fn present_edge(
        &self,
        state: &State<'_, 'tcx>,
        flag: Local,
        targets: &mir::SwitchTargets,
        local: Local,
    ) -> Option<BasicBlock> {
        let values: Vec<(u128, BasicBlock)> = targets.iter().collect();
        match self.whole_writes(state, flag)?.as_slice() {
            // `js::type_of(local) == "function"`, of a kind `None`'s,
            // `"undefined"`, nor `null`'s, `"object"`, never is: its true
            // edge.
            [Write::Call(_, func, [a, b])] => {
                let [(0, _)] = values.as_slice() else {
                    return None;
                };
                let (callee, _) = func.const_fn_def()?;
                let eq = crate::lower::recognition::is_partial_eq(self.tcx, callee);
                let kind = |side: &Operand<'tcx>| self.type_of_test(state, side);
                let text = |side: &Operand<'tcx>| self.constant_text(state, side);
                let tested = match (kind(&a.node), kind(&b.node)) {
                    (Some(tested), None) => (tested, text(&b.node)?),
                    (None, Some(tested)) => (tested, text(&a.node)?),
                    _ => return None,
                };
                (eq && tested.0 == local && tested.1 != "undefined" && tested.1 != "object")
                    .then(|| targets.otherwise())
            }
            [Write::Call(_, func, [given])] => {
                // A test's: `0` its false edge, its only value, `otherwise`
                // its true one.
                let [(0, no)] = values.as_slice() else {
                    return None;
                };
                let (callee, _) = func.const_fn_def()?;
                let present = match crate::lower::recognition::is_js_truthy(self.tcx, callee) {
                    true => targets.otherwise(),
                    false => match crate::lower::recognition::option_presence_test(self.tcx, callee)? {
                        true => targets.otherwise(),
                        false => *no,
                    },
                };
                (self.tested_local(state, &given.node)? == local).then_some(present)
            }
            // `Some`'s discriminant, 1, `None`'s 0 (`Option`'s variants).
            [Write::Assign(Rvalue::Discriminant(of))] if of.projection.is_empty() && of.local == local => {
                match values.as_slice() {
                    [(0, _)] => Some(targets.otherwise()),
                    _ => values.iter().find(|(value, _)| *value == 1).map(|&(_, block)| block),
                }
            }
            _ => None,
        }
    }

    /// The local `given`, a test's argument, is of: through the copies MIR
    /// makes of it for the call, `_11 = copy _3`, the reference it lends,
    /// `_12 = &_3`, and that reference's reborrows.
    fn tested_local(&self, state: &State<'_, 'tcx>, given: &Operand<'tcx>) -> Option<Local> {
        let mut tested = given.place().filter(|p| p.projection.is_empty())?.local;
        for _ in 0..4 {
            match self.whole_writes(state, tested).as_deref() {
                Some(
                    [
                        Write::Assign(
                            Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _)
                            | Rvalue::Ref(_, BorrowKind::Shared, of),
                        ),
                    ],
                ) if of.projection.is_empty() => {
                    tested = of.local;
                }
                // A reborrow, `&(*_13)`, of what `_13` lends.
                Some([Write::Assign(Rvalue::Ref(_, BorrowKind::Shared, of))])
                    if of.projection[..] == [mir::ProjectionElem::Deref] =>
                {
                    tested = of.local;
                }
                _ => break,
            }
        }
        Some(tested)
    }

    /// The local `side`, a comparison's operand, is `js::type_of` of.
    fn type_of_test(&self, state: &State<'_, 'tcx>, side: &Operand<'tcx>) -> Option<Local> {
        let kind = self.tested_local(state, side)?;
        let writes = self.whole_writes(state, kind)?;
        let [Write::Call(_, func, [given])] = writes.as_slice() else {
            return None;
        };
        let (callee, _) = func.const_fn_def()?;
        crate::lower::recognition::is_js_type_of(self.tcx, callee).then(|| self.tested_local(state, &given.node))?
    }

    /// The text constant `side`, a comparison's operand, is, through the
    /// locals MIR holds it in and a promoted reference to it, `&"function"`.
    fn constant_text(&self, state: &State<'_, 'tcx>, side: &Operand<'tcx>) -> Option<String> {
        let constant = match side {
            Operand::Constant(c) => **c,
            _ => {
                let held = self.tested_local(state, side)?;
                let writes = self.whole_writes(state, held)?;
                let [Write::Assign(Rvalue::Use(Operand::Constant(c), _))] = writes.as_slice() else {
                    return None;
                };
                **c
            }
        };
        self.text_of(state, &constant, 0)
    }

    fn text_of(&self, state: &State<'_, 'tcx>, constant: &mir::ConstOperand<'tcx>, depth: usize) -> Option<String> {
        match constant.const_ {
            mir::Const::Val(value, ty) if ty.peel_refs().is_str() && ty.is_ref() && !ty.peel_refs().is_ref() => {
                let bytes = value.try_get_slice_bytes_for_diagnostics(self.tcx)?;
                String::from_utf8(bytes.to_vec()).ok()
            }
            mir::Const::Unevaluated(uv, _) if depth < 2 => {
                let referred = super::queries::referred_constant(&state.facts.promoted()[uv.promoted?])?;
                self.text_of(state, &referred, depth + 1)
            }
            _ => None,
        }
    }

    /// Each write of `local`, if each is whole, an assignment or a call's
    /// result, and nothing lends it as a `&mut` or sets its variant. Not a
    /// cleanup's, which writes the value a body's write of it does as the
    /// old one's drop unwinds, `active = closest(f)`'s of a `let mut active
    /// = js::null()` twice, and which the body's count of its writes leaves
    /// out (`Locals::writes`, #1933).
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
                            if !data.is_cleanup {
                                writes.push(Write::Assign(&assign.1));
                            }
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
        // A `Cell`'s or a `RefCell`'s `get()`, `take()` and `replace(v)`,
        // what the cell held (`cell_field_held`): `return_fiber.debug_task.get()`,
        // React Fizz's `let prev_key_path = task.key_path.replace(..)` (ADR
        // 0810, amended; #1933).
        if let Some(crate::lower::Std::CellGet | crate::lower::Std::CellTake | crate::lower::Std::CellReplace) = known
            && let Some(held) = (args.first())
                .and_then(|cell| self.borrowed(state, &cell.node))
                .and_then(|cell| self.cell_field_held(state, cell))
        {
            return (held, None);
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
        // What a `RefCell`'s guard derefs to, what the cell holds: of one
        // `borrow()` gave, the field it borrowed, React's `let child =
        // current.child.borrow().clone();` of a nullable `child` (#1933).
        if self
            .tcx
            .trait_of_assoc(def_id)
            .is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Deref))
            && let [guard] = args
            && let receiver = guard.node.ty(state.body, self.tcx).peel_refs()
            && (self.is_std_type(receiver, StdItem::RefCellRef) || self.is_std_type(receiver, StdItem::RefCellRefMut))
            && let Some(guard) = self.borrowed(state, &guard.node).filter(|p| p.projection.is_empty())
            && let Some([Write::Call(_, borrow, [cell])]) = self.whole_writes(state, guard.local).as_deref()
            && let Some((borrow, borrow_args)) = fn_def(borrow.ty(state.body, self.tcx))
            && matches!(
                self.recognition().classify(borrow, borrow_args),
                Some(crate::lower::Std::Borrow { lock: false, .. })
            )
            && let Some(held) = self
                .borrowed(state, &cell.node)
                .and_then(|cell| self.cell_field_held(state, cell))
        {
            return (held, None);
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
        let itself = self.option_of(state.body.local_decls[local].ty).is_some_and(|item| {
            item.is_ref()
                || item.is_primitive()
                || self.is_string_like(item)
                || self.is_std_type(item, StdItem::Rc)
                || self.is_std_type(item, StdItem::Arc)
        });
        if itself
            && (self.tcx.trait_of_assoc(def_id)).is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Clone))
            && let [arg] = args
            && let Some(borrowed) = self.borrowed(state, &arg.node)
        {
            return self.stored_in(state, &Operand::Copy(borrowed), seen);
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

/// A write of a local: an assignment's, or the result of the call its
/// block ends with.
enum Write<'b, 'tcx> {
    Assign(&'b Rvalue<'tcx>),
    Call(BasicBlock, &'b Operand<'tcx>, &'b [rustc_span::Spanned<Operand<'tcx>>]),
}
