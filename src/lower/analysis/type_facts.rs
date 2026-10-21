//! What each generic function asks of its type parameters (ADR 0145):
//! `size_of::<T>()`, `align_of::<T>()` and `type_name::<T>()`, the class
//! of a map keyed by one (ADR 0466), and what std's unstable sort of them
//! picks (ADR 0471), which a caller's type answers.

use crate::lower::recognition::{
    TypeFact, bound_by_any, debug_named, map_of_param, phantom_named, pointer_target, points_at_dyn_any,
    points_at_dyn_debug, type_facts, unstable_sort_items,
};
use crate::lower::traits::{Given, bounds, evidence_impl};
use crate::lower::{Body, FnInfo, fn_def};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty;
use rustc_middle::ty::TyCtxt;
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_span::def_id::DefId;
use std::collections::{HashMap, HashSet};

/// The facts each of the crate's generic functions asks of its type
/// parameters, by their index, directly, in a closure of its own, or
/// through a generic function of the crate's it passes the parameter on to:
/// a caller gives each, after the function's dictionaries. An impl's method,
/// called through its dictionary, asks its impl too of the impl's type
/// parameters, which the dictionary is made with, `wrapFootprint(TSize)`,
/// and gives its method. A caller asks what each impl it gives a dictionary
/// of for a callee's bound asks, a trait's method's `Self: Trait` too:
/// `stack.extend(iter)`'s `Stack<T>: Extend`, and `collect()`'s `B:
/// FromIterator` (ADR 0445). A
/// trait's method, an impl's of its own type parameters, and a library's
/// function its consumers can reach, which never see this, ask none: theirs
/// are errors.
pub(super) fn type_fact_params<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    fns: &HashMap<DefId, FnInfo>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    given: Given<'_>,
) -> HashMap<DefId, Vec<(u32, TypeFact)>> {
    let mut asked: HashSet<(DefId, u32, TypeFact)> = HashSet::new();
    // What a trait's dictionaries carry of its associated type, asked of
    // `<A as Array>::Item` (ADR 0145), and a caller's projection passed on
    // as a callee's type parameter.
    let mut carried: HashSet<(DefId, TypeFact)> = HashSet::new();
    let mut passed_carried: Vec<(DefId, (DefId, u32))> = Vec::new();
    // A caller's type parameter passed on as a callee's: `relay<U>` calling `describe::<U>`.
    let mut passed: Vec<((DefId, u32), (DefId, u32))> = Vec::new();
    // A call of the crate's function, with its arguments: what its callee asks
    // of a slot, instantiated, is of the caller's own where it's one of its
    // slots, `Self::Item` of `self.map(f).counts()`'s `K` (ADR 0466).
    let mut calls: Vec<(DefId, DefId, ty::GenericArgsRef<'tcx>)> = Vec::new();
    for body in all_bodies {
        let caller = tcx.typeck_root_def_id(body.def_id.to_def_id());
        let typing_env = ty::TypingEnv::non_body_analysis(tcx, caller);
        for expr in body.thir.exprs.iter() {
            // A map or a set of a type parameter's keys a std call makes,
            // `HashMap::new()`'s or `collect()`'s: its class, its caller's
            // (ADR 0466).
            if let ExprKind::Call { fun, .. } = expr.kind
                && let Some((callee, _)) = fn_def(body.thir[fun].ty)
                && !callee.is_local()
                && !foreign.in_library(callee)
                && let Some((key, fact)) = map_of_param(tcx, expr.ty)
                && let Some(index) = slot_of(tcx, foreign, caller, key)
            {
                asked.insert((caller, index, fact));
            }
            // A type parameter's value made a `dyn Any`, whose dictionary is
            // its `TypeId` (ADR 0331).
            if let ExprKind::PointerCoercion {
                cast: PointerCoercion::Unsize,
                source,
                ..
            } = expr.kind
                && let ty::Param(param) = pointer_target(tcx, body.thir[source].ty).kind()
                && points_at_dyn_any(tcx, expr.ty)
                && !bound_by_any(tcx, caller, param.index)
            {
                asked.insert((caller, param.index, TypeFact::Id));
                continue;
            }
            // A `PhantomData<K>` made a `dyn Debug`, a derived `Debug`'s
            // field, shows `K`'s name (ADR 0452).
            if let ExprKind::PointerCoercion {
                cast: PointerCoercion::Unsize,
                source,
                ..
            } = expr.kind
                && points_at_dyn_debug(tcx, expr.ty)
            {
                for index in phantom_named(tcx, body.thir[source].ty) {
                    asked.insert((caller, index, TypeFact::Name));
                }
                continue;
            }
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            // `fuse()` of a type parameter's iterator, whose `Fuse` keeps it
            // as it ends where it's a `FusedIterator`: its caller's, `IFused`
            // (ADR 0543).
            // Of the function's own parameter: an impl's is given no fact.
            if crate::lower::recognition::is_iterator_fuse(tcx, callee)
                && let Some(index) = args.types().next().and_then(|it| slot_of(tcx, foreign, caller, it))
                && index >= tcx.generics_of(caller).parent_count as u32
            {
                asked.insert((caller, index, TypeFact::Fused));
                continue;
            }
            // An unstable sort of a type parameter's items, whose order std
            // picks by their layout: its caller's, `TSort` (ADR 0471).
            if let Some(item) = unstable_sort_items(tcx, callee, args)
                && let Some(index) = slot_of(tcx, foreign, caller, item)
            {
                asked.insert((caller, index, TypeFact::Sort));
                continue;
            }
            let facts = type_facts(tcx, callee);
            for &fact in facts {
                // `size_of_val` of an unsized `T` is the value's, not the type's:
                // a size or an alignment is asked of a sized one, and its name
                // of any, a `T: ?Sized`'s too.
                // A downcast's type is its last, after its `Box`'s allocator.
                // A `T: Any`'s `TypeId` is its dictionary's (ADR 0331).
                if let Some(of) = args.types().last()
                    && let ty::Param(param) = of.kind()
                    && (of.is_sized(tcx, typing_env) || !matches!(fact, TypeFact::Size | TypeFact::Align))
                    && !(fact == TypeFact::Id && bound_by_any(tcx, caller, param.index))
                {
                    asked.insert((caller, param.index, fact));
                }
                // Of a type that holds them, `Cow<'_, T>`: its name, each one's.
                if fact == TypeFact::Name
                    && let Some(of) = args.types().last()
                    && !matches!(of.kind(), ty::Param(_))
                {
                    for part in of.walk().filter_map(|part| part.as_type()) {
                        if let ty::Param(param) = part.kind() {
                            asked.insert((caller, param.index, fact));
                        }
                    }
                }
                if let Some(assoc) = args.types().last().and_then(|of| carried_by(of)) {
                    carried.insert((assoc, fact));
                }
            }
            if !facts.is_empty() {
                continue;
            }
            // Each impl whose dictionary it's given for a bound, `collect()`'s
            // `B: FromIterator` of an `ArrayVec<T, CAP>`: what that asks of
            // its type parameters, of a caller's given it, the caller asks.
            for bound in bounds(tcx, foreign, given, callee) {
                let bound = ty::EarlyBinder::bind(tcx, bound).instantiate(tcx, args);
                let Ok(bound) = tcx.try_normalize_erasing_regions(typing_env, bound) else {
                    continue;
                };
                // A `Debug` of a `PhantomData<K>`, `{:?}`'s, shows `K`'s name.
                for index in debug_named(tcx, bound) {
                    asked.insert((caller, index, TypeFact::Name));
                }
                let Some((impl_id, impl_args)) = evidence_impl(tcx, typing_env, bound) else {
                    continue;
                };
                for (index, arg) in impl_args.iter().enumerate() {
                    if let Some(ty) = arg.as_type()
                        && let ty::Param(param) = ty.kind()
                    {
                        match impl_id.is_local() {
                            true => passed.push(((caller, param.index), (impl_id, index as u32))),
                            false => {
                                for (at, fact) in foreign.type_facts(impl_id) {
                                    if at as usize == index {
                                        asked.insert((caller, param.index, fact));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // A library's function, which asks what its manifest says: of a
            // caller's type parameter given it, the caller asks it too.
            if !callee.is_local() {
                for (index, fact) in foreign.type_facts(callee) {
                    if let Some(ty) = args.get(index as usize).and_then(|arg| arg.as_type())
                        && let ty::Param(param) = ty.kind()
                    {
                        asked.insert((caller, param.index, fact));
                    }
                }
                continue;
            }
            // A trait's method of the crate's, called through its dictionary
            // or resolved, of its own type parameters (ADR 0466).
            let trait_method = callee.is_local() && tcx.trait_of_assoc(callee).is_some();
            if !trait_method && (!fns.contains_key(&callee) || tcx.trait_of_assoc(callee).is_some()) {
                continue;
            }
            calls.push((caller, callee, args));
            for (index, arg) in args.iter().enumerate() {
                if let Some(assoc) = arg.as_type().and_then(|ty| carried_by(ty)) {
                    passed_carried.push((assoc, (callee, index as u32)));
                }
            }
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        // An impl's method asks its impl, whose dictionary gives it, of the
        // impl's own type parameters.
        let lifted: Vec<_> = (asked.iter())
            .filter_map(|&(id, index, fact)| {
                let impl_id = tcx.trait_impl_of_assoc(id)?;
                ((index as usize) < tcx.generics_of(impl_id).count()).then_some((impl_id, index, fact))
            })
            .collect();
        for fact in lifted {
            changed |= asked.insert(fact);
        }
        // A trait method's own type parameter's fact is the trait's, and each
        // impl's of it, as a caller through its dictionary gives it (ADR 0466).
        let own: Vec<_> = (asked.iter())
            .filter_map(|&(id, index, fact)| {
                let method = own_method(tcx, id)?;
                let generics = tcx.generics_of(id);
                let parent = generics.parent_count as u32;
                (index >= parent && index < generics.count() as u32).then(|| (method, index - parent, fact))
            })
            .collect();
        for (method, at, fact) in own {
            for id in implementors(tcx, method) {
                changed |= asked.insert((id, tcx.generics_of(id).parent_count as u32 + at, fact));
            }
        }
        for &(caller, callee, args) in &calls {
            let typing_env = ty::TypingEnv::non_body_analysis(tcx, caller);
            let facts: Vec<(u32, TypeFact)> = (asked.iter())
                .filter(|&&(id, _, _)| id == callee)
                .map(|&(_, index, fact)| (index, fact))
                .collect();
            for (index, fact) in facts {
                let slot = crate::lower::traits::drop_slot(tcx, foreign, callee, index);
                let of = ty::EarlyBinder::bind(tcx, slot)
                    .instantiate(tcx, args)
                    .skip_normalization();
                let of = tcx
                    .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(of))
                    .unwrap_or(of);
                if let Some(at) = slot_of(tcx, foreign, caller, of) {
                    changed |= asked.insert((caller, at, fact));
                }
            }
        }
        for &((caller, from), (callee, to)) in &passed {
            for fact in TypeFact::ALL {
                if asked.contains(&(callee, to, fact)) && asked.insert((caller, from, fact)) {
                    changed = true;
                }
            }
        }
        for &(assoc, (callee, to)) in &passed_carried {
            for fact in TypeFact::ALL {
                if asked.contains(&(callee, to, fact)) {
                    changed |= carried.insert((assoc, fact));
                }
            }
        }
        // Each impl's dictionary carries it of its type for it: `T`'s of
        // `type Item = T`, which its impl asks of its own parameter.
        let impls: Vec<_> = (carried.iter())
            .flat_map(|&(assoc, fact)| {
                let trait_id = tcx.trait_of_assoc(assoc).expect("an associated type");
                tcx.all_impls(trait_id)
                    .filter(|imp| imp.is_local())
                    .filter_map(move |imp| {
                        let item = *tcx.impl_item_implementor_ids(imp).get(&assoc)?;
                        Some((imp, tcx.type_of(item).instantiate_identity().skip_normalization(), fact))
                    })
            })
            .collect();
        for (imp, ty, fact) in impls {
            if let ty::Param(param) = ty.kind() {
                changed |= asked.insert((imp, param.index, fact));
            }
            if let Some(assoc) = carried_by(ty) {
                changed |= carried.insert((assoc, fact));
            }
        }
    }
    let mut params: HashMap<DefId, Vec<(u32, TypeFact)>> = HashMap::new();
    for (id, index, fact) in asked {
        // A trait's method's of its own type parameters, and an impl's
        // method's, its caller's through the dictionary (ADR 0466); and of
        // the impl's own parameters, the method's, which its dictionary,
        // asked them above, gives it of its own.
        let own = index >= tcx.generics_of(id).parent_count as u32;
        let ask = match (tcx.trait_of_assoc(id), tcx.trait_impl_of_assoc(id)) {
            (Some(_), _) => own,
            (None, Some(impl_id)) => own || (index as usize) < tcx.generics_of(impl_id).count(),
            (None, None) => true,
        };
        if ask {
            params.entry(id).or_default().push((index, fact));
        }
    }
    // An associated type's own entry: what its trait's dictionaries carry.
    for (assoc, fact) in carried {
        params.entry(assoc).or_default().push((0, fact));
    }
    for facts in params.values_mut() {
        facts.sort();
        facts.dedup();
    }
    params
}

/// The associated type of the crate's own trait `ty` projects, whose facts
/// the trait's dictionaries carry: `<A as Array>::Item`.
fn carried_by(ty: ty::Ty<'_>) -> Option<DefId> {
    match ty.kind() {
        ty::Alias(
            _,
            ty::AliasTy {
                kind: ty::Projection { def_id },
                ..
            },
        ) if def_id.is_local() => Some(*def_id),
        _ => None,
    }
}

/// The trait method of the crate's `id` is, or implements: `None` of a
/// function that's neither.
fn own_method(tcx: TyCtxt<'_>, id: DefId) -> Option<DefId> {
    if !id.is_local() {
        return None;
    }
    if tcx.trait_of_assoc(id).is_some() {
        return Some(id);
    }
    tcx.trait_impl_of_assoc(id)?;
    tcx.associated_item(id)
        .trait_item_def_id()
        .filter(|method| method.is_local())
}

/// A trait method of the crate's, and each of its impls' methods of it.
fn implementors(tcx: TyCtxt<'_>, method: DefId) -> Vec<DefId> {
    let trait_id = tcx.trait_of_assoc(method).expect("a trait's method");
    std::iter::once(method)
        .chain(
            tcx.all_impls(trait_id)
                .filter(|imp| imp.is_local())
                .filter_map(|imp| tcx.impl_item_implementor_ids(imp).get(&method).copied()),
        )
        .collect()
}

/// Which of `id`'s slots `ty` is (`drop_slot`): a type parameter's index, or
/// a std associated type of one's, after them, `Self::Item` (ADR 0466).
fn slot_of<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    id: DefId,
    ty: ty::Ty<'tcx>,
) -> Option<u32> {
    match ty.kind() {
        ty::Param(param) => Some(param.index),
        ty::Alias(..) => {
            let alias = crate::lower::traits::alias_of(ty)?;
            let at = (crate::lower::traits::drop_items(tcx, foreign, id).into_iter())
                .position(|item| crate::lower::traits::alias_of(item) == Some(alias))?;
            Some((tcx.generics_of(id).count() + at) as u32)
        }
        _ => None,
    }
}
