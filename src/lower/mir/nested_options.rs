//! The locals an `Option` of an `Option` is held in as its payload (ADR
//! 1017): its `None` `undefined`, its `Some(None)` `null` and its
//! `Some(Some(v))` `v`, where each of its reads knows what its writes made;
//! and so an `Option` of a value that's `null` but never `undefined`, its
//! `Some(v)` `v` (ADR 1017, amended).

use super::*;
use rustc_span::def_id::DefId;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `place` is a `#[rust_js::nullable]` field of an `Option` of
    /// an `Option`, WebIDL's optional nullable member, whole: held as its
    /// payload where it's made, its `None` no key, `undefined`, its
    /// `Some(None)` `null` (`bindings::holds_nested`, #470, the user's
    /// decision).
    pub(super) fn nested_field(&self, body: &mir::Body<'tcx>, place: mir::PlaceRef<'tcx>) -> bool {
        if self.held_capture_read(body, place) {
            return true;
        }
        let Some((base, PlaceElem::Field(field, _))) = place.last_projection() else {
            return false;
        };
        match base.ty(body, self.tcx).ty.kind() {
            ty::Adt(adt, _) if adt.is_struct() => {
                bindings::holds_nested(self.tcx, &adt.non_enum_variant().fields[field])
            }
            _ => false,
        }
    }

    /// Whether capture `capture` of the closure `closure` is a nested
    /// field, `entry.value` of `|| f(entry.value.flatten())`, which its
    /// maker gives held as its payload, as the field holds it (rd631): of
    /// one it borrows, `Some(true)`, what its reference refers to; of one
    /// it took by value, `Some(false)`, the capture itself.
    pub(super) fn held_capture(&self, closure: DefId, capture: FieldIdx) -> Option<bool> {
        let local = closure.as_local()?;
        if !matches!(self.tcx.type_of(closure).skip_binder().kind(), ty::Closure(..)) {
            return None;
        }
        let captured = self.tcx.closure_captures(local).get(capture.as_usize())?;
        let place = &captured.place;
        let last = place.projections.len().checked_sub(1)?;
        let rustc_middle::hir::place::ProjectionKind::Field(field, _) = place.projections[last].kind else {
            return None;
        };
        let ty::Adt(adt, _) = place.ty_before_projection(last).kind() else {
            return None;
        };
        if !adt.is_struct() || !bindings::holds_nested(self.tcx, &adt.non_enum_variant().fields[field]) {
            return None;
        }
        match captured.info.capture_kind {
            ty::UpvarCapture::ByRef(ty::BorrowKind::Immutable) => Some(true),
            ty::UpvarCapture::ByValue => Some(false),
            _ => None,
        }
    }

    /// What the closure `closure` is given of its captures, `values` of
    /// `operands`: each its body reads held as its payload (`held_capture`)
    /// as one holding it so holds it, and of one that doesn't, the box's
    /// held (`held_of_boxed`) (rd631).
    pub(super) fn held_captures(
        &mut self,
        state: &State<'_, 'tcx>,
        closure: DefId,
        operands: &[&Operand<'tcx>],
        values: Vec<Value<'tcx>>,
        out: &mut Vec<Stmt>,
    ) -> R<Vec<Value<'tcx>>> {
        (values.into_iter().enumerate())
            .map(|(i, value)| {
                let Some(by_ref) = self.held_capture(closure, FieldIdx::from_usize(i)) else {
                    return Ok(value);
                };
                // A local that holds it so; a field read whole is read as
                // the box (`mir_operand`), held again below.
                let local = operands.get(i).and_then(|operand| operand.place()?.as_local());
                if local.is_some_and(|local| self.held_nested(state, Place::from(local), 0)) {
                    // A borrow of the field: the field, read where it's
                    // used, as a shared reference's place is.
                    return Ok(match value {
                        Value::Expr(place) if by_ref && place.reads_same() => Value::Place(place),
                        value => value,
                    });
                }
                let boxed = self.value_expr(value, Span::default())?;
                Ok(Value::Expr(self.held_of_boxed(boxed, out)))
            })
            .collect()
    }

    /// Whether `place`, in a closure's body, reads a capture its maker
    /// gives held as its payload (`held_capture`): one it took by value,
    /// or what one it borrows refers to, through the environment a `Fn` or
    /// `FnMut` is given by reference or a `FnOnce` by value.
    fn held_capture_read(&self, body: &mir::Body<'tcx>, place: mir::PlaceRef<'tcx>) -> bool {
        if place.local != Local::from_u32(1) {
            return false;
        }
        let rest = (place.projection.strip_prefix(&[PlaceElem::Deref][..])).unwrap_or(place.projection);
        let [PlaceElem::Field(capture, _), ref read @ ..] = *rest else {
            return false;
        };
        match self.held_capture(body.source.def_id(), capture) {
            Some(true) => read == [PlaceElem::Deref],
            Some(false) => read.is_empty(),
            None => false,
        }
    }

    /// Whether what `place`'s first `upto` projections read is an `Option`
    /// of an `Option` held as its payload: a local `OptionWrites::nested`
    /// says, whole, or a nested field (`nested_field`).
    pub(super) fn held_nested(&self, state: &State<'_, 'tcx>, place: Place<'tcx>, upto: usize) -> bool {
        let read = mir::PlaceRef {
            local: place.local,
            projection: &place.projection[..upto],
        };
        let nested = state.facts.options().nested.contains(&place.local);
        // Through a reference to one, which is it in JS.
        (nested && (upto == 0 || (upto == 1 && place.projection[0] == PlaceElem::Deref)))
            || self.nested_field(state.body, read)
    }

    /// What a `None` or a `Some(x)`, its `operands`, makes of an `Option`
    /// of an `Option` held as its payload, a local's or a nested field's:
    /// `undefined`, and `x` with its `None` `null`, as a nullable field's
    /// is already, `previousInstance = instance`; of a value never
    /// `undefined` (`defined`), `x` (ADR 1017, amended; #470).
    pub(super) fn held_made(
        &mut self,
        state: &mut State<'_, 'tcx>,
        operands: &IndexVec<FieldIdx, Operand<'tcx>>,
        defined: bool,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        Ok(match operands.iter().next() {
            None => Expr::undefined(),
            Some(operand) if defined => self.mir_expr(state, operand, out)?,
            Some(operand) => {
                let nulled = self.stored_given(state, operand) == crate::lower::aggregates::Given::Nulled;
                let held = self.mir_expr(state, operand, out)?;
                js::Storage::NullForNone.written(held, nulled)
            }
        })
    }

    /// Whether `operand` reads an `Option` of an `Option` held as its
    /// payload whole: a local that holds it so, or a nested field.
    pub(super) fn holds_nested_operand(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> bool {
        operand
            .place()
            .is_some_and(|place| self.held_nested(state, place, place.projection.len()))
    }

    /// What a struct literal gives its `field`, `value` of `operand`: of a
    /// nested field (`bindings::holds_nested`), held as its payload, what
    /// one holding it so holds as it is, else the box's (#470).
    pub(super) fn given_field(
        &mut self,
        state: &State<'_, 'tcx>,
        field: &ty::FieldDef,
        operand: &Operand<'tcx>,
        value: Expr,
        out: &mut Vec<Stmt>,
    ) -> Expr {
        let local = operand.place().filter(|place| place.projection.is_empty());
        match bindings::holds_nested(self.tcx, field) && !local.is_some_and(|place| self.held_nested(state, place, 0)) {
            true => self.held_of_boxed(value, out),
            false => value,
        }
    }

    /// An `Option` of an `Option` held as its payload, `held`, as ADR
    /// 0051's box that code which doesn't hold it so reads: `held ===
    /// undefined ? undefined : $some(held)`, its `Some(None)` `$some(null)`.
    pub(super) fn boxed_of_held(&mut self, held: Expr, out: &mut Vec<Stmt>) -> Expr {
        let held = match held.reads_same() {
            true => held,
            false => self.spill("option", held, out),
        };
        let some = self.some(held.clone());
        Expr::cond(Expr::bin(Op::Eq, held, Expr::undefined()), Expr::undefined(), some)
    }

    /// ADR 0051's box of an `Option` of an `Option`, `boxed`, held as its
    /// payload, as a nested field holds it (`nested_field`): `undefined`
    /// of its `None`, and else what its `Some` holds, its `None` `null`,
    /// `boxed === undefined ? undefined : $someValue(boxed) ?? null`. Of
    /// what `boxed_of_held` made, what it was made of.
    pub(super) fn held_of_boxed(&mut self, boxed: Expr, out: &mut Vec<Stmt>) -> Expr {
        if let js::ExprKind::Cond(test, none, some) = &boxed.kind
            && matches!(none.kind, js::ExprKind::Undefined)
            && let js::ExprKind::Call(callee, args) = &some.kind
            && callee.helper_name() == Some("$some")
            && let [held] = args.as_slice()
            && matches!(&test.kind, js::ExprKind::Binary(Op::Eq, x, u) if **x == *held && matches!(u.kind, js::ExprKind::Undefined))
        {
            return held.clone();
        }
        if matches!(boxed.kind, js::ExprKind::Undefined) {
            return boxed;
        }
        // A `Some` just made, `$some(x)`, of an `Option` whose `None` is
        // `undefined`: `x ?? null`; and a constant that's there, itself.
        if let js::ExprKind::Call(callee, args) = &boxed.kind
            && callee.helper_name() == Some("$some")
            && let [inner] = args.as_slice()
        {
            return js::Storage::NullForNone.written(inner.clone(), false);
        }
        if boxed.is_constant() && !matches!(boxed.kind, js::ExprKind::Null) {
            return boxed;
        }
        let boxed = match boxed.reads_same() {
            true => boxed,
            false => self.spill("option", boxed, out),
        };
        let value = self.some_value(boxed.clone());
        let held = js::Storage::NullForNone.written(value, false);
        Expr::cond(Expr::bin(Op::Eq, boxed, Expr::undefined()), Expr::undefined(), held)
    }

    /// The locals `OptionWrites::nested` says (ADR 1017): each an `Option`
    /// of an `Option` whose payload never looks like `None`
    /// (`nested_payload`), or of a value never `undefined` that may be
    /// `null` (`defined_payload`), of no drop, neither a parameter nor the
    /// result, and each shared reference to one an `==` is given. Each
    /// written only `None`, `Some(..)`, another such, its `cloned()`, or a
    /// keyed read JS's read gives as it's held (`reads_held`), and read
    /// only by another such, by its discriminant, by its `Some`'s payload,
    /// `(x as Some).0`, or, through a reference, by an `==` of two such: so
    /// each read knows what its writes made. One read any other way, given
    /// to a function, returned, captured or borrowed `&mut`, isn't, nor
    /// what it's copied to or from, or compared with. And a thread-local's
    /// `with_borrow` of a closure whose own body holds its result so
    /// (`result_held`) writes one as it gives it back, each such closure
    /// the second set; of a closure whose maker holds its result so,
    /// `result`, its result is one too, given back where it returns.
    pub(super) fn mir_nested(
        &self,
        body: &mir::Body<'tcx>,
        writes: &IndexVec<Local, usize>,
        result: bool,
    ) -> (std::collections::HashSet<Local>, std::collections::HashSet<DefId>) {
        let decls = &body.local_decls;
        let mut nested: std::collections::HashSet<Local> = (decls.indices())
            .filter(|&local| local.as_usize() > body.arg_count || (result && local == mir::RETURN_PLACE))
            .filter(|&local| {
                let ty = decls[local].ty;
                (self.nested_payload(ty).is_some() || self.defined_payload(ty).is_some())
                    && !ty.needs_drop(self.tcx, self.typing_env)
            })
            .collect();
        // Each shared reference to a nested field made once, which holds
        // it as the field does (#470); and each local how many of whose
        // writes are a nested field's read, all of them of one that's the
        // field's read alone, whose `Some(None)` is its `null`.
        let assigns = || {
            (body.basic_blocks.iter().flat_map(|b| &b.statements)).filter_map(|statement| match &statement.kind {
                mir::StatementKind::Assign(assign) => assign.0.as_local().map(|to| (to, &assign.1)),
                _ => None,
            })
        };
        let field_refs: std::collections::HashSet<Local> = assigns()
            .filter(|&(to, rvalue)| {
                matches!(rvalue, Rvalue::Ref(_, BorrowKind::Shared, of)
                    if writes[to] == 1 && self.nested_field(body, of.as_ref()))
            })
            .map(|(to, _)| to)
            .collect();
        // A nested field read whole, or through a reference to it.
        let field_read = |of: &Place<'tcx>| {
            self.nested_field(body, of.as_ref())
                || (of.projection.as_slice() == [PlaceElem::Deref] && field_refs.contains(&of.local))
        };
        // What each local is written: a field's read (`None`), or a copy.
        let mut sources: HashMap<Local, Vec<Option<Local>>> = HashMap::new();
        for (to, rvalue) in assigns() {
            let source = match rvalue {
                Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _) if field_read(of) => None,
                Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _) if of.as_local().is_some() => of.as_local(),
                _ => Some(to),
            };
            sources.entry(to).or_default().push(source);
        }
        let mut of_fields: std::collections::HashSet<Local> = (sources.iter())
            .filter(|&(&local, each)| each.len() == writes[local] && !each.contains(&Some(local)))
            .map(|(&local, _)| local)
            .collect();
        loop {
            let before = of_fields.len();
            let kept: Vec<Local> = (of_fields.iter().copied())
                .filter(|local| {
                    sources[local]
                        .iter()
                        .all(|s| s.is_none_or(|from| of_fields.contains(&from)))
                })
                .collect();
            of_fields = kept.into_iter().collect();
            if of_fields.len() == before {
                break;
            }
        }
        let of_field = |local: Local| of_fields.contains(&local);
        if nested.is_empty() && field_refs.is_empty() {
            return (nested, Default::default());
        }
        // Each shared reference to one, made once.
        let mut refs: HashMap<Local, Local> = HashMap::new();
        for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
            if let mir::StatementKind::Assign(assign) = &statement.kind
                && let Some(reference) = assign.0.as_local()
                && let Rvalue::Ref(_, BorrowKind::Shared, of) = &assign.1
                && let Some(of) = of.as_local()
                && nested.contains(&of)
                && writes[reference] == 1
            {
                refs.insert(reference, of);
            }
        }
        // A read of what its `Some` holds, and through it.
        let payload = |place: &Place<'tcx>| {
            matches!(place.projection.as_slice(), [PlaceElem::Downcast(_, some), PlaceElem::Field(field, _), ..]
                if some.as_u32() == 1 && field.as_u32() == 0)
        };
        // Every mention of every local, but where it lives and dies.
        struct Mentions(Vec<(mir::Location, Local)>);
        impl<'tcx> Visitor<'tcx> for Mentions {
            fn visit_statement(&mut self, statement: &mir::Statement<'tcx>, location: mir::Location) {
                if !matches!(
                    statement.kind,
                    mir::StatementKind::StorageLive(_)
                        | mir::StatementKind::StorageDead(_)
                        | mir::StatementKind::FakeRead(..)
                        | mir::StatementKind::PlaceMention(..)
                        | mir::StatementKind::AscribeUserType(..)
                        | mir::StatementKind::Nop
                ) {
                    self.super_statement(statement, location);
                }
            }
            // A variable's name, which reads nothing.
            fn visit_var_debug_info(&mut self, _: &mir::VarDebugInfo<'tcx>) {}
            fn visit_local(&mut self, local: Local, _: PlaceContext, location: mir::Location) {
                self.0.push((location, local));
            }
        }
        let mut mentions = Mentions(Vec::new());
        mentions.visit_body(body);
        let mentions = mentions.0;
        // Where each is mentioned as it may be, and which must agree.
        let mut allowed: std::collections::HashSet<(mir::Location, Local)> = Default::default();
        let mut links: Vec<(Local, Local)> = Vec::new();
        let mut given_back: Vec<(Local, DefId)> = Vec::new();
        for (block, data) in body.basic_blocks.iter_enumerated() {
            for (statement_index, statement) in data.statements.iter().enumerate() {
                let at = mir::Location { block, statement_index };
                let mir::StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                let (to, rvalue) = (&assign.0, &assign.1);
                let whole = to.as_local().filter(|local| nested.contains(local));
                match rvalue {
                    Rvalue::Aggregate(kind, _)
                        if let Some(to) = whole
                            && matches!(**kind, AggregateKind::Adt(..)) =>
                    {
                        allowed.insert((at, to));
                    }
                    // A nested field read whole by one that holds it so,
                    // and a reference to it made once, and read through it
                    // (#470).
                    Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _)
                        if let Some(to) = whole
                            && field_read(from) =>
                    {
                        allowed.extend([(at, to), (at, from.local)]);
                    }
                    Rvalue::Discriminant(of)
                        if field_refs.contains(&of.local)
                            && of.projection.first() == Some(&PlaceElem::Deref)
                            && (of.projection.len() == 1
                                || matches!(of.projection[1..], [PlaceElem::Downcast(_, some), PlaceElem::Field(field, _), ..]
                                    if some.as_u32() == 1 && field.as_u32() == 0)) =>
                    {
                        allowed.insert((at, of.local));
                    }
                    Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _)
                    | Rvalue::Ref(_, BorrowKind::Shared, of)
                    | Rvalue::CopyForDeref(of)
                        if whole.is_none()
                            && field_refs.contains(&of.local)
                            && matches!(of.projection[..], [PlaceElem::Deref, PlaceElem::Downcast(_, some), PlaceElem::Field(field, _), ..]
                                if some.as_u32() == 1 && field.as_u32() == 0) =>
                    {
                        allowed.insert((at, of.local));
                    }
                    Rvalue::Ref(_, BorrowKind::Shared, of)
                        if let Some(reference) = to.as_local()
                            && field_refs.contains(&reference)
                            && self.nested_field(body, of.as_ref()) =>
                    {
                        allowed.insert((at, reference));
                    }
                    // One a nested field is set to, or made of.
                    Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _)
                        if self.nested_field(body, to.as_ref())
                            && let Some(from) = from.as_local()
                            && nested.contains(&from) =>
                    {
                        allowed.insert((at, from));
                    }
                    // A closure's capture of a nested field, a borrow of it
                    // or its copy, which its body reads held so
                    // (`held_capture`, rd631).
                    Rvalue::Aggregate(kind, operands) if let AggregateKind::Closure(closure, _) = **kind => {
                        for (capture, operand) in operands.iter_enumerated() {
                            if let Some(from) = operand.place().and_then(|place| place.as_local())
                                && let Some(by_ref) = self.held_capture(closure, capture)
                                && (if by_ref {
                                    field_refs.contains(&from)
                                } else {
                                    nested.contains(&from)
                                })
                            {
                                allowed.insert((at, from));
                            }
                        }
                    }
                    Rvalue::Aggregate(kind, operands)
                        if let AggregateKind::Adt(did, _, _, _, None) = **kind
                            && self.tcx.adt_def(did).is_struct() =>
                    {
                        let fields = &self.tcx.adt_def(did).non_enum_variant().fields;
                        for (field, operand) in fields.iter().zip(operands.iter()) {
                            if bindings::holds_nested(self.tcx, field)
                                && let Some(from) = operand.place().and_then(|place| place.as_local())
                                && nested.contains(&from)
                            {
                                allowed.insert((at, from));
                            }
                        }
                    }
                    Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _)
                        if let Some(to) = whole
                            && let Some(from) = from.as_local()
                            && nested.contains(&from) =>
                    {
                        allowed.extend([(at, to), (at, from)]);
                        links.push((to, from));
                    }
                    Rvalue::Ref(_, BorrowKind::Shared, of)
                        if let Some(reference) = to.as_local()
                            && let Some(of) = of.as_local()
                            && refs.get(&reference) == Some(&of) =>
                    {
                        allowed.extend([(at, reference), (at, of)]);
                    }
                    Rvalue::Discriminant(of)
                        if nested.contains(&of.local) && (of.projection.is_empty() || payload(of)) =>
                    {
                        allowed.insert((at, of.local));
                    }
                    // What its `Some` holds: its `Some`'s payload, never
                    // `None`-like; or the inner `Option` itself, whose
                    // `None` is `null` here, only where nothing tells that
                    // from the `undefined` a `None` is everywhere else
                    // (`only_tested`): a callback given it would be
                    // given `null`, and a `None` of any shape's value
                    // would be `null`, `Kind::Null`, whichever it was. A
                    // value never `undefined` is the value it is anywhere
                    // (`defined_payload`), its `null` its own.
                    Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _)
                    | Rvalue::Ref(_, BorrowKind::Shared, of)
                    | Rvalue::CopyForDeref(of)
                        if whole.is_none()
                            && nested.contains(&of.local)
                            && payload(of)
                            && (!of.projection[2..].iter().all(|elem| *elem == PlaceElem::Deref)
                                || self.defined_payload(decls[of.local].ty).is_some()
                                || of_field(of.local)
                                || to
                                    .as_local()
                                    .is_some_and(|held| self.only_tested(body, held, writes, &mentions))) =>
                    {
                        allowed.insert((at, of.local));
                    }
                    _ => {}
                }
            }
            // `is_some()` or `is_none()` of one, through a reference; and an
            // `==` of one with a constant `Some(None)`, `last == Some(None)`
            // (ADR 1017, amended).
            if let mir::TerminatorKind::Call { func, args, .. } = &data.terminator().kind
                && let Some((def_id, generic_args)) = func.const_fn_def()
            {
                let through = |operand: &Operand<'tcx>| {
                    (operand.place())
                        .and_then(|place| place.as_local())
                        .filter(|local| refs.contains_key(local) || field_refs.contains(local))
                };
                let tested = match &args[..] {
                    [a] if matches!(
                        self.recognition().classify(def_id, generic_args),
                        Some(Std::IsSome | Std::IsNone)
                    ) =>
                    {
                        through(&a.node)
                    }
                    [a, b]
                        if (self.tcx.trait_of_assoc(def_id)).is_some_and(|trait_id| {
                            matches!(
                                self.recognition().trait_call(def_id, trait_id),
                                Some(TraitCall::Equality { .. })
                            )
                        }) =>
                    {
                        match (through(&a.node), through(&b.node)) {
                            (Some(a), None) if self.nested_some_none(body, &b.node) => Some(a),
                            (None, Some(b)) if self.nested_some_none(body, &a.node) => Some(b),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                if let Some(reference) = tested {
                    allowed.insert((body.terminator_loc(block), reference));
                }
            }
            // `flatten()` of one that's a nested field's read alone, which
            // gives what it holds, its `None` `null` as the field's is
            // (#470).
            if let mir::TerminatorKind::Call { func, args, .. } = &data.terminator().kind
                && let Some((def_id, generic_args)) = func.const_fn_def()
                && self.recognition().classify(def_id, generic_args) == Some(Std::OptionFlatten)
                && let [given] = &args[..]
                && let Some(from) = given.node.place().and_then(|place| place.as_local())
                && nested.contains(&from)
                && of_field(from)
            {
                allowed.insert((body.terminator_loc(block), from));
            }
            // A keyed read JS's read itself gives as it's held
            // (`reads_held`), `map.get(key)`.
            if let mir::TerminatorKind::Call { func, destination, .. } = &data.terminator().kind
                && let Some(to) = destination.as_local()
                && nested.contains(&to)
                && let Some((def_id, args)) = func.const_fn_def()
                && self.reads_held(def_id, args, decls[to].ty)
            {
                allowed.insert((body.terminator_loc(block), to));
            }
            // What a thread-local's `with`, `with_borrow` or
            // `with_borrow_mut` gives back of a closure whose body holds its
            // result so (`result_held`): that, as it is (rd547).
            if let mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
                && let Some(to) = destination.as_local()
                && nested.contains(&to)
                && let Some((def_id, generic_args)) = func.const_fn_def()
                && matches!(
                    self.recognition().classify(def_id, generic_args),
                    Some(Std::LocalWith | Std::LocalBorrow { .. })
                )
                && let [_, f] = &args[..]
                && let ty::Closure(closure, _) = *f.node.ty(body, self.tcx).kind()
                && self.result_held(closure)
            {
                allowed.insert((body.terminator_loc(block), to));
                given_back.push((to, closure));
            }
            // Its result, given back where it returns.
            if result && matches!(data.terminator().kind, mir::TerminatorKind::Return) {
                allowed.insert((body.terminator_loc(block), mir::RETURN_PLACE));
            }
            // `cloned()` or `copied()` of one, to another, whose clone is the
            // value itself (`clone_value`'s): the value, as it's held.
            if let mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
                && let Some(to) = destination.as_local()
                && nested.contains(&to)
                && let [given] = &args[..]
                && let Some(from) = given.node.place().and_then(|place| place.as_local())
                && nested.contains(&from)
                && let Some((def_id, generic_args)) = func.const_fn_def()
                && self.recognition().classify(def_id, generic_args) == Some(Std::OptionCloned)
                && !self.needs_clone(decls[to].ty)
            {
                let at = body.terminator_loc(block);
                allowed.extend([(at, to), (at, from)]);
                links.push((to, from));
            }
            // An `==` of two of them, each through a reference, of an
            // `Option` of an `Option` (`nested_equality`'s).
            if let mir::TerminatorKind::Call { func, args, .. } = &data.terminator().kind
                && let Some((def_id, _)) = func.const_fn_def()
                && (self.tcx.trait_of_assoc(def_id)).is_some_and(|trait_id| {
                    matches!(
                        self.recognition().trait_call(def_id, trait_id),
                        Some(TraitCall::Equality { .. })
                    )
                })
                && let [a, b] = &args[..]
                && let (Some(a), Some(b)) = (a.node.place(), b.node.place())
                && let (Some(a), Some(b)) = (a.as_local(), b.as_local())
                && let (Some(&x), Some(&y)) = (refs.get(&a), refs.get(&b))
                && self.nested_payload(decls[x].ty).is_some()
            {
                let at = body.terminator_loc(block);
                allowed.extend([(at, a), (at, b)]);
                links.push((x, y));
            }
        }
        // Every other mention.
        let mut out: std::collections::HashSet<Local> = (mentions.into_iter())
            .filter(|&(at, local)| {
                (nested.contains(&local) || refs.contains_key(&local) || field_refs.contains(&local))
                    && !allowed.contains(&(at, local))
            })
            .map(|(_, local)| local)
            .collect();
        links.extend(refs.iter().map(|(&reference, &of)| (reference, of)));
        loop {
            let before = out.len();
            for &(x, y) in &links {
                if out.contains(&x) || out.contains(&y) {
                    out.extend([x, y]);
                }
            }
            if out.len() == before {
                break;
            }
        }
        nested.extend(refs.keys());
        nested.extend(field_refs.iter().copied());
        nested.retain(|local| !out.contains(local));
        let held_results = (given_back.into_iter())
            .filter(|(to, _)| nested.contains(to))
            .map(|(_, closure)| closure)
            .collect();
        (nested, held_results)
    }

    /// Whether the closure `def_id`'s own body holds what it gives back as
    /// its payload (`mir_nested` of it, its result one), each of its writes
    /// one, `|cache| cache.get(id).cloned()`: a thread-local's `with_borrow`
    /// gives it back as it is, React's Flight client's chunk cache's read
    /// (rd547, ADR 1017 amended).
    fn result_held(&self, def_id: DefId) -> bool {
        let Some(mir) = (self.krate.closures.get(&def_id)).and_then(|closure| closure.mir.as_ref()) else {
            return false;
        };
        let body = &mir.body;
        let mut writes: IndexVec<Local, usize> = IndexVec::from_elem_n(0, body.local_decls.len());
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                if let mir::StatementKind::Assign(assign) = &statement.kind {
                    writes[assign.0.local] += 1;
                }
            }
            if let mir::TerminatorKind::Call { destination, .. } = &data.terminator().kind {
                writes[destination.local] += 1;
            }
        }
        self.mir_nested(body, &writes, true).0.contains(&mir::RETURN_PLACE)
    }

    /// Whether a call of `def_id` reads a keyed collection's value as JS's
    /// read itself does where what it gives, of type `ty`, is held as its
    /// payload: `undefined` of a key it lacks, and the value of one it has,
    /// never `undefined`. A binding's of a value whose `None` it holds as
    /// `null` (ADR 0884, amended); and a std map's `get` of a value that's
    /// never `undefined`, `null` one of them (`defined_payload`), as a JS
    /// `Map` holds its values as they are, `chunkCache.get(chunkId)`, its
    /// box `$someGet`'s elsewhere (ADR 1017, amended).
    pub(super) fn reads_held(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>, ty: Ty<'tcx>) -> bool {
        bindings::reads_a_nullable_value(self.tcx, def_id, args)
            || (self.defined_payload(ty).is_some()
                && self.recognition().classify(def_id, args) == Some(Std::Map(MapOp::Get)))
    }

    /// Whether `held`, the inner `Option` a `Some` of one held as its
    /// payload holds, copied out of it or borrowed, written once, is only
    /// tested (`mir_nested`): its discriminant read, its `Some`'s payload
    /// read, or it, or a shared reference to it made once, given to
    /// `is_some()` or `is_none()`, each of which reads its `null` as the
    /// `None` it is (ADR 1017, amended).
    fn only_tested(
        &self,
        body: &mir::Body<'tcx>,
        held: Local,
        writes: &IndexVec<Local, usize>,
        mentions: &[(mir::Location, Local)],
    ) -> bool {
        let derefs = |place: &Place<'tcx>| place.projection.iter().all(|elem| elem == PlaceElem::Deref);
        let its_payload = |place: &Place<'tcx>| {
            let past: Vec<_> = (place.projection.iter())
                .skip_while(|elem| *elem == PlaceElem::Deref)
                .collect();
            matches!(past.as_slice(), [PlaceElem::Downcast(_, some), PlaceElem::Field(field, _), ..]
                if some.as_u32() == 1 && field.as_u32() == 0)
        };
        let tests = |func: &Operand<'tcx>| {
            (func.const_fn_def()).is_some_and(|(def_id, args)| {
                matches!(
                    self.recognition().classify(def_id, args),
                    Some(Std::IsSome | Std::IsNone)
                )
            })
        };
        // It, and each shared reference to it made once.
        let mut tested = std::collections::HashSet::from([held]);
        for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
            if let mir::StatementKind::Assign(assign) = &statement.kind
                && let Some(reference) = assign.0.as_local()
                && let Rvalue::Ref(_, BorrowKind::Shared, of) = &assign.1
                && of.local == held
                && derefs(of)
                && writes[reference] == 1
            {
                tested.insert(reference);
            }
        }
        writes[held] == 1
            && (mentions.iter())
                .filter(|(_, local)| tested.contains(local))
                .all(|&(at, local)| match body.stmt_at(at).left() {
                    Some(statement) => {
                        let mir::StatementKind::Assign(assign) = &statement.kind else {
                            return false;
                        };
                        let (to, rvalue) = (&assign.0, &assign.1);
                        // Its one write, and what's given to a test.
                        if to.as_local() == Some(local) {
                            return true;
                        }
                        local == held
                            && match rvalue {
                                Rvalue::Discriminant(of) => of.local == held && derefs(of),
                                Rvalue::Ref(_, BorrowKind::Shared, of)
                                    if derefs(of) && to.as_local().is_some_and(|to| tested.contains(&to)) =>
                                {
                                    of.local == held
                                }
                                Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _)
                                | Rvalue::Ref(_, BorrowKind::Shared, of)
                                | Rvalue::CopyForDeref(of) => of.local == held && its_payload(of),
                                _ => false,
                            }
                    }
                    None => matches!(
                        &body.basic_blocks[at.block].terminator().kind,
                        mir::TerminatorKind::Call { func, args, destination, .. }
                            if tests(func)
                                && destination.local != local
                                && matches!(&args[..], [arg] if arg.node.place() == Some(Place::from(local)))
                    ),
                })
    }
}
