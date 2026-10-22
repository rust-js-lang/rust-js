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
        writes: Option<usize>,
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
        // A writer callable's `Formatter`, `writes`, isn't given it, as a
        // writer function's isn't (ADR 0054).
        let given = |i: &usize| Some(*i) != writes;
        let mut exprs = (list.into_iter().enumerate())
            .filter(|(i, _)| given(i))
            .map(|(_, v)| self.value_expr(v, span))
            .collect::<R<Vec<_>>>()?;
        if let ty::Tuple(tys) = arg_tys[1].kind() {
            let tys: Vec<Ty<'tcx>> = (tys.iter().enumerate())
                .filter(|(i, _)| given(i))
                .map(|(_, t)| t)
                .collect();
            crate::lower::calls::units_left_out(&mut exprs, &tys);
        }
        // Not a closure of the crate's: a `dyn Fn`, a generic one or a
        // pointer, which may be JS's (ADR 0330).
        if !matches!(
            arg_tys[0].peel_refs().kind(),
            ty::Closure(..) | ty::CoroutineClosure(..)
        ) {
            crate::lower::calls::given_to_js(&mut exprs);
        }
        // And its options, where the crate's writers take them (ADR 0137).
        if writes.is_some() {
            exprs.extend(self.given_to_writer(None));
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
        let mut exprs = Vec::new();
        for (i, (value, &ty)) in values.into_iter().zip(arg_tys).enumerate() {
            let value = self.value_expr(value, span)?;
            exprs.push(self.binding_arg(def_id, i, value, ty));
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

    /// What binding `def_id` is given as its `i`th argument, `value` of
    /// `ty`, as JS takes it: of its call and of its arrow as a value alike
    /// (ADR 0604).
    pub(in crate::lower) fn binding_arg(
        &self,
        def_id: rustc_span::def_id::DefId,
        i: usize,
        value: Expr,
        ty: Ty<'tcx>,
    ) -> Expr {
        let tcx = self.tcx;
        let named = |names: Vec<rustc_span::Symbol>| {
            (tcx.fn_arg_idents(def_id).get(i).copied().flatten()).is_some_and(|ident| names.contains(&ident.name))
        };
        match () {
            // `()` given to JS is what a tuple is, an array (ADR 0020).
            () if ty.is_unit() => Expr::array(Vec::new()),
            // A parameter it names `#[rust_js::nullable(..)]`: its
            // `None` is `null` (ADR 0275).
            () if named(bindings::nullable_params(tcx, def_id)) => match value.kind {
                js::ExprKind::Undefined => Expr::null(),
                _ if value.is_constant() || value.is_made_here() => value,
                _ => Expr::bin(Op::Coalesce, value, Expr::null()),
            },
            // A parameter it names `#[rust_js::lazy(..)]`, a function JS
            // calls once for the value: a closure whose body does nothing
            // and gives no function is its value, `useState(0)` (ADR 0593).
            () if named(bindings::lazy_params(tcx, def_id)) => self.lazy_value(value, ty),
            // A `&dyn Any` given to JS is its value (ADR 0331).
            () => self.any_given_to_js(value, ty),
        }
    }

    /// `js::is_defined(binding)`: `typeof MessageChannel !== "undefined"`,
    /// of the global its argument names (ADR 0597): a binding function's,
    /// by its type, or a static's, `&webapi::performance`, by the static
    /// it refers to, so no temporary between hides it. A global's own name,
    /// its `js_path`: not a method's, an import's or a path's, which aren't
    /// the host's to lack. `None` of another call.
    pub(super) fn mir_defined(
        &self,
        body: &mir::Body<'tcx>,
        def_id: rustc_span::def_id::DefId,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        span: Span,
    ) -> R<Option<Expr>> {
        let tcx = self.tcx;
        if !bindings::is_binding(tcx, def_id) || !matches!(bindings::js_form(tcx, def_id), bindings::JsForm::Defined) {
            return Ok(None);
        }
        // A path of bindings, each read from the one before (ADR 0624).
        if let Some(arg) = args.first()
            && let ty::Tuple(steps) = *arg.node.ty(&body.local_decls, tcx).kind()
            && !steps.is_empty()
        {
            let path = self.defined_path(steps, span)?;
            return self.mir_defined_parts(Some(path), span);
        }
        let global = args
            .first()
            .and_then(|arg| match *arg.node.ty(&body.local_decls, tcx).kind() {
                ty::FnDef(f, _) if bindings::is_binding(tcx, f) && bindings::is_method(tcx, f) => {
                    self.member_of_global(f)
                }
                ty::FnDef(f, _) if bindings::is_binding(tcx, f) => bindings::js_path(tcx, f),
                _ => (self.static_given(body, &arg.node))
                    .filter(|&s| tcx.is_foreign_item(s))
                    .and_then(|s| bindings::js_path(tcx, s)),
            });
        self.mir_defined_parts(global, span)
    }

    /// `typeof` of each part of `global`'s path, joined by `&&`.
    fn mir_defined_parts(&self, global: Option<String>, span: Span) -> R<Option<Expr>> {
        // Each part of its path, `typeof console !== "undefined" && typeof
        // console.log !== "undefined"`: a host may lack any of them.
        let parts: Vec<&str> = global
            .as_deref()
            .map(|path| path.split('.').collect())
            .unwrap_or_default();
        if parts.is_empty() || !parts.iter().all(|part| crate::names::is_identifier(part)) {
            return Err(self.unsupported(
                span,
                "`js::is_defined` of what isn't a JS global's binding, `webapi::message_channel::new`,",
            ));
        }
        let mut read = self.js_ref(parts[0]);
        let mut asked = Vec::new();
        for (k, part) in parts.iter().enumerate() {
            if k > 0 {
                read = Expr::member(read, part.to_string());
            }
            asked.push(Expr::bin(
                Op::Ne,
                Expr::unary(js::UnaryOp::Typeof, read.clone()),
                Expr::str("undefined"),
            ));
        }
        Ok(asked.into_iter().reduce(|a, b| Expr::bin(Op::And, a, b)))
    }

    /// The path a tuple of bindings is, each step a method of what the one
    /// before gives, its first a global's binding (ADR 0624):
    /// `(Window::document, Document::create_element::<Div>)` is
    /// `window.document.createElement`.
    fn defined_path(&self, steps: &[ty::Ty<'tcx>], span: Span) -> R<String> {
        let tcx = self.tcx;
        let refused = || {
            self.unsupported(
                span,
                "`js::is_defined` of a path of bindings where a step isn't a method of what the one before gives",
            )
        };
        let mut path = String::new();
        let mut gives: Option<ty::Ty<'tcx>> = None;
        for step in steps {
            let Some((f, args)) = crate::lower::fn_def(*step) else {
                return Err(refused());
            };
            if !bindings::is_binding(tcx, f) {
                return Err(refused());
            }
            let sig = tcx.fn_sig(f).instantiate(tcx, args).skip_normalization().skip_binder();
            path = match gives {
                None if bindings::is_method(tcx, f) => self.member_of_global(f).ok_or_else(refused)?,
                None => bindings::js_path(tcx, f).ok_or_else(refused)?,
                Some(before) => {
                    let of = sig.inputs().first().map(|this| this.peel_refs());
                    let name = match bindings::js_form(tcx, f) {
                        bindings::JsForm::Call(name) | bindings::JsForm::Get(name) => name,
                        _ => return Err(refused()),
                    };
                    if !bindings::is_method(tcx, f) || of != Some(before.peel_refs()) {
                        return Err(refused());
                    }
                    format!("{path}.{name}")
                }
            };
            gives = Some(sig.output());
        }
        Ok(path)
    }

    /// `performance.now` of `Performance::now`, a method binding of a type
    /// one global holds, the one static of its crate of that type (ADR
    /// 0597): its path, the global's and the method's JS name.
    fn member_of_global(&self, method: rustc_span::def_id::DefId) -> Option<String> {
        let tcx = self.tcx;
        let name = match bindings::js_form(tcx, method) {
            bindings::JsForm::Call(name) | bindings::JsForm::Get(name) => name,
            _ => return None,
        };
        let sig = tcx
            .fn_sig(method)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let ty::Adt(of, _) = *sig.inputs().first()?.peel_refs().kind() else {
            return None;
        };
        let holds = |id: rustc_span::def_id::DefId| {
            tcx.def_kind(id)
                == DefKind::Static {
                    safety: rustc_hir::Safety::Safe,
                    mutability: rustc_hir::Mutability::Not,
                    nested: false,
                }
                && matches!(*tcx.type_of(id).instantiate_identity().skip_normalization().peel_refs().kind(),
                    ty::Adt(held, _) if held.did() == of.did())
        };
        let statics: Vec<rustc_span::def_id::DefId> = match of.did().as_local() {
            Some(_) => (tcx.hir_crate_items(()).foreign_items())
                .map(|item| item.owner_id.to_def_id())
                .filter(|&id| holds(id))
                .collect(),
            None => (tcx.module_children(of.did().krate.as_def_id()).iter())
                .filter_map(|child| child.res.opt_def_id())
                .filter(|&id| holds(id))
                .collect(),
        };
        let [global] = statics.as_slice() else { return None };
        Some(format!("{}.{name}", bindings::js_path(tcx, *global)?))
    }

    /// The static `operand` refers to: `&STATIC`, of a constant, or of the
    /// one temporary MIR makes it in, `_1 = &(*_2); _2 = const {STATIC}`.
    fn static_given(&self, body: &mir::Body<'tcx>, operand: &Operand<'tcx>) -> Option<rustc_span::def_id::DefId> {
        let local = match operand {
            Operand::Constant(c) => return self.const_static(c),
            Operand::Copy(place) | Operand::Move(place) => place.as_local()?,
            _ => return None,
        };
        let assigned: Vec<&Rvalue<'tcx>> = (body.basic_blocks.iter())
            .flat_map(|block| &block.statements)
            .filter_map(|s| match &s.kind {
                StatementKind::Assign(assign) if assign.0.as_local() == Some(local) => Some(&assign.1),
                _ => None,
            })
            .collect();
        match assigned.as_slice() {
            [Rvalue::Use(operand, _) | Rvalue::Cast(_, operand, _)] => self.static_given(body, operand),
            [Rvalue::Ref(_, _, place) | Rvalue::RawPtr(_, place)]
                if matches!(place.projection.as_slice(), [PlaceElem::Deref]) =>
            {
                self.static_given(body, &Operand::Copy(Place::from(place.local)))
            }
            _ => None,
        }
    }

    /// A lazy parameter's argument (`bindings::lazy_params`): `() => 0` is
    /// `0`, its body's value, where its body does nothing and what it gives
    /// isn't a function, which JS would call for the value in its place.
    fn lazy_value(&self, value: Expr, ty: Ty<'tcx>) -> Expr {
        let gives = match ty.kind() {
            ty::Closure(_, args) => args.as_closure().sig().output().skip_binder(),
            _ => return value,
        };
        match &value.kind {
            js::ExprKind::Arrow(params, body)
                if params.is_empty() && !self.is_callable(gives) && !self.is_unknown(gives) =>
            {
                match body.as_slice() {
                    [
                        js::Stmt {
                            kind: StmtKind::Return(Some(given)),
                            ..
                        },
                    ] if !given.has_effects() => given.clone(),
                    _ => value,
                }
            }
            _ => value,
        }
    }
}
