//! A type parameter bound by a trait whose `#[rust_js::types]` says what a
//! type it bounds is (ADR 0196, amended, #1866): react's `ReactNode`, a
//! union's trait, webapi's `IsA<T>`, its `T`. Where the parameter is one
//! value's type alone, written once among a function's parameters or a
//! struct's fields, it only spares boxing that value, so it's erased to the
//! type, `children: ReactNode` of `children: C` (a7ca4006e); where its
//! identity shows, in a function's result, written twice, or in another
//! parameter's bound, `F: Fn(&T)`, it's TypeScript's constraint, so the
//! caller's type is kept: `selected<T extends Element>(value: T): T`.

use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

use super::{type_maps::is_type_map, written::written_types};
use crate::lower::generic_facts::{type_parameter_count, type_parameter_of};

/// Each of `def_id`'s type parameters a binding trait that says what it
/// is bounds, by its index, with the bound: `(0, ReactNode)` of `fn
/// Tag<C: ReactNode>`, `(0, IsA<Element>)` of `fn selected<T: IsA<Element>>`.
fn bounded<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> Vec<(u32, ty::TraitRef<'tcx>)> {
    (tcx.clauses_of(def_id).clauses.iter())
        .filter_map(|(clause, _)| clause.as_trait_clause())
        .map(|bound| bound.skip_binder().trait_ref)
        .filter(|bound| written_types(tcx, bound.def_id).is_some())
        .filter_map(|bound| Some((type_parameter_of(bound.self_ty())?, bound)))
        .collect()
}

/// Those of `bounded`'s parameters erased to their bound's type, of no
/// identity the signature shows.
pub(super) fn erasures<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> Vec<(u32, ty::TraitRef<'tcx>)> {
    (bounded(tcx, def_id).into_iter())
        .filter(|&(index, bound)| !identity_shows(tcx, def_id, index, bound.def_id))
        .collect()
}

/// Those of `bounded`'s parameters declared as TypeScript's constraint,
/// `T extends Element`, whose identity the signature shows.
pub(super) fn constraints<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> Vec<(u32, ty::TraitRef<'tcx>)> {
    (bounded(tcx, def_id).into_iter())
        .filter(|&(index, bound)| identity_shows(tcx, def_id, index, bound.def_id))
        .collect()
}

/// Whether type parameter `index`'s identity shows in `def_id`'s
/// signature: in a function's result, or written more than once among its
/// parameters, a struct's fields, or another parameter's bound.
fn identity_shows(tcx: TyCtxt<'_>, def_id: DefId, index: u32, bound: DefId) -> bool {
    let count = |ty: Ty<'_>| type_parameter_count(ty, index);
    // In what another of its parameters is bound by, `F: Fn(&T) -> R`'s
    // `&T`, which is declared with it, `(value: T) => R`: the bounds a
    // declaration prints are a function's, each of which has its `Output`'s
    // projection, of its arguments and its result, `<F as FnOnce<(&T,)>>
    // ::Output == R`; any other bound TypeScript doesn't say.
    let in_bounds: usize = (tcx.clauses_of(def_id).clauses.iter())
        .map(|(clause, _)| clause.kind().skip_binder())
        .map(|kind| match kind {
            ty::ClauseKind::Projection(projection) if !is_param(projection.self_ty(), index) => {
                let given: usize = projection.projection_term.args.types().skip(1).map(count).sum();
                given + projection.term.as_type().map_or(0, count)
            }
            _ => 0,
        })
        .sum();
    match tcx.def_kind(def_id) {
        DefKind::Fn | DefKind::AssocFn => {
            let sig = tcx
                .fn_sig(def_id)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder();
            count(sig.output()) > 0 || sig.inputs().iter().map(|&ty| count(ty)).sum::<usize>() + in_bounds > 1
        }
        DefKind::Struct | DefKind::Enum | DefKind::Union => {
            let fields: usize = (tcx.adt_def(def_id).all_fields())
                .map(|field| count(tcx.type_of(field.did).instantiate_identity().skip_normalization()))
                .sum();
            fields + in_bounds > 1 || fields > 0 && is_type_map(tcx, bound)
        }
        _ => false,
    }
}

/// Whether `ty` is type parameter `index`.
fn is_param(ty: Ty<'_>, index: u32) -> bool {
    type_parameter_of(ty) == Some(index)
}
