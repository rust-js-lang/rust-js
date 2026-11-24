//! Which of a trait's provided methods an impl in reach writes itself
//! (#2414): std's, the crate's or a library's, every impl rustc loads. Where
//! one does, a call of it on a type parameter may reach that impl, so it
//! goes through the dictionary, and each dictionary of the trait carries it,
//! the impl's own or the default (`in_std_dictionary`); where none does,
//! the default over `Self` is what every call runs, which a call may call
//! itself (traits/calls.rs). The one answer of both.

use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// Whether an impl rustc loads of `provided`'s trait writes its own
/// `provided`, a trait's provided method: std's `Vec` its
/// `Extend::extend_reserve`, std's `&mut B` its `BufRead::read_line`, a
/// crate's iterator its `partition`. A crate built on this one, which may
/// write one too, its manifest says (#2414's second step).
pub(crate) fn overridden_in_reach(tcx: TyCtxt<'_>, provided: DefId) -> bool {
    let caches = crate::session::caches(tcx);
    if let Some(answer) = caches.overridden.get(&provided) {
        return answer;
    }
    let answer = tcx.trait_of_assoc(provided).is_some_and(|trait_id| {
        tcx.defaultness(provided).has_value()
            && tcx
                .all_impls(trait_id)
                .any(|imp| tcx.impl_item_implementor_ids(imp).contains_key(&provided))
    });
    caches.overridden.keep(provided, answer)
}

/// Whether generic code's call of `provided`, a provided method of
/// `Iterator`, is Rust's dispatch on the steps it holds, `$provided` (#2414,
/// #2421): an impl of the crate's or a library's writes its own of it, and
/// it's none of the steps a JS iterator carries of its own already
/// (`OwnStep`, `ownFold`; `size_hint`, its `hint`). std's own impls' are
/// what their JS forms step as, and give what the default does, so they ask
/// none. The one answer of where a call dispatches and of what a crate's
/// iterator carries (`own_provided`).
pub(crate) fn dispatched_override(tcx: TyCtxt<'_>, provided: DefId) -> bool {
    let iterator = super::std_item(tcx, super::StdItem::Iterator);
    let name = tcx.item_name(provided);
    tcx.trait_of_assoc(provided) == Some(iterator)
        && tcx.defaultness(provided).has_value()
        && super::OwnStep::of_method(name.as_str()).is_none()
        && name.as_str() != "size_hint"
        && tcx
            .all_impls(iterator)
            .any(|imp| !super::is_std_crate(tcx, imp) && tcx.impl_item_implementor_ids(imp).contains_key(&provided))
}

/// Whether generic code's call of `Iterator`'s provided method `id` of
/// `self_ty`, its `Self`, is Rust's dispatch on the steps it holds: `Self`
/// a type parameter, never a `&mut I` or a `Box<I>`, which pass none on, as
/// std's don't, where an impl not std's writes its own
/// (`dispatched_override`, #2414, #2421). The one answer of the lowering's
/// (`provided_dispatches`) and of the facts analysis's, which asks what the
/// dispatched call's own bounds' dictionaries ask, a `collect()`'s
/// `FromIterator` its override's entry is given (`collects_run`, #2430).
pub(crate) fn dispatched_call<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, self_ty: rustc_middle::ty::Ty<'tcx>) -> bool {
    crate::generic_facts::is_type_parameter(self_ty) && dispatched_override(tcx, id)
}
