//! std's specializations: an impl a more specific one specializes, which
//! rustc leaves unresolved where the type isn't known whole. One a fact
//! picks, `FusedIterator`'s or `OneShot`'s, both impls compiled, called by
//! the caller's fact (`fact_specialization`); one that only does less work,
//! the impl rustc selects (`optimized_default`).

use super::*;

/// A call of a std trait's method whose impl rustc leaves to the caller's
/// type, `FuseImpl::next(self)` of a `Fuse<I>` or `FlattenCompat`'s
/// `size_hint`: the method of the impl it selects for the type, and of the
/// impl that specializes that one on a type's being a `FusedIterator` or a
/// `OneShot`, which the caller's fact of it, `IFused` or `UOneShot`, picks
/// between (ADRs 0473, 0543, 0899). Another specialization, on a trait that
/// isn't a fact, `TrustedLen`'s, is std's optimization of the one selected,
/// which changes nothing a program sees (`optimized_default`).
pub(crate) struct FactSpecialization<'tcx> {
    /// The method of the impl selected, and the impls' arguments.
    pub(crate) selected: (DefId, ty::GenericArgsRef<'tcx>),
    /// The method of the impl that specializes it where `of` has `fact`.
    pub(crate) special: DefId,
    /// The type whose fact picks `special`'s impl.
    pub(crate) of: Ty<'tcx>,
    /// `Fused` or `OneShot`.
    pub(crate) fact: TypeFact,
}

/// `callee`, a trait's method, called with `args` in `typing_env`, where
/// rustc resolves it to no one impl: a `FactSpecialization`, or none.
pub(crate) fn fact_specialization<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    callee: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<FactSpecialization<'tcx>> {
    let trait_id = tcx.trait_of_assoc(callee)?;
    let tr = ty::TraitRef::from_assoc(tcx, trait_id, args);
    let tr = tcx.erase_and_anonymize_regions(tr);
    let Ok(ImplSource::UserDefined(selected)) = tcx.codegen_select_candidate(typing_env.as_query_input(tr)) else {
        return None;
    };
    let (impl_id, impl_args) = (selected.impl_def_id, selected.args);
    if !super::is_std_crate(tcx, impl_id) {
        return None;
    }
    let specializing = tcx.all_impls(trait_id).find(|&other| {
        other != impl_id
            && tcx.specializes((other, impl_id))
            && tcx.generics_of(other).count() == tcx.generics_of(impl_id).count()
            && tcx
                .impl_trait_ref(other)
                .instantiate(tcx, impl_args)
                .skip_normalization()
                == tr
    })?;
    let (of, fact) = (tcx.clauses_of(specializing).instantiate(tcx, impl_args).into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
        .find_map(|bound| Some((bound.self_ty().skip_binder(), specialized_on(tcx, bound.def_id())?)))?;
    let method = |imp: DefId| tcx.impl_item_implementor_ids(imp).get(&callee).copied();
    Some(FactSpecialization {
        selected: (method(impl_id)?, impl_args),
        special: method(specializing)?,
        of,
        fact,
    })
}

/// The fact of a type's having `bound`, a trait std specializes an impl on
/// for what a program sees: `FusedIterator`'s, `Fused`, and `OneShot`'s.
fn specialized_on(tcx: TyCtxt<'_>, bound: DefId) -> Option<TypeFact> {
    if tcx.is_lang_item(bound, LangItem::FusedIterator) {
        return Some(TypeFact::Fused);
    }
    (one_shot_trait(tcx) == Some(bound)).then_some(TypeFact::OneShot)
}

/// std's `OneShot`, private to `flatten`'s module: an iterator of one item
/// at most, `option::IntoIter`'s (ADR 0899).
pub(crate) fn one_shot_trait(tcx: TyCtxt<'_>) -> Option<DefId> {
    let core = tcx
        .crates(())
        .iter()
        .copied()
        .find(|&krate| tcx.crate_name(krate) == sym::core)?;
    (tcx.traits(core).iter().copied()).find(|&id| tcx.item_name(id).as_str() == "OneShot")
}

