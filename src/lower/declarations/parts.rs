//! What a type's parts are to its declaration: an `impl Fn`'s signature,
//! and a flattened struct's keys (ADR 0205).

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty, TyCtxt};

use super::super::bindings::{field_key, is_flatten};

/// What an `impl Fn(A, B) -> R` takes and gives, by its bounds, `Fn`'s and
/// its `Output`, of `alias`'s arguments: `None` of another `impl Trait`.
pub(super) fn opaque_fn_signature<'tcx>(
    tcx: TyCtxt<'tcx>,
    alias: ty::AliasTy<'tcx>,
) -> Option<(Vec<Ty<'tcx>>, Ty<'tcx>)> {
    let mut inputs = None;
    let mut output = tcx.types.unit;
    let ty::Opaque { def_id } = alias.kind else { return None };
    let bounds = tcx
        .explicit_item_bounds(def_id)
        .iter_instantiated_copied(tcx, alias.args);
    for (clause, _) in bounds.map(|bound| bound.skip_normalization()) {
        if let Some(bound) = clause.as_trait_clause().map(|b| b.skip_binder())
            && tcx.fn_trait_kind_from_def_id(bound.def_id()).is_some()
            && let ty::Tuple(items) = bound.trait_ref.args.type_at(1).kind()
        {
            inputs = Some(items.to_vec());
        }
        if let Some(projection) = clause.as_projection_clause().map(|p| p.skip_binder())
            && tcx.is_lang_item(projection.def_id(), LangItem::FnOnceOutput)
            && let Some(given) = projection.term.as_type()
        {
            output = given;
        }
    }
    Some((inputs?, output))
}

/// The JS names of `ty`'s fields, a flattened struct's, or one's it
/// refers to, and of those of the flattened struct in it (ADR 0205).
pub(super) fn flattened_keys<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Vec<String> {
    let ty::Adt(adt, args) = ty.peel_refs().kind() else {
        return Vec::new();
    };
    if !adt.is_struct() {
        return Vec::new();
    }
    let mut keys = Vec::new();
    for field in &adt.non_enum_variant().fields {
        if is_flatten(tcx, field) {
            keys.extend(flattened_keys(tcx, field.ty(tcx, args).skip_normalization()));
        } else {
            keys.push(field_key(tcx, field));
        }
    }
    keys
}
