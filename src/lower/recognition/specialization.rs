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
    /// Each impl that specializes it and writes the method, the most
    /// specific first: its method, and the facts of types that pick it, all
    /// of them, `Fused` of `I`, or `RandomAccess` of both a zip's sides
    /// (ADR 0975).
    pub(crate) special: Vec<(DefId, Vec<(Ty<'tcx>, TypeFact)>)>,
}

impl<'tcx> FactSpecialization<'tcx> {
    /// Each impl's method, the selected one's first.
    pub(crate) fn methods(&self) -> impl Iterator<Item = DefId> + '_ {
        std::iter::once(self.selected.0).chain(self.special.iter().map(|&(method, _)| method))
    }

    /// Each fact that picks an impl, and its type.
    pub(crate) fn facts(&self) -> impl Iterator<Item = (Ty<'tcx>, TypeFact)> + '_ {
        self.special.iter().flat_map(|(_, facts)| facts.iter().copied())
    }
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
    let method = |imp: DefId| tcx.impl_item_implementor_ids(imp).get(&callee).copied();
    let mut specializing: Vec<DefId> = (tcx.all_impls(trait_id))
        .filter(|&other| {
            other != impl_id
                && tcx.specializes((other, impl_id))
                && tcx.generics_of(other).count() == tcx.generics_of(impl_id).count()
                && tcx
                    .impl_trait_ref(other)
                    .instantiate(tcx, impl_args)
                    .skip_normalization()
                    == tr
        })
        .collect();
    // The most specific first, the one each fact of which a more general
    // one's implies: of a zip's, `TrustedRandomAccess`'s before
    // `TrustedRandomAccessNoCoerce`'s.
    specializing.sort_by(|&a, &b| match (tcx.specializes((a, b)), tcx.specializes((b, a))) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    });
    let mut special = Vec::new();
    for other in specializing {
        let facts: Vec<(Ty<'tcx>, TypeFact)> = (tcx.clauses_of(other).instantiate(tcx, impl_args).into_iter())
            .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
            .filter_map(|bound| {
                Some((
                    bound.self_ty().skip_binder(),
                    specialized_on(tcx, trait_id, bound.def_id())?,
                ))
            })
            .flat_map(|(of, fact)| held_facts(tcx, typing_env, of, fact))
            .collect();
        if facts.is_empty() {
            return None;
        }
        // One that writes no method of its own takes a more general one's,
        // which its facts pick too.
        if let Some(method) = method(other) {
            special.push((method, facts));
        }
    }
    (!special.is_empty()).then_some(())?;
    let mut selected = method(impl_id)?;
    // A default that only panics, std's "always specialized", the general
    // `ZipImpl::get_unchecked`, whose callers' bounds rule it out and which
    // no stable program can reach: not one the facts pick, its least
    // specific specialization in its place (ADR 0975).
    if never_returns(tcx, selected) {
        selected = special.pop().expect("one specializes it").0;
    }
    Some(FactSpecialization {
        selected: (selected, impl_args),
        special,
    })
}

/// A fact of `of`, or the facts of what it holds that it is, where rustc
/// can't say it of `of` and std's one impl of the trait for its type
/// constructor asks that trait alone of its parameters: `TrustedLen` of a
/// `&mut I`, `I`'s, as `impl<I: TrustedLen + ?Sized> TrustedLen for &mut
/// I` asks (ADR 0975).
pub(crate) fn held_facts<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    of: Ty<'tcx>,
    fact: TypeFact,
) -> Vec<(Ty<'tcx>, TypeFact)> {
    let Some(fact_trait) = fact_trait(tcx, fact) else {
        return vec![(of, fact)];
    };
    let held = match *of.kind() {
        ty::Ref(_, pointee, mutability) => Some((Err(mutability), vec![pointee])),
        ty::Adt(adt, args) => Some((Ok(adt.did()), args.types().collect())),
        _ => None,
    };
    let Some((head, held)) = held else {
        return vec![(of, fact)];
    };
    if trait_holds_in(tcx, typing_env, of, fact_trait).is_some() {
        return vec![(of, fact)];
    }
    // Its impl's self type's parameters, in order, each a parameter.
    let impls: Vec<(DefId, Vec<Ty<'tcx>>)> = (tcx.all_impls(fact_trait))
        .filter_map(|imp| {
            let self_ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
            match *self_ty.kind() {
                ty::Ref(_, pointee, mutability) if head == Err(mutability) => Some((imp, vec![pointee])),
                ty::Adt(adt, args) if head == Ok(adt.did()) => Some((imp, args.types().collect())),
                _ => None,
            }
        })
        .collect();
    let [(imp, params)] = &impls[..] else {
        return vec![(of, fact)];
    };
    if !params.iter().all(|param| matches!(param.kind(), ty::Param(_))) {
        return vec![(of, fact)];
    }
    let mut facts = Vec::new();
    for (clause, _) in tcx.clauses_of(*imp).instantiate_identity(tcx) {
        let Some(bound) = clause.skip_normalization().as_trait_clause() else {
            continue;
        };
        if bound.def_id() != fact_trait {
            continue;
        }
        let Some(at) = params.iter().position(|&param| param == bound.self_ty().skip_binder()) else {
            return vec![(of, fact)];
        };
        facts.extend(held_facts(tcx, typing_env, held[at], fact));
    }
    facts
}

/// Whether `method`'s body never returns, its every path a panic.
fn never_returns(tcx: TyCtxt<'_>, method: DefId) -> bool {
    tcx.is_mir_available(method)
        && (tcx.optimized_mir(method).basic_blocks.iter()).all(|data| {
            !matches!(
                data.terminator.as_ref().map(|t| &t.kind),
                Some(rustc_middle::mir::TerminatorKind::Return)
            )
        })
}

/// The fact of a type's having `bound`, a trait std specializes an impl of
/// `trait_id` on for what a program sees: `FusedIterator`'s, `Fused`, and
/// `OneShot`'s; and of a zip's private traits, its sides' being
/// `TrustedRandomAccess`, `TrustedRandomAccessNoCoerce` or `TrustedLen`, by
/// which std's zip reads each item by its index or folds only up to the
/// shorter, which a step of the longer side running code can tell (ADR
/// 0975). Elsewhere those are std's optimizations (`optimized_default`).
fn specialized_on(tcx: TyCtxt<'_>, trait_id: DefId, bound: DefId) -> Option<TypeFact> {
    const ZIPS: &[(&str, TypeFact)] = &[
        ("ZipImpl", TypeFact::RandomAccess),
        ("ZipImpl", TypeFact::RandomAccessNoCoerce),
        ("SpecFold", TypeFact::TrustedLen),
        ("SpecTrustedRandomAccess", TypeFact::RandomAccessNoCoerce),
    ];
    let of_zip = |fact: TypeFact| {
        ZIPS.iter()
            .any(|&(name, by)| by == fact && tcx.item_name(trait_id).as_str() == name)
    };
    [
        TypeFact::Fused,
        TypeFact::OneShot,
        TypeFact::RandomAccess,
        TypeFact::RandomAccessNoCoerce,
        TypeFact::TrustedLen,
    ]
    .into_iter()
    .filter(|&fact| matches!(fact, TypeFact::Fused | TypeFact::OneShot) || of_zip(fact))
    .find(|&fact| fact_trait(tcx, fact) == Some(bound))
}

/// Whether `def_id` is `TrustedRandomAccessNoCoerce`'s `MAY_HAVE_SIDE_EFFECT`,
/// whether reading an item of a type may run code, by which std's zip of it
/// reads the items it passes over (ADR 0975).
pub(crate) fn is_steps_code_const(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    fact_trait(tcx, TypeFact::RandomAccessNoCoerce).is_some_and(|no_coerce| {
        tcx.trait_of_assoc(def_id) == Some(no_coerce) && tcx.item_name(def_id).as_str() == "MAY_HAVE_SIDE_EFFECT"
    })
}

/// The trait whose impls a fact of a type is, whether it has one: the one
/// list of them.
pub(crate) fn fact_trait(tcx: TyCtxt<'_>, fact: TypeFact) -> Option<DefId> {
    match fact {
        TypeFact::Fused => tcx.lang_items().get(LangItem::FusedIterator),
        TypeFact::OneShot => one_shot_trait(tcx),
        TypeFact::RandomAccess => registry::by_path(tcx, "core", &["iter", "TrustedRandomAccess"]),
        TypeFact::RandomAccessNoCoerce => registry::by_path(tcx, "core", &["iter", "TrustedRandomAccessNoCoerce"]),
        TypeFact::TrustedLen => registry::by_path(tcx, "core", &["iter", "TrustedLen"]),
        _ => None,
    }
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

impl<'tcx> Recognition<'_, 'tcx> {
    /// Whether `ty`, a type rustc knows whole, has `trait_id`'s impl: a
    /// fact's trait's (`fact_trait`).
    pub(crate) fn implements_trait(&self, ty: Ty<'tcx>, trait_id: DefId) -> bool {
        self.trait_holds(ty, trait_id) == Some(true)
    }

    /// Whether `ty` has `trait_id`'s impl, where rustc can say it here: of
    /// one holding a type parameter too, `slice::IterMut<'_, T>`'s
    /// `TrustedRandomAccess`, which holds of any `T`. `None` where only a
    /// caller can say (ADR 0975).
    pub(crate) fn trait_holds(&self, ty: Ty<'tcx>, trait_id: DefId) -> Option<bool> {
        trait_holds_in(self.tcx, self.typing_env, ty, trait_id)
    }
}

/// `Recognition::trait_holds` in `typing_env`.
fn trait_holds_in<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: Ty<'tcx>,
    trait_id: DefId,
) -> Option<bool> {
    let tr = tcx.erase_and_anonymize_regions(ty::TraitRef::new(tcx, trait_id, [ty]));
    match tcx.codegen_select_candidate(typing_env.as_query_input(tr)) {
        Ok(_) => Some(true),
        Err(rustc_middle::traits::CodegenObligationError::Unimplemented)
            if !rustc_middle::ty::TypeVisitableExt::has_param(&ty) || !has_impl_for_head(tcx, ty, trait_id) =>
        {
            Some(false)
        }
        Err(_) => None,
    }
}

/// Whether any impl of `trait_id` may be of `ty` once its parameters are
/// known: one of the same type constructor, `&mut _` or `Skip<_>`, or one
/// for any `T`. None of a `&mut I`'s `TrustedRandomAccess`.
fn has_impl_for_head<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, trait_id: DefId) -> bool {
    let head = |ty: Ty<'tcx>| match ty.kind() {
        ty::Adt(adt, _) => Some(Ok(adt.did())),
        ty::Ref(_, _, mutability) => Some(Err(*mutability)),
        _ => None,
    };
    tcx.all_impls(trait_id).any(|imp| {
        let of = tcx.type_of(imp).instantiate_identity().skip_normalization();
        matches!(of.kind(), ty::Param(_)) || head(of).is_none() || head(of) == head(ty)
    })
}
