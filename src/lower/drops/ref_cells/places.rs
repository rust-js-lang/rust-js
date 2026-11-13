//! Where the cell a borrow is of comes from, as a body's THIR says: what's
//! lent, `x.f` of `x.f.borrow()`, a field of a struct or of what a call
//! gives, a thread-local's key, the parts a cell holds, and the places a
//! body makes and reads as properties (ADRs 0362, 0863). Split from
//! ref_cells.rs by concept, at 1,407 lines (#1334).

use super::*;

/// The cell `lent`, a `RefCell`'s method's receiver, is: `x.f` of
/// `&x.f`, a call of `&*c()`, else itself.
pub(super) fn cell_lent(
    thir: &rustc_middle::thir::Thir<'_>,
    lent: rustc_middle::thir::ExprId,
) -> rustc_middle::thir::ExprId {
    use crate::lower::body_queries::strip;
    let lent = strip(thir, lent);
    let ExprKind::Borrow { arg, .. } = thir[lent].kind else {
        return lent;
    };
    let arg = strip(thir, arg);
    match thir[arg].kind {
        ExprKind::Deref { arg: of } if matches!(thir[strip(thir, of)].kind, ExprKind::Call { .. }) => strip(thir, of),
        _ => arg,
    }
}

/// Whether `lent`, a cell given to a call, `&x.state`, is a class's
/// flattened state (`bindings::is_flattened_state_field`), whose `swap`
/// counts no borrow and lends neither cell (#687).
pub(super) fn flattened_state_lent(
    tcx: TyCtxt<'_>,
    thir: &rustc_middle::thir::Thir<'_>,
    lent: rustc_middle::thir::ExprId,
) -> bool {
    field_of(thir, through_borrows(thir, lent))
        .is_some_and(|field| crate::lower::bindings::is_flattened_state_field(tcx, field))
}

/// What `lent` lends, through each borrow and deref of it: `b.state` of
/// `&*&b.state`, a `swap`'s other cell, reborrowed.
pub(super) fn through_borrows(
    thir: &rustc_middle::thir::Thir<'_>,
    lent: rustc_middle::thir::ExprId,
) -> rustc_middle::thir::ExprId {
    use crate::lower::body_queries::strip;
    let mut lent = strip(thir, lent);
    while let ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } = thir[lent].kind {
        lent = strip(thir, arg);
    }
    lent
}

/// The fields `call`, a call that may give one in place, may give
/// (`classes::fields_given`, rd304).
pub(super) fn fields_given<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    thir: &rustc_middle::thir::Thir<'tcx>,
    call: rustc_middle::thir::ExprId,
) -> Vec<DefId> {
    let ExprKind::Call { fun, .. } = thir[call].kind else {
        return Vec::new();
    };
    match crate::lower::fn_def(thir[fun].ty) {
        Some(called) => crate::lower::classes::fields_given(recognition.tcx, recognition.typing_env, called),
        None => Vec::new(),
    }
}

/// The struct's field `expr` reads, `x.f`, if it's one.
pub(super) fn field_of<'tcx>(
    thir: &rustc_middle::thir::Thir<'tcx>,
    expr: rustc_middle::thir::ExprId,
) -> Option<&'tcx ty::FieldDef> {
    let ExprKind::Field {
        lhs,
        variant_index,
        name,
    } = thir[expr].kind
    else {
        return None;
    };
    match thir[lhs].ty.kind() {
        ty::Adt(adt, _) if adt.is_struct() => Some(&adt.variant(variant_index).fields[name]),
        _ => None,
    }
}

/// Whether a call of `id` is one of a `RefCell`'s that borrows it, or
/// holds it only while it swaps what it holds, whose borrow `counted_in`
/// counts where it may meet another, or takes it whole, `into_inner`,
/// which rustc lets no borrow outlive: what a field's use may be and the
/// field still have no reference to it elsewhere (ADR 0863, amended; #687).
pub(super) fn borrows_its_cell<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    use crate::lower::Std;
    matches!(
        recognition.classify(id, args),
        Some(
            Std::Borrow { lock: false, .. }
                | Std::TryBorrow { lock: false, .. }
                | Std::CellReplace
                | Std::CellTake
                | Std::CellGetMut
                | Std::CellReplaceWith
                | Std::CellGet
        )
    )
}

/// Whether `id` is a thread-local's key, the `const` of a `LocalKey` its
/// macro makes.
pub(super) fn thread_local_key(tcx: TyCtxt<'_>, id: DefId) -> bool {
    matches!(tcx.def_kind(id), DefKind::Const)
        && matches!(tcx.type_of(id).instantiate_identity().skip_normalization().kind(),
            ty::Adt(key, _) if crate::lower::recognition::is_std_def(tcx, key.did(), StdItem::LocalKey))
}

