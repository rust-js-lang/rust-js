//! What a place is the value of: a local, or what its box holds, which is
//! the local in JS, a box being the value it holds (ADR 0023). So a write
//! of `*b` is a write of `b`, as `b = v` is: each check of whether a local
//! is written asks here, where three asked of the local alone and read a
//! box written through as unchanged (#980, #997, #1000); and `indirect`,
//! which a box's deref inside a field isn't (#1033). `FnCx::whole`
//! extends `value_local` with what else is its value in JS, a transparent
//! wrapper's field; a body's THIR asks `changes`, which reaches a variable
//! through any deref.

use crate::collections::IndexMap;
use rustc_middle::mir::{self, Local, Place, PlaceElem, Rvalue, StatementKind};
use rustc_middle::ty::{Ty, TyCtxt};

/// What a step `elem` from a value of type `ty` reaches that's still that
/// value: what a box holds, through its deref. The one answer of whether a
/// step through a box is the value itself, which each question here and
/// `FnCx::through_values` take it from.
pub(in crate::lower) fn through_box<'tcx>(elem: PlaceElem<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match elem {
        PlaceElem::Deref => ty.boxed_ty(),
        _ => None,
    }
}

/// The local `place` is the value of: itself, `_1`, or what its box holds,
/// `(*_1)` of a `Box<T>`, through boxes of boxes too. `None` of a part of
/// it, a field's, an item's, or what a reference points to.
pub(in crate::lower) fn value_local<'tcx>(decls: &mir::LocalDecls<'tcx>, place: &Place<'tcx>) -> Option<Local> {
    let mut ty = decls[place.local].ty;
    for elem in place.projection {
        ty = through_box(elem, ty)?;
    }
    Some(place.local)
}

/// Whether `place` is reached through a reference or a pointer, so what's
/// there is what that points at, not its local's own: a deref of anything
/// but a box. MIR's `is_indirect` counts a box's deref too, where its
/// content is its local's, `*w.0` of a `W(Box<i32>)` and `(*b).0` of a
/// `Box<V>`, whose writes left the local alike, a read of it moved past
/// them (#1033).
pub(in crate::lower) fn indirect<'tcx>(tcx: TyCtxt<'tcx>, decls: &mir::LocalDecls<'tcx>, place: &Place<'tcx>) -> bool {
    (place.iter_projections())
        .any(|(base, elem)| elem == PlaceElem::Deref && through_box(elem, base.ty(decls, tcx).ty).is_none())
}

/// Each local a statement assigns, and what it's assigned where that's
/// once: `None` of one assigned again, or written through its box, `*b =
/// v`, which assigns it as `b = v` does. Not a call's result, a
/// terminator's.
pub(in crate::lower) fn assigned_once<'b, 'tcx>(
    body: &'b mir::Body<'tcx>,
) -> IndexMap<Local, Option<&'b Rvalue<'tcx>>> {
    let mut assigned: IndexMap<Local, Option<&'b Rvalue<'tcx>>> = IndexMap::default();
    for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
        let StatementKind::Assign(assign) = &statement.kind else {
            continue;
        };
        let Some(local) = value_local(&body.local_decls, &assign.0) else {
            continue;
        };
        let once = assign.0.projection.is_empty() && !assigned.contains_key(&local);
        assigned.insert(local, once.then_some(&assign.1));
    }
    assigned
}
