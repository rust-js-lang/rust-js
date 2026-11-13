//! An rvalue as JS, and an `Option` already `null` or a value; an
//! aggregate made is `aggregate.rs`'s, what field a place is `fields.rs`'s.

use super::*;
use crate::lower::pointers::Offset;
use crate::lower::temporary_names::{Base, Role};

mod aggregate;
mod fields;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `terminator` calls a `Cell`'s `get`, `set`, `replace` or
    /// `take` of `made`, a reference to it, which keeps nothing of `made`
    /// once it returns (ADR 0853).
    fn keeps_no_cell(&self, terminator: &mir::Terminator<'tcx>, made: Local) -> bool {
        let TerminatorKind::Call { func, args, .. } = &terminator.kind else {
            return false;
        };
        let Some((def_id, generic_args)) = func.const_fn_def() else {
            return false;
        };
        matches!(args.first().map(|a| &a.node), Some(Operand::Move(p) | Operand::Copy(p)) if p.as_local() == Some(made))
            && matches!(
                self.recognition().classify(def_id, generic_args),
                Some(Std::CellGet | Std::CellSet | Std::CellReplace | Std::CellTake)
            )
    }

    /// The place `handle` is on, a handle that stores what's written as
    /// its place does, `null` for `None` of a nullable one (`nullable_ref`),
    /// as the `&mut`'s place: its `value`, of a variable holding it, which
    /// tidy reads and writes as the place, as its storage stores (ADR 0791,
    /// #541). `Expr::held` of the handle itself would be the place alone,
    /// which stores what it's written.
    pub(super) fn through_handle(&mut self, handle: Expr, out: &mut Vec<Stmt>) -> Expr {
        let on = match &handle.kind {
            js::ExprKind::Handle(on, _) => Some((**on).clone()),
            _ => None,
        };
        let handle = self.spill(Role::Place, handle, out);
        if let (js::ExprKind::Var(name), Some(on)) = (&handle.kind, on) {
            self.locals.mir.made.handles.insert(name.clone(), on);
        }
        Expr::held(handle)
    }

    /// The place `place` is, of a handle `through_handle` holds, `fiber.owner`
    /// of `place.value`; `None` of any other.
    pub(super) fn unhandled(&self, place: &Expr) -> Option<Expr> {
        match &place.kind {
            js::ExprKind::Member(of, key) if key == js::HELD => match &of.kind {
                js::ExprKind::Var(name) => self.locals.mir.made.handles.get(name).cloned(),
                _ => None,
            },
            _ => None,
        }
    }

    pub(super) fn mir_rvalue(
        &mut self,
        state: &mut State<'_, 'tcx>,
        rvalue: &Rvalue<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let tcx = self.tcx;
        // Where each object a reference or a pointer is made into is
        // stored, recorded first, so its address is its container's
        // (#1113).
        if let Rvalue::Ref(_, _, place) | Rvalue::RawPtr(mir::RawPtrKind::Mut | mir::RawPtrKind::Const, place) = rvalue
        {
            self.record_stored(state, *place, span, out)?;
        }
        let decls = &state.body.local_decls;
        Ok(Value::Expr(match rvalue {
            Rvalue::Use(operand, _) => return self.mir_operand(state, operand, out),
            Rvalue::CopyForDeref(place) if self.through_values(state.body, place, true) => {
                return self.read_local_value(state, place.local, out);
            }
            // A field held as generic code's box: what it's to (#1405).
            Rvalue::CopyForDeref(place) => match self.boxed_field(state, place.as_ref()) {
                Some(crossing) => {
                    let read = self.mir_place(state, *place, span, out)?;
                    self.form_back(Form::Box, crossing, read)
                }
                None => self.mir_place(state, *place, span, out)?,
            },
            // A reference is what it refers to (ADR 0023): a `&mut` to a
            // value JS can't share, a number's, needs what isn't made yet.
            Rvalue::Ref(_, kind, place) => {
                // A field held as generic code's box (#1405): through a
                // shared reference, what it's to, as this code has it; a
                // `&mut` to it would be to the box.
                if let Some(crossing) = self.boxed_field(state, place.as_ref()) {
                    if let BorrowKind::Mut { .. } = kind {
                        return Err(self.unsupported(
                            span,
                            "a `&mut` to a field that holds a `&mut` generic code has as its box, `&mut slot.at` of `struct Slot<'a, T> { at: &'a mut T }` (#1421): write it whole, `slot.at = &mut other`, or reborrow what it's to, `&mut *slot.at`",
                        ));
                    }
                    let read = self.mir_place(state, *place, span, out)?;
                    return Ok(Value::Expr(self.form_back(Form::Box, crossing, read)));
                }
                // A field held where it's made (#470, rd645): through a
                // shared reference, the box code that doesn't hold it so
                // reads, where `assign` hasn't read it as it's held; never
                // through a `&mut`, which would write the box in it.
                if self.nested_field(state.body, place.as_ref()) {
                    if matches!(kind, BorrowKind::Mut { .. }) {
                        return Err(self.unsupported(
                            span,
                            "a `&mut` to a `#[rust_js::nullable]` field that holds its `Option` as it's made, `null` for `Some(None)` of an `Option<Option<T>>`, a `Some`'s `undefined` as it is: set the field whole, `s.field = Some(None)` or `s.field = None`",
                        ));
                    }
                    let ty = place.ty(&state.body.local_decls, self.tcx).ty;
                    let held = self.mir_place(state, *place, span, out)?;
                    return Ok(Value::Expr(self.boxed_of_held(held, ty, out)));
                }
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
                        false => self.spill(Role::Value, parent, out),
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
                // A shared reference to a `&mut` whose type says it's a cell, held
                // as its place's own name, a closure's capture of it by name or
                // an alias of one (ADR 0099): a handle on that place, the cell
                // its type says, so what reads through it reads the place, as
                // `Borrow::borrow(&r)` of a captured `r` does (#1514).
                if matches!(kind, BorrowKind::Shared)
                    && let ty::Ref(_, referent, ty::Mutability::Mut) = *place.ty(decls, tcx).ty.kind()
                    && self.mut_ref_form(referent).is_cell()
                    && self.held_as_its_place(state, place)
                {
                    let target = self.mir_place(state, *place, span, out)?;
                    return Ok(Value::Place(Expr::handle(target)));
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
                                js::ExprKind::Object(..) => {
                                    let kind = js::ExprKind::Member(Box::new(cell), "value".into());
                                    Expr {
                                        kind,
                                        ..Expr::undefined()
                                    }
                                }
                                // A handle that stores as its place does, a
                                // nullable field's cell's `get_mut()` (#541).
                                js::ExprKind::Handle(_, js::Storage::NullForNone) => {
                                    self.flush(state, out)?;
                                    self.through_handle(cell, out)
                                }
                                _ => Expr::held(cell),
                            };
                            return Ok(Value::Ref(place));
                        }
                        aliased @ Value::Ref(_) => return Ok(aliased),
                        _ => return Err(self.unsupported(span, "this reborrow, from its MIR")),
                    }
                }
                if matches!(kind, BorrowKind::Mut { .. }) && self.mut_ref_form(pointee).is_cell() {
                    let target = self.mir_place(state, *place, span, out)?;
                    // Of a reference written again, `list` of `&mut list.value`
                    // in a loop that moves `list` on: what it is now, held.
                    let target = match self.local_reference_written(state, place) {
                        true => {
                            let name = state.locals.names[place.local].clone();
                            self.flush(state, out)?;
                            let held = self.spill(Base::Named(&name), Expr::var(&name), out);
                            // A place holds no closure, so `substitute` renames
                            // it; else it's refused (#1525).
                            target
                                .substitute(&|v| (v == name).then(|| held.clone()))
                                .ok_or_else(|| {
                                    self.unsupported(span, "a place a closure reads, which can't be renamed")
                                })?
                        }
                        false => target,
                    };
                    // Through a reference a field holds, `&mut (*h.node).count`,
                    // or an item at an index set again: the place as it is
                    // now (`reached_through_written`).
                    let target = match self.reached_through_written(state, place) {
                        true => {
                            self.flush(state, out)?;
                            self.place_where_made(target, out)
                        }
                        false => target,
                    };
                    // `&mut **u`, a reborrow of a cell a place holds, an
                    // `Option`'s `&mut` through a `ref mut`: that cell's
                    // `value`, the place kept as one that holds a cell, as a
                    // variable is, so a handle on it is the cell. And `&mut
                    // *p` of a handle pointer, read and written by its `value`
                    // as a cell is: the pointer, the place it's on, not a
                    // handle made at each reborrow, which no record holds,
                    // httparse's `&mut *(s as *mut &mut [MaybeUninit<T>])`
                    // (#1513).
                    if let Some((base, PlaceElem::Deref)) = place.as_ref().last_projection()
                        && (self.is_cell(base.ty(decls, tcx).ty) || self.is_handle_pointer(base.ty(decls, tcx).ty))
                        && let js::ExprKind::Member(cell, field) = &target.kind
                        && field == js::HELD
                        && fixed_place(cell)
                    {
                        self.locals.mir.cells.push((**cell).clone());
                    }
                    return self.nullable_ref(state, *place, target, out);
                }
                // `&error.title` of a `Cell` a field holds, a handle on it,
                // through a reference written again, `error = other` after,
                // or through one a field holds, `node.0 = other` (#1034): of
                // the place as it is now, as a `&mut`'s above. Not where the
                // handle's read by its `Cell`'s `get` or `set` before what
                // it's reached through is set again, `error.title.get()`: the
                // place it reads then is the one it was (ADR 0853).
                let (local, within) = match matches!(kind, BorrowKind::Shared)
                    && (self.is_std_type(pointee, StdItem::Cell)
                        || self
                            .place_field(state, *place)
                            .is_some_and(|field| self.property_cell(Some(field), pointee).is_some()))
                {
                    true => (
                        self.local_reference_written(state, place) && !state.locals.names[place.local].is_empty(),
                        self.reached_through_written(state, place),
                    ),
                    false => (false, false),
                };
                if (local || within)
                    && !(self.is_std_type(pointee, StdItem::Cell)
                        && read_before_written(
                            state,
                            (state.location(), rvalue),
                            |t, made| self.keeps_no_cell(t, made),
                            |written| writes_reach(place, written),
                        ))
                {
                    let mut target = self.mir_place(state, *place, span, out)?;
                    self.flush(state, out)?;
                    if local {
                        let name = state.locals.names[place.local].clone();
                        let held = self.spill(Base::Named(&name), Expr::var(&name), out);
                        // A place holds no closure, so `substitute` renames it;
                        // else it's refused, never left reading the variable
                        // the snapshot stands for (#1525).
                        target = (target.substitute(&|v| (v == name).then(|| held.clone())))
                            .ok_or_else(|| self.unsupported(span, "a place a closure reads, which can't be renamed"))?;
                    }
                    if within {
                        target = self.place_where_made(target, out);
                    }
                    return Ok(Value::Place(target));
                }
                if self.through_values(state.body, place, true) {
                    let value = self.read_local_value(state, place.local, out)?;
                    return Ok(match (kind, value) {
                        // Of what can't change meanwhile, the body's own locals: read
                        // where it's used. A temporary's read of what can, a
                        // static's, is made where it's made.
                        // Of a reborrow of a variable the program names,
                        // `&*form`, a call's argument say, where it's
                        // written, for the source map: a comment before it
                        // there before it, `.r(/* .. */ form)` (rd205).
                        (BorrowKind::Shared, Value::Expr(e))
                            if e.reads_same() && movable(state, &Value::Expr(e.clone())) =>
                        {
                            let reborrow =
                                place.projection.as_slice() == [PlaceElem::Deref] && state.locals.user[place.local];
                            match reborrow && given_to_call(state) {
                                true => Value::Place(e.or_at(self.js_span(span))),
                                false => Value::Place(e),
                            }
                        }
                        // Through `outer`, a `&mut` to `list`, a `&mut` to what
                        // isn't a cell: what `list` is.
                        (_, Value::Ref(e)) if !place.projection.is_empty() && !self.mut_ref_form(pointee).is_cell() => {
                            Value::Expr(e)
                        }
                        (_, value) => value,
                    });
                }
                let mut place_expr = self.mir_place(state, *place, span, out)?;
                // A handle on a place a call reaches, `&get(c).value`'s: what
                // holds it made once, first, as the handle reads its place
                // each time it's used.
                if let js::ExprKind::Handle(on, storage) = &place_expr.kind
                    && !fixed_place(on)
                {
                    let (on, storage) = ((**on).clone(), *storage);
                    self.flush(state, out)?;
                    let on = self.fixed_place_of(on, out);
                    place_expr = Expr::stored_handle(on, storage);
                }
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
                        Value::Discriminant(_, enum_ty, _) => *enum_ty,
                        _ => ty,
                    };
                    let test = self.switch_test(&discriminant, enum_ty, variant.to_bits_unchecked(), span, None)?;
                    return Ok(Value::Expr(match op {
                        BinOp::Eq => test,
                        _ => test.negated_test(),
                    }));
                }
                // A range pattern's test, `2 <= k`: the value first, `k >= 2`;
                // of a bound the type has anyway, `0 <= k` of a `u8`, none,
                // as `binary` answers of every comparison (`at_bound`). And a
                // constant pattern's that isn't a switch, a float's, `1 ==
                // kind`: `kind === 1`.
                let flipped = match op {
                    BinOp::Le => Some(BinOp::Ge),
                    BinOp::Lt => Some(BinOp::Gt),
                    BinOp::Eq | BinOp::Ne => Some(*op),
                    _ => None,
                };
                if let Some(flipped) = flipped
                    && let Operand::Constant(_) = l
                    && !matches!(r, Operand::Constant(_))
                    && state.facts.tested().iter().any(|pattern| pattern.contains(span))
                {
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
                        .is_some_and(|p| p.projection.is_empty() && state.facts.offsets().in_units(p.local))
                };
                if *op == BinOp::Add && (in_units(l) || in_units(r)) {
                    let at = mir::Location {
                        block: state.at,
                        statement_index: state.statement,
                    };
                    let step = state.facts.offsets().step(at);
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
                    // Not one whose overflow is read as a value, an inlined
                    // `checked_add`'s, `None` where it overflows: the
                    // operation's overflowing form, `$overflowing` (ADR 0899).
                    BinOp::AddWithOverflow | BinOp::SubWithOverflow | BinOp::MulWithOverflow
                        if !state.body.source.def_id().is_local() && overflow_read(state.body, state.location()) =>
                    {
                        let wrapping = op.overflowing_to_wrapping().expect("an overflowing operation");
                        let op = NumOp::Overflowing(wrapping);
                        return Ok(Value::Expr(self.number_call(op, vec![l, r], ty, span, out)?));
                    }
                    BinOp::AddWithOverflow | BinOp::SubWithOverflow | BinOp::MulWithOverflow
                        if matches!(
                            tcx.def_kind(state.body.source.def_id()),
                            DefKind::Const | DefKind::AssocConst | DefKind::AnonConst | DefKind::Static { .. }
                        ) || !state.body.source.def_id().is_local() =>
                    {
                        let wrapping = op.overflowing_to_wrapping().expect("an overflowing operation");
                        let value = self.binary(wrapping, l, r, known, ty, span)?;
                        return Ok(Value::List(
                            vec![Value::Expr(value), Value::Expr(Expr::bool(false))],
                            js::Span::NONE,
                        ));
                    }
                    BinOp::AddWithOverflow | BinOp::SubWithOverflow | BinOp::MulWithOverflow => {
                        return Err(self.unsupported(span, "arithmetic checked for overflow, from its MIR"));
                    }
                    BinOp::Cmp => return Err(self.unsupported(span, "`cmp`, from its MIR")),
                    // `unchecked_add`'s, which rustc's MIR inlines into std's
                    // bodies, only where it can't overflow, `checked_add`'s
                    // sum past its check: the operation's (ADR 0899).
                    BinOp::AddUnchecked | BinOp::SubUnchecked | BinOp::MulUnchecked => {
                        let op = match op {
                            BinOp::AddUnchecked => BinOp::Add,
                            BinOp::SubUnchecked => BinOp::Sub,
                            _ => BinOp::Mul,
                        };
                        self.binary(op, l, r, known, ty, span)?
                    }
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
                            .dyn_drop_lost(*to, self.pointee(from), self.pointee(*to))
                        {
                            return Err(self.unsupported(span, "a `dyn` of a value with a destructor"));
                        }
                        // A binding's function lent as a `&dyn Fn`, as one as
                        // a `fn`: the JS function itself (ADR 0602, amended),
                        // whose identity a caller may compare, React's
                        // default transition indicator (rd289).
                        if from.is_ref()
                            && let ty::FnDef(def_id, args) = *self.pointee(from).kind()
                            && bindings::is_binding(self.tcx, def_id)
                        {
                            let args = args.skip_binder();
                            return Ok(Value::Expr(self.binding_fn_pointer(def_id, args, span, span)?));
                        }
                        let value = self.mir_expr(state, operand, out)?;
                        // A raw pointer's: its place's value made the `dyn`,
                        // and a pointer to that again where the target is a
                        // handle on it, `*const Foo<dyn Trait>`; a `*const dyn
                        // Trait` is the pair itself (ADR 0562).
                        if self.is_handle_pointer(from) {
                            // A pointer to a value of no size has no place;
                            // the value is `undefined`, but a function's,
                            // which is the function (`has_no_place`).
                            let place = match self.has_no_place(self.pointee(from)) {
                                true => Expr::undefined(),
                                false => self.pointee_place(value),
                            };
                            let unsized_ = self.unsize_trait(from, *to, place, span, out)?;
                            return Ok(Value::Expr(match self.is_handle_pointer(*to) {
                                true => self.pointer_to(unsized_, self.pointee(*to), Offset::Unsaid, span)?,
                                false => unsized_,
                            }));
                        }
                        if self.recognition().is_dyn_iter(*to) {
                            let iterator = from.builtin_deref(true).unwrap_or(from);
                            // Of a crate that asks a `dyn Iterator`'s hint,
                            // one whose hint isn't std's as it's made, which
                            // the `dyn` would give (#1476).
                            if self.krate.hinted.dyn_asked
                                && !self.recognition().is_dyn_iter(iterator)
                                && !self.hint_known_made_dyn(iterator)
                            {
                                return Err(self.unsupported(
                                    span,
                                    &format!("a `{iterator}` made a `dyn Iterator` in a crate that asks a `dyn Iterator`'s `size_hint()`, as its hint isn't std's as it's made (#1563)"),
                                ));
                            }
                            // A `Box`'s of what drops carries its drop, which the
                            // `Box`'s drop runs (ADR 0521, #1420); an `Rc`'s is
                            // refused above.
                            // Not one that's a `dyn Iterator` already, a `Box`'s made
                            // another's, which carries its drop as it is.
                            let carried = to.is_box()
                                && !self.recognition().is_dyn_iter(iterator)
                                && self.drops(iterator) != crate::lower::drops::Drops::Nothing;
                            let value = match carried && !value.reads_same() {
                                true => self.spill(Role::It, value, out),
                                false => value,
                            };
                            let it = match self.knows_its_back(*to) {
                                true => self.as_double_ended(value.clone(), iterator, span)?,
                                false => self.as_js_iterator(value.clone(), iterator, span)?,
                            };
                            let it = crate::lower::iterators::behind_a_dyn(it);
                            if !carried {
                                return Ok(Value::Expr(it));
                            }
                            let mut dropped = Vec::new();
                            self.drop_value(value, iterator, span, &mut dropped)?;
                            return Ok(Value::Expr(Expr::call_helper(
                                "$droppingIterator",
                                vec![it, Expr::arrow(Vec::new(), dropped)],
                            )));
                        }
                        // A `&dyn Debug`, made where it's read: shown there,
                        // by a placeholder, a builder's step or a derived
                        // `Debug`'s fields, the string it shows, as pretty as
                        // they ask (ADR 0137); stored or passed, the pair
                        // `{ value, impl }` (ADR 0060, amended, #1335).
                        if self.is_dyn_debug(*to) {
                            // A pair, read for its dictionary and its value: held
                            // once where it's made, a call's, `&*shapes[1]`.
                            let value = match self.is_pair_dyn(from) && !value.parts_read_same() {
                                true => self.spill(Role::Shown, value, out),
                                false => value,
                            };
                            return Ok(Value::DynDebug(from, *to, value));
                        }
                        let made = self.unsize_trait(from, *to, value, span, out)?;
                        let made = self.dyn_with_place(state, operand, from, made, span, out)?;
                        // A closure lent as a `&dyn Fn`, as one as a `fn`:
                        // the function of the crate's it only adapts with
                        // what it's given, so captures nothing, React's
                        // component `|_, _| transition_aware_host_component()`
                        // (ADR 0605, amended). Not one an `Rc` or a `Box`
                        // owns, whose pointer is its own.
                        if from.is_ref()
                            && matches!(self.pointee(from).kind(), ty::Closure(..))
                            && let Some(callee) = crate::lower::passed_on::passed_on(
                                &made,
                                crate::lower::passed_on::Taker::FnPointer,
                                self.forwarding(),
                            )
                        {
                            return Ok(Value::Expr(callee));
                        }
                        return Ok(Value::Expr(made));
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
                        // A std function that's `Math`'s, its function,
                        // `Math.clz32` of `u32::leading_zeros` (ADR 0887).
                        if let Some((def_id, args)) = crate::lower::fn_def(operand.ty(decls, tcx))
                            && let Some(function) = self.math_function(def_id, args)
                        {
                            return Ok(Value::Expr(function));
                        }
                        // A function's value, a generic function's instance's
                        // in its callers' forms, its `fn` pointer (#1187, #1324).
                        let value = self.mir_operand(state, operand, out)?;
                        // A closure that only calls a function of the crate's
                        // with what it's given, or none of it, is that
                        // function (ADR 0605, amended).
                        if matches!(coercion, PointerCoercion::ClosureFnPointer(_))
                            && let Value::Expr(arrow) = &value
                            && let Some(callee) = crate::lower::passed_on::passed_on(
                                arrow,
                                crate::lower::passed_on::Taker::FnPointer,
                                self.forwarding(),
                            )
                        {
                            return Ok(Value::Expr(callee));
                        }
                        return Ok(value);
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
                match self.pointer_cast(*kind, value.clone(), (from, *to), span, out)? {
                    Some(cast) => cast,
                    None => self.cast(value, from, *to, span)?,
                }
            }
            Rvalue::Cast(kind, operand, to) => {
                let from = operand.ty(decls, tcx);
                // An enum's discriminant, `ordering as i32`'s: the enum made a
                // number, as `cast` makes it.
                let v = match self.mir_operand(state, operand, out)? {
                    Value::Discriminant(value, enum_ty, _) => {
                        return Ok(Value::Expr(self.cast(value, enum_ty, *to, span)?));
                    }
                    v => self.value_expr(v, Span::default())?,
                };
                if from == *to {
                    v
                } else if *kind == mir::CastKind::Transmute {
                    // `transmute`, which rustc's MIR makes a cast, as its
                    // call is lowered (`transmute`): a library's code the
                    // crate compiles from its MIR, ppv-lite86's bytes read
                    // as a `u32x4_generic` (#1420).
                    self.transmute(v, (from, *to), span, out)?
                } else if to.is_char() {
                    Expr::call(Expr::member(Expr::var("String"), "fromCharCode"), vec![v])
                } else if self.kept_cast_at(span) {
                    v
                } else {
                    self.cast(v, from, *to, span)?
                }
            }
            // What an unwind's payload's `&JsError` is, what was thrown, is a
            // `Some`, and `?` of it goes on (ADR 0710).
            Rvalue::Discriminant(place)
                if place.projection.is_empty()
                    && let Some(&discriminant) = state.facts.options().thrown.get(&place.local) =>
            {
                Expr::int(discriminant as i128)
            }
            Rvalue::Discriminant(place)
                if place.projection.is_empty()
                    && let Some((tried, ty)) = state.branches.get(&place.local) =>
            {
                return Ok(Value::Branch(tried.clone(), *ty));
            }
            // An index found, in a string's units or a slice's, whose `None`
            // is `-1` (ADR 0642, rd316).
            Rvalue::Discriminant(place)
                if place.projection.is_empty() && state.facts.none_is_minus_one(place.local) =>
            {
                let found = self.mir_place(state, *place, span, out)?;
                return Ok(Value::Tested(crate::lower::std_types::text::found(found)));
            }
            // A strip's `Option` read as its test (ADR 0642).
            Rvalue::Discriminant(place)
                if place.projection.is_empty()
                    && let Some((test, ..)) = state.cuts.get(&place.local) =>
            {
                return Ok(Value::Tested(test.clone()));
            }
            // A value read as the registry's symbol, only tested against
            // the registry's: a `Some`, its value compared by `===` to each
            // (ADR 0634).
            Rvalue::Discriminant(place)
                if place.projection.is_empty() && state.facts.options().registered.contains(&place.local) =>
            {
                Expr::int(1)
            }
            // An `Option` of an `Option` held as its payload: a `Some` where
            // it isn't `undefined`, its `Some(None)` `null` (ADR 1017); one
            // of a value never `null`, where it isn't `null` (rd645).
            Rvalue::Discriminant(place) if self.held_nested(state, *place, place.projection.len()) => {
                let none = self.held_none(place.ty(decls, tcx).ty);
                let held = self.mir_place(state, *place, span, out)?;
                return Ok(Value::Tested(Expr::bin(Op::Ne, held, none)));
            }
            Rvalue::Discriminant(place) => {
                let ty = place.ty(decls, tcx).ty;
                let present = place.projection.is_empty() && state.facts.options().present.contains(&place.local);
                let subject = self.mir_place(state, *place, span, out)?;
                return Ok(Value::Discriminant(subject, ty, present));
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
                    && let Some(&local) = state.facts.pointed().refs.get(&place.local)
                    && state.facts.pointed().locals.contains(&local)
                {
                    let target = self.mir_place(state, Place::from(local), span, out)?;
                    return Ok(Value::Expr(self.pointer_to(
                        target,
                        place.ty(decls, tcx).ty,
                        Offset::At(0),
                        span,
                    )?));
                }
                self.check_slice_item_pointer((state.body, &state.facts.pointed().copies), *place, span)?;
                let pointee = place.ty(decls, tcx).ty;
                // A place `place_in_root` roots that reads a temporary not
                // yet made, the reference a `Ref`'s `*cell.borrow()` gives:
                // made first, so the place and its root each read it, and
                // not its name alone, which no JS declares (#1110).
                if Self::rooted_from(*place).is_some() {
                    self.read_more_than_once(state, *place, out)?;
                }
                // Of what a `&mut` std lent from inside its receiver refers to,
                // a handle on its place there: that handle, read or written
                // through (#1474).
                if let [PlaceElem::Deref] = place.projection.as_slice()
                    && self.placed_by_lend(state.body, place.local)
                {
                    return self.read_local_value(state, place.local, out);
                }
                // Of what a reborrow of a parameter given its place's pointer
                // refers to, through calls of functions that return their
                // parameter, httparse's `deinit_slice_mut(s)` given its `s`:
                // the reborrow, which is that pointer, the same place's, read
                // before its place is, which would take what it holds (#1513).
                if let [PlaceElem::Deref] = place.projection.as_slice()
                    && let ty::Ref(_, referent, ty::Mutability::Mut) = *decls[place.local].ty.kind()
                    && self.mut_ref_form(referent).is_cell()
                    && let Some(param) = self.reborrowed_from(state.body, place.local, true)
                    && param != place.local
                    && self.given_its_pointer_here(state.body, param)
                {
                    return self.read_local_value(state, place.local, out);
                }
                let target = self.mir_place(state, *place, span, out)?;
                // Only read through, of what a reference or box refers to:
                // where it is matters to nothing (`Pointed::read_only`).
                if let [PlaceElem::Deref] = place.projection.as_slice()
                    && state.facts.pointed().read_only.contains(&place.local)
                {
                    return Ok(Value::Expr(self.read_only_pointer_to(target, pointee, span)?));
                }
                // Of what a `&mut` refers to, a value JS can't change in place:
                // through its box or handle, `r.value`, whose place isn't the
                // one it refers to, which another pointer to it wouldn't be.
                if let Some((base, PlaceElem::Deref)) = place.as_ref().last_projection()
                    && let ty::Ref(_, referent, ty::Mutability::Mut) =
                        *Place::ty_from(base.local, base.projection, decls, tcx).ty.kind()
                    && self.mut_ref_form(referent).is_cell()
                {
                    let base = Place {
                        local: base.local,
                        projection: tcx.mk_place_elems(base.projection),
                    };
                    let reference = self.mir_place(state, base, span, out)?;
                    // A parameter its callers give the pointer to its place
                    // for, a function's that may let it out: that pointer,
                    // the polyfill's `r` (ADR 1114).
                    if base.projection.is_empty() && self.given_its_pointer_here(state.body, base.local) {
                        return Ok(Value::Expr(reference));
                    }
                    // An item a slice's `iter_mut()` lent, a handle on its
                    // slot: the slot's pointer, byteorder's `n as *const
                    // f32` of `for n in numbers` (#1283).
                    if base.projection.is_empty() && self.lent_by_slice_iter_mut(state.body, base.local) {
                        return Ok(Value::Expr(Expr::call_helper("$ptrOfItem", vec![reference])));
                    }
                    // What a call that names no place lends, `$unwrap(o.as_mut())`:
                    // a cell made at each call, whose pointers differ where
                    // Rust's are one place's (#1497); not a place's handle it
                    // lends, `as_mut`'s of a cell local (#1502).
                    if let js::ExprKind::Member(of, key) = &target.kind
                        && key == js::HELD
                        && let js::ExprKind::Call(callee, _) = &of.kind
                        && !callee.helper_name().is_some_and(crate::runtime::names_a_place)
                        && !crate::lower::pointers::lends_a_place_handle(of)
                    {
                        return Err(refuse(
                            tcx,
                            span,
                            Refused::Memory(
                                "a raw pointer to what a `&mut` a call lends refers to, `o.as_mut()`'s, which JS holds in a cell made at each call, a place on no record (#1513)",
                            ),
                        ));
                    }
                    if js::same_path(&target, &Expr::held(reference)) {
                        return Err(refuse(
                            tcx,
                            span,
                            Refused::Memory(
                                "a raw pointer to what a `&mut` refers to, which JS holds in a box of a copy: a function whose signature has a raw pointer in its result or another parameter is given its place's pointer (ADR 1114, #1283)",
                            ),
                        ));
                    }
                }
                // Of an object a reference or a box read from a slot or a
                // field refers to, `&*refs[0]`, `&*boxes[0]`: the object's,
                // not a handle on that slot, which its JS, the object, reads
                // as (ADR 0562), a box's value an allocation of its own
                // (#1114). And of what a reference std lent from inside what
                // holds it as the object refers to, `o.get_or_insert(p)`'s,
                // read as the `Option`'s place (#1114).
                if let Some((base, PlaceElem::Deref)) = place.as_ref().last_projection()
                    && (!base.projection.is_empty() || self.lent_from_a_holder(state.body, base.local))
                    && let held = Place::ty_from(base.local, base.projection, decls, tcx).ty
                    && (held.is_ref() || held.is_box())
                    && self.is_object(pointee)
                    && !self.mut_ref_form(pointee).is_cell()
                {
                    return Ok(Value::Expr(self.object_pointer(target, pointee, span)?));
                }
                // Its byte offset in its struct, where the struct's layout is
                // known here (ADR 0562); of what a reference made once of a
                // field refers to, `*r` of `r = &mut s.n`, that field's (#1471).
                let field = match place.projection.as_slice() {
                    [PlaceElem::Deref] => crate::lower::pointers::made_once_of(state.body, place.local),
                    _ => None,
                };
                let offset = self.field_offset(field.unwrap_or(*place), decls);
                // Of a place reached through more than one field or item, its
                // root's and its offset there, so its address is in the
                // root's allocation, `$ptr(items[1], "b", 0, items, 8)` (#1089).
                let rooted = self.place_in_root(state, *place, span, out)?;
                let pointer = self.pointer_to(target, pointee, offset, span)?;
                match (pointer.kind, rooted) {
                    (js::ExprKind::Call(callee, mut given), Some((root, at)))
                        if callee.helper_name() == Some("$ptr") && matches!(given.len(), 2 | 3) =>
                    {
                        // An array's item says no offset of its own, its
                        // root's says where it is: `$ptr(items[0], 1,
                        // undefined, items, 8)` (#1110).
                        given.resize_with(3, Expr::undefined);
                        given.extend([root, at]);
                        Expr::call(*callee, given)
                    }
                    (kind, _) => Expr { kind, ..pointer },
                }
            }
            _ => return Err(self.unsupported(span, "this value, from its MIR")),
        }))
    }

    /// Whether `local` is written once, by the item a slice's `IterMut`'s
    /// `next()` gave, `n = (next as Some).0`: a handle `$mutItems` made,
    /// which knows its slot (#1283).
    fn lent_by_slice_iter_mut(&self, body: &mir::Body<'tcx>, local: Local) -> bool {
        let writes: Vec<&Rvalue<'tcx>> = (body.basic_blocks.iter())
            .flat_map(|data| &data.statements)
            .filter_map(|statement| match &statement.kind {
                StatementKind::Assign(assign) if assign.0.local == local && assign.0.projection.is_empty() => {
                    Some(&assign.1)
                }
                _ => None,
            })
            .collect();
        let [Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _)] = writes.as_slice() else {
            return false;
        };
        let [PlaceElem::Downcast(_, some), PlaceElem::Field(field, _)] = from.projection.as_slice() else {
            return false;
        };
        if some.as_u32() != 1 || field.as_u32() != 0 {
            return false;
        }
        body.basic_blocks.iter().any(|data| match &data.terminator().kind {
            TerminatorKind::Call { func, destination, .. } if destination.local == from.local && destination.projection.is_empty() => {
                fn_def(func.ty(&body.local_decls, self.tcx)).is_some_and(|(id, args)| {
                    is_std_def(self.tcx, self.tcx.trait_of_assoc(id).unwrap_or(id), StdItem::Iterator)
                        && matches!(args.type_at(0).kind(), ty::Adt(adt, _) if self.tcx.def_path_str(adt.did()) == "std::slice::IterMut")
                })
            }
            _ => false,
        })
    }

    /// Whether `operand` is a `None` and nothing else: a local assigned
    /// once, a `None`, MIR's temporary of `S { label: None }`'s, or a
    /// `Cell::new` of one, `Cell::new(None)`'s. What an optional nullable
    /// field leaves out (ADR 0810, amended).
    pub(super) fn written_none(&self, body: &mir::Body<'tcx>, locals: &Locals, operand: &mir::Operand<'tcx>) -> bool {
        let Some(place) = operand.place() else {
            return false;
        };
        if !place.projection.is_empty() || locals.writes[place.local] != 1 {
            return false;
        }
        for block in body.basic_blocks.iter() {
            for statement in &block.statements {
                if let mir::StatementKind::Assign(assign) = &statement.kind
                    && assign.0 == place.local.into()
                {
                    return matches!(&assign.1, Rvalue::Aggregate(kind, _)
                        if matches!(**kind, AggregateKind::Adt(did, variant, ..)
                            if variant.as_u32() == 0 && self.tcx.is_lang_item(did, LangItem::Option)));
                }
            }
            if let mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &block.terminator().kind
                && *destination == place.local.into()
            {
                return (func.const_fn_def()).is_some_and(|(def_id, generic_args)| {
                    self.recognition().classify(def_id, generic_args) == Some(Std::CellNew)
                }) && matches!(&args[..], [arg] if self.written_none(body, locals, &arg.node));
            }
        }
        false
    }

    /// The place `operand`, a reference made once, borrows: `(*d).field` of
    /// `_4 = &((*d).field)`.
    pub(super) fn borrowed(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> Option<Place<'tcx>> {
        let reference = operand.place()?;
        if !reference.projection.is_empty() || state.locals.writes[reference.local] != 1 {
            return None;
        }
        match (state.body.basic_blocks.iter().flat_map(|b| &b.statements)).find_map(|s| match &s.kind {
            mir::StatementKind::Assign(assign) if assign.0 == reference.local.into() => Some(&assign.1),
            _ => None,
        }) {
            Some(Rvalue::Ref(_, _, borrowed)) => Some(*borrowed),
            _ => None,
        }
    }
}

