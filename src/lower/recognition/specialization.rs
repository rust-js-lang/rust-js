//! std's specializations: an impl a more specific one specializes, which
//! rustc leaves unresolved where the type isn't known whole. One a fact
//! picks, `FusedIterator`'s or `OneShot`'s, both impls compiled, called by
//! the caller's fact (`fact_specialization`); one that only does less work,
//! the impl rustc selects (`optimized_default`).

use super::*;
use rustc_middle::mir;

/// A call of a std trait's method whose impl rustc leaves to the caller's
/// type, `FuseImpl::next(self)` of a `Fuse<I>` or `FlattenCompat`'s
/// `size_hint`: the method of the impl it selects for the type, and of the
/// impl that specializes that one on a type's being a `FusedIterator` or a
/// `OneShot`, which the caller's fact of it, `IFused` or `UOneShot`, picks
/// between (ADRs 0473, 0543, 0899). Another specialization, on a trait that
/// isn't a fact, `TrustedLen`'s, is std's optimization of the one selected,
/// which changes nothing a program sees (`optimized_default`); but for
/// `SpecIterEq`'s, which `eq()`'s lowering asks itself (ADR 0992).
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
    let Ok(ImplSource::UserDefined(selected)) = crate::lower::traits::select(tcx, typing_env, tr) else {
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
                && same_kinds_of_generics(tcx, other, impl_id)
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

/// Whether impls `a` and `b` have as many generics of each kind, in order,
/// so one's arguments are the other's: not `SpecReadByte`'s for a
/// `StdinLock<'_>` of its general one's `R`, a lifetime where a type is.
fn same_kinds_of_generics(tcx: TyCtxt<'_>, a: DefId, b: DefId) -> bool {
    let kinds = |imp: DefId| {
        let generics = tcx.generics_of(imp);
        (0..generics.count())
            .map(|i| std::mem::discriminant(&generics.param_at(i, tcx).kind))
            .collect::<Vec<_>>()
    };
    kinds(a) == kinds(b)
}

/// A fact of `of`, or the facts of what it holds that it is, where rustc
/// can't say it of `of` and std's one impl of the trait for its type
/// constructor asks that trait alone of its parameters: `TrustedLen` of a
/// `&mut I`, `I`'s, as `impl<I: TrustedLen + ?Sized> TrustedLen for &mut
/// I` asks (ADR 0975). And the class of a map keyed by a tuple or an array
/// of a type parameter, `(usize, T)`: each part's a caller gives, which
/// `$keyClassOf` composes where it's made, never the tuple's, so what a
/// caller is asked stays its own parts' however deep a blanket impl's copy
/// at `Enumerate<Self>` builds them (#1346).
pub(crate) fn held_facts<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    of: Ty<'tcx>,
    fact: TypeFact,
) -> Vec<(Ty<'tcx>, TypeFact)> {
    if fact.map_kind().is_some()
        && let key = class_key(of)
        && mentions_param(key)
        && key_class_parts(key).is_some()
    {
        let mut held = Vec::new();
        let mut each = vec![key];
        while let Some(key) = each.pop() {
            // One known here, its class made here: none, and not taken
            // apart, as an array of half a billion bytes was one part an
            // item (#1368).
            if known_by_bounds(key) {
                held.push((key, fact));
            } else if mentions_param(key) {
                match key_class_parts(key) {
                    // An array's item once, of however many (#1371).
                    Some(parts) => each.extend(parts.types().iter().rev().map(|&part| class_key(part))),
                    // A struct made of one, which the lowering refuses
                    // (#1360): asked whole.
                    None => held.push((key, fact)),
                }
            }
        }
        return held;
    }
    // How std collects an iterator into a `Vec`, `IVecCollect`, of one
    // built of type parameters: of what says whether it's collected in
    // place, where that's a caller's, `collects_in_place`'s `None`, its
    // `IInPlace` and its items' layout, as `in_place_asked` asks them; and,
    // where it isn't, of what says whether it's `TrustedLen` (#1239).
    if fact == TypeFact::VecCollect && !known_by_bounds(of) {
        let Some(item) = iterator_item_of(tcx, typing_env, of) else {
            return Vec::new();
        };
        let in_place = collects_in_place(tcx, typing_env, of, item);
        let mut facts = Vec::new();
        if in_place.is_none() {
            facts.extend(held_facts(tcx, typing_env, of, TypeFact::InPlace));
            if fits_in_place(tcx, typing_env, item, item).is_none() {
                facts.extend([(item, TypeFact::Size), (item, TypeFact::Align)]);
            }
        }
        if in_place != Some(true) {
            facts.extend(vec_collect_trusted_facts(tcx, typing_env, of));
        }
        return facts;
    }
    // What an iterator's buffer std may collect in place is: what its
    // adapters keep's, `I`'s of a `Map<I, F>`, or a source's items' layout
    // (#672).
    if fact == TypeFact::InPlace {
        return match in_place_source(tcx, typing_env, of) {
            Some(InPlaceSource::Of(held)) => vec![(held, fact)],
            // Of a layout only a caller knows, a `vec::IntoIter<T>`'s.
            Some(InPlaceSource::Items(item)) if fits_in_place(tcx, typing_env, item, item).is_none() => {
                vec![(item, TypeFact::Size), (item, TypeFact::Align)]
            }
            Some(InPlaceSource::Items(_) | InPlaceSource::Never) => Vec::new(),
            None => vec![(of, fact)],
        };
    }
    // A reference's constant size, of a type only a caller knows, `&I`'s:
    // its pointee's, as std's `ConstSizeIntoIterator` gives `N` of a `[T;
    // N]`, a `&[T; N]` and a `&mut [T; N]` alike, and no reference to a
    // reference is an `IntoIterator` of one (ADR 0899, #1273).
    if fact == TypeFact::ConstSize
        && let ty::Ref(_, pointee, _) = *of.kind()
        && known_by_bounds(pointee)
    {
        return vec![(pointee, fact)];
    }
    let Some(fact_trait) = fact_trait(tcx, fact) else {
        return vec![(of, fact)];
    };
    (held_bounds(tcx, typing_env, of, fact_trait).into_iter())
        .map(|held| (held, fact))
        .collect()
}

