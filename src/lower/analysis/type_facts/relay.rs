//! What one function's ask asks of its callers (ADR 0145): a fact of a
//! slot, of a type built of its slots, or one its trait's dictionaries
//! carry, said in a caller's terms, and the sets the fixpoint keeps them
//! in. Split from type_facts.rs by concept, as a file is at most 1500
//! lines.

use super::*;

/// A fact a function asks: of one of its slots, a type parameter's or an
/// associated type's, by its index; or the layout of a type built of them,
/// `Held<J>` of a `Vec<Held<J>>` collected in place, by the type, its
/// index decided once every one of its function's is found (#737), and
/// whether one of its gates' `IInPlace` must be given for it to be read;
/// or what its trait's dictionaries carry of an associated type of the
/// crate's trait or a library's, `<S as Source>::Iter`'s (ADR 0145).
#[derive(Clone, Debug)]
pub(super) enum Asked<'tcx> {
    Slot(u32, TypeFact),
    Built(ty::Ty<'tcx>, TypeFact, Gates),
    Carried(DefId, TypeFact),
}

/// What a built slot's layout is read where: `None` wherever its function
/// is called; else only where one of these slots' `IInPlace`, whether its
/// collect may be in place, isn't `undefined` (`traits::BuiltSlot`).
pub(super) type Gates = Option<std::collections::BTreeSet<u32>>;

/// `a` and `b` together: read where either is.
pub(super) fn either_gates(a: &Gates, b: &Gates) -> Gates {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.union(b).copied().collect()),
        _ => None,
    }
}

/// Whether `ty`, of no slot of its own, is built of a function's, a type
/// whose layout rustc can't say there, as only its caller knows it,
/// `Held<J>`: a fact asked of it, its size or alignment, or what std's
/// unstable sort picks by it, `sort_by_cached_key`'s `(K, usize)`'s (#925),
/// is its caller's.
pub(super) fn built_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: ty::Ty<'tcx>,
    fact: TypeFact,
) -> bool {
    if crate::lower::recognition::known_by_bounds(ty) || !ty.has_param() {
        return false;
    }
    // A std trait std's zip is specialized on, which a type built of its
    // slots may have only as its callers' types say, `vec::IntoIter<<I as
    // Iterator>::Item>`'s `TrustedRandomAccessNoCoerce`, a `Copy` item's
    // (#1389): not one rustc says here, of any types given (#1385).
    if matches!(
        fact,
        TypeFact::RandomAccess | TypeFact::RandomAccessNoCoerce | TypeFact::TrustedLen
    ) {
        return crate::lower::recognition::fact_trait(tcx, fact).is_some_and(|fact_trait| {
            crate::lower::recognition::trait_holds_in(tcx, typing_env, ty, fact_trait).is_none()
        });
    }
    matches!(
        fact,
        TypeFact::Size | TypeFact::Room | TypeFact::Align | TypeFact::Sort | TypeFact::StableSort
    ) && tcx.layout_of(typing_env.as_query_input(ty)).is_err()
}

/// Where `caller` asks `(of, fact)`, `of` in its own terms: a slot's, or
/// a built type's (`built_of`), read where `gates` says; none of a type it
/// knows.
pub(super) fn asked_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    caller: DefId,
    (of, fact): (ty::Ty<'tcx>, TypeFact),
    gates: &Gates,
) -> Option<Asked<'tcx>> {
    if let Some(slot) = slot_of(tcx, foreign, caller, of) {
        return Some(Asked::Slot(slot, fact));
    }
    // Of an associated type whose trait's dictionaries carry it, theirs,
    // wherever a callee asks it of what it's given, a flatten's items'
    // constant size of `(1..4).flat_map(S::hidden)` (#1312, #1341).
    if let Some(assoc) = carried_by(foreign, of)
        && carries(foreign, assoc, fact)
    {
        return Some(Asked::Carried(assoc, fact));
    }
    built_of(tcx, typing_env, of, fact).then(|| Asked::Built(tcx.erase_and_anonymize_regions(of), fact, gates.clone()))
}

/// Each function's built types it asks a layout of, by the type: the facts,
/// and where they're read.
pub(super) type BuiltAsks<'tcx> = IndexMap<(DefId, ty::Ty<'tcx>), (std::collections::BTreeSet<TypeFact>, Gates)>;

/// What its trait's dictionaries carry of each associated type, by the
/// type and the fact.
pub(super) type CarriedAsks = IndexSet<(DefId, TypeFact)>;

/// How deep a type built of a function's slots may hold one type
/// constructor within itself, where its layout is relayed to callers:
/// zerocopy's `impl<T: KnownLayout> KnownLayout for MaybeUninit<T>`, whose
/// `MaybeUninit` is `MaybeUninit<T::MaybeUninit>`, asks one wider of its
/// makers each round, `MaybeUninit<MaybeUninit<..>>`, which never settles
/// (#1430). Past it, the layout isn't relayed: a caller that would have to
/// give it is refused where it's lowered, as one no caller gives.
const NESTED_AT_MOST: usize = 8;