/// Whether `index`, a temporary, is a copy of a variable set again,
/// `_5 = copy i` before `i = 2`: what its read is folded into, the
/// variable, is another value by the time a handle made of it is read
/// (#988).
fn index_copied_from_set(state: &State<'_, '_>, index: mir::Local) -> bool {
    (state.body.basic_blocks.iter().flat_map(|block| &block.statements)).any(|statement| {
        matches!(&statement.kind, StatementKind::Assign(assign)
            if assign.0 == index.into()
                && matches!(&assign.1, Rvalue::Use(mir::Operand::Copy(from) | mir::Operand::Move(from), _)
                    if written_again(state, from.local)))
    })
}

/// Whether what `written`, a local's projection, writes holds what `path`,
/// another of the same local, reaches: it's a start of `path`, an item at
/// an index of an array or a slice being any item at another, `cur[0] =
/// &mut b` of `(*cur[i]).count` (#1075).
pub(super) fn within<'tcx>(path: &[PlaceElem<'tcx>], written: &[PlaceElem<'tcx>]) -> bool {
    let item = |elem: &PlaceElem<'tcx>| {
        matches!(
            elem,
            PlaceElem::Index(_) | PlaceElem::ConstantIndex { .. } | PlaceElem::Subslice { .. }
        )
    };
    path.len() >= written.len() && (path.iter().zip(written)).all(|(a, b)| a == b || (item(a) && item(b)))
}

