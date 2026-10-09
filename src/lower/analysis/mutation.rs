//! The types something changes in place, which a copy is made of where
//! it's read (ADR 0052).

use crate::lower::recognition::replaces_whole;
use crate::lower::{Body, FnInfo, fn_def, strip};
use rustc_ast::Mutability;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty;
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::DesugaringKind;
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// The types whose JS objects get changed in place somewhere in the crate:
/// `a.b.c = ..` changes the object `a.b`, so it's `a.b`'s type. Only these
/// ever need copying (ADR 0020).
pub(super) fn mutated_types<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> HashSet<Ty<'tcx>> {
    let mut mutated = HashSet::new();
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            // An enum something takes `&mut` of, or matches with a `ref mut`
            // binding, may have a variant's field changed through it.
            let enum_ty = |ty: Ty<'tcx>| matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if adt.is_enum());
            // A range's bounds are changed by std's methods, as `next()` (ADR 0129),
            // though not by a `for`, which steps through its own.
            let range_ty = |ty: Ty<'tcx>| {
                !expr.span.is_desugaring(DesugaringKind::ForLoop)
                    && matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if [LangItem::Range, LangItem::RangeInclusiveStruct, LangItem::RangeFrom]
                    .into_iter()
                    .any(|item| tcx.is_lang_item(adt.did(), item)))
            };
            match expr.kind {
                ExprKind::Borrow {
                    borrow_kind: BorrowKind::Mut { .. },
                    arg,
                } if enum_ty(body.thir[arg].ty) || range_ty(body.thir[arg].ty) => {
                    mutated.insert(body.thir[arg].ty.peel_refs());
                }
                ExprKind::Match {
                    scrutinee, ref arms, ..
                } if enum_ty(body.thir[scrutinee].ty)
                    && arms.iter().any(|&arm| binds_ref_mut(&body.thir[arm].pattern)) =>
                {
                    mutated.insert(body.thir[scrutinee].ty.peel_refs());
                }
                ExprKind::Let {
                    expr: scrutinee,
                    ref pat,
                } if enum_ty(body.thir[scrutinee].ty) && binds_ref_mut(pat) => {
                    mutated.insert(body.thir[scrutinee].ty.peel_refs());
                }
                _ => {}
            }
            // `*r = v` through a `&mut` changes an object in place (ADR 0147),
            // as `mem::swap`, `mem::replace` and `mem::take` of one do.
            if let ExprKind::Assign { lhs, .. } = expr.kind
                && let ExprKind::Deref { arg } = body.thir[strip(&body.thir, lhs)].kind
                && matches!(body.thir[arg].ty.kind(), ty::Ref(_, _, Mutability::Mut))
                && object_like(tcx, body.thir[lhs].ty)
            {
                mutated.insert(body.thir[lhs].ty);
            }
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && let ty::FnDef(def_id, _) = *body.thir[fun].ty.kind()
                && replaces_whole(tcx, def_id)
            {
                for &arg in args.iter() {
                    if let ty::Ref(_, inner, Mutability::Mut) = *body.thir[arg].ty.kind()
                        && object_like(tcx, inner)
                    {
                        mutated.insert(inner);
                    }
                }
            }
            // `a[i] = ..` changes the array `a` the same way.
            if let ExprKind::Assign { lhs, .. } | ExprKind::AssignOp { lhs, .. } = expr.kind
                && let ExprKind::Field { lhs: object, .. } | ExprKind::Index { lhs: object, .. } =
                    body.thir[strip(&body.thir, lhs)].kind
            {
                mutated.insert(body.thir[object].ty);
            }
        }
    }
    mutated
}

/// May `ty` be a JS object, which a `&mut` replaces in place (ADR 0147)? A
/// number, a string, an `Option` and a `Box` aren't: a `&mut` to one is its
/// place's, and a string can't change.
fn object_like<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Adt(adt, _) => {
            !adt.is_box()
                && ![LangItem::String, LangItem::Option]
                    .into_iter()
                    .any(|item| tcx.is_lang_item(adt.did(), item))
        }
        ty::Tuple(items) => !items.is_empty(),
        ty::Array(..) | ty::Slice(_) => true,
        _ => false,
    }
}

