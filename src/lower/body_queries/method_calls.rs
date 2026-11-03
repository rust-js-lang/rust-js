//! A member called as its value's method (rd334):
//! `function::call(js::method((listener, "handleEvent")), Some(listener),
//! (event,))`, JS's `listener.handleEvent(event)`, of a value of any shape
//! and a key, or of a value and a binding's method.

use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, Thir};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use std::collections::HashSet;

use super::strip;
use crate::lower::bindings::{self, JsForm};
use crate::lower::recognition::is_function_call;

/// Where each such call is, its `fn_span`, and where the `js::method` it
/// calls is, its own: a `function::call` of a `js::method` of a path of
/// a value and a method or a key, whose `this` is `Some` of the path's
/// value, the same variable. Its call is the method's, `value.name(..)`,
/// whose `this` is that value, as `f.call(value, ..)` gives it, where
/// the JS reads the method in place of the variable it's given
/// (`Expr::own_method_call`); a key's read is what's read of it nowhere
/// else (`mir_keyed_method`).
pub(in crate::lower) fn method_calls<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>) -> (HashSet<Span>, HashSet<Span>) {
    let (mut calls, mut methods) = (HashSet::new(), HashSet::new());
    for expr in thir.exprs.iter() {
        let ExprKind::Call {
            fun, ref args, fn_span, ..
        } = expr.kind
        else {
            continue;
        };
        let ty::FnDef(call, _) = *thir[fun].ty.kind() else {
            continue;
        };
        let [method, this, _] = args[..] else { continue };
        if !is_function_call(tcx, call) {
            continue;
        }
        let ExprKind::Call {
            fun: read,
            args: ref path,
            fn_span: read_span,
            ..
        } = thir[through_borrows(thir, method)].kind
        else {
            continue;
        };
        let ty::FnDef(read, _) = *thir[read].ty.kind() else {
            continue;
        };
        if !bindings::is_binding(tcx, read) || !matches!(bindings::js_form(tcx, read), JsForm::Method) {
            continue;
        }
        let [path] = path[..] else { continue };
        let ExprKind::Tuple { ref fields } = thir[strip(thir, path)].kind else {
            continue;
        };
        let [value, _] = fields[..] else { continue };
        let ExprKind::Adt(ref some) = thir[strip(thir, this)].kind else {
            continue;
        };
        // `Some(value)`, an `Option<&dyn IntoUnknown>`'s one of a field.
        let [ref given] = some.fields[..] else { continue };
        if variable(thir, value).is_some_and(|of| variable(thir, given.expr) == Some(of)) {
            calls.insert(fn_span);
            methods.insert(read_span);
        }
    }
    (calls, methods)
}

/// What `e` is through the borrows, derefs and coercions of it: what a
/// `&F` is given of a `&dyn Fn`, its reborrow.
fn through_borrows(thir: &Thir<'_>, mut e: ExprId) -> ExprId {
    loop {
        e = strip(thir, e);
        match thir[e].kind {
            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } | ExprKind::PointerCoercion { source: arg, .. } => {
                e = arg
            }
            _ => return e,
        }
    }
}

/// The variable `e` reads, through its borrows, derefs and coercions, a
/// closure's captured one too.
fn variable(thir: &Thir<'_>, e: ExprId) -> Option<LocalVarId> {
    match thir[through_borrows(thir, e)].kind {
        ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => Some(id),
        _ => None,
    }
}
