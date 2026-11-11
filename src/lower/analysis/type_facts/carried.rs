//! What a trait's dictionaries carry of associated types, which its
//! impls know where a generic caller doesn't: of the crate's own trait or
//! a library's, the facts its code asks or its manifest says (#476); of a
//! std supertrait's of a default's `Self`, `<Self as Iterator>::Item`'s
//! (#990). Split from type_facts.rs by concept, as a file is at most 1500
//! lines.

use crate::collections::IndexMap;
use crate::lower::recognition::TypeFact;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;

/// The trait and the std associated type of `of` where `caller` is one of
/// the trait's defaults and `of` is `<Self as Trait>::Item` of a std
/// supertrait of it, `Iterator::Item` of a `Tools: Iterator`'s (#990).
pub(super) fn default_super_item(tcx: TyCtxt<'_>, caller: DefId, of: ty::Ty<'_>) -> Option<(DefId, DefId)> {
    let trait_id = tcx
        .trait_of_assoc(caller)
        .filter(|_| tcx.defaultness(caller).has_value())?;
    let ty::Alias(
        _,
        alias @ ty::AliasTy {
            kind: ty::Projection { def_id },
            ..
        },
    ) = *of.kind()
    else {
        return None;
    };
    let own_self = matches!(alias.self_ty().kind(), ty::Param(param) if param.index == 0);
    (own_self && crate::lower::recognition::is_std_crate(tcx, def_id)).then_some((trait_id, def_id))
}

/// Of each of the crate's traits, the facts its dictionaries carry of an
/// associated type of a std supertrait of their `Self`, `IteratorItemSize`
/// of `<Self as Iterator>::Item` of a `Tools: Iterator` whose default asks
/// it (#990): by the trait, the associated type and the fact. Its manifest
/// says them, which a consumer's impls' dictionaries carry; a library's
/// trait's are its manifest's (`Foreign::carried_super`).
pub(in crate::lower) type CarriedSuper = IndexMap<DefId, Vec<(DefId, TypeFact)>>;

/// `<self_ty as Trait>::assoc`, where its bounds hold, of an impl's
/// `Self`: `<I as Iterator>::Item` of a `Wrap<I>` whose `Item` is `I`'s.
pub(in crate::lower) fn super_item_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    assoc: DefId,
    self_ty: ty::Ty<'tcx>,
) -> ty::Ty<'tcx> {
    let projection = ty::Ty::new_projection(tcx, ty::IsRigid::No, assoc, [self_ty]);
    tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(projection))
        .unwrap_or(projection)
}

/// `default_super_item` of `id`'s slot `index`.
pub(super) fn slot_super_item<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    id: DefId,
    index: u32,
) -> Option<(DefId, DefId)> {
    crate::lower::traits::slot_item(tcx, id, index)?;
    let slot = crate::lower::traits::drop_slot(tcx, (foreign, &Default::default()), id, index);
    default_super_item(tcx, id, slot)
}

/// The associated type of the crate's own trait or a library's `ty`
/// projects, whose facts the trait's dictionaries carry: `<A as
/// Array>::Item`. Not std's, whose facts a caller gives (ADR 0466).
pub(super) fn carried_by(foreign: &crate::lower::library::Foreign<'_, '_>, ty: ty::Ty<'_>) -> Option<DefId> {
    match ty.kind() {
        ty::Alias(
            _,
            ty::AliasTy {
                kind: ty::Projection { def_id },
                ..
            },
        ) if def_id.is_local() || foreign.is_library(def_id.krate) => Some(*def_id),
        _ => None,
    }
}

/// Whether the dictionaries of `assoc`'s trait carry `fact` of it: of the
/// crate's own trait, each its code asks; of a library's, only what its
/// manifest says they carry, which its consumer's code may read of any of
/// them, and the consumer's dictionaries of its own impls carry too
/// (`Foreign::carried`, #476). Its `Room` where it carries its `Size`, the
/// same key, `ItemSize` (#1260).
pub(super) fn carries(foreign: &crate::lower::library::Foreign<'_, '_>, assoc: DefId, fact: TypeFact) -> bool {
    let carried = foreign.carried(assoc);
    assoc.is_local() || carried.contains(&fact) || fact == TypeFact::Room && carried.contains(&TypeFact::Size)
}

