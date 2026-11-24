//! A call from MIR of what isn't std's (ADR 0364): a closure's, a binding's,
//! the crate's own function's, and an operator's that assigns through a
//! `&mut`. `mir_call_values` tells which a call is, and hands it here.

use super::*;
use rust_js_lower::temporary_names::Role;

mod defined;
mod method_paths;

use self::defined::value_first;

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// A call of what may read the `this` JS would give it
    /// (`may_read_this`): a JS function, a with_this function or a
    /// binding's a fn pointer holds, whose callee is read as a member,
    /// `s.f`, or an item, `fs[i]`, which JS would give it as its `this`.
    /// Its callee is read unbound, `(0, s.f)()`, so it's called with none,
    /// as Rust calls it (#402, ADR 0972; #427; rd579). A temporary MIR
    /// holds the read in is read unbound too, `(0, f)`: `(0, s.f)` where tidy
    /// reads it in place, else the variable alone, `f()`, which gives none.
    /// Rust's own function, the crate's closure, is called as it is.
    pub(super) fn called_unbound(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (call, called): (Value<'tcx>, Called<'tcx>),
    ) -> R<Value<'tcx>> {
        let Value::Expr(Expr {
            kind: js::ExprKind::Call(callee, args),
            ..
        }) = call
        else {
            return Ok(call);
        };
        let (pointers, holder) = (self.krate.cell_borrows.pointers(), self.called_holder(state, called));
        if !pointers.may_read_this(self.tcx, called, holder) {
            return Ok(Value::Expr(Expr {
                facts: pointers.call_facts(holder),
                ..Expr::call(*callee, args)
            }));
        }
        // A variable of the program's is called with none as it is, as a
        // closure that only calls it is that function (`passed_on`); MIR's
        // temporary, which tidy may read in place, and the rest are read
        // unbound, and of what JS gives no `this` of, a call's value,
        // preparation leaves it as it is.
        if let js::ExprKind::Var(name) = &callee.kind
            && !super::tidy::is_temporary(name)
        {
            return Ok(Value::Expr(Expr::call(*callee, args)));
        }
        Ok(Value::Expr(Expr::call(Expr::unbound(*callee), args)))
    }

    /// What holds what the call that ends `state.at` calls (rd579), of the
    /// place of what it calls, a function's its first argument, a fn
    /// pointer's itself, or the place a temporary of it borrows or copies,
    /// `_3 = &(*((*_1).0))`: a field of the crate's struct, `(s.f)()`'s `f`,
    /// one a symbol names too; an item of a tuple or an array, `(p.0)()`,
    /// `fs[i]()`; or what the MIR doesn't show, a thread-local's `with`'s.
    fn called_holder(&self, state: &State<'_, 'tcx>, called: Called<'tcx>) -> Holder {
        self.called_place_holder(state, called).unwrap_or(Holder::Unknown)
    }

    fn called_place_holder(&self, state: &State<'_, 'tcx>, called: Called<'tcx>) -> Option<Holder> {
        let mir::TerminatorKind::Call { func, args, .. } = &state.body[state.at].terminator().kind else {
            return None;
        };
        let through = match called {
            Called::Function(..) => &args.first()?.node,
            Called::Pointer(_) => func,
        };
        let mut place = through.place()?;
        let assigned = |local: mir::Local| {
            let statements = state.body.basic_blocks.iter().flat_map(|block| &block.statements);
            statements.into_iter().find_map(|statement| match &statement.kind {
                StatementKind::Assign(assign) if assign.0.local == local && assign.0.projection.is_empty() => {
                    match &assign.1 {
                        Rvalue::Ref(_, _, of) | Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _) => Some(*of),
                        _ => None,
                    }
                }
                _ => None,
            })
        };
        // What a call gives of what it's given: a `Cell`'s value got,
        // `(s.f.get())()`, and an `Rc`'s or a `Box`'s pointee, `(s.f)()` of an
        // `Rc<dyn Fn()>`, the place of what it's given.
        let given_of = |local: mir::Local| {
            (state.body.basic_blocks.iter()).find_map(|block| match &block.terminator().kind {
                mir::TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } if destination.local == local
                    && destination.projection.is_empty()
                    && let Some((f, _)) = func.const_fn_def()
                    && (rust_js_lower::recognition::cell_own_method(self.tcx, f)
                        == Some(rust_js_lower::recognition::CellMethod::Get)
                        || self
                            .tcx
                            .trait_of_assoc(f)
                            .is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Deref))
                            && args.first().is_some_and(|given| {
                                let ty = given.node.ty(state.body, self.tcx).peel_refs();
                                ty.is_box() || rust_js_lower::recognition::rc_pointee(self.tcx, ty).is_some()
                            })) =>
                {
                    args.first()?.node.place()
                }
                _ => None,
            })
        };
        // Through each temporary, and what it points at, `&(*_2)`, to the
        // place a field is read of.
        for _ in 0..4 {
            if !place.projection.iter().all(|elem| matches!(elem, PlaceElem::Deref)) {
                break;
            }
            place = assigned(place.local).or_else(|| given_of(place.local))?;
        }
        let (base, elem) = (place.iter_projections()).rfind(|(_, elem)| {
            matches!(
                elem,
                PlaceElem::Field(..)
                    | PlaceElem::Index(_)
                    | PlaceElem::ConstantIndex { .. }
                    | PlaceElem::Subslice { .. }
            )
        })?;
        Some(match (elem, base.ty(state.body, self.tcx).ty.kind()) {
            (PlaceElem::Field(index, _), ty::Adt(adt, _)) if adt.is_struct() => {
                Holder::Field(adt.non_enum_variant().fields[index].did)
            }
            // A tuple's, an array's or a slice's.
            (PlaceElem::Field(..), ty::Tuple(_))
            | (PlaceElem::Index(_) | PlaceElem::ConstantIndex { .. } | PlaceElem::Subslice { .. }, _) => Holder::Item,
            _ => Holder::Unknown,
        })
    }

    /// A closure called, `Fn::call(&f, (a, b))`: in JS, `f(a, b)`, and an
    /// `async` one's, `AsyncFn::async_call`, which gives its promise.
    pub(super) fn mir_closure_call(
        &mut self,
        values: Vec<Value<'tcx>>,
        arg_tys: &[Ty<'tcx>],
        writes: Option<usize>,
        span: Span,
        unbound: Option<(&mut State<'_, 'tcx>, Ty<'tcx>, &mut Vec<Stmt>)>,
    ) -> R<Value<'tcx>> {
        let mut values = values.into_iter();
        let callee = match values.next().expect("the closure") {
            Value::Ref(closure) => closure,
            value => self.value_expr(value, span)?,
        };
        // A closure of the crate's that never unwinds (ADR 0902): its call
        // throws only where what it's given does, so a guard that calls it
        // as it drops drops in a `finally` (#847).
        let callee = match arg_tys.first().map(|ty| ty.peel_refs().kind()) {
            Some(ty::Closure(id, _)) if self.krate.never_unwinding.contains(id) => callee.never_throws(),
            _ => callee,
        };
        let list = match values.next() {
            Some(Value::List(items, _)) => items,
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
        // Each where it's written, as a function's argument is, so a
        // comment before it is before it, `(d.preconnect)(/* preconnect */
        // href, cross_origin)` (rd657).
        let written = self.closure_arguments_written(span);
        let mut exprs = (list.into_iter().enumerate())
            .filter(|(i, _)| given(i))
            .map(|(i, v)| {
                let mut e = self.value_expr(v, span)?;
                if e.span.is_none()
                    && let Some(at) = written.get(i).filter(|at| !at.from_expansion())
                {
                    e.span = self.js_span(*at);
                }
                Ok(e)
            })
            .collect::<R<Vec<_>>>()?;
        if let ty::Tuple(tys) = arg_tys[1].kind() {
            let tys: Vec<Ty<'tcx>> = (tys.iter().enumerate())
                .filter(|(i, _)| given(i))
                .map(|(_, t)| t)
                .collect();
            // Rest arguments as a value, its last, spread (ADR 0851): what
            // takes them is JS's rest parameter, a closure's or a binding's.
            if tys
                .last()
                .is_some_and(|&ty| rust_js_lower::bindings::is_rest_value(self.tcx, ty))
            {
                rust_js_lower::calls::spread_rest(&mut exprs);
            }
        }
        // Not a closure of the crate's: a `dyn Fn`, a generic one or a
        // pointer, which may be JS's (ADR 0330), and read its `this`.
        let own = matches!(
            arg_tys[0].peel_refs().kind(),
            ty::Closure(..) | ty::CoroutineClosure(..)
        );
        let tys: Vec<Ty<'tcx>> = match arg_tys[1].kind() {
            ty::Tuple(tys) => (tys.iter().enumerate())
                .filter(|(i, _)| given(i))
                .map(|(_, t)| t)
                .collect(),
            _ => Vec::new(),
        };
        let absent = |i, arg: &Expr| rust_js_lower::calls::absent(self.tcx, &tys, &[], i, arg);
        match own {
            // One that may be a `function::with_this` function that reads
            // `js::arguments()`: every argument (rd509).
            false
                if self
                    .krate
                    .reads_arguments
                    .may_call(self.tcx, self.krate.foreign, arg_tys[1]) =>
            {
                rust_js_lower::calls::given_to_js(&mut exprs)
            }
            true => rust_js_lower::calls::own_absent(&mut exprs, absent),
            false => rust_js_lower::calls::given_to_js_but(&mut exprs, absent),
        }
        // Of one that may be JS's, each trailing `Option` known only where
        // it runs a call of each count, as a pointer's is (`optional_ladder`,
        // #1876); each a `Some` where it's given known there.
        let laddered = !own
            && unbound
                .as_ref()
                .is_some_and(|(state, output, _)| self.callback_may_be_js(state, (arg_tys[0], arg_tys[1], *output)));
        let optionals: Vec<(bool, bool)> = match laddered {
            true => (tys.iter())
                .map(|&ty| {
                    (
                        self.option_of(ty).is_some(),
                        rust_js_lower::analysis::nulled_params::one_none(self.tcx, ty),
                    )
                })
                .collect(),
            false => Vec::new(),
        };
        // Each a `Some` where it's given, of MIR's answer (`presence`).
        let present = match &unbound {
            Some((state, _, _)) if laddered || !own => self.presences_in_a_tuple(state),
            _ => Vec::new(),
        };
        // Of one that may be JS's, each `Option` Rust's `None` (#1840): not a
        // `Some` where it's given, there (`never_null`).
        if !own && let Some((state, output, _)) = &unbound {
            self.ones_given_to_a_callback(state, (arg_tys[0], arg_tys[1], *output), (&tys, &present), &mut exprs);
        }
        // And its options, where the crate's writers take them (ADR 0137).
        if writes.is_some() {
            exprs.extend(self.given_to_writer(None));
        }
        let call = Value::Expr(Expr::call(callee, exprs));
        let output = unbound.as_ref().map(|(_, output, _)| *output);
        let call = match unbound {
            Some((state, output, out)) if !own => {
                let call = self.called_unbound(state, (call, Called::Function(arg_tys[0], arg_tys[1], output)))?;
                match (laddered, call) {
                    (true, Value::Expr(call)) => {
                        let proofs = self.arg_proofs_in_a_tuple(state, &present);
                        let at = self.js_span(span);
                        Value::Expr(self.optional_ladder(call, None, (&optionals, &proofs), (at, out)))
                    }
                    (_, call) => call,
                }
            }
            _ => call,
        };
        // Of one that may give back a write's error, as a function's call is
        // (`fmt_result_value`): what it throws, caught, its result a value
        // (#1042). Not a writer's, whose `Formatter` it's given.
        Ok(match (call, output) {
            (Value::Expr(called), Some(output))
                if writes.is_none() && self.is_write_result(output) && self.closure_may_fail(arg_tys[0]) =>
            {
                Value::Expr(self.fmt_try(vec![StmtKind::Return(Some(called)).at(js::Span::NONE)]))
            }
            (call, _) => call,
        })
    }

    /// Whether a call of `callee`, a closure's type or what may be one, may
    /// give back a write's error (ADR 0187): the crate's own closure that
    /// `fmt_failures` found may, a function's as `call_may_fail` says, and
    /// any other, a `dyn Fn`'s or a type parameter's, where any may.
    fn closure_may_fail(&self, callee: Ty<'tcx>) -> bool {
        self.krate.failing.any
            && match *callee.peel_refs().kind() {
                ty::Closure(id, _) => self.krate.failing.fns.contains(&id),
                ty::FnDef(id, args) => self.call_may_fail(id, args.skip_binder()),
                _ => true,
            }
    }

    /// A callback of type `ty` given to std's iterator code that gives back
    /// a write's result, `map(|w| writeln!(out, "{w}"))`: what it gives an
    /// item, a value, `undefined` or its error, `$writeTry(..)` of its
    /// body, as a call of it is (`fmt_try`), not its error thrown past what
    /// steps it (#1410). Any other, `f` as it is.
    pub(super) fn write_result_valued(&mut self, f: Expr, ty: Ty<'tcx>) -> Expr {
        let callable = ty.peel_refs();
        let callable = callable.boxed_ty().unwrap_or(callable);
        let output = match *callable.kind() {
            // A `dyn FnMut(&str) -> io::Result<()>`'s, its `Output`.
            ty::Dynamic(bounds, ..) => match bounds.projection_bounds().find_map(|p| p.skip_binder().term.as_type()) {
                Some(output) => output,
                None => return f,
            },
            ty::Closure(_, args) => args.as_closure().sig().output().skip_binder(),
            ty::FnDef(id, args) => self
                .tcx
                .fn_sig(id)
                .instantiate(self.tcx, args.skip_binder())
                .skip_normalization()
                .output()
                .skip_binder(),
            ty::FnPtr(sig, header) => sig.with(header).output().skip_binder(),
            _ => return f,
        };
        if !self.is_write_result(output) || !self.closure_may_fail(ty) {
            return f;
        }
        match f.kind {
            js::ExprKind::Arrow(params, body) => Expr::arrow(
                params,
                vec![StmtKind::Return(Some(self.fmt_try(body))).at(js::Span::NONE)],
            ),
            kind => {
                let args = self.fresh(Role::Args);
                let called = Expr::call(Expr { kind, ..f }, vec![Expr::spread(Expr::var(&args))]);
                Expr::arrow(
                    vec![js::Pattern::Rest(args)],
                    vec![
                        StmtKind::Return(Some(
                            self.fmt_try(vec![StmtKind::Return(Some(called)).at(js::Span::NONE)]),
                        ))
                        .at(js::Span::NONE),
                    ],
                )
            }
        }
    }

    /// `a += b` of a number or a string, through its `&mut`: the place
    /// assigned, as the operator's is; `None` of another call.
    pub(super) fn mir_assign_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (values, arg_tys): (&[Value<'tcx>], &[Ty<'tcx>]),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let tcx = self.tcx;
        // Of a number or a string: another's is its impl's, or its dictionary's
        // (ADR 0396); and so is the crate's own, `impl AddAssign<Cents> for i32`.
        if let Some(trait_id) = tcx.trait_of_assoc(def_id)
            && !self.resolves_to_crate(def_id, generic_args)?
            && let Some(op) = rust_js_lower::recognition::assign_operator(tcx, trait_id)
            && let [place, rhs] = values
            && let ty = arg_tys[0].peel_refs()
            && (self.is_lang_adt(ty, LangItem::String)
                || rust_js_lower::representation::Num::of(ty).is_some()
                || ty.is_bool())
            && let Value::Ref(place) = place
        {
            // A place reached through a call, `*first(&mut v) += &five`'s
            // `first(v).value`, or `or_insert(10)`'s handle's `value`, read
            // and then written: what holds it named where it's reached,
            // before what's added is, so the call runs once. MIR gives the
            // operator a reborrow, `&mut *x`, a place, of every `&mut`.
            let place = match !rust_js_lower::held::fixed_place(place) && place.has_effects() {
                true => {
                    self.flush(state, out)?;
                    self.fixed_place_of(place.clone(), out)
                }
                false => place.clone(),
            };
            let place = &place;
            let rhs = self.value_expr(rhs.clone(), span)?;
            let rhs = rust_js_lower::std_types::number::shift_amount_of(op, rhs, ty, arg_tys[1]);
            let value = if self.is_lang_adt(ty, LangItem::String) && op == BinOp::Add {
                Expr::text_joined(place.clone(), rhs)
            } else if rust_js_lower::recognition::has_number_arithmetic(ty) || ty.is_bool() {
                self.binary(op, place.clone(), rhs, None, ty, span)?
            } else if let Some((def_id, generic_args, known)) =
                self.recognition().assigned_by(trait_id, ty, arg_tys[1].peel_refs())
            {
                // A `Duration`'s or a clock's: its own `+` (ADRs 0188, 0447).
                let tys = [ty, arg_tys[1]];
                let given = rust_js_lower::calls::Given::default();
                let call = rust_js_lower::calls::Call::given((def_id, generic_args), &tys, given, span);
                self.std_values(known, call, vec![place.clone(), rhs], false, ty, out)?
            } else {
                return Err(self.unsupported(
                    span,
                    &format!("`{}` of a `{ty}`, from its MIR", tcx.def_path_str(def_id)),
                ));
            };
            self.flush(state, out)?;
            out.push(StmtKind::Assign(place.clone(), value).at(self.js_span(span)));
            return Ok(Some(Value::Expr(Expr::js_undefined())));
        }
        Ok(None)
    }

    /// A binding called: the JS function, method, property or operator it
    /// names (ADRs 0019, 0020), given its arguments, each whether it's an
    /// `Option` `null` or a value already (`nulled`).
    pub(super) fn mir_binding_call(
        &mut self,
        state: &State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (values, arg_tys, nulled): (Vec<Value<'tcx>>, &[Ty<'tcx>], &[bool]),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let tcx = self.tcx;
        if rustc_hir::find_attr!(tcx, def_id, RustcEiiForeignItem) {
            return Err(self.unsupported(span, "externally implementable items, `#[eii]`,"));
        }
        // A value read as the registry's symbol (ADR 0634): itself where
        // it's only tested against the registry's, `switch (type)`; else
        // itself where it's one, `undefined` where not, `$symbolFor(value)`.
        if rust_js_lower::recognition::is_symbol_for_read(tcx, def_id) {
            let value = values.into_iter().next().expect("the value read");
            let read = rust_js_lower::body_queries::the_call_at(self.frame.thir, span);
            return Ok(
                match read.is_some_and(|read| self.frame.facts.registered_reads.contains(&read)) {
                    true => value,
                    false => Value::Expr(Expr::call_helper("$symbolFor", vec![self.value_expr(value, span)?])),
                },
            );
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
                if let js::ExprKind::Object(fields, ..) = &mut element.kind {
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
                for ((key, value), &ty) in keys.into_iter().zip(values).zip(arg_tys) {
                    // `{...}`: what's given spread into it, `js::object::spread`'s
                    // (ADR 0967).
                    match key.as_str() {
                        "..." => self.spread_into(value, ty, span, out, &mut fields)?,
                        _ => fields.push(Prop::Field(key, self.value_expr(value, span)?)),
                    }
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
            exprs.push(self.binding_arg(def_id, (i, nulled.get(i) == Some(&true)), value, ty));
        }
        // Where its path names it, which says where it's read (ADR 0633),
        // looked for where any path is read through a module.
        let named = match self.krate.held_reads.is_empty() {
            true => span,
            false => (self.thir_call(def_id, span)).map_or(span, |(fun, _)| self.frame.thir[fun].span),
        };
        let js_span = self.js_span(span);
        // What MIR proves of each argument where it's read (`presence`),
        // which a call of each count of its trailing `Option`s asks.
        let present = match &state.body[state.at].terminator().kind {
            mir::TerminatorKind::Call { args, .. } => {
                let operands: Vec<&Operand<'tcx>> = args.iter().map(|arg| &arg.node).collect();
                self.arg_proofs(state, &operands)
            }
            _ => Vec::new(),
        };
        let made = self.binding_values((def_id, generic_args), (exprs, &present), (js_span, named), span, out)?;
        // The crate's own binding's result, checked where it returns (#2403,
        // ADR 1195), but one its body type-tests first, whose test is its check.
        let made = match self.binding_result_tested(state.body, state.at, def_id) {
            true => made,
            false => self.checked_binding_result(def_id, made),
        };
        // Of a `dyn Iterator`, JS's own, marked so (#1359).
        let output = self
            .tcx
            .fn_sig(def_id)
            .instantiate(self.tcx, generic_args)
            .skip_normalization();
        let output = self.tcx.instantiate_bound_regions_with_erased(output).output();
        Ok(Value::Expr(rust_js_lower::iterators::made_by_a_binding(made, output)))
    }

    /// What `js::object::spread` spreads into the object it makes, `value`
    /// of `ty`: the value, `{ ...value }`, or each of a tuple's, in order,
    /// `{ ...a, ...b }` (ADR 0967, amended). Rust makes each of a tuple's
    /// before any is spread, where JS spreads one before it makes the
    /// next: where a later one runs the program's code, which may change
    /// what's spread before it, `changed(a)` of `(a, changed(a))`, each is
    /// made first, in order, and the object spreads what they made. Not
    /// one that runs none of it, a `Dict` written out, `dict::from_entries`'s.
    fn spread_into(
        &mut self,
        value: Value<'tcx>,
        ty: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
        fields: &mut Vec<Prop>,
    ) -> R<()> {
        let ty::Tuple(tys) = *ty.kind() else {
            // `None` written, `undefined`, spreads nothing: `spread(None)`
            // is `{}`, a new object of no properties, React's keyframes'
            // `[{}, {}]` (rd210).
            let value = self.value_expr(value, span)?;
            if !matches!(value.kind, js::ExprKind::Undefined | js::ExprKind::Null) {
                fields.push(Prop::Spread(value));
            }
            return Ok(());
        };
        let parts = match value {
            Value::List(items, _) => items,
            value => match self.value_expr(value, span)? {
                Expr {
                    kind: js::ExprKind::Array(items, ..),
                    ..
                } => items.into_iter().map(Value::Expr).collect(),
                tuple => {
                    let tuple = match tuple.reads_same() {
                        true => tuple,
                        false => self.spill(Role::Spread, tuple, out),
                    };
                    (0..tys.len())
                        .map(|i| Value::Expr(Expr::index(tuple.clone(), Expr::int(i as i128))))
                        .collect()
                }
            },
        };
        let mut parts = (parts.into_iter())
            .map(|part| self.value_expr(part, span))
            .collect::<R<Vec<_>>>()?;
        let runs_code = |part: &Expr| part.has_effects() && !part.facts.has(js::Known::RunsNothing);
        if parts.iter().skip(1).any(runs_code) {
            for part in &mut parts {
                if !part.reads_same() {
                    let made = std::mem::replace(part, Expr::taken());
                    *part = self.spill(Role::Spread, made, out);
                }
            }
        }
        for (part, ty) in parts.into_iter().zip(tys) {
            self.spread_into(Value::Expr(part), ty, span, out, fields)?;
        }
        Ok(())
    }

    /// Of a call of `def_id`, a namespace's method's (ADR 1037), what its
    /// lowering reads (`State::namespace_call`): whether its `self` names
    /// the namespace, and of one through a value whose result only
    /// `.flatten()` reads, that result flattened where its function makes
    /// and returns it (rd311), by its name too where each call is so
    /// (rd324). `None` of any other call.
    pub(super) fn namespace_call(
        &self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        receiver: Option<&rustc_middle::mir::Operand<'tcx>>,
        destination: rustc_middle::mir::Place<'tcx>,
    ) {
        state.flattened.namespace_call = (rust_js_lower::classes::member(self.tcx, def_id)
            == Some(rust_js_lower::classes::Member::Namespace))
        .then(|| {
            let named = receiver
                .is_some_and(|receiver| rust_js_lower::classes::names_namespace(self.tcx, state.body, receiver));
            let flattened = match named {
                // By its name, where each of its calls is flattened (rd324).
                true => {
                    rust_js_lower::analysis::flattened_by_name(&self.recognition(), state.body, (def_id, generic_args))
                }
                false => (destination.projection.is_empty())
                    .then(|| rust_js_lower::analysis::flattened_alike(&self.recognition(), (def_id, generic_args)))
                    .flatten()
                    .filter(|_| {
                        rust_js_lower::analysis::read_by_flatten(&self.recognition(), state.body, destination.local)
                    }),
            }
            .filter(|_| destination.projection.is_empty());
            (named, flattened.map(|p| (destination.local, p)))
        });
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
        // A method that reads each variant's field: that read of its value (rd317).
        if let Some(read) = self.variant_field_read(def_id, &values, span)? {
            return Ok(Value::Expr(read));
        }
        let mut exprs = self.boxed_args(state, (def_id, generic_args), (values, arg_tys), span, out)?;
        // A callback it calls once, `impl FnOnce(A0, A1) -> T`, an arrow that
        // only passes on what it's given: the function it passes on to,
        // React's `runWithFiberInDEV(fiber, commitCallbacks, queue, instance)`,
        // as a binding's (ADR 0605, amended).
        if exprs.len() == arg_tys.len() {
            for (i, expr) in exprs.iter_mut().enumerate() {
                if let Some(taker) = rust_js_lower::passed_on::Taker::parameter(self.tcx, def_id, i)
                    && let Some(callee) = rust_js_lower::passed_on::passed_on(expr, taker, self.forwarding())
                {
                    *expr = callee;
                }
            }
        }
        self.crossing_into(
            (def_id, generic_args),
            &mut exprs,
            rust_js_lower::generic_facts::Arguments::FormedOf { given: arg_tys },
            span,
            out,
        )?;
        // Its rest arguments, the slice spread (ADR 0664), which nothing
        // comes after.
        self.rest_given(def_id, &mut exprs);
        let evidence = self.evidence_args(def_id, generic_args, span)?;
        // A trailing `None` or `()` is no argument, `own()` of `own(None)`,
        // as a binding's and a function value's are (ADRs 0443, 0441): of a
        // parameter declared an `Option` or `()`, TypeScript's `x?`, not a
        // type parameter's, `createCursor(undefined)`; where nothing comes
        // after it, a dictionary, and each argument is its parameter's.
        let declared = self.tcx.fn_sig(def_id).instantiate_identity().skip_binder();
        // Of one that reads `js::arguments()`, every argument, as the Rust
        // gives it (rd332).
        if self.reads_arguments(def_id) {
            rust_js_lower::calls::given_to_js(&mut exprs);
        } else if evidence.is_empty() && exprs.len() == arg_tys.len() && declared.inputs().len() == arg_tys.len() {
            rust_js_lower::calls::own_absent(&mut exprs, |i, arg| {
                rust_js_lower::calls::absent(self.tcx, declared.inputs(), &[], i, arg)
            });
        }
        exprs.extend(evidence);
        // The copy of it, where JS calls it too, that takes what this call
        // gives as it gives it (ADR 0051, amended).
        let copy = self.called_copy((def_id, generic_args), span)?;
        // A namespace's method's through a value, the module's own JS, or
        // by its name where each of its calls is so (rd324): where the
        // call's result only `.flatten()` reads, of one that makes and
        // returns it, that JS's result is it flattened, where its JS leaves
        // out the `Some` of what the call gives alone (rd311).
        let namespace_call = state.flattened.namespace_call.take();
        if let Some((_, Some((local, p)))) = namespace_call
            && self.leaves_out_some_alone((def_id, copy), p)
        {
            state.flattened.locals.insert(local);
        }
        let named = namespace_call.map(|(named, flattened)| {
            (
                named,
                flattened.is_some_and(|(local, _)| state.flattened.locals.contains(&local)),
            )
        });
        // Of a library's function whose library keeps no capacity, which
        // the crate can't compile itself (`leaves_capacity_unknown`): each
        // `Vec` it may change of what it's given, through a `&mut` or a
        // cell, and each it gives back, of unknown capacity (#1180's (5)).
        let unkept = self.krate.foreign.leaves_capacity_unknown(def_id);
        if unkept {
            for (expr, &ty) in exprs.iter_mut().zip(arg_tys) {
                if ty.is_mutable_ptr() || !ty.is_freeze(self.tcx, self.typing_env) {
                    *expr = Expr::call_helper("$untrackedIn", vec![std::mem::replace(expr, Expr::taken())]);
                }
            }
        }
        let called = match self.krate.scopes.of_method(def_id) {
            Some(_) => self.scope_called(def_id, copy, exprs, span)?,
            None => self.crate_called_copy((def_id, copy, Some(span)), named, exprs),
        };
        // Of one that reads `js::this()`: called with none, as Rust calls
        // it, its callee read unbound, `(0, M.f)()` through a module's
        // namespace, which JS would give it as its `this`, and of a name
        // alone, which gives none, `f()` (`Expr::unbound_variable_read`,
        // rd683).
        let mut called = called;
        if self.reads_this(def_id)
            && let js::ExprKind::Call(callee, _) = &mut called.kind
        {
            **callee = Expr::unbound(std::mem::replace(&mut **callee, Expr::taken()));
        }
        // A generic `&mut T`'s box, of an object `T` here: what's in it.
        let called = self.generic_result_values((def_id, generic_args), called, span)?;
        let called = self.fmt_result_value(def_id, generic_args, called);
        let output = self
            .tcx
            .fn_sig(def_id)
            .instantiate(self.tcx, generic_args)
            .skip_binder()
            .output();
        let called = match unkept && !output.is_scalar() && !output.is_unit() {
            true => Expr::call_helper("$untrackedIn", vec![called]),
            false => called,
        };
        // A throwing function's call is a throwing call (ADR 0644).
        Ok(Value::Expr(self.catching(def_id, generic_args, called)))
    }

    /// The call of `def_id`, a scope struct's method, given `args`: of its
    /// function's function, closing over the value its call is given as its
    /// `self`, a variable, left out (ADR 1043). One that never unwinds
    /// throws only where what it's given does (ADR 0902).
    fn scope_called(
        &self,
        def_id: rustc_span::def_id::DefId,
        copy: Option<usize>,
        mut args: Vec<Expr>,
        span: Span,
    ) -> R<Expr> {
        let name = (self.scope_functions.get(&def_id))
            .filter(|_| self.scope_copies.get(&def_id) == Some(&copy))
            .ok_or_else(|| self.unsupported(span, "a scope struct's method, called outside its function"))?;
        if !matches!(args.first().map(|e| &e.kind), Some(js::ExprKind::Var(_))) {
            return Err(self.unsupported(span, "a scope struct's method, given a `self` that's no variable"));
        }
        args.remove(0);
        let callee = match self.krate.never_unwinding.contains(&def_id) {
            true => Expr::var(name).never_throws(),
            false => Expr::var(name),
        };
        Ok(Expr::call(callee, args))
    }

    /// Whether the JS a namespace's method's value has of `def_id`, its own,
    /// leaves out the `Some` of `p` where a call of it needs `copy` of it,
    /// and of no other: of the crate's, `copy` takes its type parameters as
    /// its own JS does, which takes any JS gives, but `p` boxed; of a
    /// library's, its export takes `p` as JS gives it, and the crate
    /// compiles none of it for another (rd311, ADRs 0051, 0914).
    fn leaves_out_some_alone(&self, (def_id, copy): (rustc_span::def_id::DefId, Option<usize>), p: u32) -> bool {
        if !def_id.is_local() {
            return (self.krate.foreign.item(def_id).and_then(|item| item.calling.param(p)))
                .is_some_and(|param| param.has(rust_js_program::library::ParamFact::Unboxed));
        }
        let Some(at) = copy else { return false };
        let given = &self.krate.given.js_given;
        let (own, copied) = (given.main(self.tcx, def_id), given.params(self.tcx, def_id, Some(at)));
        copied.js.is_subset(&own.js)
            && copied.unboxed.is_subset(&own.unboxed)
            && own.unboxed.difference(&copied.unboxed).eq([&p])
    }

    /// Whether a variable holding the function `def_id`, of `count`
    /// parameters, is called as that variable: a binding's JS function or
    /// getter; the crate's given no dictionaries, boxes or rest arguments
    /// (ADR 0664). And `js::SymbolFor`, which is `Symbol.for` (ADR 0964).
    pub(super) fn called_as_held(&self, def_id: rustc_span::def_id::DefId, count: usize) -> bool {
        // A binding's: a JS function, or a getter's, which holds its read,
        // `() => m.version` (#581).
        if rust_js_lower::bindings::is_binding(self.tcx, def_id) {
            return !rust_js_lower::bindings::is_method(self.tcx, def_id)
                && matches!(
                    rust_js_lower::bindings::js_form(self.tcx, def_id),
                    rust_js_lower::bindings::JsForm::Call(_) | rust_js_lower::bindings::JsForm::Get(_)
                );
        }
        if rust_js_lower::recognition::is_symbol_for_constructor(self.tcx, def_id) {
            return true;
        }
        self.krate.fns.contains_key(&def_id)
            && !self.takes_rest(def_id)
            && self.tcx.generics_of(def_id).count() == 0
            && (0..count).all(|i| !self.param_is_box(def_id, i))
    }
}