/// The types `ty` is made of, but what a thread-local's key holds, which is
/// the thread-local's own place, whatever its type (ADR 0863, amended).
pub(super) fn held_parts<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<Ty<'tcx>> {
    let mut parts = Vec::new();
    let mut walk = ty.walk();
    while let Some(part) = walk.next() {
        let Some(part) = part.as_type() else { continue };
        if let ty::Adt(key, _) = part.kind()
            && crate::lower::recognition::is_std_def(tcx, key.did(), StdItem::LocalKey)
        {
            walk.skip_current_subtree();
            continue;
        }
        parts.push(part);
    }
    parts
}

/// Of a body, each expression whose value is the cell a place is made of,
/// the place's own and no value of its cell's type (ADR 0863, amended): of a
/// struct's or a variant's literal, each `RefCell` field's that holds its
/// value by its declaration (`RefCells::holds_value`), `current: RefCell::new(value)`;
/// and of a thread-local's `init`, what it gives, the thread-local's cell.
pub(super) fn made_places<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &crate::lower::Body<'tcx>,
) -> IndexSet<rustc_middle::thir::ExprId> {
    let thir = &body.thir;
    let mut made: Vec<rustc_middle::thir::ExprId> = Vec::new();
    for expr in thir.exprs.iter() {
        if let ExprKind::Adt(ref adt) = expr.kind
            && !adt.adt_def.is_union()
        {
            for field in adt.fields.iter() {
                let def = &adt.adt_def.variant(adt.variant_index).fields[field.name];
                let declared = tcx.type_of(def.did).instantiate_identity().skip_normalization();
                if matches!(declared.kind(), ty::Adt(cell, _) if crate::lower::recognition::is_std_def(tcx, cell.did(), StdItem::RefCell))
                    && object_field(tcx, def.did)
                {
                    made.push(field.expr);
                }
            }
        }
    }
    if tcx
        .opt_parent(body.def_id)
        .is_some_and(|parent| thread_local_key(tcx, parent))
    {
        made.push(body.expr);
    }
    // Each, and what it's the scope of, of the same value.
    let mut places = IndexSet::default();
    for mut at in made {
        loop {
            places.insert(at);
            at = match thir[at].kind {
                ExprKind::Scope { value: inner, .. }
                | ExprKind::ValueExpr { source: inner }
                | ExprKind::NeverToAny { source: inner }
                | ExprKind::ValueTypeAscription { source: inner, .. }
                | ExprKind::PlaceTypeAscription { source: inner, .. } => inner,
                ExprKind::Block { block } if thir[block].stmts.is_empty() => match thir[block].expr {
                    Some(tail) => tail,
                    None => break,
                },
                _ => break,
            };
        }
    }
    places
}

/// Of a body, each read of a generic struct's `RefCell<T>` field, which
/// holds its value by its declaration whatever it is (`RefCells::holds_value`),
/// and what's made of one there, a borrow of it, its guard, a call given it:
/// a box of the property, whatever its type, not a `RefCell<T>` of any value
/// that generic code is given (ADR 0863, amended).
pub(super) fn property_reads<'tcx>(
    tcx: TyCtxt<'tcx>,
    (thir, places): (&rustc_middle::thir::Thir<'tcx>, &BoundPlaces),
) -> IndexSet<rustc_middle::thir::ExprId> {
    use crate::lower::body_queries::strip;
    let generic = |field: DefId| {
        let declared = tcx.type_of(field).instantiate_identity().skip_normalization();
        let ty::Adt(cell, args) = declared.kind() else {
            return false;
        };
        crate::lower::recognition::is_std_def(tcx, cell.did(), StdItem::RefCell)
            && object_field(tcx, field)
            && crate::lower::analysis::head(tcx, args.type_at(0)).is_none()
    };
    let mut reads = IndexSet::default();
    for _ in 0..2 {
        for (id, expr) in thir.exprs.iter_enumerated() {
            let read = match expr.kind {
                ExprKind::Field { .. } => field_of(thir, id).is_some_and(|field| generic(field.did)),
                // A binding that's a field's own place, as its read (#503).
                ExprKind::VarRef { id } => places.get(&id).is_some_and(|&field| generic(field)),
                ExprKind::Borrow { arg, .. } | ExprKind::RawBorrow { arg, .. } | ExprKind::Deref { arg } => {
                    reads.contains(&strip(thir, arg))
                }
                ExprKind::Call { fun, ref args, .. } => {
                    let read = args.first().is_some_and(|&first| reads.contains(&strip(thir, first)));
                    // And what it calls, `Deref::deref::<Ref<T>>`.
                    if read {
                        reads.insert(fun);
                    }
                    read
                }
                // A scope of one, as `strip` reads through.
                _ => strip(thir, id) != id && reads.contains(&strip(thir, id)),
            };
            if read {
                reads.insert(id);
            }
        }
    }
    reads
}
