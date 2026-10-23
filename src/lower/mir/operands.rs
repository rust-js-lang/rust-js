//! An operand as JS, and a constant: its value tree, or its MIR lowered
//! as code.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(super) fn mir_operand(
        &mut self,
        state: &mut State<'_, 'tcx>,
        operand: &Operand<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        // A `Copy` value a variable holds, of a type changed in place: a copy
        // of it. A temporary's is its own,
        // but not what it refers to, a static's.
        let copied = matches!(operand, Operand::Copy(place)
            if place.is_indirect() || !state.pending.iter().any(|(l, _)| *l == place.local));
        // A copy of a closure that changes what it captured would share it,
        // as a copy of a JS function does (ADR 0246).
        if let Operand::Copy(place) = operand
            && state.locals.reads[place.local] > 1
            && self.copies_own_captures(operand.ty(&state.body.local_decls, self.tcx))
        {
            let span = state.body.local_decls[place.local].source_info.span;
            return Err(self.unsupported(span, "copying a closure that changes what it captured"));
        }
        match self.mir_operand_read(state, operand, out)? {
            Value::Expr(e) if copied => {
                let ty = operand.ty(&state.body.local_decls, self.tcx);
                Ok(Value::Expr(self.copy_if_needed(e, ty)))
            }
            value => Ok(value),
        }
    }

    /// A field of a list made and not yet read, `copy _5.0`, or a reborrow
    /// of what it refers to, `&(*(_5.0))`, a `format_args!`'s argument: its
    /// item, where those before it are taken already, or move freely.
    pub(super) fn take_field(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: &Place<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let (PlaceElem::Field(field, _), [] | [PlaceElem::Deref]) = (
            place.projection.first().copied().unwrap_or(PlaceElem::Deref),
            place.projection.get(1..).unwrap_or(&[]),
        ) else {
            return Ok(None);
        };
        let field = field.as_usize();
        let pending = state.pending.iter().find(|(l, _)| *l == place.local).map(|(_, v)| v);
        let Some(Value::List(items)) = pending else {
            return Ok(None);
        };
        // Read out of order, or one that does something before others that
        // do, which made after it would run first: the list made whole
        // first, its fields read by its name.
        let mut in_order = items[..field].iter().all(|item| movable(state, item))
            && (movable(state, &items[field]) || items[field + 1..].iter().all(|item| movable(state, item)));
        // None read yet: each that does something held first, in order, the
        // rest read where they are: `_arg`, named for what it's given as once
        // tidied, `const key = …`, or `arg`.
        let acting: Vec<usize> = (items.iter().enumerate())
            .filter(|(_, item)| !movable(state, item) && matches!(item, Value::Expr(e) if e.has_effects()))
            .map(|(k, _)| k)
            .collect();
        if !in_order && !acting.is_empty() && !state.taken.contains(&place.local) {
            let at = state
                .pending
                .iter()
                .position(|(l, _)| *l == place.local)
                .expect("pending");
            let rest = state.pending.split_off(at);
            self.flush(state, out)?;
            state.pending = rest;
            let span = state.body.local_decls[place.local].source_info.span;
            for k in acting {
                let Value::List(items) = &mut state.pending[0].1 else {
                    unreachable!("a list")
                };
                let item = std::mem::replace(&mut items[k], Value::Expr(Expr::undefined()));
                let item = self.value_expr(item, span)?;
                let held = self.spill(tidy::HELD_ARG, item, out);
                let Value::List(items) = &mut state.pending[0].1 else {
                    unreachable!("a list")
                };
                items[k] = Value::Expr(held);
            }
            in_order = true;
        }
        if !in_order || !self.admit(state, &[place.local], true, out)? {
            if let Some(at) = state.pending.iter().position(|(l, _)| *l == place.local) {
                let rest = state.pending.split_off(at + 1);
                self.flush(state, out)?;
                state.pending = rest;
            }
            return Ok(None);
        }
        let at = state
            .pending
            .iter()
            .position(|(l, _)| *l == place.local)
            .expect("admitted");
        let Value::List(items) = &mut state.pending[at].1 else {
            unreachable!("a list")
        };
        let item = std::mem::replace(&mut items[field], Value::Expr(Expr::undefined()));
        state.taken.push(place.local);
        if state.taken.iter().filter(|&&l| l == place.local).count() == items.len() {
            state.pending.remove(at);
        }
        Ok(Some(item))
    }

    fn mir_operand_read(
        &mut self,
        state: &mut State<'_, 'tcx>,
        operand: &Operand<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        if let Operand::Copy(place) | Operand::Move(place) = operand
            && let Some(item) = self.take_field(state, place, out)?
        {
            return Ok(item);
        }
        match operand {
            // A local's value, read through references to it, as it is.
            Operand::Copy(place) | Operand::Move(place) if self.through_values(state.body, place, true) => {
                match self.read_local_value(state, place.local, out)? {
                    // Through a `&mut` to a place: what the place holds.
                    Value::Ref(target) if place.is_indirect() => Ok(Value::Expr(target)),
                    value => Ok(value),
                }
            }
            Operand::Copy(place) | Operand::Move(place) => {
                let span = state.body.local_decls[place.local].source_info.span;
                let read = self.mir_place(state, *place, span, out)?;
                Ok(Value::Expr(self.with_default(state, *place, read, span)))
            }
            // A function given by reference, `&hello`, a promoted constant:
            // the function, as a reference is what it refers to (ADR 0023).
            Operand::Constant(c)
                if let Const::Unevaluated(uv, _) = c.const_
                    && let Some(promoted) = uv.promoted
                    && let Some(referred) = referred_constant(&state.promoted[promoted])
                    && fn_def(referred.ty()).is_some() =>
            {
                self.mir_operand_read(state, &Operand::Constant(Box::new(referred)), out)
            }
            // A function as a value, `.map(double)` or `f as fn()`.
            Operand::Constant(c) if let Some((def_id, args)) = fn_def(c.ty()) => {
                match self.fn_item_value(def_id, args, c.ty(), c.span, out)? {
                    Some(f) => Ok(Value::Expr(f)),
                    None => Ok(Value::Expr(self.mir_called_value(state, (def_id, args), c.span)?)),
                }
            }
            // A pointer to a `static mut`, a handle on its `{ value }`'s place
            // (ADRs 0096, 0407); a reference to one, the place.
            Operand::Constant(c)
                if let Some(def_id) = self.const_static(c)
                    && self.tcx.is_mutable_static(def_id)
                    && self.krate.fns.contains_key(&def_id) =>
            {
                Ok(match self.is_handle_pointer(c.ty()) {
                    true => Value::Expr(Expr::call(
                        Expr::helper("$ptr"),
                        vec![self.fn_ref(def_id), Expr::str(js::HELD), Expr::int(0)],
                    )),
                    false => Value::Ref(Expr::held(self.fn_ref(def_id))),
                })
            }
            // A closure that captures nothing, which optimized MIR names as a
            // constant: made here, as one made of its captures is.
            Operand::Constant(c)
                if let ty::Closure(def_id, _) = *c.ty().kind()
                    && matches!(c.const_, Const::Val(mir::ConstValue::ZeroSized, _)) =>
            {
                Ok(Value::Expr(self.mir_closure(state, def_id, Vec::new(), c.span, out)?))
            }
            Operand::Constant(c) => Ok(Value::Expr(self.mir_const(state, c)?)),
            #[allow(unreachable_patterns)]
            _ => Err(self.unsupported(Span::default(), "this operand, from its MIR")),
        }
    }

    pub(super) fn mir_expr(
        &mut self,
        state: &mut State<'_, 'tcx>,
        operand: &Operand<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let value = self.mir_operand(state, operand, out)?;
        self.value_expr(value, Span::default())
    }

    /// A constant's JS, by its value tree.
    /// A constant of the crate's rustc can't say as a value, one of a type
    /// parameter's or a const generic's: its MIR, given `args`, its uses',
    /// lowered as code.
    pub(in crate::lower) fn crate_const_as_code(
        &mut self,
        def_id: rustc_span::def_id::DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> R<Expr> {
        let tcx = self.tcx;
        let instantiate = |body: mir::Body<'tcx>| {
            tcx.instantiate_and_normalize_erasing_regions(args, self.typing_env, ty::EarlyBinder::bind(tcx, body))
        };
        let body = instantiate(tcx.mir_for_ctfe(def_id).clone());
        let promoted = tcx
            .promoted_mir(def_id)
            .iter()
            .map(|p| instantiate(p.clone()))
            .collect();
        self.const_as_code(Mir::new(tcx, body, promoted))
    }

    /// A constant rustc can't say as a value, its MIR lowered as code: what
    /// it returns, or a function of it, called.
    fn const_as_code(&mut self, mir: Mir<'tcx>) -> R<Expr> {
        let (_, stmts) = self.mir_function(&mir, Some(Vec::new()), None)?;
        Ok(match stmts.as_slice() {
            [
                Stmt {
                    kind: StmtKind::Return(Some(value)),
                    ..
                },
            ] => value.clone(),
            _ => Expr::call(Expr::arrow(Vec::new(), stmts), Vec::new()),
        })
    }

    fn mir_const(&mut self, state: &State<'_, 'tcx>, c: &ConstOperand<'tcx>) -> R<Expr> {
        let ty = c.ty();
        if ty.is_unit() || matches!(ty.kind(), ty::FnDef(..)) {
            return Ok(Expr::undefined());
        }
        let tcx = self.tcx;
        // A promoted reference to a named constant or a static, `&LOG`:
        // what it refers to, as the reference is (ADR 0023).
        if let Const::Unevaluated(uv, _) = c.const_
            && let Some(promoted) = uv.promoted
            && let Some(referred) = referred_constant(&state.promoted[promoted])
            && self.const_static(c).is_none()
        {
            // Of the crate's named one, the one there is, uncopied, as it's
            // only lent shared.
            if let Const::Unevaluated(named, _) = referred.const_
                && named.promoted.is_none()
                && matches!(tcx.def_kind(named.def), DefKind::Const { .. })
                && self.krate.fns.contains_key(&named.def)
                && referred.ty().is_freeze(tcx, self.typing_env)
            {
                return Ok(self.fn_ref(named.def));
            }
            return self.mir_const(state, &referred);
        }
        // A named constant (ADR 0031); a `const { .. }` block, its value, or as
        // one (ADR 0127).
        if let Const::Unevaluated(uv, _) = c.const_
            && uv.promoted.is_none()
        {
            match tcx.def_kind(uv.def) {
                DefKind::Const { .. } | DefKind::AssocConst { .. } => {
                    return self.named_const(uv.def, uv.args, ty, c.span);
                }
                DefKind::AnonConst => {
                    return match self
                        .mir_const_value(c.const_, ty, c.span)
                        .and_then(|v| const_js(tcx, v))
                    {
                        Some(value) => Ok(value),
                        None => self.named_const(uv.def, uv.args, ty, c.span),
                    };
                }
                _ => {}
            }
        }
        // A const generic parameter, its value as the function's caller
        // gives it; a constant named in a type, as one named anywhere.
        if let Const::Ty(_, ct) = c.const_ {
            match ct.kind() {
                ty::ConstKind::Param(_) => return self.const_arg(ct, c.span),
                ty::ConstKind::Alias(_, alias) => {
                    let (ty::AliasConstKind::Projection { def_id }
                    | ty::AliasConstKind::Inherent { def_id }
                    | ty::AliasConstKind::Free { def_id }
                    | ty::AliasConstKind::Anon { def_id }) = alias.kind;
                    return self.named_const(def_id, alias.args, ty, c.span);
                }
                _ => {}
            }
        }
        // A reference to a static, which is the static (ADR 0023): a JS
        // global (ADR 0021), or the crate's, its module's `const` (ADR 0096).
        let pointer = crate::lower::pointers::raw_pointee(tcx, ty);
        if (matches!(ty.kind(), ty::Ref(_, _, ty::Mutability::Not))
            || pointer.is_some_and(|(_, mutability)| mutability.is_not()))
            && let Some(def_id) = self.const_static(c)
            && !tcx.is_mutable_static(def_id)
        {
            let referred = match tcx.is_foreign_item(def_id) {
                true => Some(self.js_global(def_id, c.span)),
                false => self.krate.fns.contains_key(&def_id).then(|| self.fn_ref(def_id)),
            };
            if let Some(referred) = referred {
                // A raw pointer to one, a handle on it (ADR 0407): of what isn't
                // an object, a cell of its value, which nothing writes.
                return Ok(match pointer {
                    Some((pointee, _)) if self.is_handle_pointer(ty) => {
                        match self.is_object(pointee) && !self.is_cell_pointee(pointee) {
                            true => self.pointer_to(referred, pointee, Some(0), c.span)?,
                            false => Expr::object(vec![Prop::Field(js::HELD.into(), referred)]),
                        }
                    }
                    _ => referred,
                });
            }
        }
        let value = match (self.mir_const_value(c.const_, ty, c.span)).and_then(|value| const_js(tcx, value)) {
            Some(value) => value,
            // A promoted value rustc's can't say as JS's, `&[("double",
            // double)]` of functions: its MIR, lowered as code.
            None if let Const::Unevaluated(uv, _) = c.const_
                && let Some(promoted) = uv.promoted =>
            {
                return self.const_as_code(Mir::new(tcx, state.promoted[promoted].clone(), IndexVec::new()));
            }
            None => return Err(self.unsupported(c.span, "this constant, from its MIR")),
        };
        // A string is as its literal is written, across lines or in
        // `concat!`'s pieces (ADR 0641); a number is where its literal is,
        // which it's written as (ADR 0627).
        Ok(match &value.kind {
            js::ExprKind::Str(text) => crate::lower::literals::string_literal(tcx, text, c.span).or_at(value.span),
            js::ExprKind::Num(_) => value.or_at(self.js_span(c.span)),
            _ => value,
        })
    }

    /// A JS global a binding names (ADR 0021), read as its name, as
    /// TypeScript's `declare var` is: whether the host has it is
    /// `js::is_defined`'s (ADR 0597). Read where its path names it,
    /// at `named`, through the module that holds it (ADR 0633).
    fn js_global(&self, def_id: rustc_span::def_id::DefId, named: Span) -> Expr {
        self.binding_ref(&bindings::js_name(self.tcx, def_id), named)
    }

    /// The static a constant points to the start of: `&LOG`, a promoted
    /// `&STATIC`, `&raw mut COUNT`.
    pub(super) fn const_static(&self, c: &ConstOperand<'tcx>) -> Option<rustc_span::def_id::DefId> {
        let value = match c.const_ {
            Const::Val(value, _) => value,
            Const::Unevaluated(uv, _) if uv.promoted.is_some() => {
                self.tcx.const_eval_resolve(self.typing_env, uv, c.span).ok()?
            }
            _ => return None,
        };
        let mir::ConstValue::Scalar(mir::interpret::Scalar::Ptr(ptr, _)) = value else {
            return None;
        };
        let (provenance, offset) = ptr.into_raw_parts();
        match self.tcx.global_alloc(provenance.alloc_id()) {
            mir::interpret::GlobalAlloc::Static(def_id) if offset.bytes() == 0 => Some(def_id),
            _ => None,
        }
    }

    fn mir_const_value(&self, c: Const<'tcx>, ty: Ty<'tcx>, span: Span) -> Option<ty::Value<'tcx>> {
        match c {
            Const::Ty(_, ct) => ct.try_to_value(),
            Const::Unevaluated(uv, _) => {
                let valtree = crate::lower::representation::valtree(
                    self.tcx,
                    self.typing_env,
                    uv.def,
                    uv.args,
                    uv.promoted,
                    span,
                )?;
                Some(ty::Value { ty, valtree })
            }
            Const::Val(value, _) => {
                // A struct's, a tuple's or an enum's, as rustc stores it, a
                // `(0, Some(0))` in memory or a `None` of an `Option<Box<T>>`:
                // a tree of its fields, each as it's stored, after its
                // variant's index where it's an enum's.
                if matches!(ty.kind(), ty::Adt(..) | ty::Tuple(_))
                    && let Some(made) = self.tcx.try_destructure_mir_constant_for_user_output(value, ty)
                {
                    let mut parts = Vec::new();
                    if let Some(variant) = made.variant {
                        let index = ty::ValTree::from_scalar_int(self.tcx, variant.as_u32().into());
                        parts.push(ty::Const::new_value(self.tcx, index, self.tcx.types.u32));
                    }
                    for &(field, field_ty) in made.fields.iter() {
                        let field = self.mir_const_value(Const::Val(field, field_ty), field_ty, span)?;
                        parts.push(ty::Const::new_value(self.tcx, field.valtree, field_ty));
                    }
                    return Some(ty::Value {
                        ty,
                        valtree: ty::ValTree::from_branches(self.tcx, parts),
                    });
                }
                // `None` of an `Option` of a reference to a type parameter,
                // whose layout rustc can't say: a null pointer, as its niche
                // is.
                if let ty::Adt(adt, args) = ty.kind()
                    && self.tcx.is_lang_item(adt.did(), LangItem::Option)
                    && args.type_at(0).is_ref()
                    && value.try_to_scalar_int().is_some_and(|scalar| scalar.is_null())
                {
                    let index = ty::ValTree::from_scalar_int(self.tcx, 0u32.into());
                    let index = ty::Const::new_value(self.tcx, index, self.tcx.types.u32);
                    return Some(ty::Value {
                        ty,
                        valtree: ty::ValTree::from_branches(self.tcx, [index]),
                    });
                }
                if let Some(scalar) = value.try_to_scalar_int() {
                    return Some(ty::Value {
                        ty,
                        valtree: ty::ValTree::from_scalar_int(self.tcx, scalar),
                    });
                }
                // A reference to bytes rustc made, a `format_args!` template's.
                if let mir::ConstValue::Scalar(mir::interpret::Scalar::Ptr(ptr, _)) = value
                    && let ty::Ref(_, inner, _) = ty.kind()
                    && let ty::Array(item, len) = inner.kind()
                    && *item == self.tcx.types.u8
                {
                    let (provenance, offset) = ptr.into_raw_parts();
                    let alloc = self.tcx.global_alloc(provenance.alloc_id()).unwrap_memory();
                    let len = len.try_to_target_usize(self.tcx)? as usize;
                    let start = offset.bytes_usize();
                    let bytes = alloc
                        .inner()
                        .inspect_with_uninit_and_ptr_outside_interpreter(start..start + len);
                    return Some(ty::Value {
                        ty,
                        valtree: ty::ValTree::from_raw_bytes(self.tcx, bytes),
                    });
                }
                if !matches!(value, mir::ConstValue::Slice { .. }) {
                    return None;
                }
                let bytes = value.try_get_slice_bytes_for_diagnostics(self.tcx)?;
                Some(ty::Value {
                    ty,
                    valtree: ty::ValTree::from_raw_bytes(self.tcx, bytes),
                })
            }
        }
    }
}
