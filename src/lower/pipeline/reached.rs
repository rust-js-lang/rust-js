//! What's written only where something reads it: derived impls, a marker
//! trait's dictionaries, those only generic code reads, a `Hash`'s among
//! them, a `PartialEq` whose `==` is `===`, a library's the crate compiles
//! itself, and a fieldless enum's own names of the crate alone, only called.

use super::super::Body;
use super::super::library::Foreign;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// Of the crate's `trait_impls` and its lowered functions, `lowered`: the
/// types equal only to themselves, which its contract lists for its
/// consumers (ADR 0864, amended); and what's written only where it's
/// reached. Derived Debug implementations, a marker trait's dictionaries,
/// which only a `dyn` reads (ADR 0307), and those only generic code reads,
/// their methods called as they are anywhere else (`read_only_generically`),
/// a `Hash`'s and a `Hasher`'s among them, which a map never calls (ADR
/// 0463); a `PartialEq` of a type equal only to itself, whose `==` is `===`
/// and whose sets and maps are JS's (ADR 0864, amended); and a function each
/// call of which is what it's given, a fieldless enum's `name(self)`
/// matching each variant to its name, which no other crate reaches,
/// written where something reads it as a value, as its calls don't (ADR
/// 0264, amended).
pub(super) fn written_where_reached<'tcx>(
    tcx: TyCtxt<'tcx>,
    (trait_impls, foreign): (&[DefId], &Foreign<'_, 'tcx>),
    function_bodies: &HashMap<DefId, &Body<'tcx>>,
    lowered: impl Iterator<Item = DefId>,
) -> ((Vec<DefId>, Vec<DefId>), HashSet<DefId>) {
    let recognition = super::super::recognition::Recognition {
        tcx,
        typing_env: ty::TypingEnv::fully_monomorphized(),
        trait_impls,
        foreign,
    };
    // A `PartialEq` whose `==` is `===`, whatever its `Hash`; and of a
    // type equal only to itself, whose `Hash` is its address's too.
    let identity_eq = |id: DefId, trait_id: DefId, keyed: bool| {
        let ty = tcx.type_of(id).instantiate_identity().skip_normalization();
        tcx.is_lang_item(trait_id, rustc_hir::attrs::lang_items::LangItem::PartialEq)
            && match keyed {
                true => super::super::std_impls::equal_only_to_itself(&recognition, function_bodies, ty),
                false => super::super::std_impls::eq_by_address(&recognition, function_bodies, ty),
            }
    };
    let trait_of = |id: DefId| {
        tcx.impl_trait_ref(id)
            .instantiate_identity()
            .skip_normalization()
            .def_id
    };
    let of_impls = |keyed: bool| -> Vec<DefId> {
        (trait_impls.iter())
            .filter(|&&id| identity_eq(id, trait_of(id), keyed))
            .filter_map(|&id| tcx.type_of(id).instantiate_identity().skip_normalization().ty_adt_def())
            .map(|adt| adt.did())
            .collect()
    };
    let identities = (of_impls(true), of_impls(false));
    let derived: HashSet<DefId> = trait_impls
        .iter()
        .filter(|&&id| {
            let trait_id = trait_of(id);
            super::super::recognition::known_derive(tcx, id)
                || super::super::traits::is_marker(tcx, foreign, trait_id)
                || identity_eq(id, trait_id, false)
                // A library's the crate compiles itself, only what it uses
                // of it (ADR 0914, amended).
                || foreign.is_instance(id)
        })
        .flat_map(|&id| std::iter::once(id).chain(tcx.associated_item_def_ids(id).iter().copied()))
        // And their methods, each called as it is, by its name, where
        // anything calls it: `From<Lane>`'s `from` nothing calls is
        // written nowhere (ADR 0477, amended).
        .chain(
            (trait_impls.iter().copied())
                .filter(|&id| super::super::traits::read_only_generically(tcx, trait_of(id)))
                .flat_map(|id| std::iter::once(id).chain(tcx.associated_item_def_ids(id).iter().copied())),
        )
        .chain(lowered.filter(|&id| {
            id.is_local()
                && !super::super::library::reachable(tcx, id)
                && super::super::patterns::gives_its_argument(tcx, function_bodies, id)
        }))
        .collect();
    (identities, derived)
}

/// Whether `id`, a body of the crate's, is lowered as a function of its
/// own. Not a trait's default, copied into each impl (ADR 0049), but a
/// library's, which a consumer's impl of its trait calls, a function over
/// `Self`, the consumer's own (ADRs 0185, 0914, amended), and std's,
/// compiled into `@rust-js/std`, its own too (ADR 0561); nor a trait of
/// classes' method that gives a field, which is that field, read in place
/// wherever it's called (rd304, ADR 1038); nor a namespace's method that's
/// the function it passes its parameters on to (`forwarded`, rd292b).
pub(super) fn lowered_here(
    tcx: TyCtxt<'_>,
    (foreign, forwarded): (&Foreign<'_, '_>, &HashMap<DefId, DefId>),
    exports_reachable: bool,
    id: DefId,
) -> bool {
    if super::super::classes::field_accessor(tcx, id).is_some() || forwarded.contains_key(&id) {
        return false;
    }
    tcx.trait_of_assoc(id).is_none()
        || foreign.is_instance(id)
        || (exports_reachable && super::super::library::reachable(tcx, id))
}

/// Each namespace's method of the crate that's the function of another of
/// its modules it only passes its parameters on to
/// (`passed_on::forwarded_function`, rd292b, ADR 1037 amended), by that
/// function: one its module re-exports by the method's name, `export {
/// batchedUpdates } from "./ReactFiberWorkLoop.js"`, where the two take
/// what JS gives them alike, their parameters and, as they're of the same
/// generic parameters, each given alike, what's given of those: its
/// caller's location of both or neither, and the dictionaries of the same
/// bounds; a type's facts and drops each asks of what it passes on alike.
/// Not one its callers need a copy of (ADR 0051), which is its own.
pub(super) fn forwarded<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[&Body<'tcx>],
    (given, js_given): (super::super::traits::Given<'_>, &super::super::analysis::JsGiven),
    caller_locations: &HashSet<DefId>,
    foreign: &Foreign<'_, 'tcx>,
) -> HashMap<DefId, DefId> {
    let bounds = |id: DefId| super::super::traits::bounds(tcx, foreign, given, id);
    (bodies.iter())
        .filter_map(|body| {
            let method = body.def_id;
            let (target, args) = super::super::passed_on::forwarded_function(tcx, method, &body.thir)?;
            let theirs: Vec<_> = (bounds(target).into_iter())
                .map(|bound| {
                    ty::EarlyBinder::bind(tcx, bound)
                        .instantiate(tcx, args)
                        .skip_normalization()
                })
                .collect();
            (caller_locations.contains(&method) == caller_locations.contains(&target)
                && bounds(method) == theirs
                && js_given.copies(method).next().is_none())
            .then_some((method, target))
        })
        .collect()
}
