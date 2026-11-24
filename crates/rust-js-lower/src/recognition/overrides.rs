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

/// Whether `ty`, of which an override of `provided` asks a fact, in the
/// terms of `provided`'s own parameters, is built of the method's own type
/// parameters alone, `B` of `collect<B>`'s `size_of::<B>()`, which only a
/// call knows: a fact of it is the dispatched call's to give the override's
/// entry (`provided_fact_convention`), and its caller's to give the call
/// where it's the caller's own (the facts analysis's, #2449). Not one that
/// names the impl's own parameters too, which the entry's maker gives.
pub(crate) fn given_by_dispatch<'tcx>(tcx: TyCtxt<'tcx>, provided: DefId, ty: rustc_middle::ty::Ty<'tcx>) -> bool {
    let generics = tcx.generics_of(provided);
    let own: Vec<rustc_middle::ty::Ty<'tcx>> = (generics.own_params.iter())
        .filter_map(|param| tcx.mk_param_from_def(param).as_type())
        .collect();
    let params: Vec<rustc_middle::ty::Ty<'tcx>> = (ty.walk())
        .filter_map(|part| part.as_type())
        .filter(|part| crate::generic_facts::is_type_parameter(*part))
        .collect();
    !params.is_empty() && params.iter().all(|param| own.contains(param))
}

/// An override in reach of a trait's provided method (`overrides_in_reach`):
/// its method, its impl's type, and its instance's arguments.
#[derive(Clone, Copy)]
pub(crate) struct Override<'tcx> {
    pub(crate) method: DefId,
    pub(crate) self_ty: rustc_middle::ty::Ty<'tcx>,
    pub(crate) args: rustc_middle::ty::GenericArgsRef<'tcx>,
}

/// Each override in reach of `provided`, a trait's provided method: each
/// impl not std's that writes it, its method, the impl's type, and the
/// instance's arguments `provided` resolves to of that type, the method's
/// own parameters as `provided`'s, so what the method asks of them is in
/// `provided`'s own terms. The one list of the lowering's calling convention
/// (`provided_fact_convention`) and of the facts analysis's relay of what
/// a dispatched call gives its overrides (#2449).
pub(crate) fn overrides_in_reach<'tcx>(tcx: TyCtxt<'tcx>, provided: DefId) -> Vec<Override<'tcx>> {
    use rustc_middle::ty;
    let Some(trait_id) = tcx.trait_of_assoc(provided) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for imp in tcx.all_impls(trait_id) {
        if super::is_std_crate(tcx, imp) {
            continue;
        }
        let Some(&method) = tcx.impl_item_implementor_ids(imp).get(&provided) else {
            continue;
        };
        let self_ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
        let args = ty::GenericArgs::for_item(tcx, provided, |param, _| match param.kind {
            ty::GenericParamDefKind::Lifetime => tcx.lifetimes.re_erased.into(),
            _ if param.index == 0 => self_ty.into(),
            _ => tcx.mk_param_from_def(param),
        });
        if let Ok(Some(instance)) =
            crate::traits::resolve(tcx, ty::TypingEnv::non_body_analysis(tcx, method), provided, args)
        {
            found.push(Override {
                method,
                self_ty,
                args: instance.args,
            });
        }
    }
    found
}