/// Whether the reference a part of `local` holds, `holder`, `h.node` or a
/// box's `*h.node`, is given another anywhere in `body`, `h.node = &mut
/// other`, or what holds it is, `h.node = Box::new(..)`, or either is lent
/// to be written, `|| h.node = other` (`written_or_lent`): a borrow
/// through it, `&mut (*h.node).count`, may outlive that write, as borrowck
/// lets it (#988, #1034).
fn reference_written<'tcx>(body: &mir::Body<'tcx>, local: mir::Local, holder: &[PlaceElem<'tcx>]) -> bool {
    (body.basic_blocks.iter().flat_map(|block| &block.statements)).any(|statement| {
        written_or_lent(statement).any(|written| {
            written.local == local && !written.projection.is_empty() && within(holder, written.projection)
        })
    })
}

impl<'tcx> FnCx<'_, 'tcx> {
    /// Whether `place` is reached through a reference its local holds as
    /// its value, `*r` (`through_local_reference`), and the local is set
    /// again, `r = other`: a borrow through it is of what it is now, the
    /// local held where the borrow's made.
    fn local_reference_written(&self, state: &State<'_, 'tcx>, place: &Place<'tcx>) -> bool {
        self.through_local_reference(state.body, place) && written_again(state, place.local)
    }

    /// Whether `place` is reached through what's set again within its
    /// local while a borrow of it lives: a reference a part of the local
    /// holds, `(*h.node).count`, given another, `h.node = other` (#988), or
    /// the local set again whole, `h = H(other)` (#1034); or an item at an
    /// index a variable set again holds, `a[i]` before `i = 2`. A borrow of
    /// it is of the place as it is now (`place_where_made`). The reference
    /// the local is, `r` of `(*r).next` set again, is the local's own
    /// (`local_reference_written`).
    fn reached_through_written(&self, state: &State<'_, 'tcx>, place: &Place<'tcx>) -> bool {
        let body = state.body;
        let mut ty = mir::PlaceTy::from_ty(body.local_decls[place.local].ty);
        // What's reached so far: the local's value, through its boxes; a
        // part of it; or what a reference refers to.
        let (mut value, mut own) = (true, true);
        for (at, elem) in place.projection.iter().enumerate() {
            let written = match elem {
                PlaceElem::Deref if ty.ty.is_ref() => {
                    !value
                        && ((own && written_again(state, place.local))
                            || reference_written(body, place.local, &place.projection[..at]))
                }
                PlaceElem::Index(index) => written_again(state, index) || index_copied_from_set(state, index),
                _ => false,
            };
            if written {
                return true;
            }
            let boxed = crate::lower::body_queries::through_box(elem, ty.ty).is_some();
            value &= boxed;
            own &= boxed || !matches!(elem, PlaceElem::Deref);
            ty = ty.projection_ty(self.tcx, elem);
        }
        false
    }
}