/// Whether std collects an iterator of `iter` into a `Vec` of `item`s in
/// place: by its `InPlaceSource`, `None` where that's a caller's, of what
/// its adapters keep of a type parameter's, or of a source's items whose
/// layout is (#672).
pub(crate) fn collects_in_place<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    iter: Ty<'tcx>,
    item: Ty<'tcx>,
) -> Option<bool> {
    let in_place = match in_place_source(tcx, typing_env, iter) {
        None => None,
        Some(InPlaceSource::Never) => Some(false),
        Some(InPlaceSource::Items(src)) => fits_in_place(tcx, typing_env, src, item),
        Some(InPlaceSource::Of(_)) => None,
    };
    let answer = match in_place {
        Some(true) => "in-place",
        Some(false) => "apart",
        None => "callers",
    };
    crate::decisions::record("collect-in-place", answer);
    in_place
}

/// How std's `SpecFromIter` collects an iterator into a `Vec`,
/// `IVecCollect`'s value: in place where it is, else by its fold where it's
/// `TrustedLen`, else by `next` (#1239). Of what a generic caller's caller
/// gives, the runtime's `$vecCollectWay` answers it; test/vec-collect-ways.txt
/// holds both to one table.
pub(crate) fn vec_collect_way(in_place: bool, trusted: bool) -> &'static str {
    let way = match (in_place, trusted) {
        (true, _) => "inPlace",
        (false, true) => "fold",
        (false, false) => "next",
    };
    crate::decisions::record("vec-collect-way", way);
    way
}

/// The facts of what an iterator of `of`, built of type parameters, holds
/// that say whether it's `TrustedLen`, where rustc can't say it of `of`,
/// `I`'s of a `Map<I, F>`, or of one known by its bounds, its own: none
/// where it can, of a type known here or a `Filter<I, P>`, never one
/// (#1239).
pub(crate) fn vec_collect_trusted_facts<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    of: Ty<'tcx>,
) -> Vec<(Ty<'tcx>, TypeFact)> {
    if known_by_bounds(of) {
        return vec![(of, TypeFact::TrustedLen)];
    }
    if is_trusted_len(tcx, typing_env, of) {
        return Vec::new();
    }
    let held = held_facts(tcx, typing_env, of, TypeFact::TrustedLen);
    match held.iter().any(|&(held, _)| held == of) {
        true => Vec::new(),
        false => held,
    }
}

