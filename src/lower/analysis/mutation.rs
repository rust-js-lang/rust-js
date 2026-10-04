//! The types something changes in place, which a copy is made of where
//! it's read (ADR 0052).

use crate::lower::recognition::replaces_whole;
use crate::lower::{Body, strip};
use rustc_ast::Mutability;
use rustc_hir::LangItem;
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty;
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::DesugaringKind;
use std::collections::HashSet;

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
