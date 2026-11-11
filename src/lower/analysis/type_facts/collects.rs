//! What a collect asks of its type parameters (ADR 0145): how std
//! collects into a `Vec` what its caller gives, `IVecCollect`
//! (`vec_collected`), the buffer it may collect in in place, `IInPlace`
//! (`in_place_asked`), the size its hint grows by (`grown_asked`), and of a
//! trait's default, each impl's that copies it (`collect_asks`). Split
//! unchanged from `type_facts.rs` near its cap (#1334).

use super::*;

/// The iterator a collect into a `Vec`, `collect()` or `Vec::from_iter`,
/// collects as it's given, a type parameter's or an associated type's of
/// one: how std's `SpecFromIter` collects it, in place, by its fold or by
/// `next`, is its caller's, `IVecCollect`, of items that drop or not
/// (#1239). Of what std grows as a `Vec`, not a `LinkedList`, whose own
/// `extend` collects it.
fn vec_collected<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
) -> Option<ty::Ty<'tcx>> {
    let (this, output, _, false) =
        crate::lower::recognition::collected_items(recognition.tcx, typing_env, callee, args)?
    else {
        return None;
    };
    (recognition.grows_as_vec(output) && crate::lower::recognition::known_by_bounds(this)).then_some(this)
}

/// What `caller`'s call asks as a collect, `vec_collected`'s, `in_place_asked` and
/// `grown_asked`: of a trait's default, of each of the crate's impls that
/// copies it, in its terms, of its own type parameters, `I` of `impl<I:
/// Iterator> Sorted for I`, which its dictionary is given, as the copied
/// default reads them (ADR 0049).
pub(super) fn collect_asks<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    (drops, keeps_capacity): (ItemDrops<'_, 'tcx>, bool),
    caller: DefId,
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
    carried_super: &mut IndexSet<(DefId, DefId, TypeFact)>,
) -> Vec<(DefId, Asked<'tcx>)> {
    let (tcx, foreign) = (recognition.tcx, recognition.foreign);
    let asked_by = |typing_env: ty::TypingEnv<'tcx>, owner: DefId, args: ty::GenericArgsRef<'tcx>| {
        // Of a collect into a `Vec` of what its caller gives, how std
        // collects it, `IVecCollect`, which answers what `IInPlace` would.
        let mut asked = match vec_collected(recognition, typing_env, (callee, args)) {
            Some(this) => Vec::from_iter(asked_of(
                tcx,
                foreign,
                typing_env,
                owner,
                (this, TypeFact::VecCollect),
                &None,
            )),
            None => in_place_asked(tcx, foreign, typing_env, drops, owner, (callee, args)),
        };
        asked.extend(grown_asked(
            recognition,
            typing_env,
            owner,
            (callee, args),
            keeps_capacity,
        ));
        asked
    };
    let default_of = tcx
        .trait_of_assoc(caller)
        .filter(|_| tcx.defaultness(caller).has_value());
    let Some(trait_id) = default_of else {
        return (asked_by(typing_env, caller, args).into_iter())
            .map(|asked| (caller, asked))
            .collect();
    };
    let parents = tcx.generics_of(caller).parent_count;
    let own = ty::GenericArgs::identity_for_item(tcx, caller);
    let mut asked = Vec::new();
    for &imp in tcx.all_local_trait_impls(()).get(&trait_id).into_iter().flatten() {
        let imp = imp.to_def_id();
        if tcx.impl_item_implementor_ids(imp).contains_key(&caller) {
            continue;
        }
        // In the impl's terms as the lowering has them, and where they're
        // known as it knows them (`default_args`, `default_typing_env`).
        let trait_args = tcx.impl_trait_ref(imp).instantiate_identity().skip_normalization().args;
        let in_impl = tcx.mk_args_from_iter(trait_args.iter().chain(own.iter().skip(parents)));
        let in_impl = crate::lower::traits::default_args(tcx, imp, caller, in_impl);
        let args = ty::EarlyBinder::bind(tcx, args)
            .instantiate(tcx, in_impl)
            .skip_normalization();
        let impl_env = crate::lower::traits::default_typing_env(tcx, imp, caller, in_impl);
        // Not what names the default's own parameters, which aren't the
        // impl's (`default_args`): its caller gives those (#804).
        let first = tcx.generics_of(imp).count() as u32;
        for each in asked_by(impl_env, imp, args) {
            if !matches!(&each, Asked::Built(ty, ..) if names_params(*ty, |index| index >= first)) {
                asked.push((imp, each));
            }
        }
    }
    // What names only its own, in its trait's terms, the default's own.
    for each in asked_by(typing_env, caller, args) {
        let own = match &each {
            Asked::Built(ty, ..) => names_params(*ty, |index| index >= parents as u32),
            // Its trait's dictionaries', whoever asks.
            Asked::Carried(..) => true,
            Asked::Slot(index, fact) => {
                // Of `<Self as Iterator>::Item`, what its trait's
                // dictionaries carry, for a library's default compiled over
                // `Self`, which its copies' impls asked above (#990).
                if let Some((trait_id, assoc)) = slot_super_item(tcx, foreign, caller, *index) {
                    carried_super.insert((trait_id, assoc, *fact));
                }
                *index >= parents as u32 && *index < own.len() as u32
            }
        };
        if own {
            asked.push((caller, each));
        }
    }
    asked
}