/// What an iterator of `iterator` gives, its `Item`, normalized.
fn iterator_item_of<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, iterator: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let item = trait_method(tcx, std_item(tcx, StdItem::Iterator), "Item");
    let projection = Ty::new_projection(tcx, ty::IsRigid::No, item, [iterator]);
    tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(projection))
        .ok()
}

/// The types whose `trait_id` is `of`'s, `held_facts`' of a trait: `of`,
/// where rustc can say it of `of` or can't tell it by them; or, where std's
/// one impl of the trait for its type constructor asks that trait of its
/// parameters, what `of` holds there, each's.
fn held_bounds<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    of: Ty<'tcx>,
    trait_id: DefId,
) -> Vec<Ty<'tcx>> {
    let held = match *of.kind() {
        ty::Ref(_, pointee, mutability) => Some((Err(mutability), vec![pointee])),
        ty::Adt(adt, args) => Some((Ok(adt.did()), args.types().collect())),
        _ => None,
    };
    let Some((head, held)) = held else {
        return vec![of];
    };
    if trait_holds_in(tcx, typing_env, of, trait_id).is_some() {
        return vec![of];
    }
    // Its impl's self type's parameters, in order, each a parameter.
    let impls: Vec<(DefId, Vec<Ty<'tcx>>)> = (tcx.all_impls(trait_id))
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
        return vec![of];
    };
    if !params.iter().all(|param| matches!(param.kind(), ty::Param(_))) {
        return vec![of];
    }
    let mut bounds = Vec::new();
    for (clause, _) in tcx.clauses_of(*imp).instantiate_identity(tcx) {
        let Some(bound) = clause.skip_normalization().as_trait_clause() else {
            continue;
        };
        if bound.def_id() != trait_id {
            // One the impl asks of its parameters by another trait that
            // what `of` holds there may not have, a `vec::IntoIter<T>`'s
            // `TrustedRandomAccessNoCoerce` of `T: NonDrop`, a `Copy`
            // item's: not its parts' facts of this one, so asked of the
            // whole (#1389). Not one it has, `Vec<T, Global>`'s `A:
            // Allocator`.
            if let Some(at) = params.iter().position(|&param| param == bound.self_ty().skip_binder())
                && trait_holds_in(tcx, typing_env, held[at], bound.def_id()) != Some(true)
            {
                return vec![of];
            }
            continue;
        }
        let Some(at) = params.iter().position(|&param| param == bound.self_ty().skip_binder()) else {
            return vec![of];
        };
        bounds.extend(held_bounds(tcx, typing_env, held[at], trait_id));
    }
    bounds
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

/// What a call of `callee` with `args` asks of its caller's types for
/// `==` of `Rc`s or `Arc`s it makes, by its bounds: of each `X: PartialEq`
/// whose `X` holds `Rc<T>`s, through references, tuples, arrays and std's
/// types, `T: PartialEq` of `contains` of `Rc<T>`s, `Rc<T>: PartialEq` of
/// `==` itself, whether `T` is `Eq`, where rustc can't say it here: std's
/// `RcEqIdent` asks whether they're one allocation first of an `Eq` `T`,
/// which only the caller's type says (#931). Each as `held_facts` has it,
/// `T`'s of a `Vec<T>`.
pub(crate) fn rc_eq_asked<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    callee: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Vec<(Ty<'tcx>, TypeFact)> {
    let (Some(partial_eq), Some(eq)) = (tcx.lang_items().get(LangItem::PartialEq), fact_trait(tcx, TypeFact::Eq))
    else {
        return Vec::new();
    };
    fn pointees<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, found: &mut Vec<Ty<'tcx>>) {
        match ty.kind() {
            ty::Ref(_, inner, _) | ty::Array(inner, _) | ty::Slice(inner) => pointees(tcx, *inner, found),
            ty::Tuple(parts) => parts.iter().for_each(|part| pointees(tcx, part, found)),
            ty::Adt(..) if let Some(pointee) = rc_pointee(tcx, ty) => found.push(pointee),
            ty::Adt(adt, args) if super::is_std_crate(tcx, adt.did()) => {
                args.types().for_each(|arg| pointees(tcx, arg, found));
            }
            _ => {}
        }
    }
    let mut found = Vec::new();
    for (clause, _) in tcx.clauses_of(callee).instantiate(tcx, args) {
        let Some(bound) = clause.skip_normalization().as_trait_clause() else {
            continue;
        };
        if bound.def_id() != partial_eq {
            continue;
        }
        let of = tcx
            .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(bound.self_ty().skip_binder()))
            .unwrap_or(bound.self_ty().skip_binder());
        pointees(tcx, of, &mut found);
    }
    (found.into_iter())
        .filter(|&pointee| {
            rustc_middle::ty::TypeVisitableExt::has_param(&pointee)
                && trait_holds_in(tcx, typing_env, pointee, eq) != Some(true)
        })
        .flat_map(|pointee| held_facts(tcx, typing_env, pointee, TypeFact::Eq))
        .collect()
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
        TypeFact::RandomAccess => registry::by_path(tcx, StdCrate::Core, &["iter", "TrustedRandomAccess"]),
        TypeFact::RandomAccessNoCoerce => {
            registry::by_path(tcx, StdCrate::Core, &["iter", "TrustedRandomAccessNoCoerce"])
        }
        TypeFact::TrustedLen => registry::by_path(tcx, StdCrate::Core, &["iter", "TrustedLen"]),
        TypeFact::Eq => Some(super::std_item(tcx, StdItem::Eq)),
        _ => None,
    }
}

