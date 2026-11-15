//! What a `js::Accessor<T>` is read as (ADR 1164): a struct's field of
//! one is its live getter in JS, `get canceled() { return canceled; }`,
//! whose read runs it, so `details.canceled.get()` is `details.canceled`.
//! What would read it otherwise is refused: the field moved or copied out,
//! which JS would read the value of where it's moved, not where Rust's
//! `get` calls it; and `get` of an accessor that isn't a struct's field, a
//! local's, whose JS is its function, not what it gives.

use crate::lower::Body;
use rustc_middle::mir::{self, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};

/// Whether `ty` is a `js::Accessor<T>`.
fn is_accessor(tcx: TyCtxt<'_>, ty: ty::Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if crate::lower::recognition::is_js_accessor(tcx, adt.did()))
}

/// Whether `place` is a struct's field, `(*_1).1`.
fn is_field(place: mir::Place<'_>) -> bool {
    matches!(place.projection.last(), Some(mir::PlaceElem::Field(..)))
}

/// Refuses each read of an accessor its JS would read otherwise, in each
/// of `bodies`.
pub(super) fn refuse_accessor_reads<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) {
    for body in bodies {
        let Some(mir) = body.mir.as_ref() else { continue };
        let mir = &mir.body;
        let decls = &mir.local_decls;
        // Each local made a reference to a struct's accessor field, `_3 =
        // &((*_1).1)`, which `get` may be given.
        let field_refs: Vec<mir::Local> = (mir.basic_blocks.iter())
            .flat_map(|data| &data.statements)
            .filter_map(|statement| match &statement.kind {
                StatementKind::Assign(assign) => match &assign.1 {
                    Rvalue::Ref(_, _, place) if is_field(*place) && is_accessor(tcx, place.ty(decls, tcx).ty) => {
                        assign.0.as_local()
                    }
                    _ => None,
                },
                _ => None,
            })
            .collect();
        let moved_out = |operand: &Operand<'tcx>| {
            operand
                .place()
                .is_some_and(|place| is_field(place) && is_accessor(tcx, place.ty(decls, tcx).ty))
        };
        for data in mir.basic_blocks.iter() {
            for statement in &data.statements {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let Rvalue::Use(operand, _) = &assign.1
                    && moved_out(operand)
                {
                    tcx.dcx().span_err(
                        statement.source_info.span,
                        "rust-js: a `js::Accessor`'s field is read only by its `get`, `details.canceled.get()`: in JS it's a getter, whose value a move of the field would read there, not where Rust's `get` calls it (ADR 1164)",
                    );
                }
            }
            let Some(terminator) = &data.terminator else { continue };
            let TerminatorKind::Call { func, args, .. } = &terminator.kind else {
                continue;
            };
            if args.iter().any(|arg| moved_out(&arg.node)) {
                tcx.dcx().span_err(
                    terminator.source_info.span,
                    "rust-js: a `js::Accessor`'s field is read only by its `get`, `details.canceled.get()`: in JS it's a getter, whose value a move of the field would read there, not where Rust's `get` calls it (ADR 1164)",
                );
            }
            let Some((callee, _)) = func.const_fn_def() else {
                continue;
            };
            let is_get = crate::lower::recognition::is_js_accessor_get(tcx, callee);
            if !is_get {
                continue;
            }
            let of_a_field = (args.first().and_then(|receiver| receiver.node.place()))
                .and_then(|place| place.as_local())
                .is_some_and(|local| field_refs.contains(&local));
            if !of_a_field {
                tcx.dcx().span_err(
                    terminator.source_info.span,
                    "rust-js: `js::Accessor::get` reads a struct's field, `details.canceled.get()`, whose JS is its getter: an accessor elsewhere is its function in JS, not what it gives (ADR 1164)",
                );
            }
        }
    }
}