/// Does `pat` bind a variable by `ref mut`, or through a `&mut` subject?
pub(super) fn binds_ref_mut(pat: &rustc_middle::thir::Pat<'_>) -> bool {
    let mut found = false;
    pat.walk_always(|p| {
        if let rustc_middle::thir::PatKind::Binding { mode, .. } = p.kind
            && matches!(mode.0, rustc_hir::ByRef::Yes(_, rustc_ast::Mutability::Mut))
        {
            found = true;
        }
    });
    found
}

/// The type parameters whose `Copy` bound takes a copy function, `TCopy`
/// (ADR 0289): every one of a function another crate or a dictionary may
/// call, a trait's and a public one of a library's, and of the crate's own
/// others those a caller gives a value whose copy may not be the value
/// itself, or a type parameter of its own that is one. A reference, a
/// number, text, a function, and an `Option` of one is its own copy.
pub(in crate::lower) fn copied_params<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    fns: &HashMap<DefId, FnInfo>,
    library: bool,
) -> HashSet<(DefId, u32)> {
    let mut given = HashSet::new();
    for &id in fns.keys() {
        if !copies_by_callers(tcx, id) || (library && crate::lower::library::reachable(tcx, id)) {
            given.extend(copy_bounded(tcx, id).into_iter().map(|index| (id, index)));
        }
    }
    let mut passed = Vec::new();
    for body in all_bodies {
        let caller = tcx.typeck_root_def_id(body.def_id.to_def_id());
        for expr in body.thir.exprs.iter() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            if !fns.contains_key(&callee) {
                continue;
            }
            for index in copy_bounded(tcx, callee) {
                let Some(given_ty) = args.get(index as usize).and_then(|arg| arg.as_type()) else {
                    continue;
                };
                match given_ty.kind() {
                    ty::Param(param) => passed.push(((caller, param.index), (callee, index))),
                    _ if copies_itself(tcx, given_ty) => {}
                    _ => {
                        given.insert((callee, index));
                    }
                }
            }
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for &(from, to) in &passed {
            if given.contains(&from) && given.insert(to) {
                changed = true;
            }
        }
    }
    given
}

/// Is `id` one only its crate's callers call, whose `Copy` bounds take a
/// copy function only where they give one (ADR 0289)? A trait's method, an
/// impl's, and their closures are called through dictionaries, and a trait
/// impl's own bounds are its dictionary's.
pub(in crate::lower) fn copies_by_callers(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let id = tcx.typeck_root_def_id(id);
    id.is_local()
        && !matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true })
        && tcx.trait_of_assoc(id).is_none()
        && tcx.trait_impl_of_assoc(id).is_none()
}

/// The type parameters of `id` bound by `Copy`, by their indices, an
/// impl's with its own.
fn copy_bounded(tcx: TyCtxt<'_>, id: DefId) -> Vec<u32> {
    let Some(copy) = tcx.lang_items().copy_trait() else {
        return Vec::new();
    };
    let mut indices = Vec::new();
    for (clause, _) in tcx.clauses_of(id).instantiate_identity(tcx) {
        if let ty::ClauseKind::Trait(predicate) = clause.skip_normalization().kind().skip_binder()
            && predicate.def_id() == copy
            && let ty::Param(param) = predicate.self_ty().kind()
            && !indices.contains(&param.index)
        {
            indices.push(param.index);
        }
    }
    indices
}

/// Is a copy of `ty` the value itself, whatever changes: a shared
/// reference, a number, `bool`, text, a function, a closure that changes
/// nothing it captured, and an `Option` of one?
fn copies_itself<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Ref(_, _, Mutability::Not) => true,
        ty::Bool | ty::Char | ty::Int(_) | ty::Uint(_) | ty::Float(_) | ty::Str | ty::Never => true,
        ty::FnDef(..) | ty::FnPtr(..) => true,
        // A closure that changes what it captured has copies of its own,
        // which rust-js refuses where it's copied (ADR 0246).
        ty::Closure(_, args) => args.as_closure().kind() == ty::ClosureKind::Fn,
        ty::Tuple(items) => items.is_empty(),
        ty::Adt(adt, args) if tcx.is_lang_item(adt.did(), LangItem::Option) => copies_itself(tcx, args.type_at(0)),
        _ => false,
    }
}