/// What std's `collect()` of an iterator into a `Vec` may collect it in,
/// in place, the buffer of the `vec::IntoIter` or `BinaryHeap`'s it steps:
/// through adapters that keep it, each `InPlaceIterable` and `SourceIter`
/// of one that's `AsVecIntoIter`. Where a panic stops a collect in place,
/// what it collected is dropped before what's left, its guard the fold's;
/// otherwise what's left first, the fold's own, then the `Vec` (#423).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum InPlaceSource<'tcx> {
    /// None: an adapter in it is never `InPlaceIterable` or a `SourceIter`,
    /// a `Fuse` or a `Chain`, or what it steps is no `AsVecIntoIter`.
    Never,
    /// The buffer of a source of these items.
    Items(Ty<'tcx>),
    /// What a type parameter's iterator, or an associated type's, may be
    /// collected in, which the adapters over it keep: its caller's,
    /// `IInPlace` (#672).
    Of(Ty<'tcx>),
}

/// `iter`'s `InPlaceSource`, in `typing_env`: `None` where the adapters'
/// impls can't tell it, as each asks the traits of one it holds.
pub(crate) fn in_place_source<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    iter: Ty<'tcx>,
) -> Option<InPlaceSource<'tcx>> {
    let [in_place, source_iter] = in_place_traits(tcx)?;
    let as_vec = alloc_trait(tcx, "AsVecIntoIter")?;
    if !may_collect_in_place(tcx, typing_env, iter) {
        return Some(InPlaceSource::Never);
    }
    // What the adapters keep in place, the one each's impl asks both of.
    let [held] = held_bounds(tcx, typing_env, iter, in_place)[..] else {
        return None;
    };
    if held_bounds(tcx, typing_env, iter, source_iter) != [held] {
        return None;
    }
    if known_by_bounds(held) {
        return Some(InPlaceSource::Of(held));
    }
    let holds = |ty: Ty<'tcx>, trait_id: DefId| trait_holds_in(tcx, typing_env, ty, trait_id);
    if holds(held, in_place) != Some(true) || holds(held, source_iter) != Some(true) {
        return None;
    }
    let projected = |assoc: DefId, of: Ty<'tcx>| {
        let ty = Ty::new_projection(tcx, ty::IsRigid::No, assoc, [of]);
        tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))
            .ok()
    };
    let source = projected(trait_method(tcx, source_iter, "Source"), held)?;
    match holds(source, as_vec)? {
        true => Some(InPlaceSource::Items(projected(
            trait_method(tcx, as_vec, "Item"),
            source,
        )?)),
        false => Some(InPlaceSource::Never),
    }
}