/// Whether what the statement at `at` assigns, an `AddWithOverflow`'s pair,
/// has its overflow read as a value, an inlined `checked_add`'s, not only
/// asserted, as an overflow check of `#[rustc_inherit_overflow_checks]`
/// arithmetic is, which the release profile never makes (ADR 0899). Its
/// value alone, `.0`, read is no read of it.
fn overflow_read(body: &mir::Body<'_>, at: mir::Location) -> bool {
    use rustc_middle::mir::visit::{PlaceContext, Visitor};
    let Some(statement) = body.basic_blocks[at.block].statements.get(at.statement_index) else {
        return true;
    };
    let mir::StatementKind::Assign(assign) = &statement.kind else {
        return true;
    };
    struct Reads<'b, 'tcx> {
        body: &'b mir::Body<'tcx>,
        pair: mir::Place<'tcx>,
        made: mir::Location,
        read: bool,
    }
    impl<'tcx> Visitor<'tcx> for Reads<'_, 'tcx> {
        fn visit_place(&mut self, place: &mir::Place<'tcx>, _: PlaceContext, location: mir::Location) {
            if location == self.made
                || place.local != self.pair.local
                || !place.projection.starts_with(self.pair.projection)
            {
                return;
            }
            let block = &self.body.basic_blocks[location.block];
            let asserted = location.statement_index == block.statements.len()
                && matches!(&block.terminator().kind,
                    mir::TerminatorKind::Assert { cond: mir::Operand::Copy(p) | mir::Operand::Move(p), .. } if p == place);
            let field = place.projection.get(self.pair.projection.len());
            let value_alone = matches!(field, Some(mir::ProjectionElem::Field(field, _)) if field.as_u32() == 0);
            if !asserted && !value_alone {
                self.read = true;
            }
        }
    }
    let mut reads = Reads {
        body,
        pair: assign.0,
        made: at,
        read: false,
    };
    reads.visit_body(body);
    reads.read
}

/// Whether the statement being lowered makes a call's argument: what it
/// assigns, moved into a call, the one that ends its block (rd205) or a
/// later one's, of an argument made before another that's a call itself,
/// `f(href, nonce.map(..))`'s `href` (rd710).
fn given_to_call(state: &State<'_, '_>) -> bool {
    let data = &state.body.basic_blocks[state.at];
    let Some(mir::Statement {
        kind: StatementKind::Assign(assign),
        ..
    }) = data.statements.get(state.statement)
    else {
        return false;
    };
    state.body.basic_blocks.iter().any(|block| {
        matches!(&block.terminator().kind, mir::TerminatorKind::Call { args, .. }
            if args.iter().any(|arg| matches!(arg.node, Operand::Move(given) if given == assign.0)))
    })
}
