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
/// of its prototype (rd304, ADR 1035 amended): of the crate's, as its HIR
/// reads; of a library's, its library's, as its MIR does, which is all its
/// consumer has of it (#1032, #1073). `None` of any other.
pub(crate) fn field_accessor(tcx: TyCtxt<'_>, def_id: DefId) -> Option<Vec<(rustc_abi::FieldIdx, DefId)>> {
    use rustc_middle::ty::adjustment::{Adjust, DerefAdjustKind};
    trait_impl_class(tcx, tcx.trait_impl_of_assoc(def_id)?)?;
    let Some(local) = def_id.as_local() else {
        return field_accessor_of_mir(tcx, def_id);
    };
    let body = tcx.hir_body_owned_by(local);
    let [this, ..] = body.params else { return None };
    let typeck = tcx.typeck(local);
    // Its blocks' expression, of blocks of no statements: one that runs
    // anything before it gives more than its field (#1074).
    let mut given = body.value;
    while let rustc_hir::ExprKind::Block(block, _) = given.kind {
        if !block.stmts.is_empty() {
            return None;
        }
        given = block.expr?;
    }
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

/// The path of fields `own`, a library's method, gives of its `self`, of
/// its optimized MIR, which its consumer has where its library's HIR isn't:
/// a body of nothing but `_0 = &((*_1).slot)`, as `field_accessor` reads a
/// crate's `&self.slot`. Its only deref is `self`'s: optimized MIR derefs
/// only at a place's start (rustc's `Derefer`), so one through a field,
/// `&self.limits.most`, is of a local it copied that field to, a statement
/// of its own.
fn field_accessor_of_mir(tcx: TyCtxt<'_>, own: DefId) -> Option<Vec<(rustc_abi::FieldIdx, DefId)>> {
    use rustc_middle::mir;
    if tcx.def_kind(own) != DefKind::AssocFn || !tcx.is_mir_available(own) {
        return None;
    }
    let body = tcx.optimized_mir(own);
    let block = &body.basic_blocks[mir::START_BLOCK];
    if !matches!(block.terminator().kind, mir::TerminatorKind::Return) {
        return None;
    }
    let mut given = None;
    for statement in &block.statements {
        match &statement.kind {
            mir::StatementKind::StorageLive(_) | mir::StatementKind::StorageDead(_) | mir::StatementKind::Nop => {}
            mir::StatementKind::Assign(assign) if assign.0 == mir::RETURN_PLACE.into() => {
                let mir::Rvalue::Ref(_, mir::BorrowKind::Shared, place) = &assign.1 else {
                    return None;
                };
                given = Some(*place);
            }
            _ => return None,
        }
    }
    let given = given.filter(|place| place.local.as_usize() == 1)?;
    let mut fields = Vec::new();
    for (base, elem) in given.iter_projections() {
        match elem {
            mir::ProjectionElem::Deref => {}
            mir::ProjectionElem::Field(field, _) => {
                let ty::Adt(adt, _) = base.ty(&body.local_decls, tcx).ty.kind() else {
                    return None;
                };
                fields.push((field, adt.non_enum_variant().fields[field].did));
            }
            _ => return None,
        }
    }
    Some(fields)
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