/// What std's adapter `adt` holds that it reaches its in-place source
/// through, its `SourceIter::as_inner`'s one step: the place its body
/// gives on to the `as_inner` of what it holds, `&mut self.iter` of a
/// `Map`'s, `&mut self.a` of a `Zip`'s, `self.iter`'s `Some` of a `Fuse`'s,
/// `&mut self.inner.iter` of a `Flatten`'s, each step from `*self`.
/// A collect in place drops what's left of the source alone under its
/// guard, the adapters after it (#852). `None` of one that's no
/// `SourceIter`.
pub(crate) fn source_iter_step(tcx: TyCtxt<'_>, adt: DefId) -> Option<Vec<SourceStep>> {
    let [_, source_iter] = in_place_traits(tcx)?;
    let as_inner = trait_method(tcx, source_iter, "as_inner");
    let imp = (tcx.all_impls(source_iter)).find(|&imp| {
        matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
            ty::Adt(of, _) if of.did() == adt)
    })?;
    let method = *tcx.impl_item_implementor_ids(imp).get(&as_inner)?;
    let body = tcx.optimized_mir(method);
    let mut steps = (body.basic_blocks.iter()).filter_map(|data| {
        let TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
            return None;
        };
        let (callee, _) = func.const_fn_def()?;
        let [arg] = &args[..] else {
            return None;
        };
        (callee == as_inner).then(|| borrowed_from_self(body, arg.node.place()?))?
    });
    let step = steps.next();
    debug_assert!(
        step.is_some() && steps.next().is_none(),
        "`{}`'s `as_inner` gives on one place of its own",
        tcx.def_path_str(adt)
    );
    // Each projection, of the type it's of: a struct's field, or the
    // payload of an `Option`'s `Some`, its downcast then its field.
    let mut of = mir::PlaceTy::from_ty(tcx.type_of(adt).instantiate_identity().skip_normalization());
    let mut found = Vec::new();
    for elem in step? {
        match (elem, of.ty.kind()) {
            (mir::ProjectionElem::Downcast(..), ty::Adt(option, _))
                if tcx.is_diagnostic_item(sym::Option, option.did()) => {}
            (mir::ProjectionElem::Field(..), ty::Adt(option, _))
                if tcx.is_diagnostic_item(sym::Option, option.did()) =>
            {
                found.push(SourceStep::SomeValue)
            }
            (mir::ProjectionElem::Field(field, _), ty::Adt(of, _)) if of.is_struct() => {
                found.push(SourceStep::Field(field))
            }
            _ => unreachable!("`as_inner` reads a field of a struct or an `Option`'s `Some`, not {elem:?}"),
        }
        of = of.projection_ty(tcx, elem);
    }
    Some(found)
}

/// One step of a `SourceIter::as_inner`'s place (`source_iter_step`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SourceStep {
    /// A struct's field.
    Field(rustc_abi::FieldIdx),
    /// What an `Option`'s `Some` holds.
    SomeValue,
}

/// What `place` of `body` holds, followed through what it's assigned, a
/// copy or a `Some` made of one, to a borrow of a place of `*self`,
/// `&mut (*_1).0`: that place's projections from `*self`. A `Fuse`'s
/// `as_inner` matches its `Option` by `as_mut`, inlined, its `Some` made
/// of the borrow then read (#852).
fn borrowed_from_self<'tcx>(body: &mir::Body<'tcx>, place: mir::Place<'tcx>) -> Option<Vec<mir::PlaceElem<'tcx>>> {
    let assigned =
        (body.basic_blocks.iter().flat_map(|data| &data.statements)).find_map(|statement| match &statement.kind {
            mir::StatementKind::Assign(assign) if assign.0.as_local() == Some(place.local) => Some(&assign.1),
            _ => None,
        })?;
    let from_operand = |operand: &mir::Operand<'tcx>| borrowed_from_self(body, operand.place()?);
    match (assigned, &place.projection[..]) {
        (mir::Rvalue::Ref(_, _, of), []) if of.local == mir::Local::from_u32(1) => match &of.projection[..] {
            [mir::ProjectionElem::Deref, rest @ ..] => Some(rest.to_vec()),
            _ => None,
        },
        (mir::Rvalue::Use(operand, _), []) => from_operand(operand),
        (
            mir::Rvalue::Aggregate(kind, operands),
            [
                mir::ProjectionElem::Downcast(_, variant),
                mir::ProjectionElem::Field(field, _),
            ],
        ) if matches!(**kind, mir::AggregateKind::Adt(_, made, ..) if made == *variant) => {
            from_operand(&operands[*field])
        }
        _ => None,
    }
}

