//! What a call is given for each of its arguments, in the form what's
//! called takes it: a `&mut` to a number's place boxed and copied back
//! (ADR 0074), an object's given as it is or as generic code's box of it
//! (ADRs 0099, 0454), a reference as its pointer (ADR 1114). Split from
//! `assign.rs` by concept (#1300).

use super::*;
use crate::lower::temporary_names::{Base, Role};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether a trait method's call of `def_id` gives a `&mut` std's
    /// dictionary can't take as it's given. One of `values` is a place's, a
    /// number's or text's, which std's own lowering takes as its place. Or
    /// one of `given`, their types, is a `&mut`, of a receiver that's no
    /// `dyn` and an impl that's std's: a std dictionary's entry reads a
    /// `&mut` as generic code's box, where a type known here has its
    /// object, `m.borrow_mut()` of a `Vec` read as `m.value` (#1487). Such
    /// a call is its recogniser's, or refused. A `dyn`'s dictionary is its
    /// own, given as the `dyn` holds it, and an impl rust-js compiled, a
    /// library's, takes the crate's forms.
    pub(in crate::lower::mir) fn gives_a_mut(
        &self,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        values: &[Value<'tcx>],
        given: &[Ty<'tcx>],
    ) -> R<bool> {
        let of_dyn = (given.first()).is_some_and(|me| matches!(me.peel_refs().kind(), ty::Dynamic(..)));
        let a_mut = |ty: &Ty<'tcx>| matches!(ty.kind(), ty::Ref(_, _, ty::Mutability::Mut));
        let compiled = (self.resolve_self_instance(def_id, generic_args)?)
            .is_some_and(|resolved| self.krate.foreign.compiled_by_rust_js(resolved.def_id()));
        Ok(values.iter().any(|v| matches!(v, Value::Ref(_))) || (!of_dyn && !compiled && given.iter().any(a_mut)))
    }

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
                exprs.push(self.form_given(Form::Handle, (id, raw(declared), raw(instantiated)), pointer, span)?);
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
                    exprs.push(self.form_given(Form::Handle, (id, declared, instantiated), pointer, span)?);
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
                    exprs.push(self.form_given(Form::Box, crossing, object, span)?);
                    continue;
                }
                // A `&mut` to a `&mut` to an object held as a value, a
                // closure's parameter, given where generic code has a box of
                // its type parameter: its box, `$heldCellOf(cell)`, as one of
                // a place is below (#1303).
                if !matches!(value, Value::Ref(_)) && self.form_differs(Form::HeldBox, crossing) {
                    let held = self.value_expr(value, span)?;
                    exprs.push(self.form_given(Form::HeldBox, crossing, held, span)?);
                    continue;
                }
                // A write's result given where generic code has a `Result` of
                // its type parameters: tagged, `$writeTagged(r)`, its form
                // there; one held inside what's given, refused (ADR 0461,
                // amended, #1408).
                if let Some(held) = self.write_result_inside(crossing) {
                    return Err(self.unsupported(span, &crate::lower::generic_facts::held_write_result(held)));
                }
                if self.write_result_through_bound(crossing) {
                    return Err(self.unsupported(
                        span,
                        &format!(
                            "a write's result, `{}`, given for `{}`, which a bound reads as a `Result` of type parameters, `{{ TAG }}` there (#1416)",
                            crossing.2.peel_refs(),
                            crossing.1.peel_refs()
                        ),
                    ));
                }
                if self.form_differs(Form::Tagged, crossing) {
                    let result = self.value_expr(value, span)?;
                    exprs.push(self.form_given(Form::Tagged, crossing, result, span)?);
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
                    let given = arg_tys.get(i).copied();
                    let mut unboxed = callee.and_then(|(id, args)| self.unboxed_by_function(id, i, args));
                    // A generic function's instance that takes the forms
                    // generic code gives it, and gives back what it takes
                    // back: the function itself, not its value, which takes
                    // its callers' (#975, #1324).
                    let itself = given.and_then(crate::lower::fn_def).filter(|&function| {
                        unboxed
                            .as_ref()
                            .is_some_and(|forms| self.takes_forms_given(function, forms))
                    });
                    let mut f = match (itself, given) {
                        (Some(function), Some(ty)) => {
                            unboxed = None;
                            self.fn_value_in_generic_forms(state, function, (ty, span), out)?
                        }
                        _ => self.value_expr(value, span)?,
                    };
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
                                f = self.spill(Role::F, f, out);
                            }
                            Some(f.clone())
                        }
                        false => None,
                    };
                    exprs.push(match unboxed {
                        Some((unboxed, back)) => {
                            let params: Vec<String> = (0..unboxed.len())
                                .map(|at| self.fresh(Base::Numbered(Role::X, at)))
                                .collect();
                            let given = (params.iter().zip(&unboxed))
                                .map(|(param, &unboxed)| match unboxed {
                                    Some((form, crossing)) => self.form_back(form, crossing, Expr::var(param)),
                                    None => Expr::var(param),
                                })
                                .collect();
                            let called = Expr::call(f, given);
                            let called = match back {
                                Some(Form::Handle) => Expr::call_helper("$ptrTo", vec![called]),
                                // A write's result it gives, `undefined` or its
                                // error thrown, where generic code takes a
                                // `Result` of its type parameters: `{ TAG }`
                                // of it, `$writeResult(() => f())` (#1408).
                                Some(Form::Tagged) => Expr::call_helper(
                                    "$writeResult",
                                    vec![Expr::arrow(
                                        Vec::new(),
                                        vec![StmtKind::Return(Some(called)).at(js::Span::NONE)],
                                    )],
                                ),
                                // Held as a value, `undefined` or its error,
                                // where generic code holds a type parameter
                                // (#1410).
                                Some(Form::Valued) => {
                                    self.fmt_try(vec![StmtKind::Return(Some(called)).at(js::Span::NONE)])
                                }
                                _ => called,
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
            let held_crossing = callee.and_then(|(id, args)| {
                let sig = (self.tcx).instantiate_bound_regions_with_erased(
                    self.tcx.fn_sig(id).instantiate_identity().skip_normalization(),
                );
                let declared = *sig.inputs().get(i)?;
                let crossing = (id, declared, self.instantiated(declared, args));
                self.form_differs(Form::HeldBox, crossing).then_some(crossing)
            });
            // What each `&mut` is to, whose box the held box is.
            let inner = held_crossing.and_then(|(id, declared, instantiated)| {
                Some((id, declared.builtin_deref(true)?, instantiated.builtin_deref(true)?))
            });
            let through = |cx: &Self, boxed: Expr| match held_crossing {
                Some(crossing) => cx.form_given(Form::HeldBox, crossing, boxed, span),
                None => Ok(boxed),
            };
            if let Some(boxed) = self.box_of_place(state, &place) {
                exprs.push(through(self, boxed)?);
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
            // What it returns, or another parameter, can hold the borrow, or a
            // raw pointer it keeps of it (#1781): a box would be copied back
            // before it's used.
            if self.borrow_outlives_call(def_id, i) || self.pointer_may_keep(def_id, i, param) {
                // Of a type of one value, `()`'s: a box of it, which no write
                // changes, whatever keeps it (#1061).
                if one_value && !fixed_place(&place) {
                    exprs.push(crate::lower::mut_refs::boxed(place));
                    continue;
                }
                if !fixed_place(&place) {
                    return Err(self.unsupported(span, "a `&mut` of a number given back, from its MIR"));
                }
                exprs.push(through(self, Expr::handle(place))?);
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
                    let made = std::mem::replace(expr, Expr::taken());
                    *expr = self.spill(Role::Arg, made, out);
                }
            }
            let names = self.tcx.fn_arg_idents(def_id);
            let base = names
                .get(i)
                .copied()
                .flatten()
                .map_or("value".to_string(), |i| i.name.to_string());
            let name = self.fresh(Base::Named(&crate::lower::camel_case(&base)));
            let held = match inner {
                Some(crossing) => self.form_given(Form::Box, crossing, place.clone(), span)?,
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
        // A `read_to_string` of a `String` proved fresh-empty is told so,
        // whatever reader generic code is given (#1297).
        if self.fresh_string_read_into(state) {
            exprs.push(Expr::bool(true));
        }
        Ok(exprs)
    }
}
