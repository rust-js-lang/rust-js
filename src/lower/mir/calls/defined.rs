//! `js::is_defined` and `js::is_function` (ADRs 0597, 0942): whether the
//! host has a global, a binding's path or member, or a value JS gave is a
//! function, `typeof MessageChannel !== "undefined"`; and the path of
//! bindings both they and `js::method` read.

use super::*;
use crate::lower::bindings::Asked;
use crate::lower::mir::locals::user_variable;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `js::is_defined(binding)`: `typeof MessageChannel !== "undefined"`,
    /// of the global its argument names (ADR 0597), and `js::is_function`'s
    /// `typeof setTimeout === "function"` of the same (ADR 0942): a binding
    /// function's, by its type, or a static's, `webapi::performance`, by the
    /// static it is, so no temporary between hides it. A global's own name,
    /// its `js_path`: not an import's, which isn't the host's to lack.
    /// And `js::is_function` of a value JS gave, `typeof callback ===
    /// "function"` (`asks_of_value`); and `js::method`'s read of a path,
    /// its member (`mir_method`). `None` of another call.
    pub(in crate::lower::mir) fn mir_defined(
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

    /// The value a path from a value starts at, the tuple's first item, as
    /// it's read where the path is: of the tuple written in the call; or of
    /// one held before it, `const _15 = [element, ..]`, which a value with a
    /// drop live makes, as the call may unwind past it (rd233), a variable
    /// the program names read again, nothing run between.
    pub(super) fn path_value(
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
                    if at.projection.is_empty() && user_variable(&body.local_decls[at.local]) {
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
    pub(super) fn steps_named(
        &self,
        body: &mir::Body<'tcx>,
        tuple: &Operand<'tcx>,
        steps: &[Ty<'tcx>],
    ) -> Vec<Ty<'tcx>> {
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
    pub(in crate::lower::mir) fn reads_a_value(
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
    pub(in crate::lower::mir) fn asks_of_value(
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
    /// `is_number` or `is_object`, each before the last whether the host has
    /// it and the last whether it's of that kind, so a host without
    /// `performance` gives `false`, never a `ReferenceError` (rd320, ADR
    /// 0942 amended); a test of the global the program wrote before it,
    /// `typeof performance === "object" &&`, says the first already
    /// (`prepare::typeofs`).
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
            match asked.type_of() {
                Some(kind) if k + 1 == parts.len() => tests.push(Expr::bin(Op::Eq, typeof_, Expr::str(kind))),
                _ if k + 1 == parts.len() || from_value || !(k == 0 && every_host_has(first)) => {
                    tests.push(Expr::bin(Op::Ne, typeof_, Expr::str("undefined")));
                }
                _ => {}
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
    pub(super) fn defined_path(
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
pub(super) fn value_first<'tcx>(steps: &[ty::Ty<'tcx>]) -> Option<ty::Ty<'tcx>> {
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