/// A call of `callee` with `args` that collects an iterator into a `Vec`,
/// or a `Result` or an `Option` of one, `collect()` or `from_iter`: the
/// iterator's type, the collection's, the `Ok`'s or the `Some`'s, and the
/// type of the items it collects, each `Ok`'s or `Some`'s (#672, #796),
/// and whether it's a `Result`'s or an `Option`'s, which std's
/// `GenericShunt` collects (#1180). Another collection's too, which the
/// caller tells apart.
pub(crate) fn collected_items<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    callee: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<(Ty<'tcx>, Ty<'tcx>, Ty<'tcx>, bool)> {
    let iterator = std_item(tcx, StdItem::Iterator);
    let projected = |trait_id: DefId, name: &str, of: Ty<'tcx>| {
        let ty = Ty::new_projection(tcx, ty::IsRigid::No, trait_method(tcx, trait_id, name), [of]);
        tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty)
    };
    let (iter, output) = if callee == trait_method(tcx, iterator, "collect") {
        (args.type_at(0), args.type_at(1))
    } else {
        let given = from_iter_given(tcx, callee, args)?;
        let into_iter = std_item(tcx, StdItem::IntoIterator);
        (projected(into_iter, "IntoIter", given), args.type_at(0))
    };
    let item = projected(iterator, "Item", iter);
    let fallible = matches!(output.kind(), ty::Adt(adt, _)
        if [sym::Result, sym::Option].into_iter().any(|name| tcx.is_diagnostic_item(name, adt.did())));
    match (fallible, item.kind(), output.kind()) {
        (true, ty::Adt(_, item_args), ty::Adt(_, output_args)) => {
            Some((iter, output_args.types().next()?, item_args.types().next()?, true))
        }
        (true, ..) => None,
        (false, ..) => Some((iter, output, item, false)),
    }
}

/// Whether a buffer of `src`s is collected into as one of `dest`s, as
/// alloc's `in_place_collectible` asks of one item a step: its items as
/// aligned as `dest`s and no smaller, neither zero-sized. `None` where
/// either's layout is a caller's.
pub(crate) fn fits_in_place<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    src: Ty<'tcx>,
    dest: Ty<'tcx>,
) -> Option<bool> {
    let layout = |ty: Ty<'tcx>| tcx.layout_of(typing_env.as_query_input(ty)).ok();
    let (src, dest) = (layout(src)?, layout(dest)?);
    Some(!src.is_zst() && !dest.is_zst() && src.align.abi == dest.align.abi && src.size >= dest.size)
}

/// Whether std may collect `iter`, of a type holding a type parameter, in
/// place, for some type its parameters are given:
/// not where an adapter in it is never `InPlaceIterable` or never a
/// `SourceIter`, a `Fuse`, a `Peekable` or a `Chain`, as each impl of
/// those of an adapter asks them of what it holds, `Map<I, F>`'s of `I`
/// (#672). Of a type parameter, or an associated type's, its caller's,
/// which may be.
pub(crate) fn may_collect_in_place<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, iter: Ty<'tcx>) -> bool {
    let Some(traits) = in_place_traits(tcx) else {
        return false;
    };
    traits.into_iter().all(|trait_id| {
        held_bounds(tcx, typing_env, iter, trait_id)
            .into_iter()
            .all(|held| known_by_bounds(held) || trait_holds_in(tcx, typing_env, held, trait_id) != Some(false))
    })
}

/// std's `InPlaceIterable` and `SourceIter`, the traits of an iterator its
/// collect into a `Vec` may collect in place.
fn in_place_traits(tcx: TyCtxt<'_>) -> Option<[DefId; 2]> {
    Some([
        registry::by_path(tcx, StdCrate::Core, &["iter", "InPlaceIterable"])?,
        registry::by_path(tcx, StdCrate::Core, &["iter", "SourceIter"])?,
    ])
}

/// One of alloc's own traits, crate-private, by its name.
fn alloc_trait(tcx: TyCtxt<'_>, name: &str) -> Option<DefId> {
    let alloc = std_crate_num(tcx, StdCrate::Alloc)?;
    (tcx.traits(alloc).iter().copied()).find(|&id| tcx.item_name(id).as_str() == name)
}

