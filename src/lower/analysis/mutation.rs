//! The types something changes in place, which a copy is made of where
//! it's read (ADR 0052).

use crate::lower::recognition::{flattens_mut, replaces_whole};
use crate::lower::{Body, FnInfo, fn_def, strip};
use rustc_ast::Mutability;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty;
use rustc_middle::ty::{Ty, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// The types whose JS objects get changed in place somewhere in the crate:
/// `a.b.c = ..` changes the object `a.b`, so it's `a.b`'s type. Only these
/// ever need copying (ADR 0020).
pub(super) fn mutated_types<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> HashSet<Ty<'tcx>> {
    let mut mutated = HashSet::new();
    for body in all_bodies {
        // A `let`'s pattern's too, `let (a, _) = &mut t;`.
        for stmt in body.thir.stmts.iter() {
            if let rustc_middle::thir::StmtKind::Let { ref pattern, .. } = stmt.kind {
                ref_mut_holders(tcx, pattern, &mut mutated);
            }
        }
        for expr in body.thir.exprs.iter() {
            // An enum something takes `&mut` of may have a variant's field
            // changed through it (and one matched with a `ref mut` binding,
            // `ref_mut_holders`, below). Not an `Option`, whose `Some(x)` is
            // `x` (ADR 0033): a `&mut` of one is a handle on its place, and
            // what its `Some` holds is changed as itself.
            let enum_ty = |ty: Ty<'tcx>| {
                let ty = ty.peel_refs();
                matches!(ty.kind(), ty::Adt(adt, _) if adt.is_enum()) && object_like(tcx, ty)
            };
            match expr.kind {
                ExprKind::Borrow {
                    borrow_kind: BorrowKind::Mut { .. },
                    arg,
                } if enum_ty(body.thir[arg].ty) => {
                    mutated.insert(body.thir[arg].ty.peel_refs());
                }
                // An array something takes `&mut` of may have its items changed
                // through it, as a `&mut [T]` std's slice methods, `iter_mut()`
                // or a function of a slice changes them by.
                ExprKind::Borrow {
                    borrow_kind: BorrowKind::Mut { .. },
                    arg,
                } if matches!(body.thir[arg].ty.kind(), ty::Array(..)) => {
                    mutated.insert(body.thir[arg].ty);
                }
                // And `as_flattened_mut()`, the items of each array of a slice
                // of arrays, which it changes through: each array's.
                ExprKind::Call { fun, ref args, .. }
                    if fn_def(body.thir[fun].ty).is_some_and(|(id, _)| flattens_mut(tcx, id))
                        && let [rows] = &args[..]
                        && let ty::Slice(row) | ty::Array(row, _) = body.thir[*rows].ty.peel_refs().kind() =>
                {
                    mutated.insert(*row);
                }
                _ => {}
            }
            // A `ref mut` binding of a pattern, through a `&mut` subject too,
            // `let (a, _) = &mut t`, may change the field it binds, of the
            // tuple or struct it's in, at any depth.
            match expr.kind {
                ExprKind::Match { ref arms, .. } => {
                    for &arm in arms.iter() {
                        ref_mut_holders(tcx, &body.thir[arm].pattern, &mut mutated);
                    }
                }
                ExprKind::Let { ref pat, .. } => {
                    ref_mut_holders(tcx, pat, &mut mutated);
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
            // What a raw pointer points to has a place of its own, which a copy
            // shared with another wouldn't be (ADR 0407): copied as if changed.
            // Not std's own, inside a macro of its, `thread_local!`'s.
            if let Some((pointee, _)) = crate::lower::pointers::raw_pointee(tcx, expr.ty)
                && !expr.span.in_external_macro(tcx.sess.source_map())
                && object_like(tcx, pointee)
            {
                mutated.insert(pointee);
            }
            // `a[i] = ..` changes the array `a` the same way, and so does a
            // `&mut` of a part that's its place, text or a number, not an
            // object changed in place: `a.0.push('!')`'s, `a[0] += "!"`.
            let place = match expr.kind {
                ExprKind::Assign { lhs, .. } | ExprKind::AssignOp { lhs, .. } => Some(lhs),
                ExprKind::Borrow {
                    borrow_kind: BorrowKind::Mut { .. },
                    arg,
                } if !object_like(tcx, body.thir[arg].ty) => Some(arg),
                _ => None,
            };
            if let Some(place) = place
                && let ExprKind::Field { lhs: object, .. } | ExprKind::Index { lhs: object, .. } =
                    body.thir[strip(&body.thir, place)].kind
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

/// Each object-like type a `ref mut` binding of `pat` is a part of, which
/// a write through it changes in place, into `mutated`, an enum's variant's
/// too; and whether `pat` holds one.
fn ref_mut_holders<'tcx>(
    tcx: TyCtxt<'tcx>,
    pat: &rustc_middle::thir::Pat<'tcx>,
    mutated: &mut HashSet<Ty<'tcx>>,
) -> bool {
    use rustc_middle::thir::PatKind;
    let mut holds = false;
    let mut subpatterns: Vec<&rustc_middle::thir::Pat<'tcx>> = Vec::new();
    match &pat.kind {
        PatKind::Binding { mode, subpattern, .. } => {
            holds = matches!(mode.0, rustc_hir::ByRef::Yes(_, Mutability::Mut));
            subpatterns.extend(subpattern.as_deref());
        }
        PatKind::Leaf { subpatterns: fields }
        | PatKind::Variant {
            subpatterns: fields, ..
        } => {
            subpatterns.extend(fields.iter().map(|field| &field.pattern));
        }
        PatKind::Deref { subpattern, .. } => subpatterns.push(subpattern),
        PatKind::Slice { prefix, slice, suffix } | PatKind::Array { prefix, slice, suffix } => {
            subpatterns.extend(prefix.iter().chain(slice.as_deref()).chain(suffix.iter()));
        }
        PatKind::Or { pats } => subpatterns.extend(pats.iter()),
        _ => {}
    }
    let parts = subpatterns
        .into_iter()
        .fold(false, |any, sub| ref_mut_holders(tcx, sub, mutated) | any);
    if parts
        && !matches!(
            pat.kind,
            PatKind::Binding { .. } | PatKind::Deref { .. } | PatKind::Or { .. }
        )
        && object_like(tcx, pat.ty.peel_refs())
    {
        mutated.insert(pat.ty.peel_refs());
    }
    holds || parts
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
        let caller = tcx.typeck_root_def_id(body.def_id);
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
