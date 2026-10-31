//! A call from MIR of what isn't std's (ADR 0364): a closure's, a binding's,
//! the crate's own function's, and an operator's that assigns through a
//! `&mut`. `mir_call_values` tells which a call is, and hands it here.

use super::*;
use crate::lower::bindings::Asked;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of what may be a with_this function, a `dyn Fn`'s or a
    /// generic one's of its signature, or a binding's JS function a fn
    /// pointer holds (`may_read_this`), whose callee is read as a member,
    /// `s.f`, or an item, `fs[i]`, which JS would give it as its `this`:
    /// its callee held first, `const f = s.f; f()`, so it's called with
    /// none, as Rust calls it (#402, ADR 0972; #427). A temporary MIR holds
    /// the read in, which tidy reads in place, is held as its field is
    /// named. Not of a field the crate fills with its own closures alone
    /// (#405).
    pub(super) fn called_unbound(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (call, called): (Value<'tcx>, Called<'tcx>),
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let Value::Expr(Expr {
            kind: js::ExprKind::Call(callee, args),
            ..
        }) = call
        else {
            return Ok(call);
        };
        let (pointers, field) = (self.krate.cell_borrows.pointers(), self.called_field(state, called));
        if !pointers.may_read_this(self.tcx, called, field) {
            return Ok(Value::Expr(Expr::call(*callee, args)));
        }
        let name = match &callee.kind {
            js::ExprKind::Member(_, property) | js::ExprKind::OptionalMember(_, property) => property.clone(),
            js::ExprKind::Index(..) => "f".to_string(),
            js::ExprKind::Var(name)
                if super::tidy::is_temporary(name)
                    && let Some(field) = field =>
            {
                crate::lower::camel_case(self.tcx.item_name(field).as_str())
            }
            _ => return Ok(Value::Expr(Expr::call(*callee, args))),
        };
        self.flush(state, out)?;
        let callee = self.spill(&name, *callee, out);
        Ok(Value::Expr(Expr::call(callee, args)))
    }

    /// The field of the crate's struct the call that ends `state.at` calls,
    /// `(s.f)()`'s `f`: of the place of what it calls, a function's its
    /// first argument, a fn pointer's itself, or the place a temporary of
    /// it borrows or copies, `_3 = &(*((*_1).0))`.
    fn called_field(&self, state: &State<'_, 'tcx>, called: Called<'tcx>) -> Option<rustc_span::def_id::DefId> {
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
        if place.projection.is_empty() {
            place = match assigned(place.local) {
                Some(of) => of,
                // A `Cell`'s value got, `(s.f.get())()`: the cell's place.
                None => {
                    let got = (state.body.basic_blocks.iter()).find_map(|block| match &block.terminator().kind {
                        mir::TerminatorKind::Call {
                            func,
                            args,
                            destination,
                            ..
                        } if destination.local == place.local
                            && destination.projection.is_empty()
                            && let Some((f, _)) = func.const_fn_def()
                            && crate::lower::recognition::cell_own_method(self.tcx, f)
                                == Some(crate::lower::recognition::CellMethod::Get) =>
                        {
                            args.first()?.node.place()
                        }
                        _ => None,
                    })?;
                    match got.projection.is_empty() {
                        true => assigned(got.local)?,
                        false => got,
                    }
                }
            };
        }
        let (base, elem) = (place.iter_projections()).rfind(|(_, elem)| matches!(elem, PlaceElem::Field(..)))?;
        let PlaceElem::Field(index, _) = elem else { return None };
        match base.ty(state.body, self.tcx).ty.kind() {
            ty::Adt(adt, _) if adt.is_struct() => Some(adt.non_enum_variant().fields[index].did),
            _ => None,
        }
    }

    /// A closure called, `Fn::call(&f, (a, b))`: in JS, `f(a, b)`, and an
    /// `async` one's, `AsyncFn::async_call`, which gives its promise.
    pub(super) fn mir_closure_call(
        &mut self,
        values: Vec<Value<'tcx>>,
        arg_tys: &[Ty<'tcx>],
        writes: Option<usize>,
        span: Span,
        unbound: Option<(&mut State<'_, 'tcx>, &mut Vec<Stmt>, Ty<'tcx>)>,
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
            crate::lower::calls::left_out(self.tcx, &mut exprs, &tys, &[]);
            // Rest arguments as a value, its last, spread (ADR 0851): what
            // takes them is JS's rest parameter, a closure's or a binding's.
            if tys
                .last()
                .is_some_and(|&ty| crate::lower::bindings::is_rest_value(self.tcx, ty))
            {
                crate::lower::calls::spread_rest(&mut exprs);
            }
        }
        // Not a closure of the crate's: a `dyn Fn`, a generic one or a
        // pointer, which may be JS's (ADR 0330), and read its `this`.
        let own = matches!(
            arg_tys[0].peel_refs().kind(),
            ty::Closure(..) | ty::CoroutineClosure(..)
        );
        if !own {
            crate::lower::calls::given_to_js(&mut exprs);
        }
        // And its options, where the crate's writers take them (ADR 0137).
        if writes.is_some() {
            exprs.extend(self.given_to_writer(None));
        }
        let call = Value::Expr(Expr::call(callee, exprs));
        match unbound {
            Some((state, out, output)) if !own => {
                self.called_unbound(state, (call, Called::Function(arg_tys[1], output)), out)
            }
            _ => Ok(call),
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
            && let Some(op) = crate::lower::recognition::assign_operator(tcx, trait_id)
            && let [place, rhs] = values
            && let ty = arg_tys[0].peel_refs()
            && (self.is_lang_adt(ty, LangItem::String)
                || crate::lower::representation::Num::of(ty).is_some()
                || ty.is_bool())
            && let Value::Ref(place) = place
        {
            // A place reached through a call, `*first(&mut v) += &five`'s
            // `first(v).value`, or `or_insert(10)`'s handle's `value`, read
            // and then written: what holds it named where it's reached,
            // before what's added is, so the call runs once. MIR gives the
            // operator a reborrow, `&mut *x`, a place, of every `&mut`.
            let place = match !super::queries::fixed_place(place) && place.has_effects() {
                true => {
                    self.flush(state, out)?;
                    self.fixed_place_of(place.clone(), out)
                }
                false => place.clone(),
            };
            let place = &place;
            let rhs = self.value_expr(rhs.clone(), span)?;
            let rhs = crate::lower::std_types::number::shift_amount_of(op, rhs, ty, arg_tys[1]);
            let value = if self.is_lang_adt(ty, LangItem::String) && op == BinOp::Add {
                Expr::text_joined(place.clone(), rhs)
            } else if crate::lower::recognition::has_number_arithmetic(ty) || ty.is_bool() {
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
                    nulled: &[],
                    quiet: false,
                    momentary: false,
                    null_for_none: false,
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
    /// names (ADRs 0019, 0020), given its arguments, each whether it's an
    /// `Option` `null` or a value already (`nulled`).
    pub(super) fn mir_binding_call(
        &mut self,
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
        if crate::lower::recognition::is_symbol_for_read(tcx, def_id) {
            let value = values.into_iter().next().expect("the value read");
            return Ok(match self.frame.facts.registered_reads.contains(&span) {
                true => value,
                false => Value::Expr(Expr::call_helper("$symbolFor", vec![self.value_expr(value, span)?])),
            });
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
        Ok(Value::Expr(self.binding_values(
            (def_id, generic_args),
            exprs,
            (js_span, named),
            span,
            out,
        )?))
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
            Value::List(items) => items,
            value => match self.value_expr(value, span)? {
                Expr {
                    kind: js::ExprKind::Array(items),
                    ..
                } => items.into_iter().map(Value::Expr).collect(),
                tuple => {
                    let tuple = match tuple.reads_same() {
                        true => tuple,
                        false => self.spill("spread", tuple, out),
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
                    let made = std::mem::replace(part, Expr::undefined());
                    *part = self.spill("spread", made, out);
                }
            }
        }
        for (part, ty) in parts.into_iter().zip(tys) {
            self.spread_into(Value::Expr(part), ty, span, out, fields)?;
        }
        Ok(())
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
        // A namespace's method called in one of its own on its `self`, its
        // module's: by its name, as on the namespace named (ADR 1037).
        if let Some(receiver) = exprs.first_mut()
            && super::queries::namespace_self(self.tcx, state, def_id, receiver)
        {
            *receiver = Expr::undefined();
        }
        // A callback it calls once, `impl FnOnce(A0, A1) -> T`, an arrow that
        // only passes on what it's given: the function it passes on to,
        // React's `runWithFiberInDEV(fiber, commitCallbacks, queue, instance)`,
        // as a binding's (ADR 0605, amended).
        if exprs.len() == arg_tys.len() {
            for (i, expr) in exprs.iter_mut().enumerate() {
                if let Some(taker) = crate::lower::passed_on::Taker::parameter(self.tcx, def_id, i)
                    && let Some(callee) = crate::lower::passed_on::passed_on(expr, taker, self.forwarding())
                {
                    *expr = callee;
                }
            }
        }
        self.given_iterators(def_id, &mut exprs, arg_tys, span, out)?;
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
        if evidence.is_empty() && exprs.len() == arg_tys.len() && declared.inputs().len() == arg_tys.len() {
            crate::lower::calls::left_out(self.tcx, &mut exprs, declared.inputs(), &[]);
        }
        exprs.extend(evidence);
        // The copy of it, where JS calls it too, that takes what this call
        // gives as it gives it (ADR 0051, amended).
        let copy = self.called_copy((def_id, generic_args), span)?;
        let called = self.crate_called_copy((def_id, copy, Some(span)), exprs);
        // A generic `&mut T`'s box, of an object `T` here: what's in it.
        let called = self.generic_result_values((def_id, generic_args), called, span)?;
        let called = self.fmt_result_value(def_id, generic_args, called);
        // A throwing function's call is a throwing call (ADR 0644).
        Ok(Value::Expr(self.catching(def_id, generic_args, called)))
    }

    /// The call of the crate's function `def_id`, given `args`: of a class's
    /// constructor, `new Component(..)`, and of its method, a call of the
    /// prototype's, `component.setState(..)` (ADR 0761); of an object's
    /// method, its own property's, `scope.containsNode(..)` (ADR 0943); of
    /// any other, `f(..)`.
    pub(in crate::lower) fn crate_called(&self, def_id: rustc_span::def_id::DefId, args: Vec<Expr>) -> Expr {
        self.crate_called_copy((def_id, None, None), args)
    }

    /// `crate_called`'s, of its own JS or of a copy its callers need (ADR
    /// 0051, amended), as the path at `named`, the call's, names it, which
    /// may be through the module that has it (rd114, ADR 1018).
    fn crate_called_copy(
        &self,
        (def_id, copy, named): (rustc_span::def_id::DefId, Option<usize>, Option<Span>),
        mut args: Vec<Expr>,
    ) -> Expr {
        // A namespace's method (ADR 1037): through the value it's called on,
        // `NoopRenderer.createContainer(..)`, one JS may have made of
        // another instance of its module; of the namespace named, the item
        // (`namespace_value`), or `undefined`, its own method's `self` left
        // out, the function itself, `createContainer(..)`.
        if crate::lower::classes::member(self.tcx, def_id) == Some(crate::lower::classes::Member::Namespace)
            && !args.is_empty()
        {
            let receiver = args.remove(0);
            let named = match &receiver.kind {
                js::ExprKind::Undefined => true,
                js::ExprKind::Symbol(_) | js::ExprKind::Var(_) => receiver.facts.has(js::Known::Item),
                _ => false,
            };
            if !named {
                return Expr::call(
                    Expr::member(receiver, crate::lower::classes::property(self.tcx, def_id)),
                    args,
                );
            }
        }
        let callee = match (copy, named) {
            (_, Some(named)) => self.named_fn_ref(def_id, copy, named),
            (Some(_), None) => self.own_fn_ref(def_id, copy),
            (None, None) => self.fn_ref(def_id),
        };
        match (crate::lower::classes::member(self.tcx, def_id), callee.kind) {
            (Some(crate::lower::classes::Member::Constructor), js::ExprKind::Member(class, _)) => {
                Expr::new_(*class, args)
            }
            (Some(crate::lower::classes::Member::Method), js::ExprKind::Member(_, name)) if !args.is_empty() => {
                let this = args.remove(0);
                Expr::call(Expr::member(this, name), args)
            }
            // A getter's, its read, `position.leftDiagonal` (#357).
            (Some(crate::lower::classes::Member::Getter), js::ExprKind::Member(_, name)) if args.len() == 1 => {
                Expr::member(args.remove(0), name)
            }
            // An object's method, its own property's (ADR 0943).
            (Some(crate::lower::classes::Member::Own), _) if !args.is_empty() => {
                let this = args.remove(0);
                Expr::call(
                    Expr::member(this, crate::lower::classes::property(self.tcx, def_id)),
                    args,
                )
            }
            (_, kind) => Expr::call(Expr { kind, ..callee }, args),
        }
    }

    /// Whether a variable holding the crate's function `def_id`, of `count`
    /// parameters, is called as that variable: one given no dictionaries,
    /// boxes or rest arguments (ADR 0664). And `js::SymbolFor`, which is
    /// `Symbol.for` (ADR 0964).
    pub(super) fn called_as_held(&self, def_id: rustc_span::def_id::DefId, count: usize) -> bool {
        if crate::lower::recognition::is_symbol_for_constructor(self.tcx, def_id) {
            return true;
        }
        self.krate.fns.contains_key(&def_id)
            && !self.takes_rest(def_id)
            && self.tcx.generics_of(def_id).count() == 0
            && (0..count).all(|i| !self.param_is_box(def_id, i))
    }

    /// What binding `def_id` is given as its `i`th argument, `value` of
    /// `ty`, as JS takes it: of its call and of its arrow as a value alike
    /// (ADR 0604); `nulled` where it's an `Option` `null` or a value
    /// already, a `Some(..)` (`nulled`'s).
    pub(in crate::lower) fn binding_arg(
        &self,
        def_id: rustc_span::def_id::DefId,
        (i, nulled): (usize, bool),
        value: Expr,
        ty: Ty<'tcx>,
    ) -> Expr {
        let tcx = self.tcx;
        let named = |names: Vec<rustc_span::Symbol>| {
            (tcx.fn_arg_idents(def_id).get(i).copied().flatten()).is_some_and(|ident| names.contains(&ident.name))
        };
        let given = match () {
            // A `()` is `undefined`, given as it is, or a list of none,
            // `[]`, where it's a list's (ADR 0775).
            () if ty.is_unit() => {
                crate::lower::representation::unit_given(tcx, crate::lower::representation::UnitTo::Param(def_id, i))
                    .unwrap_or(value)
            }
            // A parameter it names `#[rust_js::nullable(..)]`: its
            // `None` is `null`, and a `Some(..)` its value (ADR 0275); of a
            // type parameter, where it's given an `Option` (ADR 0586,
            // amended); a `Some` of a `&dyn Any` its value, as below (rd170).
            () if bindings::takes_nullable(tcx, def_id, i)
                && self
                    .option_of(ty)
                    .is_some_and(|payload| bindings::holds_none_as_null(tcx, payload)) =>
            {
                js::Storage::NullForNone.written(self.any_given_to_js(value, ty), nulled)
            }
            // Entries written where they're given of such a value, an
            // `Option` of what never looks like `None`: each `null` for
            // `None`, `new Map([[key, null]])`, as `set` writes it (ADR
            // 0884, amended); `bindings::reject_nullable_held` refuses
            // entries made before.
            () if bindings::takes_nullable_entries(tcx, def_id, i) && self.entries_of_options(ty) => {
                self.nulled_entries(self.any_given_to_js(value, ty))
            }
            // A parameter it names `#[rust_js::lazy(..)]`, a function JS
            // calls once for the value: a closure whose body does nothing
            // and gives no function is its value, `useState(0)` (ADR 0593).
            () if named(bindings::lazy_params(tcx, def_id)) => self.lazy_value(value, ty),
            // A `&dyn Any` given to JS is its value (ADR 0331).
            () => self.any_given_to_js(value, ty),
        };
        // A callback JS calls once that only passes on what it's given is
        // the function it passes on to, `p.then(ping)` (ADR 0262's
        // exception).
        match crate::lower::passed_on::Taker::parameter(tcx, def_id, i) {
            Some(taker) => crate::lower::passed_on::passed_on(&given, taker, self.forwarding()).unwrap_or(given),
            None => given,
        }
    }

    /// Whether `ty`, entries given a binding, holds pairs whose value is an
    /// `Option` of what never looks like `None`.
    fn entries_of_options(&self, ty: Ty<'tcx>) -> bool {
        ty.walk().any(|part| {
            part.as_type().is_some_and(|t| match t.kind() {
                ty::Tuple(items) if items.len() == 2 => self.option_of(items[1]).is_some_and(|payload| {
                    !self.boxed_payload(payload) && crate::lower::bindings::holds_none_as_null(self.tcx, payload)
                }),
                _ => false,
            })
        })
    }

    /// Entries written where they're given, `[[key, value], ..]`, each
    /// value `null` for `None`; `undefined`, of none, as it is. Any other
    /// is refused before (`bindings::reject_nullable_held`).
    fn nulled_entries(&self, entries: Expr) -> Expr {
        match entries.kind {
            js::ExprKind::Array(items) => {
                let items = (items.into_iter())
                    .map(|item| match item.kind {
                        js::ExprKind::Array(mut pair) if pair.len() == 2 => {
                            let value = pair.pop().expect("a pair's value");
                            pair.push(js::Storage::NullForNone.written(value, false));
                            Expr {
                                kind: js::ExprKind::Array(pair),
                                ..item
                            }
                        }
                        kind => Expr { kind, ..item },
                    })
                    .collect();
                Expr {
                    kind: js::ExprKind::Array(items),
                    ..entries
                }
            }
            js::ExprKind::Undefined => entries,
            _ => {
                self.tcx.dcx().err("rust-js: entries of `Option` values, which a collection holds as `null`, not written where they're given (ADR 0884, amended)");
                entries
            }
        }
    }

    /// `js::is_defined(binding)`: `typeof MessageChannel !== "undefined"`,
    /// of the global its argument names (ADR 0597), and `js::is_function`'s
    /// `typeof setTimeout === "function"` of the same (ADR 0942): a binding
    /// function's, by its type, or a static's, `webapi::performance`, by the
    /// static it is, so no temporary between hides it. A global's own name,
    /// its `js_path`: not an import's, which isn't the host's to lack.
    /// And `js::is_function` of a value JS gave, `typeof callback ===
    /// "function"` (`asks_of_value`); and `js::method`'s read of a path,
    /// its member (`mir_method`). `None` of another call.
    pub(super) fn mir_defined(
        &mut self,
        state: &mut State<'_, 'tcx>,
        def_id: rustc_span::def_id::DefId,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let tcx = self.tcx;
        let body = state.body;
        if !bindings::is_binding(tcx, def_id) {
            return Ok(None);
        }
        // A path's method read as a value, `js::method`'s (rd181).
        if let Some(method) = self.mir_method(state, def_id, args, span, out)? {
            return Ok(Some(method));
        }
        let bindings::JsForm::Asks(asked) = bindings::js_form(tcx, def_id) else {
            return Ok(None);
        };
        if self.asks_of_value(body, def_id, args) {
            let mut values = self.take_operands(state, &[&args[0].node], out)?;
            let value = self.value_expr(values.remove(0), span)?;
            let typeof_ = Expr::unary(js::UnaryOp::Typeof, value);
            return Ok(Some(
                Expr::bin(Op::Eq, typeof_, Expr::str("function")).or_at(self.js_span(span)),
            ));
        }
        // A path of bindings, each read from the one before (ADR 0624); or
        // from a value, its first, `(selection, Selection::extend)`'s
        // `typeof selection.extend === "function"` (ADR 0942, amended).
        if let Some(arg) = args.first()
            && let ty::Tuple(steps) = *arg.node.ty(&body.local_decls, tcx).kind()
            && !steps.is_empty()
        {
            let steps = &self.steps_named(body, &arg.node, steps)[..];
            if let Some(value_ty) = value_first(steps) {
                let (path, read) = self.defined_path(&steps[1..], Some(value_ty), asked.function(), span)?;
                self.asked_of_number(asked, read, span)?;
                let value = self.path_value(state, &arg.node, asked.function(), (span, out))?;
                return self.mir_defined_parts(Some(path), Some(value), asked, span);
            }
            let (path, read) = self.defined_path(steps, None, asked.function(), span)?;
            self.asked_of_number(asked, read, span)?;
            return self.mir_defined_parts(Some(path), None, asked, span);
        }
        let Some(arg) = args.first() else {
            return self.mir_defined_parts(None, None, asked, span);
        };
        let given = arg.node.ty(&body.local_decls, tcx);
        // A std function that's `Math`'s, its function's, `typeof
        // Math.clz32 !== "undefined"` (ADR 0887).
        if let Some((f, f_args)) = crate::lower::fn_def(given)
            && let Some(function) = self.math_function(f, f_args)
            && let js::ExprKind::Member(_, name) = function.kind
        {
            self.asked_of_number(asked, given, span)?;
            return self.mir_defined_parts(Some(format!("Math.{name}")), None, asked, span);
        }
        let global = match *given.kind() {
            // `js::SymbolFor`, `Symbol.for`, which a host may lack (ADR 0964).
            ty::FnDef(f, _) if crate::lower::recognition::is_symbol_for_constructor(tcx, f) => {
                Some("Symbol.for".to_string())
            }
            // A binding's class, `js::class::<Promise<()>>`, the global
            // class TypeScript names, `typeof Promise === "function"` (rd235).
            ty::FnDef(f, f_args)
                if bindings::is_binding(tcx, f) && matches!(bindings::js_form(tcx, f), bindings::JsForm::Class) =>
            {
                f_args
                    .skip_binder()
                    .types()
                    .next()
                    .and_then(|ty| self.recognition().class_value(ty))
            }
            ty::FnDef(f, _) if bindings::is_binding(tcx, f) && bindings::is_method(tcx, f) => self.member_of_global(f),
            ty::FnDef(f, _) if bindings::is_binding(tcx, f) => bindings::js_path(tcx, f),
            _ => match self.static_given(body, &arg.node).filter(|&s| tcx.is_foreign_item(s)) {
                // A static is given as itself, as a function binding is,
                // `is_defined(js::symbol::to_string_tag)`; a reference to
                // it is refused, naming the one way (ADR 0645).
                Some(s) if refs(given) > refs(tcx.type_of(s).instantiate_identity().skip_normalization()) => {
                    let function = asked.function();
                    return Err(self.tcx.dcx().span_err(
                        span,
                        format!(
                            "`{function}` of a static is given the static itself, \
                             `{function}(webapi::performance)`, not a reference to it"
                        ),
                    ));
                }
                Some(s) => bindings::js_path(tcx, s),
                None => None,
            },
        };
        if global.is_some() {
            let read = match crate::lower::fn_def(given) {
                Some((f, args)) => read_by(tcx, f, args, given),
                None => given,
            };
            self.asked_of_number(asked, read, span)?;
        }
        self.mir_defined_parts(global, None, asked, span)
    }

    /// `js::method`'s method a path from a value names, read as a value,
    /// the JS function itself: `(element, HTMLElement::focus)`'s
    /// `element.focus`, its value read as it's read in the path, each step
    /// a member (rd181). As its call reads it, a getter may answer it of a
    /// value of any shape's type (ADR 0750). `None` of another call.
    fn mir_method(
        &mut self,
        state: &mut State<'_, 'tcx>,
        def_id: rustc_span::def_id::DefId,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let tcx = self.tcx;
        let body = state.body;
        if !bindings::is_binding(tcx, def_id) || !matches!(bindings::js_form(tcx, def_id), bindings::JsForm::Method) {
            return Ok(None);
        }
        let refused = |cx: &Self| {
            Err(cx.unsupported(
                span,
                "`js::method` of what isn't a path of a value and its method, `(element, HTMLElement::focus)`,",
            ))
        };
        let [arg] = args else { return refused(self) };
        let ty::Tuple(steps) = *arg.node.ty(&body.local_decls, tcx).kind() else {
            return refused(self);
        };
        let steps = self.steps_named(body, &arg.node, steps);
        let Some(value_ty) = value_first(&steps) else {
            return refused(self);
        };
        let (path, _) = self.defined_path(&steps[1..], Some(value_ty), "js::method", span)?;
        let value = self.path_value(state, &arg.node, "js::method", (span, out))?;
        let getters = (steps[1..].iter()).any(|&step| {
            crate::lower::fn_def(step)
                .is_some_and(|(f, _)| crate::lower::binding_calls::method_may_be_a_getters(tcx, f))
        });
        let method = path.split('.').fold(value, Expr::member);
        let method = match getters {
            true => method.answered_by_a_getter(),
            false => method,
        };
        Ok(Some(method.or_at(self.js_span(span))))
    }

    /// The value a path from a value starts at, the tuple's first item, as
    /// it's read where the path is: of the tuple written in the call; or of
    /// one held before it, `const _15 = [element, ..]`, which a value with a
    /// drop live makes, as the call may unwind past it (rd233), a variable
    /// the program names read again, nothing run between.
    fn path_value(
        &mut self,
        state: &mut State<'_, 'tcx>,
        tuple: &Operand<'tcx>,
        function: &str,
        (span, out): (Span, &mut Vec<Stmt>),
    ) -> R<Expr> {
        let mut values = self.take_operands(state, &[tuple], out)?;
        let refused =
            |cx: &Self| cx.unsupported(span, &format!("`{function}` of a path whose value isn't written in it"));
        match self.value_expr(values.remove(0), span)?.kind {
            js::ExprKind::Array(mut items) => Ok(items.swap_remove(0)),
            js::ExprKind::Var(_) => {
                let body = state.body;
                let first = tuple.place().and_then(|place| place.as_local()).and_then(|local| {
                    (body.basic_blocks.iter())
                        .flat_map(|block| &block.statements)
                        .find_map(|s| match &s.kind {
                            StatementKind::Assign(assign) if assign.0.as_local() == Some(local) => match &assign.1 {
                                Rvalue::Aggregate(kind, items) if matches!(**kind, mir::AggregateKind::Tuple) => {
                                    items.iter().next().cloned()
                                }
                                _ => None,
                            },
                            _ => None,
                        })
                });
                // Through the temporaries MIR makes of it, a copy or a
                // reborrow, `_8 = &(*_2)`, to the variable.
                let assigned = |local: mir::Local| {
                    (body.basic_blocks.iter())
                        .flat_map(|block| &block.statements)
                        .find_map(|s| match &s.kind {
                            StatementKind::Assign(assign) if assign.0.as_local() == Some(local) => {
                                Some(assign.1.clone())
                            }
                            _ => None,
                        })
                };
                let mut at = match first {
                    Some(Operand::Copy(place) | Operand::Move(place)) => place,
                    _ => return Err(refused(self)),
                };
                for _ in 0..4 {
                    if at.projection.is_empty() && body.local_decls[at.local].is_user_variable() {
                        let mut values = self.take_operands(state, &[&Operand::Copy(at)], out)?;
                        return self.value_expr(values.remove(0), span);
                    }
                    at = match at.as_local().and_then(assigned) {
                        Some(Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _)) => of,
                        Some(Rvalue::Ref(_, _, of)) | Some(Rvalue::CopyForDeref(of))
                            if matches!(of.projection[..], [mir::ProjectionElem::Deref]) =>
                        {
                            mir::Place::from(of.local)
                        }
                        _ => return Err(refused(self)),
                    };
                }
                Err(refused(self))
            }
            _ => Err(refused(self)),
        }
    }

    /// The type of each step of a path `tuple` is, a binding's function's;
    /// of one cast to a `fn` pointer, the function it's cast from, read of
    /// the tuple MIR builds: `CSSStyleDeclaration::set_property as fn(_, _,
    /// &'static str, _)`, whose `impl IntoUnknown` no turbofish names
    /// (rd154).
    fn steps_named(&self, body: &mir::Body<'tcx>, tuple: &Operand<'tcx>, steps: &[Ty<'tcx>]) -> Vec<Ty<'tcx>> {
        let assigned = |local: mir::Local| {
            let mut found = (body.basic_blocks.iter())
                .flat_map(|block| &block.statements)
                .filter_map(|s| match &s.kind {
                    StatementKind::Assign(assign) if assign.0.as_local() == Some(local) => Some(&assign.1),
                    _ => None,
                });
            match (found.next(), found.next()) {
                (Some(rvalue), None) => Some(rvalue),
                _ => None,
            }
        };
        let items = match tuple.place().and_then(|place| place.as_local()).and_then(assigned) {
            Some(Rvalue::Aggregate(kind, items)) if matches!(**kind, mir::AggregateKind::Tuple) => Some(items),
            _ => None,
        };
        // Through the copies MIR makes of the cast, `_5 = copy _6`.
        let cast_from = |k: usize| {
            let mut item = items?.get(rustc_abi::FieldIdx::from_usize(k))?;
            loop {
                match assigned(item.place()?.as_local()?)? {
                    Rvalue::Use(copied, _) => item = copied,
                    Rvalue::Cast(
                        mir::CastKind::PointerCoercion(ty::adjustment::PointerCoercion::ReifyFnPointer(_), _),
                        from,
                        _,
                    ) => {
                        let from = from.ty(&body.local_decls, self.tcx);
                        return crate::lower::fn_def(from).map(|_| from);
                    }
                    _ => return None,
                }
            }
        };
        (steps.iter().enumerate())
            .map(|(k, &step)| match step.kind() {
                ty::FnPtr(..) => cast_from(k).unwrap_or(step),
                _ => step,
            })
            .collect()
    }

    /// `js::is_number` of what's a number in Rust, `f64`, or an `Option`
    /// of one, as a getter's or a static's is (rd195); `js::is_object` of
    /// what's no number, text, `bool` or function (rd282): of what isn't,
    /// a function's binding, it would be false, so it's refused.
    fn asked_of_number(&self, asked: Asked, read: Ty<'tcx>, span: Span) -> R<()> {
        let number = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Float(_) | ty::Int(_) | ty::Uint(_));
        let read = read.peel_refs();
        let payload = self.option_of(read).map(|payload| payload.peel_refs());
        let (asks, of) = match asked {
            Asked::Number => (number(read) || payload.is_some_and(number), "a number"),
            Asked::Object => {
                let recognition = self.recognition();
                let object = |ty: Ty<'tcx>| {
                    !number(ty)
                        && !ty.is_bool()
                        && !recognition.is_string_like(ty)
                        && !matches!(
                            ty.kind(),
                            ty::FnDef(..) | ty::FnPtr(..) | ty::Closure(..) | ty::Dynamic(..)
                        )
                };
                (object(payload.unwrap_or(read)), "an object")
            }
            Asked::Defined | Asked::Function => return Ok(()),
        };
        if asks {
            return Ok(());
        }
        let function = asked.function();
        Err(self.tcx.dcx().span_err(
            span,
            format!(
                "`{function}` of what isn't {of} in Rust is false: it's asked of a getter's or a \
                 static's binding of one, `js::is_number(webapi::window_global::device_pixel_ratio)`, \
                 `js::is_object(webapi::window_global::navigation)`"
            ),
        ))
    }

    /// Whether a call of `js::is_defined` or `js::is_function`, binding
    /// `def_id`, given `args`, reads a value it's given, of a function's
    /// type or a path's first (`value_first`), which it reads, where a
    /// global's argument isn't (`unread_defined_operands`).
    pub(super) fn reads_a_value(
        &self,
        body: &mir::Body<'tcx>,
        def_id: rustc_span::def_id::DefId,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
    ) -> bool {
        let path_from_value = (args.first()).is_some_and(|arg| {
            matches!(*arg.node.ty(&body.local_decls, self.tcx).kind(), ty::Tuple(steps) if value_first(steps).is_some())
        });
        path_from_value || self.asks_of_value(body, def_id, args)
    }

    /// Whether a call of binding `def_id` given `args` is `js::is_function`
    /// of a value, not of a global: one argument, of a type whose value is
    /// a function JS may give, which JS may give as anything else, React's
    /// SchedulerMock's task `callback` (`function_given`); not a static,
    /// which is asked as the global it is (ADR 0645). Its argument is read,
    /// where a global's isn't (`unread_defined_operands`).
    pub(super) fn asks_of_value(
        &self,
        body: &mir::Body<'tcx>,
        def_id: rustc_span::def_id::DefId,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
    ) -> bool {
        let tcx = self.tcx;
        let [arg] = args else { return false };
        matches!(bindings::js_form(tcx, def_id), bindings::JsForm::Asks(Asked::Function))
            && self.function_given(arg.node.ty(&body.local_decls, tcx))
            && self
                .static_given(body, &arg.node)
                .filter(|&s| tcx.is_foreign_item(s))
                .is_none()
    }

    /// Whether `ty` is a function a JS caller may give, so may give as
    /// anything else: a `fn`, a `dyn Fn` lent or boxed, a type parameter's
    /// `impl Fn`, a transparent struct of one, or an `Option` of one. Not a
    /// closure or a function item, which only Rust makes.
    fn function_given(&self, ty: ty::Ty<'tcx>) -> bool {
        let tcx = self.tcx;
        let ty = ty.peel_refs();
        if let Some(payload) = self.option_of(ty) {
            return self.option_of(payload.peel_refs()).is_none() && self.function_given(payload);
        }
        match *ty.kind() {
            ty::Adt(adt, args) if adt.is_box() => self.function_given(args.type_at(0)),
            ty::Adt(adt, args) => crate::lower::recognition::transparent_field(tcx, adt).is_some_and(|i| {
                self.function_given(
                    adt.non_enum_variant().fields[rustc_abi::FieldIdx::from_usize(i)]
                        .ty(tcx, args)
                        .skip_normalization(),
                )
            }),
            ty::Closure(..) | ty::FnDef(..) => false,
            _ => self.is_callable(ty),
        }
    }

    /// `typeof` of each part of `global`'s path, joined by `&&`: of
    /// `js::is_defined`, each whether the host has it; of `js::is_function`,
    /// the last alone, whether it's a function, as upstream's `typeof
    /// performance.getEntriesByType === 'function'` asks, which throws where
    /// the host hasn't `performance` (ADR 0942, amended).
    /// Of a path from a value, `base`, each part is a member of it, the
    /// value itself no global, so not asked of.
    fn mir_defined_parts(
        &self,
        global: Option<String>,
        base: Option<Expr>,
        asked: Asked,
        span: Span,
    ) -> R<Option<Expr>> {
        // Each part of its path, `typeof console !== "undefined" && typeof
        // console.log !== "undefined"`: a host may lack any of them.
        // Its first a name, each after it a name or a quoted key,
        // `window['$RT']` (ADR 1033).
        let parts = global.as_deref().and_then(|path| {
            let first = bindings::first_name(path);
            let steps = bindings::steps(&path[first.len()..]).ok()?;
            crate::names::is_identifier(first).then(|| {
                std::iter::once(bindings::Step::Named(first))
                    .chain(steps)
                    .collect::<Vec<_>>()
            })
        });
        let Some(parts) = parts else {
            return Err(self.unsupported(
                span,
                &format!(
                    "`{}` of what isn't a JS global's binding, `webapi::message_channel::new`,",
                    asked.function()
                ),
            ));
        };
        let from_value = base.is_some();
        let first = global.as_deref().map(bindings::first_name).unwrap_or_default();
        let mut read = base.unwrap_or_else(|| self.js_ref(first));
        let mut tests = Vec::new();
        for (k, &part) in parts.iter().enumerate() {
            if k > 0 || from_value {
                read = part.of(read);
            }
            let typeof_ = Expr::unary(js::UnaryOp::Typeof, read.clone());
            if let Some(kind) = asked.type_of() {
                if k + 1 == parts.len() {
                    tests.push(Expr::bin(Op::Eq, typeof_, Expr::str(kind)));
                }
            } else if k + 1 == parts.len() || from_value || !(k == 0 && every_host_has(first)) {
                tests.push(Expr::bin(Op::Ne, typeof_, Expr::str("undefined")));
            }
        }
        // Where the call's written, a comment before it is before it
        // (ADR 0444).
        let at = self.js_span(span);
        Ok((tests.into_iter())
            .map(|test| test.or_at(at))
            .reduce(|a, b| Expr::bin(Op::And, a, b).or_at(at)))
    }

    /// The path a tuple of bindings is, each step a method of what the one
    /// before gives, its first a global's binding (ADR 0624):
    /// `(Window::document, Document::create_element::<Div>)` is
    /// `window.document.createElement`; and the type of what its last
    /// step reads (`read_by`).
    fn defined_path(
        &self,
        steps: &[ty::Ty<'tcx>],
        from: Option<ty::Ty<'tcx>>,
        function: &str,
        span: Span,
    ) -> R<(String, ty::Ty<'tcx>)> {
        let tcx = self.tcx;
        let refused = || {
            self.unsupported(
                span,
                &format!("`{function}` of a path of bindings where a step isn't a method of what the one before gives"),
            )
        };
        if from.is_some() && steps.is_empty() {
            return Err(refused());
        }
        let mut path = String::new();
        let mut gives: Option<ty::Ty<'tcx>> = from;
        let mut read = None;
        for (k, step) in steps.iter().enumerate() {
            let Some((f, args)) = crate::lower::fn_def(*step) else {
                return Err(refused());
            };
            // A constructor is what's asked, the last step: what it makes
            // isn't read of it (ADR 0891).
            let constructs = matches!(bindings::js_form(tcx, f), bindings::JsForm::New(_));
            if !bindings::is_binding(tcx, f) || (constructs && k + 1 < steps.len()) {
                return Err(refused());
            }
            let sig = tcx.fn_sig(f).instantiate(tcx, args).skip_normalization().skip_binder();
            path = match gives {
                None if bindings::is_method(tcx, f) => self.member_of_global(f).ok_or_else(refused)?,
                None => bindings::js_path(tcx, f).ok_or_else(refused)?,
                Some(before) => {
                    let of = sig.inputs().first().map(|this| this.peel_refs());
                    let Some(name) = member_read(tcx, f) else {
                        return Err(refused());
                    };
                    // Of any value, a method of any type, as TypeScript
                    // reads `any`'s: `HTMLElement.prototype.focus` (rd181).
                    let any = matches!(before.peel_refs().kind(), ty::Adt(adt, _) if crate::lower::declarations::any_shape(tcx, adt.did()));
                    // Or of a class it extends, as its `Deref` says,
                    // `CSSStyleProperties`' `CSSStyleDeclaration::set_property`
                    // (rd232).
                    let extends = match (before.peel_refs().kind(), of.map(|of| of.kind())) {
                        (ty::Adt(value, _), Some(ty::Adt(owner, _))) => {
                            self.recognition().superclasses(value.did()).contains(&owner.did())
                        }
                        _ => false,
                    };
                    if !bindings::is_method(tcx, f) || (of != Some(before.peel_refs()) && !any && !extends) {
                        return Err(refused());
                    }
                    match path.is_empty() {
                        true => name.to_string(),
                        false => format!("{path}.{name}"),
                    }
                }
            };
            gives = Some(sig.output());
            read = Some(read_by(tcx, f, args, *step));
        }
        Ok((path, read.ok_or_else(refused)?))
    }

    /// `performance.now` of `Performance::now`, a method binding of a type
    /// one global holds, the one static of its crate of that type (ADR
    /// 0597): its path, the global's and the method's JS name, or the
    /// constructor's it makes, `window.ErrorEvent` (ADR 0891).
    fn member_of_global(&self, method: rustc_span::def_id::DefId) -> Option<String> {
        let tcx = self.tcx;
        let name = member_read(tcx, method)?;
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
            Operand::Copy(place) | Operand::Move(place) => match place.projection.as_slice() {
                [] => place.local,
                // The static's value, read through its address (ADR 0645).
                [PlaceElem::Deref] => return self.static_given(body, &Operand::Copy(Place::from(place.local))),
                _ => return None,
            },
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

/// What a method binding reads of its receiver, which `js::is_defined`
/// asks of: its method's or its property's name, or the path of the
/// constructor it calls `new` of, `ErrorEvent` of `window.ErrorEvent`
/// (ADR 0891). `None` of another form.
fn member_read(tcx: rustc_middle::ty::TyCtxt<'_>, method: rustc_span::def_id::DefId) -> Option<String> {
    match bindings::js_form(tcx, method) {
        bindings::JsForm::Call(name) | bindings::JsForm::Get(name) | bindings::JsForm::New(name) => Some(name),
        _ => None,
    }
}

/// What `binding` (`f` of `args`, of type `step`) reads where it's
/// asked of: a getter's value, its output's type; another's the function
/// itself, its own.
fn read_by<'tcx>(
    tcx: rustc_middle::ty::TyCtxt<'tcx>,
    f: rustc_span::def_id::DefId,
    args: rustc_middle::ty::GenericArgsRef<'tcx>,
    step: ty::Ty<'tcx>,
) -> ty::Ty<'tcx> {
    match bindings::js_form(tcx, f) {
        bindings::JsForm::Get(_) => tcx
            .fn_sig(f)
            .instantiate(tcx, args)
            .skip_normalization()
            .skip_binder()
            .output(),
        _ => step,
    }
}

/// The value a path of `steps` starts from, its first, where that's no
/// binding, `(selection, Selection::extend)`'s `selection` (ADR 0942,
/// amended): `None` of a path of bindings.
fn value_first<'tcx>(steps: &[ty::Ty<'tcx>]) -> Option<ty::Ty<'tcx>> {
    let first = *steps.first()?;
    (steps.len() > 1 && crate::lower::fn_def(first).is_none()).then_some(first)
}

/// Whether every JS host has `global`, one of ECMAScript's first edition's
/// global objects, which a path through it isn't asked of: `typeof
/// Object.preventExtensions === "function"`, as `Math.clz32` is (ADRs 0887,
/// 0942). A later edition's, `Symbol`, `Reflect`, is asked of.
fn every_host_has(global: &str) -> bool {
    matches!(
        global,
        "Object" | "Function" | "Array" | "String" | "Boolean" | "Number" | "Math" | "Date"
    )
}

/// How many references `ty` is of what it refers to: `&&Symbol`'s two.
fn refs(ty: Ty<'_>) -> usize {
    match *ty.kind() {
        ty::Ref(_, inner, _) => 1 + refs(inner),
        _ => 0,
    }
}
