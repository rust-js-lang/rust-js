//! A place as JS, for a read or a write: its projections, a slice's rest,
//! a field, and a range pattern's.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A place, for a read or a write.
    pub(super) fn mir_place(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut e = self.place_expr(state, place, span, out)?;
        // An `Option` whose `None` is only one of `undefined` and `null`,
        // its own or through a reference to it: its read says which (ADR
        // 0298, amended).
        if matches!(place.projection.as_slice(), [] | [PlaceElem::Deref])
            && let Some(&none) = state.facts.options().nones.get(&place.local)
        {
            e.facts = e.facts.with(none);
        }

        // A field's path, `element.editable`: whether it's a `bool`, which
        // tidy writes an `||` of as one.
        if let Some(path) = tidy::field_path(&e) {
            let bool = place.ty(state.body, self.tcx).ty.is_bool();
            *self.locals.mir.paths.entry(path).or_insert(bool) &= bool;
        }
        Ok(e)
    }

    fn place_expr(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        // A field of an update's derived `Default` base: its default.
        if let Some(defaults) = state.defaults.get(&place.local)
            && let [PlaceElem::Field(field, _)] = place.projection.as_slice()
        {
            return Ok(defaults[field.as_usize()].clone());
        }
        // A strip's `Option` read as a test and a cut: its `Some`'s, the cut
        // (ADR 0642).
        if let Some((_, cut)) = state.cuts.get(&place.local)
            && let [PlaceElem::Downcast(_, variant), PlaceElem::Field(..)] = place.projection.as_slice()
            && variant.as_u32() == 1
        {
            return Ok(cut.clone());
        }
        if let Some((tried, ty)) = state.branches.get(&place.local).cloned() {
            return match place.projection.as_slice() {
                [PlaceElem::Downcast(_, variant), PlaceElem::Field(..)] if variant.as_u32() == 0 => {
                    Ok(match self.is_write_result(ty) {
                        true => Expr::undefined(),
                        false => self.tried_value(tried, ty),
                    })
                }
                // The residual, `None` or the `Err` itself.
                [PlaceElem::Downcast(..), PlaceElem::Field(..)] => Ok(match self.option_of(ty) {
                    Some(_) => Expr::undefined(),
                    None => tried,
                }),
                _ => Err(self.unsupported(span, "this part of what `?` made, from its MIR")),
            };
        }
        let mut placed = false;
        let mut value = if place.projection.is_empty() {
            match (
                state.in_place.get(&place.local),
                state.locals.names[place.local].as_str(),
            ) {
                (Some(value), _) => value.clone(),
                (None, "") => Expr::undefined(),
                (None, name) => {
                    state.named.insert(place.local);
                    match state.facts.pointed().locals.contains(&place.local) {
                        true => Expr::held(Expr::var(name)),
                        false => state.facts.variable(place.local, name),
                    }
                }
            }
        } else if matches!(
            state.body.local_decls[place.local].ty.peel_refs().kind(),
            ty::Closure(..) | ty::Coroutine(..) | ty::CoroutineClosure(..)
        ) && !state.facts.captures().is_empty()
        {
            // A closure's environment, read only through its fields.
            Expr::undefined()
        } else {
            match self.read_local_value(state, place.local, out)? {
                // Through a `&mut` to a place: the place.
                Value::Ref(place) => {
                    placed = true;
                    place
                }
                value => self.value_expr(value, span)?,
            }
        };
        let mut ty = mir::PlaceTy::from_ty(state.body.local_decls[place.local].ty);
        let last = place.projection.len();
        for (at, elem) in place.projection.iter().enumerate() {
            // Its bounds checked here: `$index(xs, i)` where it's read,
            // `xs[$at(xs, i)]` where it's the place written.
            if let PlaceElem::Index(local) = elem
                && let Some(write) = state.deferred.remove(&local)
            {
                let items = match value.reads_same() {
                    true => value,
                    false => self.spill("items", value, out),
                };
                let index = self.read_local(state, local, out)?;
                // A constant index of an array as long as that has no check
                // to defer: `assert_check` knows it in bounds.
                value = match write && at + 1 == last {
                    true => Expr::index(items.clone(), Expr::call(Expr::helper("$at"), vec![items, index])),
                    false => Expr::call(Expr::helper("$index"), vec![items, index]),
                };
                placed = false;
                ty = ty.projection_ty(self.tcx, elem);
                continue;
            }
            // Through a handle, or a box, of a `&mut` to a number: its
            // `value`, but where the place is already named.
            if elem == PlaceElem::Deref
                && let ty::Ref(_, pointee, ty::Mutability::Mut) = ty.ty.kind()
                && self.is_cell_pointee(*pointee)
            {
                if !std::mem::take(&mut placed) {
                    value = Expr::held(value);
                }
                ty = ty.projection_ty(self.tcx, elem);
                continue;
            }
            // What the `Some` of an `Option` of an `Option` held as its
            // payload holds: the payload itself, its `None` `null` (ADR 1017).
            if at == 1
                && matches!(place.projection[0], PlaceElem::Downcast(_, some) if some.as_u32() == 1)
                && state.facts.options().nested.contains(&place.local)
            {
                placed = false;
                ty = ty.projection_ty(self.tcx, elem);
                continue;
            }
            placed = false;
            value = match (ty.ty.peel_refs().kind(), elem) {
                // Of a closure's environment, `(*_1).0`: what it captured.
                (ty::Closure(..) | ty::Coroutine(..) | ty::CoroutineClosure(..), PlaceElem::Field(field, _))
                    if !state.facts.captures().is_empty() =>
                {
                    let (capture, is_place) = state.facts.captures()[field.as_usize()].clone();
                    placed = is_place;
                    capture
                }
                _ => self.project_mir(state, value, ty, elem, span, out)?,
            };
            ty = ty.projection_ty(self.tcx, elem);
        }
        Ok(value)
    }

    fn project_mir(
        &mut self,
        state: &mut State<'_, 'tcx>,
        base: Expr,
        ty: mir::PlaceTy<'tcx>,
        elem: PlaceElem<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        Ok(match elem {
            // A reference is what it refers to (ADR 0023), and so are a box
            // and a pointer, a `static mut`'s.
            // A raw pointer's place, a handle's `value` (ADR 0407).
            PlaceElem::Deref if self.is_handle_pointer(ty.ty) => self.pointee_place(base),
            PlaceElem::Deref if ty.ty.is_ref() || ty.ty.is_box() || ty.ty.is_raw_ptr() => base,
            // JS's props have no `rest`: what's left of them is taken apart
            // from the rest, `{ href, ...rest }` (ADR 0195).
            PlaceElem::Field(_, field_ty) if bindings::is_rest(self.tcx, field_ty) => {
                // Bound by a pattern other than a `let`'s, a `match`'s.
                return Err(self.unsupported(
                    span,
                    match state.binding_var {
                        true => {
                            "a `Rest` of props taken apart here: take them apart where they're given, `fn f(Props { a, rest }: Props)`"
                        }
                        false => "reading a `Rest` of props: take it apart from them, `LinkProps { href, rest }: LinkProps`",
                    },
                ));
            }
            PlaceElem::Field(field, _) => self.mir_field(base, ty, field, span)?,
            PlaceElem::Downcast(..) | PlaceElem::OpaqueCast(_) => base,
            PlaceElem::Index(local) => {
                let index = self.read_local(state, local, out)?;
                Expr::index(base, index)
            }
            PlaceElem::ConstantIndex {
                offset,
                from_end: false,
                ..
            } => Expr::index(base, Expr::int(offset.into())),
            // Counted from the end, a slice pattern's suffix:
            // `xs[xs.length - 1]`, or of an array, its index.
            PlaceElem::ConstantIndex {
                offset, from_end: true, ..
            } if base.reads_same() => {
                let index = match self.array_length(ty.ty) {
                    Some(n) => Expr::int(n - i128::from(offset)),
                    None => Expr::bin(Op::Sub, Expr::member(base.clone(), "length"), Expr::int(offset.into())),
                };
                Expr::index(base, index)
            }
            // A slice pattern's rest, `xs.slice(1, xs.length - 1)`: a copy, as
            // `&v[a..b]` is (ADR 0063); a `&mut` to it is a view (`rest_view`).
            PlaceElem::Subslice { from, to, from_end } if base.reads_same() => {
                let range = self.subslice_range(&base, ty.ty, (from, to, from_end));
                Expr::call(Expr::member(base, "slice"), range)
            }
            // Of an array made here, `let [_, x @ .., _] = [a, b, c, d]`: it held
            // first, then read so.
            PlaceElem::ConstantIndex { from_end: true, .. } | PlaceElem::Subslice { .. } => {
                self.flush(state, out)?;
                let held = self.spill("items", base, out);
                return self.project_mir(state, held, ty, elem, span, out);
            }
            _ => return Err(self.unsupported(span, "this place, from its MIR")),
        })
    }

    /// Where a slice pattern's rest, `from..` and `to` short of the end,
    /// starts and ends in `base`, its slice of `ty`: `[1, xs.length - 1]`, or
    /// `[1]` to its end. `base` is read twice, so it reads the same.
    fn subslice_range(&self, base: &Expr, ty: Ty<'tcx>, (from, to, from_end): (u64, u64, bool)) -> Vec<Expr> {
        let mut range = vec![Expr::int(from.into())];
        match (from_end, self.array_length(ty)) {
            (true, _) if to == 0 => {}
            (true, Some(n)) => range.push(Expr::int(n - i128::from(to))),
            (true, None) => range.push(Expr::bin(
                Op::Sub,
                Expr::member(base.clone(), "length"),
                Expr::int(to.into()),
            )),
            (false, _) => range.push(Expr::int(to.into())),
        }
        range
    }

    /// A `&mut` to a slice pattern's rest, `ref mut rest @ ..`: a view of
    /// those items, `$view(xs, 1)`, as `&mut xs[1..]` is (ADR 0335). Of a
    /// rest of a rest, which no pattern makes, `None`.
    pub(super) fn rest_view(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Some((&PlaceElem::Subslice { from, to, from_end }, prefix)) = place.projection.split_last() else {
            return Ok(None);
        };
        if prefix.iter().any(|elem| matches!(elem, PlaceElem::Subslice { .. })) {
            return Ok(None);
        }
        let of = Place {
            local: place.local,
            projection: self.tcx.mk_place_elems(prefix),
        };
        let ty = of.ty(&state.body.local_decls, self.tcx).ty;
        let items = self.mir_place(state, of, span, out)?;
        let items = match items.reads_same() {
            true => items,
            false => self.spill("items", items, out),
        };
        let mut list = vec![items.clone()];
        list.extend(self.subslice_range(&items, ty, (from, to, from_end)));
        Ok(Some(self.view_call("$view", list)))
    }

    /// An array type's length, rustc's.
    fn array_length(&self, ty: Ty<'tcx>) -> Option<i128> {
        match ty.kind() {
            ty::Array(_, len) => len.try_to_target_usize(self.tcx).map(i128::from),
            _ => None,
        }
    }

    /// A field: of a struct or a tuple by its shape, of a variant as its
    /// enum holds it, and of what's the value it holds, itself.
    fn mir_field(&mut self, base: Expr, ty: mir::PlaceTy<'tcx>, field: FieldIdx, span: Span) -> R<Expr> {
        let i = field.as_usize();
        if let ty::Closure(..) = ty.ty.kind() {
            return Err(self.unsupported(span, "a closure's field read outside it, from its MIR"));
        }
        if self.transparent(ty.ty).is_some() {
            return Ok(base);
        }
        match (ty.ty.kind(), ty.variant_index) {
            (ty::Adt(adt, args), Some(variant)) if adt.is_enum() => {
                if self.tcx.is_lang_item(adt.did(), LangItem::Option) {
                    let inner = self.option_of(ty.ty).expect("an `Option`");
                    return Ok(match self.boxed_payload(inner) {
                        true => self.some_value(base),
                        false => base,
                    });
                }
                if bindings::is_untagged(self.tcx, adt.did()) {
                    return Ok(base);
                }
                // A write's result's `Err`'s, its error itself; its `Ok`'s, `()`
                // (ADRs 0187, 0462).
                if self.is_write_result(ty.ty) {
                    return Ok(match variant.as_u32() {
                        0 => Expr::undefined(),
                        _ => base,
                    });
                }
                // A `Cell` it holds, as a struct's (ADR 0288): a handle on it.
                let variant = adt.variant(variant);
                let read = Expr::member(base, variant_field(self.tcx, variant, i));
                Ok(self.held(
                    read,
                    Some(&variant.fields[field]),
                    self.field_ty(&variant.fields[field], args),
                ))
            }
            // A union's, what it holds there (ADR 0406).
            (ty::Adt(adt, _), _) if adt.is_union() => self.union_read(base, ty.ty, field, span),
            // A box is the value it holds, which has no `Unique` to read: an
            // optimized body's pointer to it is read as the box where it's
            // made just before its one read (`boxes_read_as_values`); another
            // is refused (#378).
            (ty::Adt(adt, _), _) if adt.is_box() => {
                Err(self.unsupported(span, "a box's pointer, from its optimized MIR"))
            }
            (ty::Tuple(_) | ty::Adt(..), _) => Ok(self.project(base, ty.ty, i)),
            _ => Err(self.unsupported(span, "this field, from its MIR")),
        }
    }

    /// `read`, `place`'s value, of a props field with a default, what JS
    /// takes apart with it (ADR 0212): the default where it's `undefined`,
    /// `o.k === undefined ? d : o.k`, which a pattern of it writes `{ k = d }`.
    pub(super) fn with_default(&mut self, state: &State<'_, 'tcx>, place: Place<'tcx>, read: Expr, span: Span) -> Expr {
        let Some((base, PlaceElem::Field(field, _))) = place.as_ref().last_projection() else {
            return read;
        };
        let base_ty = Place::ty_from(base.local, base.projection, &state.body.local_decls, self.tcx);
        if base_ty.variant_index.is_some() || !matches!(base_ty.ty.kind(), ty::Adt(adt, _) if adt.is_struct()) {
            return read;
        }
        match self.prop_default(base_ty.ty, field.as_usize(), span) {
            Some(default) => Expr::cond(Expr::bin(js::Op::Eq, read.clone(), Expr::undefined()), default, read),
            None => read,
        }
    }

    /// Where each range pattern of the body is, in a `match`'s arms, a
    /// `let`'s or an `if let`'s.
    pub(super) fn range_patterns(&self) -> Vec<Span> {
        let mut ranges = Vec::new();
        let mut find = |pat: &thir::Pat<'tcx>| {
            pat.walk_always(|p| {
                if let thir::PatKind::Range(_) = p.kind {
                    ranges.push(p.span);
                }
            })
        };
        for arm in self.frame.thir.arms.iter() {
            find(&arm.pattern);
        }
        for expr in self.frame.thir.exprs.iter() {
            if let thir::ExprKind::Let { pat, .. } = &expr.kind {
                find(pat);
            }
        }
        for stmt in self.frame.thir.stmts.iter() {
            if let thir::StmtKind::Let { pattern, .. } = &stmt.kind {
                find(pattern);
            }
        }
        ranges
    }

    /// Whether `place` is its local's value, through what's the value it
    /// holds: a box's, or a `MaybeUninit`'s; and, of a read, a reference's.
    pub(super) fn whole(&self, body: &mir::Body<'tcx>, place: &Place<'tcx>) -> bool {
        self.through_values(body, place, false)
    }

    pub(super) fn through_values(&self, body: &mir::Body<'tcx>, place: &Place<'tcx>, read: bool) -> bool {
        let mut ty = mir::PlaceTy::from_ty(body.local_decls[place.local].ty);
        for elem in place.projection {
            let through = match elem {
                // Not through a `&mut` to a cell, a box's or a handle's `value`.
                PlaceElem::Deref => {
                    ty.ty.is_box()
                        || (read
                            && ty.ty.is_ref()
                            && !matches!(ty.ty.kind(), ty::Ref(_, inner, ty::Mutability::Mut) if self.is_cell_pointee(*inner)))
                }
                PlaceElem::Field(..) => self.transparent(ty.ty).is_some(),
                _ => false,
            };
            if !through {
                return false;
            }
            ty = ty.projection_ty(self.tcx, elem);
        }
        true
    }

    // ── Values ──────────────────────────────────────────────────────────
}