/// Whether `ty` holds a type constructor, or an associated type's
/// projection, `<<T as Shape>::Uninit as Shape>::Uninit`, within itself
/// more than `NESTED_AT_MOST` deep, `held` the constructors around it.
fn nests_past_bound(ty: ty::Ty<'_>, held: &mut Vec<DefId>) -> bool {
    let constructed = match ty.kind() {
        ty::Adt(adt, args) => Some((adt.did(), *args)),
        ty::Alias(_, alias) => match alias.kind {
            ty::Projection { def_id } => Some((def_id, alias.args)),
            _ => None,
        },
        _ => None,
    };
    if let Some((id, args)) = constructed {
        if held.iter().filter(|&&held| held == id).count() >= NESTED_AT_MOST {
            return true;
        }
        held.push(id);
        let past = args.types().any(|part| nests_past_bound(part, held));
        held.pop();
        return past;
    }
    let parts: Vec<ty::Ty<'_>> = match ty.kind() {
        ty::Ref(_, inner, _) | ty::Array(inner, _) | ty::Slice(inner) => vec![*inner],
        ty::Tuple(parts) => parts.iter().collect(),
        _ => Vec::new(),
    };
    parts.into_iter().any(|part| nests_past_bound(part, held))
}

/// `caller` asks `asked`: whether that's new.
pub(super) fn ask<'tcx>(
    (asked, built, carried): (
        &mut IndexSet<(DefId, u32, TypeFact)>,
        &mut BuiltAsks<'tcx>,
        &mut CarriedAsks,
    ),
    caller: DefId,
    each: Asked<'tcx>,
) -> bool {
    match each {
        Asked::Carried(assoc, fact) => carried.insert((assoc, fact)),
        Asked::Slot(index, fact) => asked.insert((caller, index, fact)),
        Asked::Built(ty, ..) if nests_past_bound(ty, &mut Vec::new()) => false,
        Asked::Built(ty, fact, gates) => match built.get_mut(&(caller, ty)) {
            Some((facts, read)) => {
                let wider = either_gates(read, &gates);
                let changed = facts.insert(fact) || wider != *read;
                *read = wider;
                changed
            }
            None => {
                built.insert((caller, ty), ([fact].into(), gates));
                true
            }
        },
    }
}

/// Where a library's function's slot `index` is read, as its manifest says:
/// a built slot's gates (#737).
pub(super) fn foreign_gates(foreign: &crate::lower::library::Foreign<'_, '_>, id: DefId, index: u32) -> Gates {
    (foreign.built_slot(id, index))
        .map(|built| built.gates.into_iter().collect())
        .filter(|gates: &std::collections::BTreeSet<u32>| !gates.is_empty())
}

/// What a caller asks of its own where its call of `callee` with `args`
/// asks a fact of `slot`, of `callee`'s in its own terms, a type
/// parameter, an associated type (`drop_slot`) or a type built of them:
/// its slots', `I::Item`'s of a flatten's `ConstSize` (ADR 0899), or each's
/// a type is the facts of, `I`'s `TrustedLen` of a `&mut I` (ADR 0975), or
/// a type's built of them (#737); none of a type it knows. A built slot's,
/// read where `gates` says in `callee`'s, is read where the caller's slots
/// given for them say; none where each is given a source std never collects
/// in place, whose `IInPlace` is `undefined`.
pub(super) fn asked_of_caller<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    (caller, callee, args): (DefId, DefId, ty::GenericArgsRef<'tcx>),
    (slot, fact, gates): (ty::Ty<'tcx>, TypeFact, &Gates),
) -> Vec<Asked<'tcx>> {
    use crate::lower::recognition::{InPlaceSource, held_facts, in_place_source};
    let here = |ty: ty::Ty<'tcx>| {
        let ty = ty::EarlyBinder::bind(tcx, ty)
            .instantiate(tcx, args)
            .skip_normalization();
        tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty)
    };
    let gates: Gates = match gates {
        None => None,
        Some(callee_gates) => {
            let mut read = std::collections::BTreeSet::new();
            let mut always = false;
            for &gate in callee_gates {
                let gate = here(crate::lower::traits::drop_slot(
                    tcx,
                    (foreign, &Default::default()),
                    callee,
                    gate,
                ));
                if in_place_source(tcx, typing_env, gate) == Some(InPlaceSource::Never) {
                    continue;
                }
                let held = held_facts(tcx, typing_env, gate, TypeFact::InPlace);
                always |= held.is_empty();
                for (of, fact) in held {
                    match (fact == TypeFact::InPlace)
                        .then(|| slot_of(tcx, foreign, caller, of))
                        .flatten()
                    {
                        Some(at) => {
                            read.insert(at);
                        }
                        None => always = true,
                    }
                }
            }
            match (always, read.is_empty()) {
                (true, _) => None,
                // Of a source never collected in place, never read.
                (false, true) => return Vec::new(),
                (false, false) => Some(read),
            }
        }
    };
    (held_facts(tcx, typing_env, here(slot), fact).into_iter())
        .filter_map(|each| asked_of(tcx, foreign, typing_env, caller, each, &gates))
        .collect()
}
