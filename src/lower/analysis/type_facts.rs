//! What each generic function asks of its type parameters (ADR 0145):
//! `size_of::<T>()`, `align_of::<T>()` and `type_name::<T>()`, which a
//! caller's type answers.

use crate::lower::recognition::{
    TypeFact, bound_by_any, debug_named, phantom_named, pointer_target, points_at_dyn_any, points_at_dyn_debug,
    type_facts,
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
    for body in all_bodies {
        let caller = tcx.typeck_root_def_id(body.def_id.to_def_id());
        let typing_env = ty::TypingEnv::non_body_analysis(tcx, caller);
        for expr in body.thir.exprs.iter() {
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
            // A trait's method of a type that names it, `stack.extend(iter)`
            // of a `Stack<T>`: its impl's, which asks of the impl's parameters.
            let (callee, args) = match tcx.trait_of_assoc(callee) {
                Some(_) => match ty::Instance::try_resolve(tcx, typing_env, callee, args) {
                    Ok(Some(instance)) if matches!(instance.def, ty::InstanceKind::Item(_)) => {
                        (instance.def_id(), instance.args)
                    }
                    _ => (callee, args),
                },
                None => (callee, args),
            };
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
            if !fns.contains_key(&callee) || tcx.trait_of_assoc(callee).is_some() {
                continue;
            }
            for (index, arg) in args.iter().enumerate() {
                if let Some(ty) = arg.as_type()
                    && let ty::Param(param) = ty.kind()
                {
                    passed.push(((caller, param.index), (callee, index as u32)));
                }
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
        if tcx.trait_of_assoc(id).is_some() {
            continue;
        }
        match tcx.trait_impl_of_assoc(id) {
            // Of the impl's own parameters: the method's, which its
            // dictionary, asked them above, gives it of its own.
            Some(impl_id) if (index as usize) < tcx.generics_of(impl_id).count() => {
                params.entry(id).or_default().push((index, fact));
            }
            Some(_) => {}
            None => params.entry(id).or_default().push((index, fact)),
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