/// The traits std specializes an impl on only to do less work, its result
/// the same, each of every adapter's impl or of one alone (ADR 0899):
/// `TrustedLen`; and `TrustedRandomAccess` of `Take`'s, whose `fold` of
/// one reads each item it would step by its index, in the same order,
/// the source's items never dropped (ADR 0913). Not of `Zip`'s, whose
/// specialization runs fewer of a longer side's steps; nor
/// `FusedIterator`, whose specialization of `Fuse` keeps what the generic
/// one drops, nor `OneShot`, whose `flatten()`'s `size_hint` bounds what
/// the generic one doesn't (`fact_specialization`).
const OPTIMIZING: &[(&str, Option<&str>)] = &[("TrustedLen", None), ("TrustedRandomAccess", Some("Take"))];

/// The traits std specializes type by type only to do less work: `StepBy`'s
/// of each integer range, `SpecRangeSetup`'s, which counts what's left in
/// the range's `end`, and `StepByImpl`'s and `StepByBackImpl`'s, which step
/// by that count, each item, hint and count the general impls' (ADR 0917).
/// Only its `Debug` shows the range so changed, which isn't compiled. And
/// `Filter::count`'s `SpecAssumeCount`, whose `TrustedLen` impl assumes
/// what the general one checks, `upper - count`, which a `TrustedLen`'s
/// upper bound never fails (ADR 0932).
const OPTIMIZING_BY_TYPE: &[&str] = &["SpecRangeSetup", "StepByImpl", "StepByBackImpl", "SpecAssumeCount"];

/// `callee`, a trait's method, called with `args` in `typing_env`, where
/// rustc resolves it to no one impl as a more specific impl may specialize
/// the one it selects: the selected impl's method, and its arguments, where
/// each impl that specializes it does so only on what `OPTIMIZING` names,
/// so the selected one gives the same result for every type
/// (a `TrustedLen` one's, ADR 0899).
pub(crate) fn optimized_default<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    callee: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<(DefId, ty::GenericArgsRef<'tcx>)> {
    let trait_id = tcx.trait_of_assoc(callee)?;
    let tr = ty::TraitRef::from_assoc(tcx, trait_id, args);
    let tr = tcx.erase_and_anonymize_regions(tr);
    let Ok(ImplSource::UserDefined(selected)) = tcx.codegen_select_candidate(typing_env.as_query_input(tr)) else {
        return None;
    };
    let impl_id = selected.impl_def_id;
    if !super::is_std_crate(tcx, impl_id) {
        return None;
    }
    let method = tcx.impl_item_implementor_ids(impl_id).get(&callee).copied()?;
    let own = &args[tcx.generics_of(callee).parent_count..];
    let selected_args = || tcx.mk_args_from_iter(selected.args.iter().chain(own.iter().copied()));
    if OPTIMIZING_BY_TYPE.contains(&tcx.item_name(trait_id).as_str()) {
        return Some((method, selected_args()));
    }
    // Each impl that specializes it, bound by what the selected one isn't
    // only where that's an optimization's.
    let bounds = |imp: DefId| -> Vec<DefId> {
        (tcx.clauses_of(imp).instantiate_identity(tcx).into_iter())
            .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
            .map(|bound| bound.def_id())
            .collect()
    };
    let general = bounds(impl_id);
    let specializing: Vec<DefId> = (tcx.all_impls(trait_id))
        .filter(|&other| other != impl_id && tcx.specializes((other, impl_id)))
        .collect();
    // Of the same type, apart by a bound alone: not `ConstSizeIntoIterator`'s
    // impl for `[T; N]`, which specializes the one for any `T` by its type,
    // and answers otherwise.
    let of = |imp: DefId| match tcx.type_of(imp).skip_binder().kind() {
        ty::Adt(adt, _) => Some(adt.did()),
        _ => None,
    };
    let adapter = of(impl_id).map(|adt| tcx.item_name(adt));
    let optimizing = |bound: DefId| {
        OPTIMIZING.iter().any(|&(name, only)| {
            tcx.item_name(bound).as_str() == name && only.is_none_or(|only| adapter.is_some_and(|a| a.as_str() == only))
        })
    };
    let only_optimizing = of(impl_id).is_some()
        && specializing.iter().all(|&other| {
            of(other) == of(impl_id)
                && bounds(other)
                    .into_iter()
                    .all(|bound| general.contains(&bound) || optimizing(bound))
        });
    (!specializing.is_empty() && only_optimizing).then(|| (method, selected_args()))
}
