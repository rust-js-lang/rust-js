//! A function's scope struct's methods, functions declared in it (ADR
//! 1043): each lowered as a body inside the function's, closing over the
//! variable its value is, its `self`, which no call gives it.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Of the body `state` lowers, its scope struct's methods, if it's a
    /// function with one (`analysis::Scopes`), its `own` body, not a
    /// closure's, each a function declared in `out` after the statements
    /// its value's `let` is.
    pub(super) fn declare_scope_functions(&mut self, state: &State<'_, 'tcx>, own: bool, out: &mut Vec<Stmt>) -> R<()> {
        let Some(scope) = (self.krate.scopes.of_function(state.body.source.def_id())).filter(|_| own) else {
            return Ok(());
        };
        let value = state.locals.names[scope.local].clone();
        let mut functions = Vec::new();
        for &method in &scope.methods {
            functions.push(self.scope_function(method, &value)?);
        }
        let written = self.js_span(scope.written);
        let within = |s: &Stmt| !s.span.is_none() && s.span.lo >= written.lo && s.span.hi <= written.hi;
        // After the statements its `let` is, in the list they're in.
        let mut placed = false;
        js::statement_lists(out, &mut |list| {
            if !placed && let Some(last) = list.iter().rposition(within) {
                list.splice(last + 1..last + 1, std::mem::take(&mut functions));
                placed = true;
            }
        });
        match placed {
            true => Ok(()),
            false => Err(self.unsupported(scope.written, "a scope struct's `let`, lowered to no statement")),
        }
    }

    /// `method`, a scope struct's, a function declaration closing over
    /// `value`, its `self`.
    fn scope_function(&mut self, method: rustc_span::def_id::DefId, value: &str) -> R<Stmt> {
        let span = self.tcx.def_span(method);
        let body = (self.krate.bodies.get(&method).copied())
            .ok_or_else(|| self.unsupported(span, "a scope struct's method, without its body"))?;
        let mir =
            (body.mir.as_ref()).ok_or_else(|| self.unsupported(span, "a scope struct's method, without its MIR"))?;
        // Its own typing environment, and no copy of it; its names apart
        // from every one of its function's, which it may read.
        let typing_env = std::mem::replace(&mut self.typing_env, ty::TypingEnv::post_analysis(self.tcx, method));
        let copy = self.copy.take();
        self.scope_self = Some(value.to_string());
        let nested = super::super::Nested::Closure {
            names: self.names.clone(),
        };
        let mut stmts = Vec::new();
        let lowered = self.lowered_in(body, method, nested, |cx| cx.mir_signature(body, mir, &mut stmts));
        self.scope_self = None;
        self.typing_env = typing_env;
        self.copy = copy;
        let ((mut params, is_async), hoisted) = lowered?;
        stmts.splice(0..0, hoisted);
        // Its `self`, the variable it closes over.
        match params.first() {
            Some(js::Pattern::Name(name)) if name == value => {
                params.remove(0);
            }
            _ => return Err(self.unsupported(span, "a scope struct's method whose `self` isn't its value")),
        }
        let local = method.expect_local();
        let whole = self.tcx.hir_span_with_body(self.tcx.local_def_id_to_hir_id(local));
        let function = js::Function {
            name: self.scope_functions[&method].clone(),
            params,
            body: stmts,
            export: false,
            is_async,
            noinline: false,
            span: self.js_span(function_start(self.tcx, whole, self.tcx.def_ident_span(method))),
            name_span: (self.tcx.def_ident_span(method)).map_or(js::Span::NONE, |s| self.js_span(s)),
            param_spans: Vec::new(),
            set_apart: true,
        };
        Ok(StmtKind::Function(Box::new(function)).at(self.js_span(span)))
    }
}
