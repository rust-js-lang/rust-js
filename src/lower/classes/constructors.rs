//! What a class's constructor gives (rd554, ADR 0761 amended): `Self`, or
//! its instance shared, `Rc<Self>` or `Arc<Self>`, as React's Flight
//! server's `RequestInstance` gives the `Rc` its constructor hands its root
//! task, `createTask(this, ..)`. An `Rc` the crate counts no count of is
//! the value it points at (ADR 0844), so `new RequestInstance(..)` is the
//! `Rc`; one it counts is `{ value, strong, weak }`, which `new` can't
//! give, so it's refused.

use super::{constructor_of, is_class};
use crate::lower::recognition::rc_pointee;
use rustc_hir::def::DefKind;
use rustc_middle::ty::{Ty, TyCtxt};

/// Whether `output`, what a class's constructor gives, is the class's
/// instance, `own`: `Self`, or an `Rc` or an `Arc` of it.
pub(super) fn gives_instance<'tcx>(tcx: TyCtxt<'tcx>, output: Ty<'tcx>, own: Ty<'tcx>) -> bool {
    output == own || rc_pointee(tcx, output) == Some(own)
}

/// Report each of the crate's classes whose constructor gives an `Rc` of
/// it that `counts` says the crate counts: JS's `new` gives the instance,
/// not the box that counts it. False if there was one.
pub(crate) fn refuse_counted_instances<'tcx>(tcx: TyCtxt<'tcx>, counts: impl Fn(Ty<'tcx>) -> bool) -> bool {
    let mut valid = true;
    for def_id in tcx.hir_crate_items(()).definitions() {
        let id = def_id.to_def_id();
        if tcx.def_kind(id) != DefKind::Struct || !is_class(tcx, id) {
            continue;
        }
        let Some(new) = constructor_of(tcx, id) else { continue };
        let output = tcx
            .fn_sig(new)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder()
            .output();
        if let Some(pointee) = rc_pointee(tcx, output)
            && counts(pointee)
        {
            tcx.dcx().span_err(
                tcx.def_span(new),
                format!(
                    "rust-js: a `#[rust_js::class]` constructor that gives `{output}` gives the instance `new` makes, an `Rc` the crate counts nothing of; this crate counts it, `{{ value, strong, weak }}`, as it reads its counts or its last owner runs a destructor: give `Self`, `fn new(..) -> Self`, and `Rc::new(..)` it where it's made (rd554, ADR 0761)"
                ),
            );
            valid = false;
        }
    }
    valid
}
