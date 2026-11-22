//! What JS reaches of the crate's items (#2024, ADR 1179): what it can
//! name, as its module's JS exports it (`js_names`), and the types it may
//! hold a value of and give back (`js_holds`). Beside what the
//! declarations declare, whose `.d.ts` is what a JS caller can name.

use rust_js_program::collections::IndexSet;
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;
use std::rc::Rc;

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

/// Whether JS may hold a value of `adt`, a struct, an enum or a union of
/// the crate's, and give it back (#2024 part 2, ADR 1179): one it names, or
/// one an item it names hands it, in a function's signature, a named
/// field's type or a static's, through what each is made of. rustc's
/// `is_reachable` asks it of the crate's API, which leaves out a `pub` item
/// of a private module its module's JS exports (#2025). Found once a
/// session (`js_held`).
pub(in crate::lower) fn js_holds(tcx: TyCtxt<'_>, adt: DefId) -> bool {
    let caches = crate::lower::session::caches(tcx);
    caches.js_held.get_or_init(|| Rc::new(js_held(tcx))).contains(&adt)
}

/// Each of the crate's types JS may hold (`js_holds`): of each item JS
/// names, each type of the crate's its signature, its fields JS names, or
/// its type, is made of.
fn js_held(tcx: TyCtxt<'_>) -> IndexSet<DefId> {
    let mut held = IndexSet::default();
    let mut give = |ty: Ty<'_>| {
        for part in ty.walk().filter_map(|part| part.as_type()) {
            if let ty::Adt(adt, _) = part.kind()
                && adt.did().is_local()
            {
                held.insert(adt.did());
            }
        }
    };
    for local in tcx.hir_crate_items(()).definitions() {
        let id = local.to_def_id();
        if !js_names(tcx, id) {
            continue;
        }
        match tcx.def_kind(id) {
            DefKind::Fn | DefKind::AssocFn => {
                let sig = tcx
                    .instantiate_bound_regions_with_erased(tcx.fn_sig(id).instantiate_identity().skip_normalization());
                for ty in sig.inputs_and_output {
                    give(ty);
                }
            }
            DefKind::Struct | DefKind::Enum | DefKind::Union => {
                give(tcx.type_of(id).instantiate_identity().skip_normalization());
                for field in tcx.adt_def(id).all_fields() {
                    if js_names(tcx, field.did) {
                        give(tcx.type_of(field.did).instantiate_identity().skip_normalization());
                    }
                }
            }
            DefKind::Static { .. } => give(tcx.type_of(id).instantiate_identity().skip_normalization()),
            _ => {}
        }
    }
    held
}
