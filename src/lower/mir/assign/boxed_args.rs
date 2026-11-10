//! What a call is given for each of its arguments, in the form what's
//! called takes it: a `&mut` to a number's place boxed and copied back
//! (ADR 0074), an object's given as it is or as generic code's box of it
//! (ADRs 0099, 0454), a reference as its pointer (ADR 1114). Split from
//! `assign.rs` by concept (#1300).

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(in crate::lower::mir) fn boxed_args(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (values, arg_tys): (Vec<Value<'tcx>>, &[Ty<'tcx>]),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Vec<Expr>> {
        let callee = self.call_callee(def_id, generic_args)?;
        let fn_id = callee.map(|(id, _)| id);
        let mut exprs: Vec<Expr> = Vec::new();
        for (i, value) in values.into_iter().enumerate() {
            // A reference given as the pointer to its place, made so where
            // it's made (`Pointed::lent_as_pointers`), and in generic
            // code's form, a slice's `$ptrTo(array)` (ADR 1114).
            if let Some((id, args)) = callee
                && self.given_its_pointer(id, i)
            {
                let pointer = self.value_expr(value, span)?;
                let declared = (self
                    .tcx
                    .fn_sig(id)
                    .instantiate_identity()
                    .skip_normalization()
                    .skip_binder())
                .inputs()[i];
                // Made as the caller's raw pointer to the place, so given as
                // one is: an object's a handle already, a slice's the array.
                let raw = |ty: Ty<'tcx>| Ty::new_mut_ptr(self.tcx, ty.builtin_deref(true).expect("a reference"));
                let instantiated = self.instantiated(declared, args);
                exprs.push(self.form_given(Form::Handle, (id, raw(declared), raw(instantiated)), pointer));
                continue;
            }
            // Not a C-variadic function's arguments past its parameters.
            if let Some((id, args)) = callee
                && let Some(&declared) = (self
                    .tcx
                    .fn_sig(id)
                    .instantiate_identity()
                    .skip_normalization()
                    .skip_binder())
                .inputs()
                .get(i)
            {
                // A raw pointer given where generic code's goes: in its form,
                // a slice's `$ptrTo(array)` (ADR 1114).
                if crate::lower::pointers::raw_pointee(self.tcx, declared).is_some() {
                    let pointer = self.value_expr(value, span)?;
                    let instantiated = self.instantiated(declared, args);
                    exprs.push(self.form_given(Form::Handle, (id, declared, instantiated), pointer));
                    continue;
                }
                // A shared reference to what JS holds as itself, a number,
                // given to generic code that may let it out as a raw pointer:
                // its pointer would be of a copy (ADR 1114). Not generic
                // code's own `&T`, of a type only its callers know.
                if self.lets_pointers_out(id, i)
                    && let Some(referent) = self.copied_referent(declared, self.instantiated(declared, args))
                {
                    return Err(self.unsupported(
                        span,
                        &format!("a `&{referent}` given to generic code that may let it out as a raw pointer, which would be of a copy (ADR 1114)"),
                    ));
                }
                // A `&mut` to an object given where generic code has its box,
                // a `D` given `&mut v` and through shared references too: in
                // its form, an in-place cell of the object, whose write
                // changes it in place, as a value's boundary gives it (ADRs
                // 0454, 1115).
                let crossing = (id, declared, self.instantiated(declared, args));
                if self.form_differs(Form::Box, crossing) {
                    let object = self.value_expr(value, span)?;
                    exprs.push(self.form_given(Form::Box, crossing, object));
                    continue;
                }
            }
            let place = match value {
                // A `&mut` to a function or an iterator given for a type
                // parameter read as one that's called or stepped, `P: Is<F>`:
                // the function or the iterator itself (#992).
                Value::Ref(place)
                    if callee.is_some_and(|(id, args)| {
                        let sig = self.tcx.instantiate_bound_regions_with_erased(
                            self.tcx.fn_sig(id).instantiate_identity().skip_normalization(),
                        );
                        sig.inputs()
                            .get(i)
                            .is_some_and(|&input| self.given_as_the_value(id, input, self.instantiated(input, args)))
                    }) =>
                {
                    exprs.push(place);
                    continue;
                }
                Value::Ref(place) => place,
                // A function given for an `F: FnOnce(&mut T)` of a `T` that's
                // an object here: one taking generic code's box of it, which
                // it gives the object, `(x) => f(x.value)` (ADR 0454).
                value => {
                    let mut f = self.value_expr(value, span)?;
                    let given = arg_tys.get(i).copied();
                    let unboxed = callee.and_then(|(id, args)| self.unboxed_by_function(id, i, args, given));
                    // A boxed `dyn Fn*` that drops, held where it's read
                    // twice, by the arrow and by its drop (#1220).
                    let boxed_drop = match given.is_some_and(|ty| {
                        unboxed.is_some()
                            && ty
                                .boxed_ty()
                                .is_some_and(|inner| matches!(inner.kind(), ty::Dynamic(..)))
                            && self.drops(ty) != crate::lower::drops::Drops::Nothing
                    }) {
                        true => {
                            if !f.reads_same() {
                                f = self.spill("f", f, out);
                            }
                            Some(f.clone())
                        }
                        false => None,
                    };
                    exprs.push(match unboxed {
                        Some((unboxed, wrapped)) => {
                            let params: Vec<String> =
                                (0..unboxed.len()).map(|at| self.fresh(&format!("x{at}"))).collect();
                            let given = (params.iter().zip(&unboxed))
                                .map(|(param, &unboxed)| match unboxed {
                                    true => Expr::held(Expr::var(param)),
                                    false => Expr::var(param),
                                })
                                .collect();
                            let called = Expr::call(f, given);
                            let called = match wrapped {
                                true => Expr::call_helper("$ptrTo", vec![called]),
                                false => called,
                            };
                            let body = vec![StmtKind::Return(Some(called)).at(js::Span::NONE)];
                            let arrow = Expr::arrow(params.into_iter().map(Into::into).collect(), body);
                            // Of a boxed function that drops, what disposes
                            // it disposes the box, as what's given it, std's
                            // `Scan`, drops its `F` by `$dispose` (#1220).
                            match boxed_drop {
                                Some(f) => Expr::call_helper(
                                    "$dropping",
                                    vec![
                                        arrow,
                                        Expr::arrow(
                                            Vec::new(),
                                            vec![
                                                StmtKind::Expr(Expr::call_helper("$dispose", vec![f]))
                                                    .at(js::Span::NONE),
                                            ],
                                        ),
                                    ],
                                ),
                                None => arrow,
                            }
                        }
                        None => f,
                    });
                    continue;
                }
            };
            // Where what's called, an impl's method resolved, takes an object
            // by its own `&mut`, a copied default's `&mut self` given to its
            // impl's method of an object `Self` (ADR 0099): the object, which
            // it changes in place, a box's `value`. A copied default's
            // `Self` is its impl's, lowered in its terms (#803).
            let holds = fn_id.is_some_and(|fn_id| {
                let sig = (self.tcx).instantiate_bound_regions_with_erased(
                    self.tcx.fn_sig(fn_id).instantiate_identity().skip_normalization(),
                );
                let param = sig.inputs().get(i).copied();
                !self.param_is_box(fn_id, i)
                    && matches!(param.map(|p| *p.kind()), Some(ty::Ref(_, inner, ty::Mutability::Mut)) if self.is_object(inner))
            });
            if holds {
                exprs.push(place);
                continue;
            }
            // A `&mut` to a place that holds a `&mut` to an object, given
            // where generic code has a box of its type parameter, `&mut W` of
            // `&mut by`, `let by = &mut v` (#1300): the box holds what's in
            // the place in generic code's form, the object's in-place cell,
            // `$cellOf(by)`, as a `W` given `&mut v` by value is, and what's
            // copied back is the `&mut` that holds, the cell's `value`. One
            // that's a box already, a cell's, is read through,
            // `$heldCellOf(cell)`.
            let inner = callee.and_then(|(id, args)| {
                let sig = (self.tcx).instantiate_bound_regions_with_erased(
                    self.tcx.fn_sig(id).instantiate_identity().skip_normalization(),
                );
                let declared = *sig.inputs().get(i)?;
                let instantiated = self.instantiated(declared, args);
                let crossing = (id, declared.builtin_deref(true)?, instantiated.builtin_deref(true)?);
                self.form_differs(Form::Box, crossing).then_some(crossing)
            });
            let through = |boxed: Expr| match inner {
                Some(_) => Expr::call_helper("$heldCellOf", vec![boxed]),
                None => boxed,
            };
            if let Some(boxed) = self.box_of_place(state, &place) {
                exprs.push(through(boxed));
                continue;
            }
            // Its lifetimes, the signature's own, erased: a type with them
            // still bound, `&mut &'a dyn Fn()`'s `dyn`, has no layout to ask
            // (#460).
            let param = (self.tcx)
                .instantiate_bound_regions_with_erased(
                    self.tcx
                        .fn_sig(def_id)
                        .instantiate(self.tcx, generic_args)
                        .skip_normalization(),
                )
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
                // Of a type of one value, `()`'s: a box of it, which no write
                // changes, whatever keeps it (#1061).
                if one_value && !fixed_place(&place) {
                    exprs.push(crate::lower::mut_refs::boxed(place));
                    continue;
                }
                if !fixed_place(&place) {
                    return Err(self.unsupported(span, "a `&mut` of a number given back, from its MIR"));
                }
                exprs.push(through(Expr::handle(place)));
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
            let held = match inner {
                Some(crossing) => self.form_given(Form::Box, crossing, place.clone()),
                None => place.clone(),
            };
            let boxed = crate::lower::mut_refs::boxed(held);
            out.push(StmtKind::Const(name.clone(), boxed).at(self.js_span(span)));
            if fixed_place(&place) {
                let back = Expr::held(Expr::var(&name));
                let back = match inner {
                    Some(crossing) => self.form_back(Form::Box, crossing, back),
                    None => back,
                };
                state.copy_backs.push((place, back));
            }
            exprs.push(Expr::var(&name));
        }
        Ok(exprs)
    }
}