/// What each of the crate's traits' dictionaries carry whatever asks it:
/// the constant size of each associated type that's an `IntoIterator`
/// its bounds don't make an `Iterator` (`const_size_unknown`),
/// which a flatten of what it gives asks, held, std's struct (ADR 0899),
/// wherever generic code flattens it, a consumer's included, which can't
/// add to a library's dictionaries: none but an array's, which is left
/// out (the user's decision through m6, #1341). And of a library, each of
/// its reachable traits' sized associated types' `Size` and `Align`, which
/// a consumer's generic code may ask, `size_of::<Self::Item>()` of rand_core's
/// `BlockRngCore` in rand's `ReseedingCore` (#1420), as its own may of its
/// own trait's: where each of the library's impls can give it
/// (`layouts_given`), else none, as before. Its manifest says so.
pub(super) fn carried_by_every_impl<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    library: bool,
) -> Vec<(DefId, TypeFact)> {
    let into_iterator = crate::lower::recognition::std_item(tcx, crate::lower::recognition::StdItem::IntoIterator);
    let traits =
        (tcx.hir_crate_items(()).definitions()).filter(|&id| tcx.def_kind(id) == rustc_hir::def::DefKind::Trait);
    let mut carried = Vec::new();
    for trait_id in traits {
        let typing_env = ty::TypingEnv::non_body_analysis(tcx, trait_id);
        for item in tcx.associated_items(trait_id).in_definition_order() {
            if !item.is_type() {
                continue;
            }
            let args = ty::GenericArgs::identity_for_item(tcx, item.def_id);
            let projection = ty::Ty::new_projection(tcx, ty::IsRigid::No, item.def_id, args);
            if crate::lower::recognition::trait_holds_in(tcx, typing_env, projection, into_iterator) == Some(true)
                && crate::lower::recognition::const_size_unknown(tcx, typing_env, projection)
            {
                carried.push((item.def_id, TypeFact::ConstSize));
            }
            if library
                && tcx.effective_visibilities(()).is_reachable(trait_id)
                && tcx.generics_of(item.def_id).own_params.is_empty()
                && projection.is_sized(tcx, typing_env)
                && layouts_given(tcx, foreign, trait_id.to_def_id(), item.def_id)
            {
                carried.extend([(item.def_id, TypeFact::Size), (item.def_id, TypeFact::Align)]);
            }
        }
    }
    carried
}

/// Whether each of the crate's impls of `trait_id` can give the layout of
/// its type for `assoc`: of a type known here, or of one of its slots its
/// makers give, or of a type built of them a library's contract can say,
/// which its consumers make (`Foreign::templates`). Not of
/// `x2<<W as AndNot>::Output, G>`, ppv-lite86's, of a projection of its own
/// trait, nor of another trait's associated type: carried, its impl would
/// be refused where it compiles now (#1420).
fn layouts_given<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    trait_id: DefId,
    assoc: DefId,
) -> bool {
    let impls = tcx
        .all_impls(trait_id)
        .filter(|&imp| crate::lower::traits::is_lowered_impl(foreign, imp));
    impls.into_iter().all(|imp| {
        let Some(&item) = tcx.impl_item_implementor_ids(imp).get(&assoc) else {
            return false;
        };
        let ty = tcx.type_of(item).instantiate_identity().skip_normalization();
        let typing_env = ty::TypingEnv::non_body_analysis(tcx, imp);
        [TypeFact::Size, TypeFact::Align].into_iter().all(|fact| {
            match super::asked_of(tcx, foreign, typing_env, imp, (ty, fact), &None) {
                // Of a type rustc knows the layout of here, of no parameter
                // or whatever they are (`layout_bytes`): not of a std
                // associated type of one no caller gives, typenum's `<<U as
                // Len>::Output as Add<B1>>::Output`.
                None => tcx.layout_of(typing_env.as_query_input(ty)).is_ok(),
                Some(super::Asked::Slot(..)) => true,
                Some(super::Asked::Carried(..)) => false,
                Some(super::Asked::Built(built, ..)) => foreign.templates(imp, built),
            }
        })
    })
}
