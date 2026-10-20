//! The types the crate clones (ADRs 0522, 0525): each a `Clone` bound of a
//! function called is given, `Clone::clone`'s own `Self: Clone` too, and
//! what cloning each clones in turn, its `Clone` impl's bounds, as rustc
//! proves them: `Map<I, F>: Clone` clones its `I` and its `F`.

use crate::lower::Body;
use rustc_hir::def_id::DefId;
use rustc_middle::thir::{ExprKind, Thir};
use rustc_middle::traits::ImplSource;
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use std::collections::HashSet;

/// The types `all_bodies` clone, regions erased, and those their clones
/// clone, to a fixed point.
pub(super) fn cloned_types<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> HashSet<Ty<'tcx>> {
    let mut cloned: HashSet<Ty<'tcx>> = HashSet::new();
    // What an `impl Clone` hides, revealed: a caller's clone of what a
    // function gives is of what that function makes.
    let mut todo: Vec<Ty<'tcx>> = (all_bodies.iter())
        .flat_map(|body| {
            let typing_env = ty::TypingEnv::post_analysis(tcx, body.def_id);
            called_clones(tcx, &body.thir)
                .map(move |ty| {
                    (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))).unwrap_or(ty)
                })
                .collect::<Vec<_>>()
        })
        .collect();
    while let Some(ty) = todo.pop() {
        let ty = tcx.erase_and_anonymize_regions(ty);
        if cloned.insert(ty) {
            todo.extend(clones_in_turn(tcx, ty));
        }
    }
    cloned
}

/// Whether `id` is `Clone`, or a trait that extends it.
fn is_clone(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let clone = tcx.lang_items().clone_trait();
    Some(id) == clone || ty::elaborate::supertrait_def_ids(tcx, id).any(|s| Some(s) == clone)
}

/// The types `thir`'s calls clone: of each, each type a `Clone` bound of
/// the function called, or of a trait that extends it, is given.
fn called_clones<'a, 'tcx>(tcx: TyCtxt<'tcx>, thir: &'a Thir<'tcx>) -> impl Iterator<Item = Ty<'tcx>> + 'a {
    thir.exprs.iter().flat_map(move |expr| {
        let ExprKind::Call { fun, .. } = expr.kind else {
            return Vec::new();
        };
        let ty::FnDef(id, args) = *thir[fun].ty.kind() else {
            return Vec::new();
        };
        (tcx.clauses_of(id).instantiate(tcx, args.skip_binder()).into_iter())
            .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
            .filter(|p| is_clone(tcx, p.def_id()))
            .map(|p| p.self_ty().skip_binder())
            .collect()
    })
}

/// What a clone of `ty` clones: its `Clone` impl's bounds' types, or of
/// what's cloned field by field, a tuple's, an array's or a closure's, each
/// part. Of a type a caller knows, nothing here: the caller's clone gives it.
fn clones_in_turn<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<Ty<'tcx>> {
    match ty.kind() {
        ty::Tuple(parts) => parts.to_vec(),
        ty::Array(item, _) => vec![*item],
        ty::Closure(_, args) => args.as_closure().upvar_tys().to_vec(),
        _ if ty.has_non_region_param() || ty.has_aliases() => Vec::new(),
        _ => {
            let Some(clone) = tcx.lang_items().clone_trait() else {
                return Vec::new();
            };
            let typing_env = ty::TypingEnv::fully_monomorphized();
            let tr = ty::TraitRef::new(tcx, clone, [ty]);
            let Ok(ImplSource::UserDefined(imp)) = tcx.codegen_select_candidate(typing_env.as_query_input(tr)) else {
                return Vec::new();
            };
            (tcx.clauses_of(imp.impl_def_id).instantiate(tcx, imp.args).into_iter())
                .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
                .filter(|p| is_clone(tcx, p.def_id()))
                .map(|p| p.self_ty().skip_binder())
                .collect()
        }
    }
}
