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
/// (`Foreign::carried`, #476).
pub(super) fn carries(foreign: &crate::lower::library::Foreign<'_, '_>, assoc: DefId, fact: TypeFact) -> bool {
    assoc.is_local() || foreign.carried(assoc).contains(&fact)
}
