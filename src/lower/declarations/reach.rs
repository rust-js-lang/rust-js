//! What JS reaches of the crate's items (#2024, ADR 1179): what it can
//! name, as its module's JS exports it (`js_names`). Beside what the
//! declarations declare, whose `.d.ts` is what a JS caller can name.

use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// Whether JS can name `id`, an item of the crate's: what its module's JS
/// exports, private module or not, as a function is (`exported_function`,
/// #2025): so a JS caller may write a `pub` field of a type it names, read
/// or write a `pub` static or thread-local key, and call a `pub` trait's
/// impl through its exported dictionary (#2024). Another crate's reach of
/// an item (`library::escapes`) is within it, as what it reaches is `pub`.
/// A field is named only of a type that is: a value an export hands JS of a
/// type it can't name it holds and gives back as it was given, plain data
/// (ADR 0262's difference 60), writing only what its `.d.ts` names.
pub(in crate::lower) fn js_names(tcx: TyCtxt<'_>, id: DefId) -> bool {
    match tcx.def_kind(id) {
        DefKind::Fn | DefKind::AssocFn if tcx.trait_impl_of_assoc(id).is_none() => {
            crate::lower::analysis::exported_function(tcx, id)
        }
        // A trait impl's method, through its trait's exported dictionary.
        DefKind::AssocFn => tcx
            .trait_impl_of_assoc(id)
            .and_then(|imp| tcx.impl_opt_trait_id(imp))
            .or_else(|| tcx.trait_of_assoc(id))
            .is_some_and(|tr| js_names(tcx, tr)),
        DefKind::Field => {
            let of = tcx.parent(id);
            let of = match tcx.def_kind(of) {
                DefKind::Variant => tcx.parent(of),
                _ => of,
            };
            tcx.visibility(id).is_public() && js_names(tcx, of)
        }
        _ => tcx.visibility(id).is_public(),
    }
}
