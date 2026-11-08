//! The sorts std's slices make, and what of their items' types picks how
//! std sorts them (ADRs 0342, 0471; #925).

use super::*;
use rustc_middle::ty::TypeVisitableExt;

/// What a function a comparison calls is, of those whose answers make a
/// total order of total ones (#936).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum OrderingFn {
    /// `Ord::cmp`, of its `Self`.
    Cmp,
    /// A float's `total_cmp`.
    TotalCmp,
    /// JS's `localeCompare`, a builtins binding.
    LocaleCompare,
    /// `Ordering`'s own `reverse()`, `then(o)` and `then_with(f)`.
    Reverse,
    Then,
    ThenWith,
    /// `==` and `!=` of two `Ordering`s.
    Eq {
        negated: bool,
    },
}

/// `def_id` as an `OrderingFn`, if it's one.
pub(crate) fn ordering_fn(tcx: TyCtxt<'_>, def_id: DefId, args: ty::GenericArgsRef<'_>) -> Option<OrderingFn> {
    let name = tcx.item_name(def_id);
    let of_ordering =
        |ty: Ty<'_>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::OrderingEnum));
    let own = tcx
        .inherent_impl_of_assoc(def_id)
        .map(|imp| tcx.type_of(imp).skip_binder());
    let of_trait = tcx.trait_of_assoc(def_id);
    Some(match name.as_str() {
        "cmp" if of_trait == Some(std_item(tcx, StdItem::Ord)) => OrderingFn::Cmp,
        "total_cmp" if own.is_some_and(|own| own.is_floating_point()) => OrderingFn::TotalCmp,
        "reverse" if own.is_some_and(of_ordering) => OrderingFn::Reverse,
        "then" if own.is_some_and(of_ordering) => OrderingFn::Then,
        "then_with" if own.is_some_and(of_ordering) => OrderingFn::ThenWith,
        "eq" | "ne"
            if of_trait.is_some_and(|tr| tcx.is_lang_item(tr, LangItem::PartialEq))
                && args.types().next().is_some_and(|ty| of_ordering(ty.peel_refs())) =>
        {
            OrderingFn::Eq {
                negated: name.as_str() == "ne",
            }
        }
        _ if crate::lower::bindings::is_locale_compare(tcx, def_id) => OrderingFn::LocaleCompare,
        _ => return None,
    })
}

/// Whether the variant `variant` of `adt` is `Ordering::Equal`.
pub(crate) fn is_ordering_equal(tcx: TyCtxt<'_>, adt: ty::AdtDef<'_>, variant: rustc_abi::VariantIdx) -> bool {
    tcx.is_lang_item(adt.did(), LangItem::OrderingEnum) && adt.variant(variant).name.as_str() == "Equal"
}

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

/// The entries a B-tree's bulk build sorts, of a call of std's `def_id`
/// given `args` that makes a `BTreeMap` or a `BTreeSet` of items, its
/// `collect()`, `from_iter`, or `from` or `into` of an array: `(K, V)` of a
/// map's, `T` of a set's, where its key holds a type parameter, whose
/// caller's type picks how std's stable sort sorts them, `TStableSort` and
/// `TSize` (#926).
pub(crate) fn tree_built_entries<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<Ty<'tcx>> {
    let trait_id = tcx.trait_of_assoc(def_id)?;
    let made = match tcx.item_name(def_id).as_str() {
        "collect" if is_iterator_trait(tcx, trait_id) => args.types().nth(1)?,
        "from_iter" if trait_id == std_item(tcx, StdItem::FromIterator) => args.types().next()?,
        "from" if trait_id == std_item(tcx, StdItem::From) => args.types().next()?,
        "into" if trait_id == std_item(tcx, StdItem::Into) => args.types().nth(1)?,
        _ => return None,
    };
    let ty::Adt(adt, of) = made.kind() else { return None };
    // `alloc`'s, which a `#![no_std]` crate may not load (#983).
    let map = is_std_def(tcx, adt.did(), StdItem::BTreeMap);
    let set = is_std_def(tcx, adt.did(), StdItem::BTreeSet);
    let key = of.types().next()?;
    if !(map || set) || !key.has_param() {
        return None;
    }
    Some(match map {
        true => Ty::new_tup(tcx, &[key, of.types().nth(1)?]),
        false => key,
    })
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
