//! `js::method`'s read of a path from a value (rd181, ADR 1025): a
//! binding's method, `element.focus`, or a member by its key of a value of
//! any shape, `listener.handleEvent`, read only as its value's method
//! (rd334, ADR 1025 amended).

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `js::method`'s method a path from a value names, read as a value,
    /// the JS function itself: `(element, HTMLElement::focus)`'s
    /// `element.focus`, its value read as it's read in the path, each step
    /// a member (rd181). As its call reads it, a getter may answer it of a
    /// value of any shape's type (ADR 0750). `None` of another call.
    pub(super) fn mir_method(
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
        // A key's, a member of a value of any shape, called as its method
        // alone, as it may be no function (rd334).
        if let [_, key] = steps[..]
            && matches!(key.kind(), ty::Ref(_, key, _) if !key.is_fn())
        {
            return self.mir_keyed_method(state, &arg.node, span, out).map(Some);
        }
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

    /// `js::method` of a value of any shape and a key, the member it
    /// names, `listener.handleEvent`, as `js::get` reads it, which a getter
    /// may answer: only the callee of a `function::call` whose `this` is
    /// the value, the method's call (`method_calls`, rd334). Its type is a
    /// function's, where JS's value may be anything, so it's refused
    /// anywhere else.
    fn mir_keyed_method(
        &mut self,
        state: &mut State<'_, 'tcx>,
        tuple: &Operand<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let called = |at| self.frame.facts.called_methods.contains(&at);
        if !crate::lower::body_queries::each_call_at(self.frame.thir, span, called) {
            let why = "a member by its key may be no function, so it's only called as its value's method, \
                 `function::call(js::method((value, key)), Some(value), args)`, the value a variable; \
                 `js::get(value, key)` reads it as a value, and `js::classify`'s `Kind::Function` holds one that's a function";
            let refused = crate::lower::refusal::Refused::Because("`js::method` of a key read otherwise", why);
            return Err(crate::lower::refusal::refuse(self.tcx, span, refused));
        }
        let mut values = self.take_operands(state, &[tuple], out)?;
        match self.value_expr(values.remove(0), span)?.kind {
            js::ExprKind::Array(items) if items.len() == 2 => {
                let [value, key] = <[Expr; 2]>::try_from(items).unwrap_or_else(|_| unreachable!("two items"));
                Ok(crate::lower::binding_calls::keyed(value, key)
                    .answered_by_a_getter()
                    .keyed_method()
                    .or_at(self.js_span(span)))
            }
            _ => Err(self.unsupported(span, "`js::method` of a key whose path isn't written in it")),
        }
    }

    /// `js::optional_call`'s `value.method?.(args)` (#1859, ADR 1156): the
    /// path's method read once, called with its value as its `this` where
    /// it's there, `hop.closest?.(selector)`. Rust evaluates the value, then
    /// the arguments, before the call, so each that runs code is held first,
    /// in that order, `const log = node(); const arg = f(); log.closest?.(arg)`.
    /// What it gives is `Option<R>`: of an `R` that can't look like `None`,
    /// the call itself; of one whose `None` is `null`, a binding's `T |
    /// null`, its `Some(None)` boxed, `$someDefined(..)`, which `flatten()`
    /// takes off again (ADR 0950, amended); of one that may be `undefined`,
    /// refused, as `?.` gives that of a method that isn't there too.
    pub(super) fn mir_optional_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        def_id: rustc_span::def_id::DefId,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let tcx = self.tcx;
        let body = state.body;
        if !bindings::is_binding(tcx, def_id)
            || !matches!(bindings::js_form(tcx, def_id), bindings::JsForm::OptionalCall)
        {
            return Ok(None);
        }
        let refused = |cx: &Self, what: &str| Err(cx.unsupported(span, what));
        let [path, given] = args else {
            return refused(self, "`js::optional_call` of what isn't a path and its arguments");
        };
        let ty::Tuple(steps) = *path.node.ty(&body.local_decls, tcx).kind() else {
            return refused(
                self,
                "`js::optional_call` of what isn't a path of a value and its method, `(hop, Element::closest)`,",
            );
        };
        let steps = self.steps_named(body, &path.node, steps);
        let Some(value_ty) = value_first(&steps) else {
            return refused(
                self,
                "`js::optional_call` of what isn't a path of a value and its method, `(hop, Element::closest)`,",
            );
        };
        let Some((method, method_args)) = steps.last().and_then(|&step| crate::lower::fn_def(step)) else {
            return refused(
                self,
                "`js::optional_call` of a member by its key, which `function::call(js::method((value, key)), ..)` calls,",
            );
        };
        // A method JS calls, not a getter's or another form's.
        if !matches!(bindings::js_form(tcx, method), bindings::JsForm::Call(_)) || !bindings::is_method(tcx, method) {
            let what = format!(
                "rust-js: `js::optional_call` calls a method where it's there, and `{}` is no method JS calls: read a getter as it is, and ask whether it's there with `js::is_defined`",
                tcx.def_path_str(method)
            );
            return Err(crate::lower::refusal::error(tcx, span, what));
        }
        let (named, _) = self.defined_path(&steps[1..], Some(value_ty), "js::optional_call", span)?;
        // What it gives: refused where `?.`'s `undefined` couldn't tell it.
        let output = tcx
            .fn_sig(method)
            .instantiate(tcx, method_args)
            .skip_normalization()
            .output()
            .skip_binder();
        let output = tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(output))
            .unwrap_or(output);
        let boxed = self.boxed_payload(output);
        let null_alone = self.option_of(output).is_some()
            && crate::lower::representation::none_form(
                tcx,
                crate::lower::representation::NonePlace::CallResult(method),
            ) == crate::lower::representation::NoneForm::Null;
        if boxed && !null_alone {
            let what = format!(
                "rust-js: `js::optional_call` of a method that may give `undefined`, a `{output}`: JS's `?.` gives that of one that isn't there too, \
                 so read it once and tell them apart, `js::method(path)` into a variable, then `function::call` of it"
            );
            return Err(crate::lower::refusal::error(tcx, span, what));
        }
        // The value, which MIR made first, as lowering holds what runs code
        // before what runs after it; then the arguments, each that runs
        // code held, as JS's `?.` wouldn't run one where the method isn't
        // there.
        let mut tuple = self.take_operands(state, &[&path.node], out)?;
        let value = match self.value_expr(tuple.remove(0), span)?.kind {
            js::ExprKind::Array(mut items) => items.swap_remove(0),
            // Held, as MIR made it, before the arguments: its value.
            js::ExprKind::Var(held) => Expr::index(Expr::var(&held), Expr::int(0)),
            _ => return refused(self, "`js::optional_call` of a path whose value isn't written in it"),
        };
        let ty::Tuple(arg_tys) = *given.node.ty(&body.local_decls, tcx).kind() else {
            return refused(self, "`js::optional_call` of arguments that aren't a tuple");
        };
        let mut values = self.take_operands(state, &[&given.node], out)?;
        let items = match self.value_expr(values.remove(0), span)?.kind {
            js::ExprKind::Array(items) => items,
            js::ExprKind::Undefined if arg_tys.is_empty() => Vec::new(),
            _ => {
                return refused(
                    self,
                    "`js::optional_call` of arguments that aren't written in it, `(selector,)`",
                );
            }
        };
        let mut called = Vec::new();
        for (i, (item, ty)) in items.into_iter().zip(arg_tys.iter()).enumerate() {
            let item = match item.has_effects() {
                true => self.spill(Role::Arg, item, out),
                false => item,
            };
            called.push(self.binding_arg(method, (i + 1, false), item, ty));
        }
        let read = named.split('.').fold(value, Expr::member);
        let call = Expr::optional_call(read, called);
        let call = match boxed {
            true => Expr::call(Expr::helper("$someDefined"), vec![call]),
            false => call,
        };
        Ok(Some(call.or_at(self.js_span(span))))
    }
}
