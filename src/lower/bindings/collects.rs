//! `#[rust_js::collects]`: a binding that makes an array of what it's
//! given, `Array.from(items)`, which TypeScript types of any iterable.
//! Rust already collects what it iterates itself, the one way: a JS `Set`
//! by `set.values().collect()`, a `Map` by `map.entries().collect()`, which
//! are `Array.from(set)` and `Array.from(map)`, and a `Vec`, a slice or an
//! array by `to_vec()`. A call of one given those is refused, naming that
//! way (ADR 1052).

use crate::lower::Body;
use crate::lower::body_queries::strip;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

/// Whether `def_id` is a binding marked `#[rust_js::collects]`.
fn collects(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    let path = [Symbol::intern("rust_js"), Symbol::intern("collects")];
    tcx.get_attrs_by_path(def_id, &path).next().is_some()
}

/// What Rust writes to collect a value of `ty` itself, if it does: of the
/// js crate's `Set` and `Map`, their own iteration's, and of a `Vec`, a
/// slice and an array, `to_vec()`.
fn collected_by_rust<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<&'static str> {
    use crate::lower::recognition::{StdItem, is_std_def, js_set_or_map};
    match ty.peel_refs().kind() {
        ty::Slice(_) | ty::Array(..) => Some("`to_vec()`"),
        ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Vec) => Some("`to_vec()`"),
        ty::Adt(adt, _) => js_set_or_map(tcx, adt.did()).map(|set| match set {
            true => "`set.values().collect()`",
            false => "`map.entries().collect()`",
        }),
        _ => None,
    }
}

/// Report each call of a `#[rust_js::collects]` binding given what Rust
/// collects itself, naming how. False if there was one.
fn reject_collected_calls<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> bool {
    let mut valid = true;
    for body in bodies {
        let thir = &body.thir;
        for expr in thir.exprs.iter() {
            let ExprKind::Call {
                fun, ref args, fn_span, ..
            } = expr.kind
            else {
                continue;
            };
            let ty::FnDef(callee, _) = *thir[strip(thir, fun)].ty.kind() else {
                continue;
            };
            if !collects(tcx, callee) {
                continue;
            }
            // What it's given, before it's made a `&dyn Iterable`.
            let Some(&first) = args.first() else { continue };
            let mut given = strip(thir, first);
            while let ExprKind::PointerCoercion { source, .. } = thir[given].kind {
                given = strip(thir, source);
            }
            if let Some(way) = collected_by_rust(tcx, thir[given].ty) {
                let ty = thir[given].ty.peel_refs();
                let message = format!(
                    "rust-js: `Array.from` of a `{ty}` is what Rust collects itself, the one way: {way} (ADR 1052)"
                );
                tcx.dcx().span_err(fn_span, message);
                valid = false;
            }
        }
    }
    valid
}

/// Report each call its binding's attribute refuses: one read as a value
/// alone, `#[rust_js::value_only]` (ADR 1020), and one given what Rust
/// collects itself, `#[rust_js::collects]`. False if there was one.
pub(in crate::lower) fn reject_attributed_calls<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> bool {
    let value_only = super::value_only::reject_value_only_calls(tcx, bodies);
    reject_collected_calls(tcx, bodies) && value_only
}
