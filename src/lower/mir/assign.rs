//! An assignment: a place given a value, a `let`'s rest, and the `&mut`s
//! a call of the crate's is given.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(super) fn mir_assign(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: &Place<'tcx>,
        rvalue: &Rvalue<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        // A writer's `Formatter`, which it writes to as `f`, and its
        // `fmt::Result`, `Ok(())` or what a write gave: nothing.
        if let Some((aliases, _)) = &state.writer {
            if place.projection.is_empty()
                && (aliases.contains(&place.local) || state.builders.contains_key(&place.local))
            {
                return Ok(());
            }
            if place.local == mir::RETURN_PLACE {
                return match rvalue {
                    Rvalue::Use(..) => Ok(()),
                    Rvalue::Aggregate(kind, _) if matches!(**kind, AggregateKind::Adt(_, VariantIdx::ZERO, ..)) => {
                        Ok(())
                    }
                    // `Err(fmt::Error)`, thrown (ADR 0187).
                    Rvalue::Aggregate(kind, _) if matches!(**kind, AggregateKind::Adt(..)) => {
                        self.flush(state, out)?;
                        let thrown = Expr::call(Expr::helper("$fmtError"), Vec::new());
                        out.push(StmtKind::Expr(thrown).at(self.js_span(span)));
                        Ok(())
                    }
                    _ => Err(self.unsupported(span, "this `fmt::Result`, from its MIR")),
                };
            }
        }
        // The `()` of a block of only comments, `if c { // why }`'s, which
        // MIR has no other statement of: where its comments end, which
        // they're printed before (ADR 0753).
        if let Rvalue::Use(Operand::Constant(c), _) = rvalue
            && c.ty().is_unit()
            && let Some(end) = crate::lower::sources::comments_end(self.tcx, self.krate.sources, span)
        {
            out.push(StmtKind::Comments.at(end));
        }
        // What the function gives, a variable of its own nothing reads after:
        // that, handed over uncopied. Not of a `Cell` in it, which
        // a drop after may set.
        let handed;
        let rvalue = match rvalue {
            Rvalue::Use(Operand::Copy(from), retag)
                if *place == Place::from(mir::RETURN_PLACE)
                    && from.projection.is_empty()
                    && (from.ty(state.body, self.tcx).ty).is_freeze(self.tcx, self.typing_env) =>
            {
                handed = Rvalue::Use(Operand::Move(*from), *retag);
                &handed
            }
            _ => rvalue,
        };
        // A named constant only read or lent shared, `XS.contains(&x)`'s: the
        // one there is, uncopied, as nothing changes it.
        // Not what the function gives, which its caller may change.
        if place.projection.is_empty()
            && place.local != mir::RETURN_PLACE
            && let Rvalue::Use(Operand::Constant(c), _) = rvalue
            && let Const::Unevaluated(uv, _) = c.const_
            && uv.promoted.is_none()
            && matches!(self.tcx.def_kind(uv.def), DefKind::Const { .. })
            && self.krate.fns.contains_key(&uv.def)
            && c.ty().is_freeze(self.tcx, self.typing_env)
            && only_read(state.body, place.local)
        {
            let constant = self.fn_ref(uv.def);
            return self.mir_store(state, *place, Value::Expr(constant), span, out);
        }
        // A `&mut` a variable keeps to a variable that's a `&mut` itself,
        // `let outer = &mut list`: `list`, where it's used, so `*outer = ..`
        // gives `list` it.
        if place.projection.is_empty()
            && state.locals.user[place.local]
            && state.locals.writes[place.local] == 1
            && !state.pointed.borrowed.contains(&place.local)
            && let Rvalue::Ref(_, BorrowKind::Mut { .. }, target) = rvalue
            && target.projection.is_empty()
            && let ty::Ref(_, pointee, ty::Mutability::Mut) = state.body.local_decls[target.local].ty.kind()
            && !self.is_cell_pointee(*pointee)
        {
            let list = self.mir_place(state, *target, span, out)?;
            if matches!(list.kind, js::ExprKind::Var(_)) {
                state.refs.insert(place.local, list);
                return Ok(());
            }
        }
        // And to a part of one that's a `&mut` itself, `ref mut x` of an
        // `Option<&mut Foo>`'s: that part, where it's used. Of a place no
        // index variable reads, which borrowck keeps as it is meanwhile.
        if place.projection.is_empty()
            && state.locals.user[place.local]
            && state.locals.writes[place.local] == 1
            && !state.pointed.borrowed.contains(&place.local)
            && let Rvalue::Ref(_, BorrowKind::Mut { .. }, target) = rvalue
            && !target.projection.is_empty()
            && let ty::Ref(_, _, ty::Mutability::Mut) = *target.ty(state.body, self.tcx).ty.kind()
        {
            let part = self.mir_place(state, *target, span, out)?;
            if steady_place(&part) {
                state.refs.insert(place.local, part);
                return Ok(());
            }
        }
        // A `None` only nullable fields read: `null` (ADR 0275).
        if place.projection.is_empty()
            && state.options.null_nones.contains(&place.local)
            && let Rvalue::Aggregate(kind, _) = rvalue
            && let AggregateKind::Adt(_, variant, ..) = **kind
            && variant.as_u32() == 0
        {
            return self.mir_store(state, *place, Value::Expr(Expr::null()), span, out);
        }
        // One with a flattened field, made outside JSX, an object of its
        // fields and the flattened one's (ADR 0204).
        if let Rvalue::Aggregate(kind, _) = rvalue
            && let AggregateKind::Adt(..) = **kind
            && !state.jsx_props.contains(&place.local)
        {
            let ty = place.ty(&state.body.local_decls, self.tcx).ty;
            if bindings::has_flatten(self.tcx, ty) {
                let made = self.mir_rvalue(state, rvalue, span, out)?;
                let made = self.value_expr(made, span)?;
                let made = Value::Expr(self.flattened_object(made, ty));
                return self.mir_store(state, *place, made, span, out);
            }
        }
        // A part of a parameter its pattern binds, bound there already.
        if let Some(&arg) = state.parts.get(&place.local)
            && place.projection.is_empty()
            && matches!(rvalue, Rvalue::Use(Operand::Move(p) | Operand::Copy(p), _) if p.local == arg)
        {
            return Ok(());
        }
        // A `let`'s pattern's rest, `let P { href, rest } = p` (ADR 0291).
        if place.projection.is_empty() && state.rests.contains(&place.local) {
            return self.mir_let_rest(state, place.local, rvalue, span, out);
        }
        state.binding_var = place.projection.is_empty() && state.locals.user[place.local];
        // A flattened struct a `match`'s pattern binds, which JS's props hold
        // flat, among their other fields.
        if state.binding_var
            && let Rvalue::Use(Operand::Move(read) | Operand::Copy(read), _) = rvalue
            && let Some((base, PlaceElem::Field(field, _))) = read.as_ref().last_projection()
            && bindings::is_flatten_field(
                self.tcx,
                Place::ty_from(base.local, base.projection, &state.body.local_decls, self.tcx).ty,
                field.as_usize(),
            )
        {
            return Err(self.unsupported(
                span,
                "flattened props taken apart here: take them apart where they're given, `fn f(Props { a, anchor }: Props)`",
            ));
        }
        // A pointer nothing reads, which making does nothing (ADR 0407).
        if let Rvalue::RawPtr(..) = rvalue
            && place.projection.is_empty()
            && state.locals.reads[place.local] == 0
            && !state.locals.borrowed[place.local]
        {
            state.binding_var = false;
            return Ok(());
        }
        let value = self.mir_rvalue(state, rvalue, span, out);
        state.binding_var = false;
        let value = match rvalue {
            Rvalue::Aggregate(..) => value?,
            _ => self.nullable_written(state, *place, value?, None, span)?,
        };
        // A `#[rust_js::nullable]` field assigned, `p.title = None`: its
        // `None` `null`, as its cell's `set` writes it (ADRs 0772, 0810); a
        // `Some(..)`, or what's `null` or a value already, as it is. MIR
        // assigns what it makes through a temporary, which `nulled` follows.
        let value = match self.place_field(state, *place) {
            Some(field) if bindings::is_nullable(self.tcx, field) => {
                let nulled = match rvalue {
                    Rvalue::Use(operand, _) => self.nulled(state, operand),
                    _ => false,
                };
                Value::Expr(js::Storage::NullForNone.written(self.value_expr(value, span)?, nulled))
            }
            _ => value,
        };
        self.mir_store(state, *place, value, span, out)
    }

    /// What's written to a `#[rust_js::nullable]` function's result, where
    /// it's written (ADR 0578): a `None` it makes is `null` already, and a
    /// `Some(x)` is `x` (`null_nones`); a call of a nullable function,
    /// `made_by`, this one or another, a binding too, is `null` for `None`
    /// as it is (ADR 0638); another `Option`, a call's or a variable's, is
    /// `null` for `undefined`, `heap[0] ?? null`. Each `return` gives what
    /// was written, so it's written where it's made.
    pub(super) fn nullable_written(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        value: Value<'tcx>,
        made_by: Option<rustc_span::def_id::DefId>,
        span: Span,
    ) -> R<Value<'tcx>> {
        if place != Place::from(mir::RETURN_PLACE)
            || state.writer.is_some()
            || !bindings::nullable_result(self.tcx, state.body.source.def_id())
            || made_by.is_some_and(|made_by| bindings::nullable_result(self.tcx, made_by))
        {
            return Ok(value);
        }
        let value = self.value_expr(value, span)?;
        Ok(Value::Expr(js::Storage::NullForNone.written(value, false)))
    }

    /// `rest = p.rest` of a `let`'s pattern, `let P { href, rest, .. } = p`:
    /// what's left of the props, each of their other fields named,
    /// `const { href: _href, title: _title, ...rest } = p;`, whose names tidy
    /// makes the pattern's where it binds them.
    fn mir_let_rest(
        &mut self,
        state: &mut State<'_, 'tcx>,
        local: Local,
        rvalue: &Rvalue<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let decls = &state.body.local_decls;
        let props = match rvalue {
            Rvalue::Use(Operand::Move(read) | Operand::Copy(read), _) => match read.as_ref().last_projection() {
                Some((base, PlaceElem::Field(field, _))) => {
                    let ty = Place::ty_from(base.local, base.projection, decls, self.tcx).ty;
                    let place = Place {
                        local: base.local,
                        projection: self.tcx.mk_place_elems(base.projection),
                    };
                    bindings::is_rest_field(self.tcx, ty, field.as_usize()).then_some((place, ty))
                }
                _ => None,
            },
            _ => None,
        };
        let Some((place, ty)) = props.filter(|_| !state.locals.declared[local]) else {
            return Err(self.unsupported(span, "this rest of props, from its MIR"));
        };
        let Shape::Object(fields) = self.shape(ty) else {
            return Err(self.unsupported(span, "this rest of props, from its MIR"));
        };
        let value = self.mir_place(state, place, span, out)?;
        let tcx = self.tcx;
        let keys: Vec<&str> = (fields.iter().enumerate())
            .filter(|&(i, _)| !bindings::is_rest_field(tcx, ty, i))
            .map(|(_, (key, _))| key.as_str())
            .collect();
        let named = (keys.iter().zip(self.left_out(&keys)))
            .map(|(key, unused)| (key.to_string(), js::Pattern::Name(unused), None))
            .collect();
        self.flush(state, out)?;
        state.locals.declared[local] = true;
        out.push(
            StmtKind::Destructure {
                pattern: js::Pattern::Object(named, Some(state.locals.names[local].clone())),
                value,
                mutable: false,
            }
            .at(self.js_span(span)),
        );
        Ok(())
    }

    /// `place = value`: a temporary's value kept for where it's used, a
    /// variable's `const` where it's made once, or an assignment.
    /// Each `&mut` named where it's used, `state.refs`, whose place is
    /// through `local`, `x.value`, made of what `local` holds now, held:
    /// `local` is about to be written.
    fn hold_refs_through(&mut self, state: &mut State<'_, 'tcx>, local: Local, out: &mut Vec<Stmt>) -> R<()> {
        let name = state.locals.names[local].clone();
        if name.is_empty()
            || !state
                .refs
                .values()
                .any(|target| root_var(target) == Some(name.as_str()))
        {
            return Ok(());
        }
        self.flush(state, out)?;
        let held = self.spill(&name, Expr::var(&name), out);
        for target in state.refs.values_mut() {
            if root_var(target) == Some(name.as_str()) {
                *target = (target.substitute(&|v| (v == name).then(|| held.clone()))).unwrap_or(target.clone());
            }
        }
        Ok(())
    }

    pub(super) fn mir_store(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        value: Value<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        // What's made is where its Rust is, for the source map.
        let value = match value {
            Value::Expr(mut e) if e.span.is_none() => {
                e.span = self.js_span(span);
                Value::Expr(e)
            }
            value => value,
        };
        // A write's result a function gives, of what it called last, that
        // may hold an `Err`, `$writeTry(..)`'s or `$writeOf(..)`'s: thrown on,
        // as `?` of it is, and `undefined` given (ADRs 0187, 0462). A call's
        // own throws it.
        if place == Place::from(mir::RETURN_PLACE)
            && self
                .recognition()
                .is_write_result(state.body.local_decls[mir::RETURN_PLACE].ty)
            && let Value::Expr(value) = &value
            && let js::ExprKind::Call(callee, _) = &value.kind
            && matches!(callee.helper_name(), Some("$writeTry" | "$writeOf"))
        {
            self.flush(state, out)?;
            self.fmt_check(value.clone(), self.js_span(span), out);
            if state.writer.is_some() {
                return Ok(());
            }
            return self.mir_store(state, place, Value::Expr(Expr::undefined()), span, out);
        }
        // A union's field written: the one it holds now (ADR 0406).
        if let Some((base, PlaceElem::Field(field, _))) = place.as_ref().last_projection()
            && self.is_union(base.ty(state.body, self.tcx).ty)
        {
            let union_ty = base.ty(state.body, self.tcx).ty;
            let base = Place {
                local: base.local,
                projection: self.tcx.mk_place_elems(base.projection),
            };
            let value = self.value_expr(value, span)?;
            self.flush(state, out)?;
            let union = self.mir_place(state, base, span, out)?;
            let written = self.union_write(union, union_ty, field, value);
            out.push(StmtKind::Expr(written).at(self.js_span(span)));
            return Ok(());
        }
        // What a strip made, read as its test and its cut where it's read
        // (ADR 0642).
        if place.projection.is_empty()
            && let Value::Cut(test, cut) = value
        {
            state.cuts.insert(place.local, (test, cut));
            return Ok(());
        }
        // A strip's cut given to a local read once: read there, `stack =
        // stack.slice(29)`.
        if place.projection.is_empty()
            && state.offsets.taken_in_place(place.local)
            && let Value::Expr(cut) = &value
        {
            state.in_place.insert(place.local, cut.clone());
            return Ok(());
        }
        let local = place.local;
        let place = match self.whole(state.body, &place) {
            true => Place::from(local),
            false => place,
        };
        // A reference reassigned, `x = z`, that a `&mut` named where it's
        // used is through, `x.value` of `&mut *x` made before: what `x` is
        // now, held, so it's still the place it was.
        if place.projection.is_empty() {
            self.hold_refs_through(state, local, out)?;
        }
        // A field's path stored in a local, a getter's `element.isContentEditable`,
        // or a call's, `isItem()` of `isItem(item)`: whether it's a `bool`, as
        // of a place read (`mir_place`).
        if place.projection.is_empty()
            && let Value::Expr(e) = &value
            && let Some(path) = tidy::field_path(e).or_else(|| tidy::call_path(e))
        {
            let bool = state.body.local_decls[local].ty.is_bool();
            *self.locals.mir.paths.entry(path).or_insert(bool) &= bool;
        }
        // Of a binding, one a guarded arm's pattern makes that only the
        // guard reads, `first` of `Kind::Array(&[first, _]) if f(first) =>
        // ..`: the arm's body, which reads none (ADR 0751).
        let user = state.locals.user[local] && !locals::bound_for_a_guarded_arm(state.body, local);
        let unread = !user && state.locals.reads[local] == 0 && !state.locals.borrowed[local];
        if place.projection.is_empty() && (state.locals.names[local].is_empty() || unread) {
            // Of `()`, or a temporary nothing reads: what made it runs. Of
            // a reference, `let _ = &mut p.y` through `deref_mut`: what makes
            // its place, `deref_mut(p)`, no handle on it.
            let expr = match value {
                Value::Ref(mut place) => {
                    while let js::ExprKind::Member(object, _) = place.kind {
                        place = *object;
                    }
                    place
                }
                value => self.value_expr(value, span)?,
            };
            if expr.has_effects() {
                self.flush(state, out)?;
                out.push(StmtKind::Expr(expr).at(self.js_span(span)));
            }
            return Ok(());
        }
        // What `?` made: the value it tried, read where the branch is.
        if place.projection.is_empty()
            && !state.body.local_decls[local].ty.is_integral()
            && let Value::Branch(tried, ty) = &value
        {
            let mut tried = tried.clone();
            // An `Option`'s held, which is what a `let` binds of it, `let f =
            // xs.find(..)?;`: held under that name, `f`, bound there already.
            if let js::ExprKind::Var(held) = &tried.kind
                && self.option_of(*ty).is_some_and(|inner| !self.boxed_payload(inner))
                && let Some((bound, from)) = continued_binding(state, local)
                && let Some(Stmt {
                    kind: StmtKind::Const(name, _),
                    ..
                }) = (out.iter_mut().rev()).find(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == held))
            {
                *name = state.locals.names[bound].clone();
                tried = Expr::var(name);
                state.parts.insert(bound, from);
                state.locals.declared[bound] = true;
            }
            state.branches.insert(local, (tried, *ty));
            return Ok(());
        }
        // A `&mut` a temporary holds to a place, a guard's `value` its
        // `deref_mut` gives, `&mut copy.pages`: that place, where it's used
        // and written, which nothing else changes while it's borrowed.
        if place.projection.is_empty()
            && !state.locals.user[local]
            && state.locals.writes[local] == 1
            && let Value::Place(target) | Value::Expr(target) = &value
            && matches!(
                state.body.local_decls[local].ty.kind(),
                ty::Ref(_, _, ty::Mutability::Mut)
            )
            && matches!(target.kind, js::ExprKind::Member(..) | js::ExprKind::Index(..))
            && fixed_place(target)
        {
            state.in_place.insert(local, target.clone());
            return Ok(());
        }
        // A `&mut` a variable keeps, `let first = &mut h.list[0]`: its place
        // fixed where it's borrowed, what it's in and where (`fixed_kept`), so
        // what changes `h.list` later changes no borrow.
        let value = match value {
            Value::Ref(target) if place.projection.is_empty() && state.locals.user[local] => {
                Value::Ref(self.fixed_kept(state, target, out)?)
            }
            value => value,
        };
        // A `&mut` to a place that's always the same, made once: the
        // place, named where it's used, as borrowck keeps it the same.
        // Not one borrowed whole, `&r`, which is a value of its own then.
        if place.projection.is_empty()
            && state.locals.writes[local] == 1
            && !state.pointed.borrowed.contains(&local)
            && let Value::Ref(target) = &value
            && same_place(state, target)
        {
            state.refs.insert(local, target.clone());
            return Ok(());
        }
        // A shared reference a temporary or a variant's binding holds, made
        // once, to a place of a variable nothing changes, `&route.title`: the
        // place, read where it's used (ADR 0023);
        // borrowck keeps it the same meanwhile.
        // And a tuple of them, `format_args!`'s.
        // And a kept `&mut`'s, `&r` of `let r = &mut n`: a handle on its
        // place, which nothing else changes while it's borrowed.
        let held = |value: &Value<'tcx>, ty: Ty<'tcx>| match value {
            Value::Place(target)
                if same_place(state, target)
                    && root_var(target).is_some_and(|root| self.locals.mir.alike.contains(root)) =>
            {
                Some(target.clone())
            }
            Value::Ref(target)
                if matches!(ty.kind(), ty::Ref(_, _, ty::Mutability::Not))
                    && same_place(state, target)
                    && fixed_place(target) =>
            {
                Some(Expr::handle(target.clone()))
            }
            _ => None,
        };
        // A payload's of a temporary keeps its name, which the temporary
        // takes (tidy's `named_temporaries`).
        let of_temporary = matches!(&value, Value::Place(target) if matches!(&target.kind, js::ExprKind::Var(name)
            if (state.locals.names.iter_enumerated()).any(|(l, n)| n == name && !state.locals.user[l])));
        let one = place.projection.is_empty()
            && (!state.locals.user[local] || (state.payloads.contains(&local) && !of_temporary))
            && state.locals.writes[local] == 1
            && !state.locals.borrowed[local];
        // Each item's type, a tuple's field's.
        let ty = state.body.local_decls[local].ty;
        let item_ty = |i: usize| match ty.kind() {
            ty::Tuple(tys) => tys.get(i).copied().unwrap_or(ty),
            _ => ty,
        };
        let places = match &value {
            Value::List(items) => (items.iter().enumerate())
                .map(|(i, item)| held(item, item_ty(i)))
                .collect::<Option<Vec<_>>>()
                .map(|items| Value::List(items.into_iter().map(Value::Place).collect())),
            value => held(value, ty).map(Value::Place),
        };
        if one && let Some(places) = places {
            state.places.insert(local, places);
            return Ok(());
        }
        // One read by its fields, each once, a `format_args!`'s tuple: a list
        // whose items are each read where its field is.
        let parts = match &value {
            Value::List(items) => {
                let mut read = state.locals.fields[local].clone();
                read.sort();
                read == (0..items.len()).collect::<Vec<_>>()
            }
            _ => false,
        };
        if place.projection.is_empty()
            && !state.pointed.locals.contains(&local)
            && self.foldable(state, local)
            && (state.locals.reads[local] == 1 || parts)
        {
            state.pending.push((local, value));
            return Ok(());
        }
        let expr = self.value_expr(value, span)?;
        let js_span = self.js_span(span);
        // A temporary made once of what reads the same anywhere, however
        // often it's read, or what a variant's pattern binds to it: written
        // where it is.
        // A payload's of a temporary keeps its name, which the temporary
        // takes (tidy's `aliases`).
        let of_temporary = matches!(&expr.kind, js::ExprKind::Var(name)
            if (state.locals.names.iter_enumerated()).any(|(l, n)| n == name && !state.locals.user[l]));
        if place.projection.is_empty()
            && !state.pointed.locals.contains(&local)
            && (!state.locals.user[local] || (state.payloads.contains(&local) && !of_temporary))
            && state.locals.writes[local] == 1
            && !state.locals.borrowed[local]
            && !state.named.contains(&local)
            && self.reads_same_anywhere(&expr, state.body.local_decls[local].ty)
        {
            state.in_place.insert(local, expr);
            return Ok(());
        }
        if place.projection.is_empty() && !state.locals.declared[local] {
            // A variable of a type JS can't hold: refused.
            self.check_variable(state, local, span)?;
            self.flush(state, out)?;
            state.locals.declared[local] = true;
            let name = state.locals.names[local].clone();
            // A part borrowed mutably, `ref mut n` of an `Option<i32>`, may be
            // the variable itself in JS, written through the borrow: a `let`,
            // which tidy makes a `const` where nothing assigns it.
            let js_span = state.let_spans.get(&local).copied().unwrap_or(js_span);
            // One a pointer is taken to, a cell (ADR 0407).
            if state.pointed.locals.contains(&local) {
                let cell = Expr::object(vec![js::Prop::Field(js::HELD.into(), expr)]);
                out.push(StmtKind::Const(name, cell).at(js_span));
                return Ok(());
            }
            out.push(
                match state.locals.borrowed[local] {
                    true => StmtKind::Let(name, Some(expr)),
                    false => StmtKind::Const(name, expr),
                }
                .at(js_span),
            );
            return Ok(());
        }
        if place.projection.is_empty() && !state.locals.user[local] && crate::lower::iterators::is_items_map(&expr) {
            state.maps.insert(local, expr.clone());
        }
        // The value is made before the place is reached, as Rust makes them:
        // but of a variable's fields, the object JS reaches first is the
        // one, which what makes the value can't make another, a closure's
        // `more = ..` assigning it in place: `more.has = work();` (ADR 0607).
        let mut value = expr;
        let fixed = (place.projection.iter()).all(|elem| matches!(elem, PlaceElem::Field(..)));
        if !place.projection.is_empty() && value.has_effects() && !fixed {
            value = self.spill("value", value, out);
        }
        let target = self.mir_place(state, place, span, out)?;
        self.flush(state, out)?;
        // `*r = v` of a `&mut` to an object: the object becomes `v`, so each
        // name for it sees it.
        let decls = &state.body.local_decls;
        if let Some((base, PlaceElem::Deref)) = place.as_ref().last_projection()
            && matches!(
                Place::ty_from(base.local, base.projection, decls, self.tcx).ty.kind(),
                ty::Ref(_, _, ty::Mutability::Mut)
            )
            && self.written_in_place(place.ty(decls, self.tcx).ty)
        {
            let assigned = Expr::call(Expr::helper("$assign"), vec![target, value]);
            out.push(StmtKind::Expr(assigned).at(js_span));
            return Ok(());
        }
        // `*r = v` of a `&mut` the variable `r` is, to what's neither an object
        // nor a cell, a closure: JS would only give `r` another, so it's
        // refused (not an alias `r` names a place by, `*outer = ..`).
        if let Some((base, PlaceElem::Deref)) = place.as_ref().last_projection()
            && base.projection.is_empty()
            && matches!(target.kind, js::ExprKind::Var(ref name) if *name == state.locals.names[base.local])
        {
            return Err(self.unsupported(span, "assigning a whole value through a `&mut`"));
        }
        // What an `Option`'s box holds, `$someValue(o)` read through
        // `$someMut`: the `Option` given a `Some` of it (ADR 0486).
        if let js::ExprKind::Call(callee, args) = &target.kind
            && callee.helper_name() == Some("$someValue")
            && let [option] = args.as_slice()
        {
            let some = self.some(value);
            out.push(StmtKind::Assign(option.clone(), some).at(js_span));
            return Ok(());
        }
        out.push(StmtKind::Assign(target, value).at(js_span));
        Ok(())
    }

    /// The arguments of a call of the crate's: a `&mut` to a value JS can't
    /// change in place is a box, named for its parameter, copied back once
    /// the call returns (ADR 0074). What's given before it that does
    /// something runs first, before the box reads the place.
    pub(super) fn boxed_args(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        values: Vec<Value<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Vec<Expr>> {
        // What's called: a trait's method resolved to its impl's, whose
        // parameters say which are boxes, or the crate's trait's own, a
        // dictionary's, and `Hash`'s of a type only a caller knows, whose
        // hasher is a `&mut H` of its own (ADR 0463), and `Extend`'s, whose
        // `&mut Self` its dictionary takes in a box, an `I::Item`'s that's an
        // `IntoIterator` too (ADR 0478); not a `dyn`'s, whose pair is its
        // `&mut`, nor std's.
        let callee = match self.tcx.trait_of_assoc(def_id) {
            None => Some((def_id, generic_args)),
            Some(trait_id) => match self.impl_method(def_id, generic_args)? {
                Some(method) => Some(method),
                None if (self.is_rust_trait(trait_id)
                    || (is_std_def(self.tcx, trait_id, StdItem::Hash)
                        || crate::lower::recognition::is_extend(self.tcx, trait_id))
                        && self.is_unknown(generic_args.type_at(0)))
                    && !matches!(generic_args.type_at(0).kind(), ty::Dynamic(..)) =>
                {
                    Some((def_id, generic_args))
                }
                None => None,
            },
        };
        let fn_id = callee.map(|(id, _)| id);
        // Generic code's box of what its caller has as an object, a `D` given
        // `&mut v` too (ADR 0454).
        let given_box = |cx: &Self, i: usize| {
            callee.is_some_and(|(id, args)| {
                let inputs = cx
                    .tcx
                    .fn_sig(id)
                    .instantiate_identity()
                    .skip_normalization()
                    .skip_binder()
                    .inputs();
                inputs
                    .get(i)
                    .is_some_and(|&input| cx.boxed_in_call(id, input, args).is_some())
            })
        };
        let mut exprs: Vec<Expr> = Vec::new();
        for (i, value) in values.into_iter().enumerate() {
            let object = !matches!(value, Value::Ref(_));
            let place = match value {
                // A shared reference to what generic code has in a box, an
                // impl's for `&mut T`'s `&self`: a box of it, which nothing
                // writes through, so nothing's copied back (ADR 0454).
                Value::Place(object) if given_box(self, i) => {
                    exprs.push(crate::lower::mut_refs::boxed(object));
                    continue;
                }
                Value::Ref(place) => place,
                // An object given where a box goes, a generic `&mut T`'s: boxed,
                // and copied back where it's a place.
                Value::Expr(object) if fn_id.is_some_and(|fn_id| self.param_is_box(fn_id, i)) || given_box(self, i) => {
                    object
                }
                // A function given for an `F: FnOnce(&mut T)` of a `T` that's
                // an object here: one taking generic code's box of it, which
                // it gives the object, `(x) => f(x.value)` (ADR 0454).
                value => {
                    let f = self.value_expr(value, span)?;
                    let unboxed = callee.and_then(|(id, args)| self.unboxed_by_function(id, i, args));
                    exprs.push(match unboxed {
                        Some(unboxed) => {
                            let params: Vec<String> =
                                (0..unboxed.len()).map(|at| self.fresh(&format!("x{at}"))).collect();
                            let given = (params.iter().zip(&unboxed))
                                .map(|(param, &unboxed)| match unboxed {
                                    true => Expr::held(Expr::var(param)),
                                    false => Expr::var(param),
                                })
                                .collect();
                            let body = vec![StmtKind::Return(Some(Expr::call(f, given))).at(js::Span::NONE)];
                            Expr::arrow(params.into_iter().map(Into::into).collect(), body)
                        }
                        None => f,
                    });
                    continue;
                }
            };
            // Where what's called, an impl's method resolved, takes an object
            // by its own `&mut`, a copied default's `&mut self` given to its
            // impl's method of an object `Self`, or of a type parameter that's
            // a JS iterator or function, which isn't boxed either (ADR 0099):
            // the object, which it changes in place, a box's `value` or a
            // generic `self` boxed for none.
            let holds = fn_id.is_some_and(|fn_id| {
                let sig = self.tcx.fn_sig(fn_id).instantiate_identity().skip_normalization();
                let param = sig.skip_binder().inputs().get(i).copied();
                let typing_env = ty::TypingEnv::post_analysis(self.tcx, fn_id);
                let itself = |inner: Ty<'tcx>| {
                    let generic = self.recognition().is_unknown_in(inner, typing_env);
                    self.is_object(inner) || (generic && !self.is_generic_boxed(inner, typing_env.param_env))
                };
                !self.param_is_box(fn_id, i)
                    && matches!(param.map(|p| *p.kind()), Some(ty::Ref(_, inner, ty::Mutability::Mut)) if itself(inner))
            });
            if holds {
                exprs.push(place);
                continue;
            }
            // A box this function was given, given on as it is, and a cell a
            // place holds, a reborrow's, `&mut **self` of an impl's for
            // `&mut T`: the cell, not a box of a copy.
            if let js::ExprKind::Member(boxed, field) = &place.kind
                && field == js::HELD
                && let js::ExprKind::Var(name) = &boxed.kind
                && state.boxes.contains(name)
            {
                exprs.push((**boxed).clone());
                continue;
            }
            if let Some(cell) = self.held_cell(&place) {
                exprs.push(cell);
                continue;
            }
            let param = self
                .tcx
                .fn_sig(def_id)
                .instantiate(self.tcx, generic_args)
                .skip_binder()
                .inputs()[i];
            // A box of a place that isn't fixed couldn't be copied back; one
            // of a type of one value, `()`'s, a unit struct's, needn't be: no
            // write changes it.
            let one_value = param
                .builtin_deref(true)
                .is_some_and(|pointee| self.has_one_value(pointee));
            // A field or an item of an object reached another way, `es[0].n`'s
            // `$index(es, 0)`, a handle's `value` a call made among them,
            // `first_mut().unwrap()`'s: that object named where it's
            // borrowed, the same object, so the place is fixed (ADR 0099).
            let place = match place.kind {
                js::ExprKind::Member(..) | js::ExprKind::Index(..) if !fixed_place(&place) && !one_value => {
                    self.fixed_place_of(place, out)
                }
                _ => place,
            };
            if !fixed_place(&place) && !one_value {
                return Err(self.unsupported(span, "a `&mut` of a number given, from its MIR"));
            }
            // What it returns, or another parameter, can hold the borrow: a
            // box would be copied back before it's used.
            if self.borrow_outlives_call(def_id, i) {
                if !fixed_place(&place) {
                    return Err(self.unsupported(span, "a `&mut` of a number given back, from its MIR"));
                }
                exprs.push(Expr::handle(place));
                continue;
            }
            // A `dyn`'s, or a slice's, the callee can't replace whole: its box
            // is never written, so it isn't copied back. A `str`'s is: its
            // text is written whole, `make_ascii_uppercase`'s.
            if param
                .builtin_deref(true)
                .is_some_and(|pointee| !pointee.is_sized(self.tcx, self.typing_env) && !pointee.is_str())
            {
                exprs.push(crate::lower::mut_refs::boxed(place));
                continue;
            }
            self.flush(state, out)?;
            for expr in &mut exprs {
                if expr.has_effects() {
                    let made = std::mem::replace(expr, Expr::undefined());
                    *expr = self.spill("arg", made, out);
                }
            }
            let names = self.tcx.fn_arg_idents(def_id);
            let base = names
                .get(i)
                .copied()
                .flatten()
                .map_or("value".to_string(), |i| i.name.to_string());
            let name = self.fresh(&crate::lower::camel_case(&base));
            // An object's box holds a copy of it: what the call moves out of
            // the box, `mem::replace`'s old value, is that copy, where the
            // object, written back after in place, is the new value.
            // Where the callee may move it out (`may_move_out`).
            let held = match object && crate::lower::mut_refs::may_move_out(self.tcx, def_id, i) {
                true => Expr::call_helper("$apart", vec![place.clone()]),
                false => place.clone(),
            };
            let boxed = crate::lower::mut_refs::boxed(held);
            out.push(StmtKind::Const(name.clone(), boxed).at(self.js_span(span)));
            if fixed_place(&place) {
                state.copy_backs.push((place, name.clone(), object));
            }
            exprs.push(Expr::var(&name));
        }
        Ok(exprs)
    }

    /// A variable the program declares, refused: of what drops as JS can't drop
    /// it, or of a type JS can't hold. Not std's MIR put in the body,
    /// nor a JS iterator.
    pub(super) fn check_variable(&self, state: &State<'_, 'tcx>, local: Local, span: Span) -> R<()> {
        let ty = state.body.local_decls[local].ty;
        if !state.locals.user[local] || inlined(state.body, local) || self.implements_iterator(ty) {
            return Ok(());
        }
        self.check_held_drops(ty, span)?;
        self.check_value_ty(ty, span)
    }

    /// `place`, a `&mut` a variable keeps, fixed: the object it's in held
    /// where it may change while it's borrowed, `const list = h.list`. Only
    /// a reference can, a variable of one written again too; MIR holds an
    /// index already.
    fn fixed_kept(&mut self, state: &mut State<'_, 'tcx>, place: Expr, out: &mut Vec<Stmt>) -> R<Expr> {
        // A cell made here, `leak()`'s `{ value: s }`: only it has it.
        if let js::ExprKind::Member(object, _) = &place.kind
            && matches!(object.kind, js::ExprKind::Object(..))
        {
            return Ok(place);
        }
        let (object, part) = match place.kind {
            js::ExprKind::Member(object, field) => (*object, Ok(field)),
            js::ExprKind::Index(object, index) => (*object, Err(*index)),
            kind => return Ok(Expr { kind, ..place }),
        };
        let stays = match &object.kind {
            js::ExprKind::Var(name) => (state.locals.names.iter_enumerated())
                .find(|(_, n)| *n == name)
                .is_none_or(|(l, _)| state.locals.writes[l] <= 1 || !state.body.local_decls[l].ty.is_ref()),
            _ => object.is_constant(),
        };
        let object = match stays {
            true => object,
            false => {
                self.flush(state, out)?;
                let name = match &object.kind {
                    js::ExprKind::Member(_, field) => crate::lower::camel_case(field),
                    _ => "item".to_string(),
                };
                self.spill(&name, object, out)
            }
        };
        Ok(match part {
            Ok(field) => Expr::member(object, field),
            Err(index) => Expr::index(object, index),
        })
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `place`, a field or an item, fixed: what holds it named, where it
    /// isn't fixed, an object that's the same however it's reached, and an
    /// index that isn't a constant or a name named too.
    pub(in crate::lower) fn fixed_place_of(&mut self, place: Expr, out: &mut Vec<Stmt>) -> Expr {
        let span = place.span;
        let mut fixed = |e: Expr, name: &str, this: &mut Self| match fixed_place(&e) {
            true => e,
            false => this.spill(name, e, out),
        };
        match place.kind {
            js::ExprKind::Member(object, field) => Expr::member(fixed(*object, "item", self), &field).or_at(span),
            js::ExprKind::Index(object, index) => {
                let object = fixed(*object, "items", self);
                let index = match index.is_constant() || matches!(index.kind, js::ExprKind::Var(_)) {
                    true => *index,
                    false => self.spill("at", *index, out),
                };
                Expr::index(object, index).or_at(span)
            }
            kind => Expr { kind, ..place },
        }
    }
}
