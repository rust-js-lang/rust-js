//! A call from MIR of what isn't std's (ADR 0364): a closure's, a binding's,
//! the crate's own function's, and an operator's that assigns through a
//! `&mut`. `mir_call_values` tells which a call is, and hands it here.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A closure called, `Fn::call(&f, (a, b))`: in JS, `f(a, b)`, and an
    /// `async` one's, `AsyncFn::async_call`, which gives its promise.
    pub(super) fn mir_closure_call(
        &mut self,
        values: Vec<Value<'tcx>>,
        arg_tys: &[Ty<'tcx>],
        span: Span,
    ) -> R<Value<'tcx>> {
        let mut values = values.into_iter();
        let callee = match values.next().expect("the closure") {
            Value::Ref(closure) => closure,
            value => self.value_expr(value, span)?,
        };
        let list = match values.next() {
            Some(Value::List(items)) => items,
            Some(Value::Expr(e)) if matches!(e.kind, js::ExprKind::Undefined) => Vec::new(),
            // The arguments' tuple, made before: each of its items.
            Some(Value::Expr(tuple) | Value::Place(tuple))
                if tuple.reads_same()
                    && let ty::Tuple(tys) = arg_tys[1].kind() =>
            {
                (0..tys.len())
                    .map(|i| Value::Expr(Expr::index(tuple.clone(), Expr::int(i as i128))))
                    .collect()
            }
            _ => return Err(self.unsupported(span, "this closure call, from its MIR")),
        };
        let mut exprs = list
            .into_iter()
            .map(|v| self.value_expr(v, span))
            .collect::<R<Vec<_>>>()?;
        if let ty::Tuple(tys) = arg_tys[1].kind() {
            crate::lower::calls::units_left_out(&mut exprs, tys);
        }
        // Not a closure of the crate's: a `dyn Fn`, a generic one or a
        // pointer, which may be JS's (ADR 0330).
        if !matches!(
            arg_tys[0].peel_refs().kind(),
            ty::Closure(..) | ty::CoroutineClosure(..)
        ) {
            crate::lower::calls::given_to_js(&mut exprs);
        }
        Ok(Value::Expr(Expr::call(callee, exprs)))
    }

    /// `a += b` of a number or a string, through its `&mut`: the place
    /// assigned, as the operator's is; `None` of another call.
    pub(super) fn mir_assign_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        def_id: rustc_span::def_id::DefId,
        (values, arg_tys): (&[Value<'tcx>], &[Ty<'tcx>]),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let tcx = self.tcx;
        // Of a number or a string: another's is its impl's, or its dictionary's
        // (ADR 0396).
        if let Some(trait_id) = tcx.trait_of_assoc(def_id)
            && let Some(op) = crate::lower::recognition::assign_operator(tcx, trait_id)
            && let [place, rhs] = values
            && let ty = arg_tys[0].peel_refs()
            && (self.is_lang_adt(ty, LangItem::String) || crate::lower::representation::Num::of(ty).is_some())
            && (matches!(place, Value::Ref(_)) || self.is_cell(arg_tys[0]))
        {
            // Through a handle a call gave, `or_insert(10)`'s: its `value`,
            // the handle held where it's made, before what's added is.
            let place = match place.clone() {
                Value::Ref(place) => place,
                cell => {
                    let cell = self.value_expr(cell, span)?;
                    let cell = match cell.reads_same() {
                        true => cell,
                        false => {
                            self.flush(state, out)?;
                            self.spill("cell", cell, out)
                        }
                    };
                    Expr::held(cell)
                }
            };
            let place = &place;
            let rhs = self.value_expr(rhs.clone(), span)?;
            let rhs = crate::lower::std_types::number::shift_amount_of(op, rhs, ty, arg_tys[1]);
            let value = if self.is_lang_adt(ty, LangItem::String) && op == BinOp::Add {
                Expr::bin(Op::Add, place.clone(), rhs)
            } else if crate::lower::recognition::has_number_arithmetic(ty) {
                self.binary(op, place.clone(), rhs, None, ty, span)?
            } else if let Some((def_id, generic_args, known)) =
                self.recognition().assigned_by(trait_id, ty, arg_tys[1].peel_refs())
            {
                // A `Duration`'s or a clock's: its own `+` (ADRs 0188, 0447).
                let tys = [ty, arg_tys[1]];
                let call = crate::lower::calls::Call {
                    def_id,
                    generic_args,
                    tys: &tys,
                    span,
                };
                self.std_values(known, call, vec![place.clone(), rhs], false, ty, out)?
            } else {
                return Err(self.unsupported(
                    span,
                    &format!("`{}` of a `{ty}`, from its MIR", tcx.def_path_str(def_id)),
                ));
            };
            self.flush(state, out)?;
            out.push(StmtKind::Assign(place.clone(), value).at(self.js_span(span)));
            return Ok(Some(Value::Expr(Expr::undefined())));
        }
        Ok(None)
    }

    /// A binding called: the JS function, method, property or operator it
    /// names (ADRs 0019, 0020), given its arguments.
    pub(super) fn mir_binding_call(
        &mut self,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        values: Vec<Value<'tcx>>,
        arg_tys: &[Ty<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let tcx = self.tcx;
        if rustc_hir::find_attr!(tcx, def_id, RustcEiiForeignItem) {
            return Err(self.unsupported(span, "externally implementable items, `#[eii]`,"));
        }
        match bindings::js_form(tcx, def_id) {
            bindings::JsForm::Jsx(tag) => {
                let given = self.jsx_args(def_id, values, arg_tys, span, |cx, i, ty| match (tag.as_str(), i) {
                    // A JS module's component is its import, as a tag must be.
                    ("*", 0) => match *ty.peel_refs().kind() {
                        ty::FnDef(id, _) => cx.binding_component_of(id),
                        _ => None,
                    },
                    _ => None,
                })?;
                return Ok(Value::Expr(self.jsx_values(&tag, given, span, out)?));
            }
            bindings::JsForm::Prop(name) => {
                let mut given = self
                    .jsx_args(def_id, values, arg_tys, span, |_, _, _| None)?
                    .into_iter();
                let element = given.next().expect("the element");
                let name = match name {
                    Some(name) => name,
                    None => match given.next().map(|name| name.value.kind) {
                        Some(js::ExprKind::Str(name)) => name,
                        _ => return Err(self.unsupported(span, "an attribute's name other than a string literal")),
                    },
                };
                let value = given.next().expect("the value");
                let mut element = element.value;
                // `style.color("red")` on an object being built: one more field.
                if let js::ExprKind::Object(fields) = &mut element.kind {
                    fields.push(match name.as_str() {
                        "..." => Prop::Spread(value.value),
                        _ => Prop::Field(name, value.value),
                    });
                    return Ok(Value::Expr(element));
                }
                if !matches!(element.kind, js::ExprKind::Jsx(_)) {
                    return Err(self.one_expression(span));
                }
                self.jsx_spread_checked(&name, value.ty, value.span)?;
                let made = self.jsx_prop_values(name, element, value, (Vec::new(), true), out)?;
                return Ok(Value::Expr(made));
            }
            bindings::JsForm::Object(keys) => {
                if keys.len() != values.len() {
                    return Err(self.unsupported(span, "an object binding whose fields don't match its arguments"));
                }
                let mut fields = Vec::new();
                for (key, value) in keys.into_iter().zip(values) {
                    fields.push(Prop::Field(key, self.value_expr(value, span)?));
                }
                return Ok(Value::Expr(Expr::object(fields)));
            }
            bindings::JsForm::Import { .. } => {
                return Err(self.unsupported(span, "a dynamic import, from its MIR"));
            }
            _ => {}
        }
        let nullable = bindings::nullable_params(tcx, def_id);
        let idents = tcx.fn_arg_idents(def_id);
        let mut exprs = Vec::new();
        for (i, (value, &ty)) in values.into_iter().zip(arg_tys).enumerate() {
            let value = self.value_expr(value, span)?;
            exprs.push(match () {
                // `()` given to JS is what a tuple is, an array (ADR 0020).
                () if ty.is_unit() => Expr::array(Vec::new()),
                // A parameter it names `#[rust_js::nullable(..)]`: its
                // `None` is `null` (ADR 0275).
                () if idents
                    .get(i)
                    .copied()
                    .flatten()
                    .is_some_and(|ident| nullable.contains(&ident.name)) =>
                {
                    match value.kind {
                        js::ExprKind::Undefined => Expr::null(),
                        _ if value.is_constant() || value.is_made_here() => value,
                        _ => Expr::bin(Op::Coalesce, value, Expr::null()),
                    }
                }
                // A `&dyn Any` given to JS is its value (ADR 0331).
                () => self.any_given_to_js(value, ty),
            });
        }
        let js_span = self.js_span(span);
        Ok(Value::Expr(self.binding_values(
            (def_id, generic_args),
            exprs,
            js_span,
            span,
            out,
        )?))
    }

    /// The crate's own function called, given its dictionaries (ADR 0049).
    pub(super) fn mir_crate_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        values: Vec<Value<'tcx>>,
        arg_tys: &[Ty<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let mut exprs = self.boxed_args(state, (def_id, generic_args), values, span, out)?;
        self.given_iterators(def_id, &mut exprs, arg_tys, span)?;
        exprs.extend(self.evidence_args(def_id, generic_args, span)?);
        let called = Expr::call(self.fn_ref(def_id), exprs);
        // A generic `&mut T`'s box, of an object `T` here: what's in it.
        let called = self.generic_result_values((def_id, generic_args), called, span)?;
        Ok(Value::Expr(self.fmt_result_value(def_id, generic_args, called)))
    }
}
