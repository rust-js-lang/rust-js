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