/// std's `OneShot`, private to `flatten`'s module: an iterator of one item
/// at most, `option::IntoIter`'s (ADR 0899).
pub(crate) fn one_shot_trait(tcx: TyCtxt<'_>) -> Option<DefId> {
    let core = std_crate_num(tcx, StdCrate::Core)?;
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
/// upper bound never fails (ADR 0932). And `Bytes::next`'s `SpecReadByte`,
/// whose `BufReader` impl takes the byte from its buffer where it has one,
/// else reads it as the general impl does, a one-byte `read` of the
/// `BufReader`, which takes it from that buffer, filled as it would be:
/// the same byte, and the same reads of what it holds (#1156).
const OPTIMIZING_BY_TYPE: &[&str] = &[
    "SpecRangeSetup",
    "StepByImpl",
    "StepByBackImpl",
    "SpecAssumeCount",
    "SpecReadByte",
];

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
    let Ok(ImplSource::UserDefined(selected)) = crate::lower::traits::select(tcx, typing_env, tr) else {
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

    /// Whether `ty`, a type of this body's type parameters, may have
    /// `trait_id`'s impl as its own bounds say: `Take<I>`'s
    /// `DoubleEndedIterator` where `I` is bound by `Iterator` alone may
    /// not, whatever its callers give, so nothing it's given to uses it
    /// (#1240).
    pub(crate) fn may_hold_by_bounds(&self, ty: Ty<'tcx>, trait_id: DefId) -> bool {
        use rustc_trait_selection::infer::TyCtxtInferExt;
        use rustc_trait_selection::traits::query::evaluate_obligation::InferCtxtExt;
        let infcx = self.tcx.infer_ctxt().build(ty::TypingMode::PostAnalysis);
        let obligation = rustc_trait_selection::traits::Obligation::new(
            self.tcx,
            rustc_trait_selection::traits::ObligationCause::dummy(),
            self.typing_env.param_env,
            ty::TraitRef::new(self.tcx, trait_id, [self.tcx.erase_and_anonymize_regions(ty)]),
        );
        infcx.predicate_may_hold(&obligation)
    }

    /// Whether `ty`, a type of generic code's, `I` or `Take<I>` of an
    /// `I: Iterator`, is never a `DoubleEndedIterator`, whatever its
    /// callers give, as its own bounds say: nothing steps it from its back,
    /// here or where it's given, so a map of it made for its hint,
    /// `$mapEnds`, is made of it as it is (#1240).
    pub(crate) fn never_double_ended(&self, ty: Ty<'tcx>) -> bool {
        rustc_middle::ty::TypeVisitableExt::has_param(&ty)
            && self
                .double_ended_iterator()
                .is_some_and(|double_ended| !self.may_hold_by_bounds(ty, double_ended))
    }
}

/// `Recognition::trait_holds` in `typing_env`.
pub(crate) fn trait_holds_in<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: Ty<'tcx>,
    trait_id: DefId,
) -> Option<bool> {
    let tr = tcx.erase_and_anonymize_regions(ty::TraitRef::new(tcx, trait_id, [ty]));
    match crate::lower::traits::select(tcx, typing_env, tr) {
        Ok(_) => Some(true),
        Err(rustc_middle::traits::CodegenObligationError::Unimplemented)
            if !rustc_middle::ty::TypeVisitableExt::has_param(&ty) || !has_impl_for_head(tcx, ty, trait_id) =>
        {
            Some(false)
        }
        // Of one holding a type parameter that no types given for it can
        // make hold (`may_hold_for_some`), never, whatever its callers give.
        Err(_) if !crate::lower::generic_facts::may_hold_for_some(tcx, ty, trait_id) => Some(false),
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

#[cfg(test)]
mod tests {
    /// `vec_collect_way` answers test/vec-collect-ways.txt's table, as
    /// the runtime's `$vecCollectWay` does (test/runtime-package.test.ts).
    #[test]
    fn vec_collect_ways() {
        let table = include_str!("../../../test/vec-collect-ways.txt");
        let rows: Vec<_> = (table.lines())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| line.split('\t').collect::<Vec<_>>())
            .collect();
        assert_eq!(rows.len(), 4);
        for row in rows {
            let [in_place, trusted, way] = row[..] else {
                panic!("a row of three: {row:?}")
            };
            assert_eq!(
                super::vec_collect_way(in_place == "true", trusted == "true"),
                way,
                "{row:?}"
            );
        }
    }
}
