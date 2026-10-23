//! An rvalue as JS, an aggregate made, and an `Option` already `null` or
//! a value.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(super) fn mir_rvalue(
        &mut self,
        state: &mut State<'_, 'tcx>,
        rvalue: &Rvalue<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let tcx = self.tcx;
        let decls = &state.body.local_decls;
        Ok(Value::Expr(match rvalue {
            Rvalue::Use(operand, _) => return self.mir_operand(state, operand, out),
            Rvalue::CopyForDeref(place) if self.through_values(state.body, place, true) => {
                return self.read_local_value(state, place.local, out);
            }
            Rvalue::CopyForDeref(place) => self.mir_place(state, *place, span, out)?,
            // A reference is what it refers to (ADR 0023): a `&mut` to a
            // value JS can't share, a number's, needs what isn't made yet.
            Rvalue::Ref(_, kind, place) => {
                // A slice pattern's rest, through a `&mut`: a view of it.
                if matches!(kind, BorrowKind::Mut { .. })
                    && place
                        .projection
                        .iter()
                        .any(|elem| matches!(elem, PlaceElem::Subslice { .. }))
                {
                    return match self.rest_view(state, *place, span, out)? {
                        Some(view) => Ok(Value::Expr(view)),
                        None => Err(self.unsupported(span, "a `&mut` to part of a slice, from its MIR")),
                    };
                }
                // A union's field, through a `&mut` (ADR 0406).
                if matches!(kind, BorrowKind::Mut { .. })
                    && let Some((base, PlaceElem::Field(..))) = place.as_ref().last_projection()
                    && self.is_union(base.ty(decls, tcx).ty)
                {
                    return self.union_field_borrowed(span).map(Value::Expr);
                }
                // A struct's last field, a `dyn`: its value, and the `impl` its
                // struct was given when it was made one, a pair; of a `&mut`,
                // one on the field's place (ADR 0409).
                if let Some((base, PlaceElem::Field(field, _))) = place.as_ref().last_projection()
                    && matches!(self.unwrapped(place.ty(decls, tcx).ty).kind(), ty::Dynamic(..))
                {
                    let base_ty = base.ty(decls, tcx).ty;
                    let base = mir::Place::from(base.local).project_deeper(base.projection, tcx);
                    let parent = self.mir_place(state, base, span, out)?;
                    let parent = match parent.reads_same() {
                        true => parent,
                        false => self.spill("value", parent, out),
                    };
                    let value = self.project(parent.clone(), base_ty, field.as_usize());
                    let dictionary = Expr::member(parent, js::DICTIONARY);
                    return Ok(Value::Expr(match kind {
                        BorrowKind::Mut { .. } => Expr::pair(value, dictionary),
                        _ => Expr::object(vec![
                            Prop::Field(js::HELD.into(), value),
                            Prop::Field(js::DICTIONARY.into(), dictionary),
                        ]),
                    }));
                }
                // `&(*(_5.0))`, the reference `_5.0` is, a list's item.
                if matches!(kind, BorrowKind::Shared)
                    && let Some(item) = self.take_field(state, place, out)?
                {
                    return Ok(item);
                }
                let pointee = place.ty(decls, tcx).ty;
                // `&mut *r`, a reborrow of a cell, `leak()`'s `{ value: s }`
                // too: that cell's `value`, the cell kept, not `s`.
                if matches!(kind, BorrowKind::Mut { .. })
                    && place.projection.as_slice() == [PlaceElem::Deref]
                    && self.is_cell(decls[place.local].ty)
                {
                    match self.read_local_value(state, place.local, out)? {
                        Value::Expr(cell) => {
                            let place = match cell.kind {
                                js::ExprKind::Object(_) => {
                                    let kind = js::ExprKind::Member(Box::new(cell), "value".into());
                                    Expr {
                                        kind,
                                        ..Expr::undefined()
                                    }
                                }
                                _ => Expr::held(cell),
                            };
                            return Ok(Value::Ref(place));
                        }
                        aliased @ Value::Ref(_) => return Ok(aliased),
                        _ => return Err(self.unsupported(span, "this reborrow, from its MIR")),
                    }
                }
                if matches!(kind, BorrowKind::Mut { .. }) && self.is_cell_pointee(pointee) {
                    let target = self.mir_place(state, *place, span, out)?;
                    // Of a reference written again, `list` of `&mut list.value`
                    // in a loop that moves `list` on: what it is now, held.
                    let root = place.projection.first() == Some(&PlaceElem::Deref)
                        && written_again(state, place.local)
                        && state.body.local_decls[place.local].ty.is_ref();
                    let target = match root {
                        true => {
                            let name = state.locals.names[place.local].clone();
                            self.flush(state, out)?;
                            let held = self.spill(&name, Expr::var(&name), out);
                            target
                                .substitute(&|v| (v == name).then(|| held.clone()))
                                .unwrap_or(target)
                        }
                        false => target,
                    };
                    // `&mut **u`, a reborrow of a cell a place holds, an
                    // `Option`'s `&mut` through a `ref mut`: that cell's
                    // `value`, the place kept as one that holds a cell, as a
                    // variable is, so a handle on it is the cell.
                    if let Some((base, PlaceElem::Deref)) = place.as_ref().last_projection()
                        && self.is_cell(base.ty(decls, tcx).ty)
                        && let js::ExprKind::Member(cell, field) = &target.kind
                        && field == js::HELD
                        && fixed_place(cell)
                    {
                        self.locals.mir.cells.push((**cell).clone());
                    }
                    return Ok(Value::Ref(target));
                }
                // `&error.title` of a `Cell` a field holds, a handle on it,
                // through a reference written again, `error = other` after:
                // of what `error` is now, held, as a `&mut`'s above.
                if matches!(kind, BorrowKind::Shared)
                    && (self.is_std_type(pointee, StdItem::Cell) || self.plain_ref_cell(pointee))
                    && place.projection.first() == Some(&PlaceElem::Deref)
                    && written_again(state, place.local)
                    && state.body.local_decls[place.local].ty.is_ref()
                    && !state.locals.names[place.local].is_empty()
                {
                    let target = self.mir_place(state, *place, span, out)?;
                    let name = state.locals.names[place.local].clone();
                    self.flush(state, out)?;
                    let held = self.spill(&name, Expr::var(&name), out);
                    let target = (target.substitute(&|v| (v == name).then(|| held.clone()))).unwrap_or(target);
                    return Ok(Value::Place(target));
                }
                if self.through_values(state.body, place, true) {
                    let value = self.read_local_value(state, place.local, out)?;
                    return Ok(match (kind, value) {
                        // Of what can't change meanwhile, the body's own locals: read
                        // where it's used. A temporary's read of what can, a
                        // static's, is made where it's made.
                        (BorrowKind::Shared, Value::Expr(e))
                            if e.reads_same() && movable(state, &Value::Expr(e.clone())) =>
                        {
                            Value::Place(e)
                        }
                        // Through `outer`, a `&mut` to `list`, a `&mut` to what
                        // isn't a cell: what `list` is.
                        (_, Value::Ref(e)) if !place.projection.is_empty() && !self.is_cell_pointee(pointee) => {
                            Value::Expr(e)
                        }
                        (_, value) => value,
                    });
                }
                let place_expr = self.mir_place(state, *place, span, out)?;
                if matches!(kind, BorrowKind::Shared) {
                    let read = self.with_default(state, *place, place_expr.clone(), span);
                    if !matches!(read.kind, js::ExprKind::Cond(..)) && place_expr.reads_same() {
                        return Ok(Value::Place(place_expr));
                    }
                    return Ok(Value::Expr(read));
                }
                place_expr
            }
            Rvalue::BinaryOp(op, operands) => {
                let (l, r) = &**operands;
                let (ty, r_ty) = (l.ty(decls, tcx), r.ty(decls, tcx));
                // An enum's discriminant compared with a variant's, an inlined
                // `is_some()`'s in a std body (ADR 0561): the variant's test.
                if let BinOp::Eq | BinOp::Ne = op
                    && let Operand::Constant(c) = r
                    && let Some(variant) = c.const_.try_eval_scalar_int(tcx, self.typing_env)
                    && let Operand::Copy(place) | Operand::Move(place) = l
                    && (state.pending.iter()).any(|(local, value)| {
                        *local == place.local && matches!(value, Value::Discriminant(..) | Value::Tested(_))
                    })
                {
                    let discriminant = self.take_operands(state, &[l], out)?.remove(0);
                    let enum_ty = match &discriminant {
                        Value::Discriminant(_, enum_ty) => *enum_ty,
                        _ => ty,
                    };
                    let test = self.switch_test(&discriminant, enum_ty, variant.to_bits_unchecked(), span)?;
                    return Ok(Value::Expr(match op {
                        BinOp::Eq => test,
                        _ => test.negated_test(),
                    }));
                }
                // A range pattern's test, `2 <= k`: the value first, `k >= 2`;
                // of a bound the type has anyway, `0 <= k` of a `u8`, none.
                let flipped = match op {
                    BinOp::Le => Some(BinOp::Ge),
                    BinOp::Lt => Some(BinOp::Gt),
                    _ => None,
                };
                if let Some(flipped) = flipped
                    && let Operand::Constant(bound) = l
                    && !matches!(r, Operand::Constant(_))
                    && state.ranges.iter().any(|range| range.contains(span))
                {
                    if *op == BinOp::Le
                        && ty.is_integral()
                        && !ty.is_signed()
                        && bound.const_.try_to_scalar_int().is_some_and(|n| n.is_null())
                    {
                        return Ok(Value::Expr(Expr::bool(true)));
                    }
                    let [r, l]: [Value<'tcx>; 2] =
                        self.take_operands(state, &[r, l], out)?.try_into().ok().expect("two");
                    let (r, l) = (self.value_expr(r, span)?, self.value_expr(l, span)?);
                    return Ok(Value::Expr(self.binary(flipped, r, l, None, ty, span)?));
                }
                // A constant's value, a named one's too: a division by it that
                // can't panic is JS's.
                let known = match r {
                    Operand::Constant(c) => {
                        (c.const_.try_eval_scalar_int(tcx, self.typing_env)).map(|n| match r_ty.is_signed() {
                            true => n.to_int(n.size()),
                            false => n.to_uint(n.size()) as i128,
                        })
                    }
                    _ => None,
                };
                // A place stepped past what was found there, in units: by the
                // literal's length in units, `at + 1`, or by a string's,
                // `at + p.length`, which can't pass the string's `length`
                // (ADR 0642).
                let in_units = |o: &Operand<'tcx>| {
                    o.place()
                        .is_some_and(|p| p.projection.is_empty() && state.offsets.in_units(p.local))
                };
                if *op == BinOp::Add && (in_units(l) || in_units(r)) {
                    let at = mir::Location {
                        block: state.at,
                        statement_index: state.statement,
                    };
                    let step = state.offsets.step(at);
                    let parts = self.take_operands(state, &[l, r], out)?;
                    let mut sum = Vec::new();
                    for (operand, part) in [l, r].into_iter().zip(parts) {
                        sum.push(match (operand, step) {
                            (Operand::Constant(_), Some(units)) => Expr::int(units as i128),
                            _ => self.value_expr(part, span)?,
                        });
                    }
                    let [a, b]: [Expr; 2] = sum.try_into().ok().expect("two");
                    return Ok(Value::Expr(Expr::bin(Op::Add, a, b)));
                }
                let [l, r]: [Value<'tcx>; 2] = self.take_operands(state, &[l, r], out)?.try_into().ok().expect("two");
                let l = self.value_expr(l, span)?;
                // A shift's amount of another type: a number, `Number(n & 63n)`.
                let r = crate::lower::std_types::number::shift_amount_of(*op, self.value_expr(r, span)?, ty, r_ty);
                // Raw pointers compare by place or address (ADR 0562).
                if self.compared_by_address(ty)
                    && let Some((pointee, _)) = crate::lower::pointers::raw_pointee(tcx, ty)
                {
                    let same = self.pointers_equal(l.clone(), r.clone(), pointee, span)?;
                    return Ok(Value::Expr(match op {
                        BinOp::Eq => same,
                        BinOp::Ne => Expr::unary(js::UnaryOp::Not, same),
                        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge if self.is_handle_pointer(ty) => {
                            let size = self.pointee_size(pointee, span)?;
                            let order = Expr::call(Expr::helper("$ptrCmp"), vec![l, r, size]);
                            let js_op = match op {
                                BinOp::Lt => Op::Lt,
                                BinOp::Le => Op::Le,
                                BinOp::Gt => Op::Gt,
                                _ => Op::Ge,
                            };
                            Expr::bin(js_op, order, Expr::int(0))
                        }
                        _ => return Err(self.unsupported(span, "this operation on raw pointers, from its MIR")),
                    }));
                }
                match op {
                    // A constant's, whose MIR checks as rustc's evaluation does:
                    // rustc evaluates it, so it never overflows; the operation,
                    // and that it didn't. And a std body's, checked as the crate
                    // that calls it is, `#[rustc_inherit_overflow_checks]`: as
                    // the release profile's, which rust-js's is (ADR 0011), it
                    // wraps, and the check is never made.
                    BinOp::AddWithOverflow | BinOp::SubWithOverflow | BinOp::MulWithOverflow
                        if matches!(
                            tcx.def_kind(state.body.source.def_id()),
                            DefKind::Const { .. }
                                | DefKind::AssocConst { .. }
                                | DefKind::AnonConst
                                | DefKind::Static { .. }
                        ) || !state.body.source.def_id().is_local() =>
                    {
                        let wrapping = op.overflowing_to_wrapping().expect("an overflowing operation");
                        let value = self.binary(wrapping, l, r, known, ty, span)?;
                        return Ok(Value::List(vec![Value::Expr(value), Value::Expr(Expr::bool(false))]));
                    }
                    BinOp::AddWithOverflow | BinOp::SubWithOverflow | BinOp::MulWithOverflow => {
                        return Err(self.unsupported(span, "arithmetic checked for overflow, from its MIR"));
                    }
                    BinOp::Cmp => return Err(self.unsupported(span, "`cmp`, from its MIR")),
                    _ => self.binary(*op, l, r, known, ty, span)?,
                }
            }
            Rvalue::UnaryOp(op, operand) => {
                let ty = operand.ty(decls, tcx);
                let a = self.mir_expr(state, operand, out)?;
                match op {
                    UnOp::Not | UnOp::Neg => self.unary(*op, a, ty, span)?,
                    UnOp::PtrMetadata => Expr::member(a, "length"),
                }
            }
            // A reference made another kind of reference is the same value: an
            // array's a slice's (ADR 0023). One made a `dyn`: a `dyn Iterator`
            // the JS iterator, another its `{ value, impl }`.
            Rvalue::Cast(mir::CastKind::PointerCoercion(coercion, _), operand, to) => {
                use rustc_middle::ty::adjustment::PointerCoercion;
                // A `dyn Future` is a promise, which a future of the crate's
                // own isn't (ADR 0425).
                if let Some(future) = self.recognition().unsized_unlowered_future(operand.ty(decls, tcx), *to) {
                    return Err(self.unlowered_future_awaited(future, span));
                }
                // Through a reference, a `Box` or an `Rc`, as `unsize_trait` sees it.
                // To a `dyn`, or a `RefCell<dyn T>`, a pair too (ADR 0397), or
                // a struct whose last field is one (ADR 0409).
                let to_dyn = matches!(self.pointee(*to).kind(), ty::Dynamic(..))
                    || self.dyn_cell_trait(*to).is_some()
                    || self.dyn_tail(operand.ty(decls, tcx), *to).is_some();
                match coercion {
                    PointerCoercion::Unsize if to_dyn => {
                        let from = operand.ty(decls, tcx);
                        // What owns a value with a destructor drops it through
                        // its dictionary's `$drop`.
                        if self
                            .drop_query()
                            .dyn_drop_lost(*to, self.impl_type(self.pointee(from)), self.pointee(*to))
                        {
                            return Err(self.unsupported(span, "a `dyn` of a value with a destructor"));
                        }
                        let value = self.mir_expr(state, operand, out)?;
                        // A raw pointer's: its place's value made the `dyn`,
                        // and a pointer to that again where the target is a
                        // handle on it, `*const Foo<dyn Trait>`; a `*const dyn
                        // Trait` is the pair itself (ADR 0562).
                        if self.is_handle_pointer(from) {
                            // A pointer to a value of no size has no place;
                            // the value is `undefined`.
                            let place = match self.has_one_value(self.pointee(from)) {
                                true => Expr::undefined(),
                                false => self.pointee_place(value),
                            };
                            let unsized_ = self.unsize_trait(from, *to, place, span, out)?;
                            return Ok(Value::Expr(match self.is_handle_pointer(*to) {
                                true => self.pointer_to(unsized_, self.pointee(*to), None, span)?,
                                false => unsized_,
                            }));
                        }
                        if self.recognition().is_dyn_iter(*to) {
                            let iterator = from.builtin_deref(true).unwrap_or(from);
                            return Ok(Value::Expr(match self.knows_its_back(*to) {
                                true => self.as_double_ended(value, iterator, span)?,
                                false => self.as_js_iterator(value, iterator, span)?,
                            }));
                        }
                        // A `&dyn Debug` is the string it shows (ADR 0060), made
                        // where it's read: by `{:#?}`, as that shows it.
                        if self.is_dyn_debug(*to) && !self.is_dyn_debug(from) {
                            // In a writer, as pretty as it's asked to be: a
                            // derived `Debug`'s fields (ADR 0137).
                            if state.writer.is_some() {
                                let pretty = self.writer_pretty();
                                let shown = self.debug_string_with(value, self.pointee(from), span, &pretty)?;
                                return Ok(Value::Expr(shown));
                            }
                            return Ok(Value::Fmt(Std::FmtDebug, self.pointee(from), value));
                        }
                        return Ok(Value::Expr(self.unsize_trait(from, *to, value, span, out)?));
                    }
                    // An array's pointer as its first item's (ADR 0407).
                    PointerCoercion::ArrayToPointer if self.is_handle_pointer(*to) => {
                        let pointer = self.mir_expr(state, operand, out)?;
                        return Ok(Value::Expr(self.first_item_pointer(pointer)));
                    }
                    PointerCoercion::Unsize | PointerCoercion::MutToConstPointer | PointerCoercion::ArrayToPointer => {
                        if *coercion == PointerCoercion::Unsize {
                            self.check_unsize(*to, span)?;
                        }
                        return self.mir_operand(state, operand, out);
                    }
                    // A function, or a closure that captures nothing, as a `fn`:
                    // a JS function already (ADR 0125).
                    PointerCoercion::ReifyFnPointer(_)
                    | PointerCoercion::UnsafeFnPointer
                    | PointerCoercion::ClosureFnPointer(_) => {
                        // A binding's, its JS function itself (ADR 0602), read
                        // where its path names it (ADR 0633).
                        if let Some((def_id, args)) = crate::lower::fn_def(operand.ty(decls, tcx))
                            && bindings::is_binding(tcx, def_id)
                        {
                            let named = match operand {
                                Operand::Constant(c) => c.span,
                                _ => span,
                            };
                            return Ok(Value::Expr(self.binding_fn_pointer(def_id, args, named, span)?));
                        }
                        return self.mir_operand(state, operand, out);
                    }
                }
            }
            // Of or to a raw pointer, a handle on a place or a slice (ADR 0407):
            // `pointer_cast`'s.
            Rvalue::Cast(kind, operand, to)
                if crate::lower::pointers::raw_pointee(tcx, operand.ty(decls, tcx)).is_some()
                    || crate::lower::pointers::raw_pointee(tcx, *to).is_some() =>
            {
                let from = operand.ty(decls, tcx);
                let value = self.mir_expr(state, operand, out)?;
                match self.pointer_cast(*kind, value.clone(), (from, *to), span)? {
                    Some(cast) => cast,
                    None => self.cast(value, from, *to, span)?,
                }
            }
            Rvalue::Cast(_, operand, to) => {
                let from = operand.ty(decls, tcx);
                // An enum's discriminant, `ordering as i32`'s: the enum made a
                // number, as `cast` makes it.
                let v = match self.mir_operand(state, operand, out)? {
                    Value::Discriminant(value, enum_ty) => {
                        return Ok(Value::Expr(self.cast(value, enum_ty, *to, span)?));
                    }
                    v => self.value_expr(v, Span::default())?,
                };
                if from == *to {
                    v
                } else if to.is_char() {
                    Expr::call(Expr::member(Expr::var("String"), "fromCharCode"), vec![v])
                } else if self.whole_cast_at(span) {
                    v
                } else {
                    self.cast(v, from, *to, span)?
                }
            }
            Rvalue::Discriminant(place)
                if place.projection.is_empty()
                    && let Some((tried, ty)) = state.branches.get(&place.local) =>
            {
                return Ok(Value::Branch(tried.clone(), *ty));
            }
            // A place found, in units, whose `None` is `-1`; a strip's
            // `Option` read as its test (ADR 0642).
            Rvalue::Discriminant(place) if place.projection.is_empty() && state.offsets.in_units(place.local) => {
                let found = self.mir_place(state, *place, span, out)?;
                return Ok(Value::Tested(crate::lower::std_types::text::found(found)));
            }
            Rvalue::Discriminant(place)
                if place.projection.is_empty()
                    && let Some((test, _)) = state.cuts.get(&place.local) =>
            {
                return Ok(Value::Tested(test.clone()));
            }
            // A value read as the registry's symbol, only tested against
            // the registry's: a `Some`, its value compared by `===` to each
            // (ADR 0634).
            Rvalue::Discriminant(place)
                if place.projection.is_empty() && state.options.registered.contains(&place.local) =>
            {
                Expr::int(1)
            }
            Rvalue::Discriminant(place) => {
                let ty = place.ty(decls, tcx).ty;
                let subject = self.mir_place(state, *place, span, out)?;
                return Ok(Value::Discriminant(subject, ty));
            }
            Rvalue::Aggregate(kind, operands) => return self.mir_aggregate(state, kind, operands, span, out),
            // `[x; N]` (ADR 0107); an item that's a constant is a named one's,
            // or a literal, `Copy`.
            Rvalue::Repeat(operand, count) => {
                let item_ty = operand.ty(decls, tcx);
                let constant = matches!(operand, Operand::Constant(_));
                let item = self.mir_expr(state, operand, out)?;
                self.repeat((item, item_ty), *count, constant, span, out)?
            }
            // A borrow only for a slice's length, a bounds check's or a slice
            // pattern's test, which nothing points through: the slice, whose
            // `PtrMetadata` is its `length`, a subslice's copy's too.
            Rvalue::RawPtr(mir::RawPtrKind::FakeForPtrMetadata, place) => self.mir_place(state, *place, span, out)?,
            // A pointer to a JS object, `std::ptr::eq`'s: the object (ADR 0285).
            Rvalue::RawPtr(_, place) if self.is_js_object(place.ty(decls, tcx).ty) => {
                self.mir_place(state, *place, span, out)?
            }
            // A pointer to a place, a handle on it (ADR 0407); of what a pointer
            // points to, `&raw const *p`, that pointer; of a slice, the slice.
            Rvalue::RawPtr(_, place) => {
                if let [PlaceElem::Deref] = place.projection.as_slice()
                    && self.is_handle_pointer(decls[place.local].ty)
                {
                    let pointer = self.read_local_value(state, place.local, out)?;
                    return Ok(pointer);
                }
                // Of what a reference made once of a whole local refers to, a
                // pointer to that local.
                if let [PlaceElem::Deref] = place.projection.as_slice()
                    && let Some(&local) = state.pointed.refs.get(&place.local)
                    && state.pointed.locals.contains(&local)
                {
                    let target = self.mir_place(state, Place::from(local), span, out)?;
                    return Ok(Value::Expr(self.pointer_to(
                        target,
                        place.ty(decls, tcx).ty,
                        Some(0),
                        span,
                    )?));
                }
                self.check_slice_item_pointer((state.body, &state.pointed.copies), *place, span)?;
                let pointee = place.ty(decls, tcx).ty;
                let target = self.mir_place(state, *place, span, out)?;
                // Only read through, of what a reference or box refers to:
                // where it is matters to nothing (`Pointed::read_only`).
                if let [PlaceElem::Deref] = place.projection.as_slice()
                    && state.pointed.read_only.contains(&place.local)
                {
                    return Ok(Value::Expr(self.read_only_pointer_to(target, pointee, span)?));
                }
                // Of what a `&mut` refers to, a value JS can't change in place:
                // through its box or handle, `r.value`, whose place isn't the
                // one it refers to, which another pointer to it wouldn't be.
                if let Some((base, PlaceElem::Deref)) = place.as_ref().last_projection()
                    && let ty::Ref(_, referent, ty::Mutability::Mut) =
                        *Place::ty_from(base.local, base.projection, decls, tcx).ty.kind()
                    && self.is_cell_pointee(referent)
                {
                    let base = Place {
                        local: base.local,
                        projection: tcx.mk_place_elems(base.projection),
                    };
                    let reference = self.mir_place(state, base, span, out)?;
                    if js::same_path(&target, &Expr::held(reference)) {
                        return Err(refuse(
                            tcx,
                            span,
                            Refused::Memory("a raw pointer to what a `&mut` refers to"),
                        ));
                    }
                }
                // Of an object a reference read from a slot or a field refers
                // to, `&*refs[0]`: the object's, not a handle on that slot,
                // which its JS, the object, reads as (ADR 0562).
                if let Some((base, PlaceElem::Deref)) = place.as_ref().last_projection()
                    && !base.projection.is_empty()
                    && matches!(
                        Place::ty_from(base.local, base.projection, decls, tcx).ty.kind(),
                        ty::Ref(..)
                    )
                    && self.is_object(pointee)
                    && !self.is_cell_pointee(pointee)
                {
                    return Ok(Value::Expr(Expr::call(Expr::helper("$ptrTo"), vec![target])));
                }
                // Its byte offset in its struct, where the struct's layout is
                // known here (ADR 0562).
                let offset = self.field_offset(*place, decls);
                self.pointer_to(target, pointee, offset, span)?
            }
            _ => return Err(self.unsupported(span, "this value, from its MIR")),
        }))
    }

    /// The byte offset of `place`'s last field in its struct, rustc's
    /// layout for wasm32 (ADR 0562): where its address is in the JS object
    /// the field is on. `None` of a place that isn't a field's, or of a
    /// struct a caller lays out.
    fn field_offset(&self, place: Place<'tcx>, decls: &IndexVec<Local, mir::LocalDecl<'tcx>>) -> Option<u64> {
        let (base, PlaceElem::Field(field, _)) = place.as_ref().last_projection()? else {
            return None;
        };
        let parent = Place::ty_from(base.local, base.projection, decls, self.tcx).ty;
        let layout = self.layout(parent)?;
        Some(layout.fields.offset(field.index()).bytes())
    }

    fn mir_aggregate(
        &mut self,
        state: &mut State<'_, 'tcx>,
        kind: &AggregateKind<'tcx>,
        operands: &IndexVec<FieldIdx, Operand<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        // `P { x, ..base }`, where THIR says it is (`written_base`): its other
        // fields `base`'s, `{ ...base, x }` (ADR 0250).
        if let AggregateKind::Adt(did, VariantIdx::ZERO, args, None, None) = *kind
            && self.tcx.adt_def(did).is_struct()
            && let ty = Ty::new_adt(self.tcx, self.tcx.adt_def(did), args)
            && let Some(spread) = self.written_base(state, ty, operands, span)
        {
            let Shape::Object(names) = self.shape(ty) else {
                unreachable!("a struct of named fields")
            };
            let base = self.mir_place(state, spread, span, out)?;
            let of_base = |i: usize, operand: &Operand<'tcx>| {
                operand.place().is_some_and(|p| {
                    p.local == spread.local
                        && p.projection.split_last().is_some_and(|(last, rest)| {
                            matches!(last, PlaceElem::Field(f, _) if f.as_usize() == i)
                                && rest == spread.projection.as_slice()
                        })
                })
            };
            let given: Vec<&Operand<'tcx>> = (operands.iter().enumerate())
                .filter(|&(i, o)| !of_base(i, o))
                .map(|(_, o)| o)
                .collect();
            let values = self.take_operands(state, &given, out)?;
            let mut props = vec![Prop::Spread(base)];
            let mut values = values.into_iter();
            for (i, operand) in operands.iter().enumerate() {
                if of_base(i, operand) {
                    continue;
                }
                let value = self.value_expr(values.next().expect("given"), span)?;
                props.push(Prop::Field(names[i].0.clone(), self.holding(value, names[i].1)));
            }
            return Ok(Value::Expr(Expr::object(props)));
        }
        let operands: Vec<&Operand<'tcx>> = operands.iter().collect();
        let values = self.take_operands(state, &operands, out)?;
        match kind {
            AggregateKind::Array(_) => Ok(Value::List(values)),
            AggregateKind::Tuple if values.is_empty() => Ok(Value::Expr(Expr::undefined())),
            AggregateKind::Tuple => Ok(Value::List(values)),
            // A union, holding its one field (ADR 0406).
            AggregateKind::Adt(did, _, args, _, Some(field)) => {
                let ty = Ty::new_adt(self.tcx, self.tcx.adt_def(*did), args);
                let value = values.into_iter().next().expect("a union's field");
                let value = self.value_expr(value, span)?;
                Ok(Value::Expr(self.union_value(ty, *field, value)))
            }
            AggregateKind::Adt(did, variant_index, args, _, _) => {
                let ty = Ty::new_adt(self.tcx, self.tcx.adt_def(*did), args);
                let exprs = values
                    .into_iter()
                    .map(|v| self.value_expr(v, span))
                    .collect::<R<Vec<_>>>()?;
                let nulled: Vec<bool> = operands.iter().map(|o| self.nulled(state, o)).collect();
                let value = self.adt_value(ty, *variant_index, exprs, &nulled, span)?;
                // A compiled std iterator's struct, `Peekable { iter, peeked }`
                // in std's own `peekable`, is the JS iterator it is where
                // it's made, stepped by the compiled impl's `next` and sized
                // by its `size_hint`, given this body's dictionaries; in its
                // `Clone`, which has none of `I: Iterator`'s, as the one it
                // clones is, `self.$steps` (ADR 0561).
                if crate::lower::recognition::is_compiled_std_iterator(self.tcx, ty) {
                    let cloning = self.tcx.trait_impl_of_assoc(self.item).is_some_and(|imp| {
                        self.tcx.is_lang_item(self.tcx.impl_trait_id(imp), LangItem::Clone)
                            && self.tcx.type_of(imp).instantiate_identity().skip_normalization() == ty
                    });
                    let steps = match cloning {
                        true => Expr::member(Expr::var(&state.locals.names[Local::from_u32(1)]), "$steps"),
                        false => self.std_iterator_steps(ty, span)?,
                    };
                    return Ok(Value::Expr(Expr::call_helper("$stdIterator", vec![value, steps])));
                }
                Ok(Value::Expr(value))
            }
            AggregateKind::Closure(def_id, _) => Ok(Value::Expr(self.mir_closure(state, *def_id, values, span, out)?)),
            // An `async` body, what it holds as a closure's environment.
            AggregateKind::Coroutine(def_id, _) if self.tcx.coroutine_is_async(*def_id) => {
                Ok(Value::Expr(self.mir_closure(state, *def_id, values, span, out)?))
            }
            // An `async` closure, which makes its body each call.
            AggregateKind::CoroutineClosure(def_id, _) => {
                Ok(Value::Expr(self.mir_closure(state, *def_id, values, span, out)?))
            }
            _ => Err(self.unsupported(span, "this aggregate, from its MIR")),
        }
    }

    /// Whether `operand`, an `Option`, is `null` or a value already, as a
    /// `#[rust_js::nullable]` field is (ADR 0275): such a field, moved where
    /// it's read, or bound to a variable, or a `Some(..)`.
    fn nulled(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> bool {
        let Some(place) = operand.place() else {
            return false;
        };
        if let Some((PlaceElem::Field(field, _), parent)) = place.projection.split_last() {
            let parent = mir::PlaceRef {
                local: place.local,
                projection: parent,
            };
            let mir::PlaceTy { ty, variant_index } = parent.ty(state.body, self.tcx);
            return match ty.kind() {
                ty::Adt(adt, _) if !adt.is_enum() || variant_index.is_some() => {
                    let variant = adt.variant(variant_index.unwrap_or(VariantIdx::ZERO));
                    bindings::is_nullable(self.tcx, &variant.fields[*field])
                }
                _ => false,
            };
        }
        if !place.projection.is_empty() {
            return false;
        }
        if state.options.null_nones.contains(&place.local) {
            return true;
        }
        if state.locals.writes[place.local] != 1 {
            return false;
        }
        // What it's assigned, once: another such, or `Some(..)`.
        let assigned = (state.body.basic_blocks.iter().flat_map(|b| &b.statements)).find_map(|s| match &s.kind {
            mir::StatementKind::Assign(assign) if assign.0 == place.local.into() => Some(&assign.1),
            _ => None,
        });
        // A clone of such, `m.rule_id.clone()`, of an item a clone gives as
        // it is, a string's, a number's or a reference's: its JS value.
        let itself = self
            .option_of(state.body.local_decls[place.local].ty)
            .is_some_and(|item| item.is_ref() || item.is_primitive() || self.is_string_like(item));
        let cloned = (state.body.basic_blocks.iter()).find_map(|b| match &b.terminator().kind {
            mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } if *destination == place.local.into() => Some((func, args)),
            _ => None,
        });
        if itself
            && let Some((func, args)) = cloned
            && let Some((def_id, _)) = fn_def(func.ty(state.body, self.tcx))
            && (self.tcx.trait_of_assoc(def_id)).is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Clone))
            && let [arg] = &args[..]
            && let Some(reference) = arg.node.place()
            && reference.projection.is_empty()
            && state.locals.writes[reference.local] == 1
            && let Some(Rvalue::Ref(_, _, borrowed)) = (state.body.basic_blocks.iter().flat_map(|b| &b.statements))
                .find_map(|s| match &s.kind {
                    mir::StatementKind::Assign(assign) if assign.0 == reference.local.into() => Some(&assign.1),
                    _ => None,
                })
        {
            return self.nulled(state, &Operand::Copy(*borrowed));
        }
        match assigned {
            Some(Rvalue::Use(operand, _)) => self.nulled(state, operand),
            Some(Rvalue::Aggregate(kind, fields)) => {
                fields.len() == 1
                    && matches!(**kind, AggregateKind::Adt(did, variant, ..)
                    if variant.as_u32() == 1 && self.tcx.is_lang_item(did, LangItem::Option))
            }
            _ => false,
        }
    }
}
