//! The sorts std's slices make, and what of their items' types picks how
//! std sorts them (ADRs 0342, 0471; #925).

use super::*;
use rustc_middle::ty::TypeVisitableExt;

/// The items of a slice `def_id`, std's, sorts by std's unstable algorithm,
/// of its `args`, whose order its layout picks, which a caller of a generic
/// one gives, `TSort` (ADR 0471).
pub(crate) fn unstable_sort_items<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<Ty<'tcx>> {
    let imp = tcx.impl_of_assoc(def_id)?;
    (is_std_crate(tcx, def_id)
        && tcx.type_of(imp).skip_binder().is_slice()
        && unstable_sort(tcx.item_name(def_id).as_str()).is_some())
    .then(|| args.types().next())
    .flatten()
}

/// The items of a stable sort, `sort`, `sort_by` or `sort_by_key` of a
/// slice, std's `def_id` given `args`: its `T`, whose caller's type picks
/// how std sorts them, `TStableSort` (#925).
pub(crate) fn stable_sort_items<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<Ty<'tcx>> {
    let imp = tcx.impl_of_assoc(def_id)?;
    (is_std_crate(tcx, def_id)
        && tcx.type_of(imp).skip_binder().is_slice()
        && matches!(tcx.item_name(def_id).as_str(), "sort" | "sort_by" | "sort_by_key"))
    .then(|| args.types().next())
    .flatten()
}

/// The pairs a slice's `sort_by_cached_key`, std's `def_id` given `args`,
/// sorts unstably, `(K, usize)` of its key `K`, where `K` holds a type
/// parameter, whose caller's type picks how std sorts them (#925).
pub(crate) fn cached_key_pairs<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<Ty<'tcx>> {
    let imp = tcx.impl_of_assoc(def_id)?;
    let key = args.types().nth(1)?;
    (is_std_crate(tcx, def_id)
        && tcx.type_of(imp).skip_binder().is_slice()
        && tcx.item_name(def_id).as_str() == "sort_by_cached_key"
        && key.has_param())
    .then(|| Ty::new_tup(tcx, &[key, tcx.types.usize]))
}

/// Whether `def_id` is std's `max` or `min` function, `std::cmp::max(a, b)`.
pub(crate) fn is_cmp_max_or_min(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(
        tcx.get_diagnostic_name(def_id).as_ref().map(Symbol::as_str),
        Some("cmp_max" | "cmp_min")
    )
}

/// Whether `trait_id` is std's `Iterator`.
pub(crate) fn is_iterator_trait(tcx: TyCtxt<'_>, trait_id: DefId) -> bool {
    tcx.is_diagnostic_item(sym::Iterator, trait_id)
}
