//! A trait of classes' method that only gives a field of its `self`, read
//! in place wherever it's called, no method of its prototype (rd304, ADR
//! 1035 amended): which methods do, of the crate's HIR or a library's MIR
//! (#1032), and which fields a call of one may give.

use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;

use super::{implementations, is_class_trait, trait_impl_class};

/// Of `def_id`, an impl's method of a trait of classes that only gives a
/// reference to a field of its `&self`, `fn internal_root(&self) ->
/// &RefCell<..> { &self.internal_root }`: that field, by the path of fields
/// to it and each one's own, read in place wherever it's called, no method
/// of its prototype (rd304, ADR 1035 amended). `None` of any other.
pub(crate) fn field_accessor(tcx: TyCtxt<'_>, def_id: DefId) -> Option<Vec<(rustc_abi::FieldIdx, DefId)>> {
    use rustc_middle::ty::adjustment::{Adjust, DerefAdjustKind};
    trait_impl_class(tcx, tcx.trait_impl_of_assoc(def_id)?)?;
    let local = def_id.as_local()?;
    let body = tcx.hir_body_owned_by(local);
    let [this, ..] = body.params else { return None };
    let typeck = tcx.typeck(local);
    let given = body.value.peel_blocks();
    let rustc_hir::ExprKind::AddrOf(rustc_hir::BorrowKind::Ref, rustc_hir::Mutability::Not, mut place) = given.kind
    else {
        return None;
    };
    // Of none, or the reborrow of what it gives back, `&*&self.f`.
    use rustc_middle::ty::adjustment::{AutoBorrow, AutoBorrowMutability};
    match typeck.expr_adjustments(given) {
        [] => {}
        [deref, borrow]
            if matches!(deref.kind, Adjust::Deref(DerefAdjustKind::Builtin))
                && matches!(borrow.kind, Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not))) => {}
        _ => return None,
    }
    // Of `self.a.b`, `b` then `a`: each base of a field the struct itself
    // but `self`, of which only a reference's own deref.
    let mut fields = Vec::new();
    loop {
        match place.kind {
            rustc_hir::ExprKind::Field(base, _) => {
                let derefs = typeck.expr_adjustments(base);
                let is_self = matches!(base.kind, rustc_hir::ExprKind::Path(rustc_hir::QPath::Resolved(None, path))
                    if path.res == rustc_hir::def::Res::Local(this.pat.hir_id));
                let adjusted = match derefs {
                    [] => false,
                    [deref] if is_self && matches!(deref.kind, Adjust::Deref(DerefAdjustKind::Builtin)) => true,
                    _ => return None,
                };
                let ty::Adt(adt, _) = typeck.expr_ty_adjusted(base).kind() else {
                    return None;
                };
                if !adt.is_struct() {
                    return None;
                }
                let index = typeck.field_index(place.hir_id);
                fields.push((index, adt.non_enum_variant().fields[index].did));
                if is_self {
                    return adjusted.then(|| fields.into_iter().rev().collect());
                }
                place = base;
            }
            _ => return None,
        }
    }
}

/// Whether `own`, a method of an impl of a trait of classes, gives its
/// field, read where it's called with no function of its own (rd304): of
/// the crate's, as its HIR reads (`field_accessor`).
pub(crate) fn gives_its_field(tcx: TyCtxt<'_>, own: DefId) -> bool {
    tcx.trait_impl_of_assoc(own)
        .is_some_and(|imp| is_class_trait(tcx, tcx.impl_trait_id(imp)))
        && field_accessor(tcx, own).is_some()
}

/// The field `own`, a library's method, gives of its `self`, `_0 =
/// &((*_1).slot)`, read of its optimized MIR, which its consumer has where
/// its library's HIR isn't (#1032): a trait of classes' (rd304), which a
/// consumer compiles where it's called. `None` of another.
pub(crate) fn field_given_of_mir(tcx: TyCtxt<'_>, own: DefId) -> Option<DefId> {
    use rustc_middle::mir;
    if own.is_local() || tcx.def_kind(own) != DefKind::AssocFn || !tcx.is_mir_available(own) {
        return None;
    }
    let body = tcx.optimized_mir(own);
    let given =
        (body.basic_blocks.iter().flat_map(|data| &data.statements)).find_map(|statement| match &statement.kind {
            mir::StatementKind::Assign(assign) if assign.0.local == mir::RETURN_PLACE => match &assign.1 {
                mir::Rvalue::Ref(_, _, place) if place.local.as_usize() == 1 => Some(*place),
                _ => None,
            },
            _ => None,
        })?;
    let mut field = None;
    for (base, elem) in given.iter_projections() {
        match elem {
            mir::ProjectionElem::Deref if field.is_none() => {}
            mir::ProjectionElem::Field(index, _) => {
                let ty::Adt(adt, _) = base.ty(&body.local_decls, tcx).ty.kind() else {
                    return None;
                };
                field = Some(adt.non_enum_variant().fields[index].did);
            }
            _ => return None,
        }
    }
    field
}

/// The fields a call of `callee`, of `args` in `typing_env`, may give in
/// place, each the last of a `field_accessor`'s path: of the one method it
/// is, or of a `Self` not known, a default's, each of its classes' that
/// gives one (rd304). Empty of any other call.
pub(crate) fn fields_given<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
) -> Vec<DefId> {
    if !(tcx.trait_of_assoc(callee)).is_some_and(|of| is_class_trait(tcx, of)) {
        return Vec::new();
    }
    let may_be = match ty::Instance::try_resolve(tcx, typing_env, callee, args) {
        Ok(Some(instance)) => vec![instance.def_id()],
        Ok(None) => (implementations(tcx, callee).into_iter().flatten())
            .map(|(own, _)| own)
            .collect(),
        Err(_) => Vec::new(),
    };
    (may_be.into_iter())
        .filter_map(|own| field_accessor(tcx, own)?.last().map(|&(_, field)| field))
        .collect()
}
