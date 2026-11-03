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
        if !self.frame.facts.called_methods.contains(&span) {
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
}