/// What a collect into a `Vec`, `collect()` or `from_iter`, of `caller`'s
/// asks, of items with a destructor a panic may leave: what std may collect
/// its iterator in, in place, where the adapters keep a type parameter's
/// iterator, `IInPlace`, and the items' layout where it's a type
/// parameter's, `TSize` and `TAlign`, by which a panic drops what it
/// collected first or what's left (#672); or a type's built of them,
/// `Held<J>`'s, read only where the source's `IInPlace` isn't `undefined`
/// (#737). As the lowering's `rest_dropped_first` asks, both of
/// `recognition::in_place_source`.
fn in_place_asked<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    drops: ItemDrops<'_, 'tcx>,
    caller: DefId,
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
) -> Vec<Asked<'tcx>> {
    use crate::lower::recognition::{InPlaceSource, collected_items, fits_in_place, held_facts, in_place_source};
    let Some((iter, _, dest, _)) = collected_items(tcx, typing_env, callee, args) else {
        return Vec::new();
    };
    if !drops.drop(typing_env, caller, dest) {
        return Vec::new();
    }
    match in_place_source(tcx, typing_env, iter) {
        Some(InPlaceSource::Of(_)) => {}
        Some(InPlaceSource::Items(src)) if fits_in_place(tcx, typing_env, src, dest).is_none() => {}
        _ => return Vec::new(),
    }
    let source = held_facts(tcx, typing_env, iter, TypeFact::InPlace);
    // The destination's layout is read where the source's `IInPlace` isn't
    // `undefined`: where each of its facts is a slot's `IInPlace`, those
    // slots'; else, of a source's items' layout, known or not, wherever.
    let gates: Gates = (source.iter())
        .map(|&(of, fact)| (fact == TypeFact::InPlace).then(|| slot_of(tcx, foreign, caller, of))?)
        .collect::<Gates>()
        .filter(|gates| !gates.is_empty());
    let mut asked: Vec<Asked<'tcx>> = (source.into_iter())
        .filter_map(|each| asked_of(tcx, foreign, typing_env, caller, each, &None))
        .collect();
    if fits_in_place(tcx, typing_env, dest, dest).is_none() {
        for fact in [TypeFact::Size, TypeFact::Align] {
            asked.extend(asked_of(tcx, foreign, typing_env, caller, (dest, fact), &gates));
        }
    }
    asked
}

/// What a collect std grows as a `Vec`, `collect()` or `from_iter`, of
/// `caller`'s asks of items whose size is a caller's, of an iterator whose
/// hint may run the crate's own (`hint_reaches_own`), or, of a crate that
/// keeps capacities, of one that isn't `TrustedLen`: their size, a slot's
/// `TSize` or a built type's, `(T, u8)`'s, by which std's `Vec` is full
/// and asks the hint again (#796, #1180). As the lowering's collect reads
/// it, `grown_item_size`, wherever it's called. And of a crate that keeps
/// capacities, of a call that adds to a `Vec`, a `VecDeque` or a
/// `BinaryHeap` (`grown_vec_item`), its items' size, by which std grows it
/// (`capacity_item_size`, #1180).
fn grown_asked<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    caller: DefId,
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
    keeps_capacity: bool,
) -> Vec<Asked<'tcx>> {
    let (tcx, foreign) = (recognition.tcx, recognition.foreign);
    // Its types as the lowering asks them, its opaque ones revealed.
    let revealed = crate::lower::recognition::Recognition {
        typing_env: typing_env.with_post_analysis_normalized(tcx),
        ..*recognition
    };
    let grown = (keeps_capacity)
        .then(|| revealed.grown_vec_item(callee, args))
        .flatten()
        .filter(|item| matches!(item.kind(), ty::Param(_)));
    if let Some(item) = grown {
        return asked_of(tcx, foreign, typing_env, caller, (item, TypeFact::Size), &None)
            .into_iter()
            .collect();
    }
    let Some((iter, output, dest, shunted)) = crate::lower::recognition::collected_items(tcx, typing_env, callee, args)
    else {
        return Vec::new();
    };
    if !revealed.grows_by_callers_size(output, iter, dest, (shunted, keeps_capacity)) {
        return Vec::new();
    }
    let size = asked_of(tcx, foreign, typing_env, caller, (dest, TypeFact::Size), &None);
    size.into_iter().collect()
}
