//! `#[rust_js::value_only = ".."]`: a binding read as a value alone,
//! `js::method((window, Window::set_timeout))`, `window.setTimeout`, which
//! a call never names, as another way does what a call of it would: a
//! window's timer, whose one call is the js crate's `js::set_timeout`
//! (ADR 1020, amended, rd314). Each call of one is refused with what the
//! attribute says to write.

use crate::lower::Body;
use crate::lower::body_queries::strip;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

/// What `#[rust_js::value_only = ".."]` of `def_id` says to write instead
/// of a call, if it's one.
fn value_only(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    let path = [Symbol::intern("rust_js"), Symbol::intern("value_only")];
    let attr = tcx.get_attrs_by_path(def_id, &path).next()?;
    Some(attr.value_str().map_or_else(String::new, |message| message.to_string()))
}

/// Report each call of a `#[rust_js::value_only]` binding, saying what
/// its attribute says. False if there was one.
pub(in crate::lower) fn reject_value_only_calls<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> bool {
    let mut valid = true;
    for body in bodies {
        let thir = &body.thir;
        for expr in thir.exprs.iter() {
            let ExprKind::Call { fun, fn_span, .. } = expr.kind else {
                continue;
            };
            let ty::FnDef(callee, _) = *thir[strip(thir, fun)].ty.kind() else {
                continue;
            };
            if let Some(message) = value_only(tcx, callee) {
                tcx.dcx().span_err(fn_span, format!("rust-js: {message}"));
                valid = false;
            }
        }
    }
    valid
}
