//! What each generic function asks of its type parameters (ADR 0145):
//! `size_of::<T>()`, `align_of::<T>()` and `type_name::<T>()`, the class
//! of a map keyed by one (ADR 0466), and what std's unstable sort of them
//! picks (ADR 0471), which a caller's type answers.

use super::iterator_steps::{body_aggregates, body_fn_defs};
use crate::collections::{IndexMap, IndexSet};
use crate::lower::recognition::{
    StdItem, TypeFact, bound_by_any, cached_key_pairs, debug_named, fact_specialization, map_of_param, phantom_named,
    pointer_target, points_at_dyn_any, points_at_dyn_debug, stable_sort_items, std_item, trait_method,
    tree_built_entries, type_facts, unstable_sort_items,
};
use crate::lower::traits::{BuiltSlot, BuiltSlots, Given, bounds, drop_items, evidence_impl, slot_index as slot_of};
use crate::lower::{Body, FnInfo, fn_def};
use rustc_middle::thir::{ExprId, ExprKind};
use rustc_middle::ty;
use rustc_middle::ty::TyCtxt;
use rustc_middle::ty::TypeVisitableExt;
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_span::def_id::DefId;

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
    fns: &IndexMap<DefId, FnInfo>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    given: Given<'_>,
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    drops: ItemDrops<'_, 'tcx>,
) -> FactParams<'tcx> {
    let iterator = std_item(tcx, StdItem::Iterator);
    // What `std_iterator_steps` steps a compiled std iterator's struct by.
    let steps = crate::lower::recognition::STRUCT_STEPS.map(|name| trait_method(tcx, iterator, name));
    let collect = trait_method(tcx, iterator, "collect");
    let compares = [trait_method(tcx, iterator, "eq"), trait_method(tcx, iterator, "ne")];
    let mut asking = Asking::default();
    for body in all_bodies {
        let caller = tcx.typeck_root_def_id(body.def_id);
        let typing_env = ty::TypingEnv::non_body_analysis(tcx, caller);
        // A body read from its MIR alone, a std one's (ADR 0561): what it
        // asks is read from each function it names, and from each struct it
        // makes of a compiled std iterator, `Fuse { iter: self }` of std's
        // `fuse`, whose steps (`STRUCT_STEPS`) ask what they ask,
        // `Flatten`'s its items' constant size (ADR 0899), a `Zip`'s `fold`
        // whether its sides are `TrustedLen` (ADR 0975).
        if body.thir.exprs.is_empty() {
            let mut named = body_fn_defs(body);
            for held in body_aggregates(tcx, body) {
                for &step in &steps {
                    // `Self` the struct, a step's own parameters, `fold`'s
                    // `B` and `F`, themselves: they pick no impl.
                    let args = ty::GenericArgs::for_item(tcx, step, |param, _| match param.index {
                        0 => held.into(),
                        _ => tcx.mk_param_from_def(param),
                    });
                    if let Ok(Some(instance)) = crate::lower::items::resolve_instance_in(tcx, typing_env, step, args)
                        && instance.def_id() != step
                    {
                        named.push((instance.def_id(), instance.args));
                    }
                }
            }
            for (callee, args) in named {
                for index in eq_asks(recognition, typing_env, caller, (callee, args), &compares) {
                    asking.asked.insert((caller, index, TypeFact::LengthFirst));
                }
                // A map or a set of a type parameter's keys a std function
                // makes, as a body read from its THIR asks it, so a library's
                // default compiled here from its MIR takes the class its
                // library's copies do (`map_made`, #990).
                if let Some((index, fact)) = map_made(tcx, foreign, caller, (callee, args)) {
                    asking.asked.insert((caller, index, fact));
                }
                // A std trait's method whose impl the caller's type picks,
                // `FuseImpl::next`'s of a `Fuse<I>`: whether `I` is a
                // `FusedIterator`, its caller's, `IFused` (ADRs 0543, 0561),
                // or `FlattenCompat`'s whether `U` is a `OneShot` (ADR 0899).
                if let Some(spec) = fact_specialization(tcx, typing_env, callee, args) {
                    for (of, fact) in spec.facts() {
                        if let Some(index) = slot_of(tcx, foreign, caller, of) {
                            asking.asked.insert((caller, index, fact));
                        }
                    }
                }
                for (id, each) in collect_asks(
                    recognition,
                    typing_env,
                    drops,
                    caller,
                    (callee, args),
                    &mut asking.carried_super,
                ) {
                    asking.ask(id, each);
                }
                asking.of_call(tcx, foreign, given, fns, typing_env, caller, callee, args);
            }
            continue;
        }
        // A maker of std's adapter made in place, `items.flatten()` stepped
        // where it's made: the recognizers', which ask nothing (ADR 0871).
        let in_place: IndexSet<ExprId> = (body.thir.exprs.iter())
            .filter_map(|expr| match expr.kind {
                ExprKind::Call { fun, fn_span, .. } if body.facts.in_place_adapters.contains(&fn_span) => Some(fun),
                _ => None,
            })
            .collect();
        for (id, expr) in body.thir.exprs.iter_enumerated() {
            if in_place.contains(&id) {
                continue;
            }
            // A map or a set of a type parameter's keys a std function makes,
            // called or given (`map_made`).
            if let ExprKind::ZstLiteral { .. } = expr.kind
                && let Some((callee, args)) = fn_def(expr.ty)
                && let Some((index, fact)) = map_made(tcx, foreign, caller, (callee, args))
            {
                asking.asked.insert((caller, index, fact));
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
                asking.asked.insert((caller, param.index, TypeFact::Id));
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
                    asking.asked.insert((caller, index, TypeFact::Name));
                }
                continue;
            }
            // A pointer to a type parameter's value cast to an integer, or
            // an integer cast to one, or two compared: the pointee's size,
            // an item's stride (ADR 0562).
            let pointee_param = |ty: ty::Ty<'tcx>| {
                let pointee = match ty.kind() {
                    ty::Ref(_, pointee, _) => *pointee,
                    _ => crate::lower::pointers::raw_pointee(tcx, ty)?.0,
                };
                match pointee.kind() {
                    ty::Param(param) => Some(param.index),
                    _ => None,
                }
            };
            match expr.kind {
                ExprKind::Cast { source } => {
                    let from = body.thir[source].ty;
                    if let Some(index) = pointee_param(expr.ty).or_else(|| pointee_param(from))
                        && (expr.ty.is_integral() || from.is_integral())
                    {
                        asking.asked.insert((caller, index, TypeFact::Size));
                    }
                }
                ExprKind::Binary { lhs, .. } if expr.ty.is_bool() => {
                    if let Some(index) = pointee_param(body.thir[lhs].ty) {
                        asking.asked.insert((caller, index, TypeFact::Size));
                    }
                }
                _ => {}
            }
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            // A pointer or a reference to a type parameter's value
            // `transmute`d, to its address: the pointee's size (ADR 0562).
            if crate::lower::recognition::is_transmute(tcx, callee)
                && let Some(index) = args.types().next().and_then(pointee_param)
            {
                asking.asked.insert((caller, index, TypeFact::Size));
                continue;
            }
            // A `collect()` into a `Vec` of what folds by a type parameter's
            // `nth_back`, a `take(n).rev()`'s of one: std's `Vec` folds a
            // `TrustedLen` one, `extend_trusted`, and steps another by
            // `next`, which asks another `nth_back`. Whether it is, its
            // caller's, `ITrustedLen` (#670).
            if callee == collect
                && let [this, output] = args.types().collect::<Vec<_>>()[..]
                && recognition.is_vec_like(output)
                && !recognition.nth_back_folded_sources(this).is_empty()
            {
                for part in this.walk().filter_map(|part| part.as_type()) {
                    if let ty::Param(_) = part.kind()
                        && let Some(index) = slot_of(tcx, foreign, caller, part)
                    {
                        asking.asked.insert((caller, index, TypeFact::TrustedLen));
                    }
                }
            }
            for index in eq_asks(recognition, typing_env, caller, (callee, args), &compares) {
                asking.asked.insert((caller, index, TypeFact::LengthFirst));
            }
            // An unstable sort of a type parameter's items, whose order std
            // picks by their layout: its caller's, `TSort` (ADR 0471).
            if let Some(item) = unstable_sort_items(tcx, callee, args)
                && let Some(index) = slot_of(tcx, foreign, caller, item)
            {
                asking.asked.insert((caller, index, TypeFact::Sort));
                continue;
            }
            // A stable sort of a type parameter's items, which is std's own
            // where its caller's type's order may be seen: what it picks by
            // their type and their size, `TStableSort` and `TSize` (#925).
            if let Some(item) = stable_sort_items(tcx, callee, args)
                && let Some(index) = slot_of(tcx, foreign, caller, item)
            {
                asking.asked.insert((caller, index, TypeFact::StableSort));
                asking.asked.insert((caller, index, TypeFact::Size));
                continue;
            }
            // A B-tree built of items, of a key that holds a type parameter,
            // whose class may be std's nodes, which sort its entries by std's
            // stable sort, as their type picks: their caller's, a slot's or a
            // built slot's (#926).
            if let Some(entries) = tree_built_entries(tcx, callee, args) {
                for fact in [TypeFact::StableSort, TypeFact::Size] {
                    if let Some(each) = asked_of(tcx, foreign, typing_env, caller, (entries, fact), &None) {
                        asking.ask(caller, each);
                    }
                }
            }
            // `sort_by_cached_key` of a key that holds a type parameter, whose
            // `(K, usize)` pairs std sorts unstably, as their type picks:
            // their caller's, a built slot's (#925).
            if let Some(pairs) = cached_key_pairs(tcx, callee, args)
                && let Some(each) = asked_of(tcx, foreign, typing_env, caller, (pairs, TypeFact::Sort), &None)
            {
                asking.ask(caller, each);
            }
            // `==` of `Rc`s of a type parameter's value, whose `Eq` only the
            // caller knows, by which std's `RcEqIdent` asks whether they're
            // one allocation first: its caller's, `TEq` (#931).
            for (of, fact) in crate::lower::recognition::rc_eq_asked(tcx, typing_env, callee, args) {
                if let Some(index) = slot_of(tcx, foreign, caller, of) {
                    asking.asked.insert((caller, index, fact));
                }
            }
            for (id, each) in collect_asks(
                recognition,
                typing_env,
                drops,
                caller,
                (callee, args),
                &mut asking.carried_super,
            ) {
                asking.ask(id, each);
            }
            asking.of_call(tcx, foreign, given, fns, typing_env, caller, callee, args);
        }
    }
    let Asking {
        mut asked,
        mut built,
        mut carried,
        passed_carried,
        passed,
        calls,
        carried_super,
    } = asking;
    // Of a library's trait, only what its dictionaries carry; and each the
    // crate's impls of one carry, as the library's code reads it of them
    // (#476).
    carried.retain(|&(assoc, fact)| carries(foreign, assoc, fact));
    for &trait_id in tcx.all_local_trait_impls(()).keys() {
        if trait_id.is_local() {
            continue;
        }
        for item in tcx.associated_items(trait_id).in_definition_order() {
            for fact in foreign.carried(item.def_id) {
                carried.insert((item.def_id, fact));
            }
        }
    }
    // And what the dictionaries of each the crate's impls of a library's
    // trait carry of a std supertrait's associated type of their `Self`, as
    // its manifest says, `IteratorItemSize`: what each asks of its own,
    // `<I as Iterator>::Item` of an `impl<I: Iterator> Tools for Wrap<I>`,
    // its slot, or of an associated type of a trait whose dictionaries carry
    // it (#990).
    for (&trait_id, impls) in tcx.all_local_trait_impls(()) {
        let facts = match trait_id.is_local() {
            true => Vec::new(),
            false => foreign.carried_super(trait_id),
        };
        for &imp in impls {
            let imp = imp.to_def_id();
            let typing_env = ty::TypingEnv::non_body_analysis(tcx, imp);
            let self_ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
            for &(assoc, fact) in &facts {
                let ty = super_item_of(tcx, typing_env, assoc, self_ty);
                if let Some(index) = slot_of(tcx, foreign, imp, ty) {
                    asked.insert((imp, index, fact));
                } else if let Some(assoc) = carried_by(foreign, ty)
                    && carries(foreign, assoc, fact)
                {
                    carried.insert((assoc, fact));
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
            let no_built = Default::default();
            let mut facts: Vec<(ty::Ty<'tcx>, TypeFact, Gates)> = (asked.iter())
                .filter(|&&(id, _, _)| id == callee)
                .map(|&(_, index, fact)| {
                    let slot = crate::lower::traits::drop_slot(tcx, (foreign, &no_built), callee, index);
                    (slot, fact, None)
                })
                .collect();
            // And the layouts of the types built of its slots it asks (#737).
            for ((_, of), (asked_facts, gates)) in built.iter().filter(|((id, _), _)| *id == callee) {
                facts.extend(asked_facts.iter().map(|&fact| (*of, fact, gates.clone())));
            }
            for (slot, fact, gates) in facts {
                let relayed = (caller, callee, args);
                for each in asked_of_caller(tcx, foreign, typing_env, relayed, (slot, fact, &gates)) {
                    changed |= ask((&mut asked, &mut built), caller, each);
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
                if asked.contains(&(callee, to, fact)) && carries(foreign, assoc, fact) {
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
            if let Some(assoc) = carried_by(foreign, ty)
                && carries(foreign, assoc, fact)
            {
                changed |= carried.insert((assoc, fact));
            }
        }
    }
    let mut params: IndexMap<DefId, Vec<(u32, TypeFact)>> = IndexMap::default();
    for (id, index, fact) in asked {
        // A trait's method's of its own type parameters, and an impl's
        // method's, its caller's through the dictionary (ADR 0466); and of
        // the impl's own parameters, the method's, which its dictionary,
        // asked them above, gives it of its own.
        let own = index >= tcx.generics_of(id).parent_count as u32;
        // A std trait's provided method compiled as a function over `Self`
        // (ADRs 0185, 0561), `Iterator::fuse`'s: of `Self` too, given as a
        // value is.
        let compiled = crate::lower::recognition::is_compiled_std(tcx, id);
        let ask = match (tcx.trait_of_assoc(id), tcx.trait_impl_of_assoc(id)) {
            (Some(_), _) => own || compiled,
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
    // Each one's built slots, after its associated types', in the order of
    // their types' names, which its callers read by index (`drop_slot`).
    let mut built_slots: BuiltSlots<'tcx> = IndexMap::default();
    let mut each_built: Vec<_> = (built.into_iter())
        .map(|((id, ty), asked)| ((tcx.def_path_hash(id), format!("{ty:?}")), id, ty, asked))
        .collect();
    each_built.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, id, ty, (facts, gates)) in each_built {
        let slots = built_slots.entry(id).or_default();
        let index = crate::lower::traits::item_slot(tcx, id, drop_items(tcx, foreign, id).len() + slots.len());
        slots.push(BuiltSlot {
            ty,
            gates: gates.into_iter().flatten().collect(),
        });
        params
            .entry(id)
            .or_default()
            .extend(facts.into_iter().map(|fact| (index, fact)));
    }
    for facts in params.values_mut() {
        facts.sort();
        facts.dedup();
    }
    // Of the crate's own traits; a library's are its manifest's.
    let mut by_trait = CarriedSuper::default();
    for (trait_id, assoc, fact) in carried_super.into_iter().filter(|(id, ..)| id.is_local()) {
        by_trait.entry(trait_id).or_default().push((assoc, fact));
    }
    for facts in by_trait.values_mut() {
        facts.sort_by_key(|&(assoc, fact)| (tcx.def_path_hash(assoc), fact));
        facts.dedup();
    }
    FactParams {
        params,
        built_slots,
        carried_super: by_trait,
    }
}

/// What `type_fact_params` finds: each function's and associated type's
/// facts, their built slots (#737), and what each of the crate's traits'
/// dictionaries carry of std supertraits' associated types (#990).
pub(super) struct FactParams<'tcx> {
    pub(super) params: IndexMap<DefId, Vec<(u32, TypeFact)>>,
    pub(super) built_slots: BuiltSlots<'tcx>,
    pub(super) carried_super: CarriedSuper,
}

/// The trait and the std associated type of `of` where `caller` is one of
/// the trait's defaults and `of` is `<Self as Trait>::Item` of a std
/// supertrait of it, `Iterator::Item` of a `Tools: Iterator`'s (#990).
fn default_super_item(tcx: TyCtxt<'_>, caller: DefId, of: ty::Ty<'_>) -> Option<(DefId, DefId)> {
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

/// What `caller` asks of its slots where it names `callee`, a std
/// function, with `args`, as a body read from its THIR or its MIR alike: a
/// map or a set of a type parameter's keys it makes, called or given,
/// `HashMap::new()`'s, `collect()`'s, `or_insert_with(BTreeMap::new)`'s,
/// what it gives, or what a `&mut` it gives points at, which it may have
/// made, an entry's `or_default()`'s. Its class, its caller's (ADR 0466).
fn map_made<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    caller: DefId,
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
) -> Option<(u32, TypeFact)> {
    if foreign.compiled_by_rust_js(callee) {
        return None;
    }
    let output = (tcx.fn_sig(callee).instantiate(tcx, args))
        .skip_normalization()
        .skip_binder()
        .output();
    let made = match *output.kind() {
        ty::Ref(_, pointee, ty::Mutability::Mut) => pointee,
        _ => output,
    };
    let (key, fact) = map_of_param(tcx, made)?;
    Some((slot_of(tcx, foreign, caller, key)?, fact))
}

/// `default_super_item` of `id`'s slot `index`.
fn slot_super_item<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    id: DefId,
    index: u32,
) -> Option<(DefId, DefId)> {
    crate::lower::traits::slot_item(tcx, id, index)?;
    let slot = crate::lower::traits::drop_slot(tcx, (foreign, &Default::default()), id, index);
    default_super_item(tcx, id, slot)
}

/// Whether dropping a value of a type runs anything where a function is,
/// given the drops it's given, by its slots (ADR 0421): what a collect's
/// items drop, `drops::drops_given`, the lowering's drops of them
/// there (#737).
#[derive(Clone, Copy)]
pub(super) struct ItemDrops<'a, 'tcx> {
    pub(super) drops: &'a ItemsDrop<'a, 'tcx>,
    pub(super) taken: &'a IndexMap<DefId, Vec<u32>>,
}

/// Whether a type held in a function, in a typing environment, given the
/// drops of those of its slots it's given them of, drops anything.
pub(super) type ItemsDrop<'a, 'tcx> = dyn Fn(ty::TypingEnv<'tcx>, ty::Ty<'tcx>, DefId, &[u32]) -> bool + 'a;

impl<'tcx> ItemDrops<'_, 'tcx> {
    /// Whether a `ty` `caller` holds, in `typing_env`, has a drop it runs:
    /// its own, or one of a type parameter's, or an associated type's, it's
    /// given.
    fn drop(&self, typing_env: ty::TypingEnv<'tcx>, caller: DefId, ty: ty::Ty<'tcx>) -> bool {
        let taken = self.taken.get(&caller).map_or(&[][..], Vec::as_slice);
        (self.drops)(typing_env, ty, caller, taken)
    }
}

/// What `caller`'s call asks as a collect, `in_place_asked` and
/// `grown_asked`: of a trait's default, of each of the crate's impls that
/// copies it, in its terms, of its own type parameters, `I` of `impl<I:
/// Iterator> Sorted for I`, which its dictionary is given, as the copied
/// default reads them (ADR 0049).
fn collect_asks<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    drops: ItemDrops<'_, 'tcx>,
    caller: DefId,
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
    carried_super: &mut IndexSet<(DefId, DefId, TypeFact)>,
) -> Vec<(DefId, Asked<'tcx>)> {
    let (tcx, foreign) = (recognition.tcx, recognition.foreign);
    let asked_by = |typing_env: ty::TypingEnv<'tcx>, owner: DefId, args: ty::GenericArgsRef<'tcx>| {
        let mut asked = in_place_asked(tcx, foreign, typing_env, drops, owner, (callee, args));
        asked.extend(grown_asked(recognition, typing_env, owner, (callee, args)));
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

/// Whether `ty` names a type parameter, and each it names is one `which`
/// says of, by index.
fn names_params(ty: ty::Ty<'_>, which: impl Fn(u32) -> bool) -> bool {
    let params: Vec<u32> = (ty.walk())
        .filter_map(|part| match part.as_type()?.kind() {
            ty::Param(param) => Some(param.index),
            _ => None,
        })
        .collect();
    !params.is_empty() && params.into_iter().all(which)
}

/// A fact a function asks: of one of its slots, a type parameter's or an
/// associated type's, by its index; or the layout of a type built of them,
/// `Held<J>` of a `Vec<Held<J>>` collected in place, by the type, its
/// index decided once every one of its function's is found (#737), and
/// whether one of its gates' `IInPlace` must be given for it to be read.
#[derive(Clone, Debug)]
enum Asked<'tcx> {
    Slot(u32, TypeFact),
    Built(ty::Ty<'tcx>, TypeFact, Gates),
}

/// What a built slot's layout is read where: `None` wherever its function
/// is called; else only where one of these slots' `IInPlace`, whether its
/// collect may be in place, isn't `undefined` (`traits::BuiltSlot`).
type Gates = Option<std::collections::BTreeSet<u32>>;

/// `a` and `b` together: read where either is.
fn either_gates(a: &Gates, b: &Gates) -> Gates {
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
fn built_of<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: ty::Ty<'tcx>, fact: TypeFact) -> bool {
    matches!(
        fact,
        TypeFact::Size | TypeFact::Align | TypeFact::Sort | TypeFact::StableSort
    ) && !crate::lower::recognition::known_by_bounds(ty)
        && ty.has_param()
        && tcx.layout_of(typing_env.as_query_input(ty)).is_err()
}

/// Where `caller` asks `(of, fact)`, `of` in its own terms: a slot's, or
/// a built type's (`built_of`), read where `gates` says; none of a type it
/// knows.
fn asked_of<'tcx>(
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
    built_of(tcx, typing_env, of, fact).then(|| Asked::Built(tcx.erase_and_anonymize_regions(of), fact, gates.clone()))
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
    let Some((iter, _, dest)) = collected_items(tcx, typing_env, callee, args) else {
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
/// hint may run the crate's own (`hint_reaches_own`): their size, a slot's
/// `TSize` or a built type's, `(T, u8)`'s, by which std's `Vec` is full
/// and asks the hint again (#796). As the lowering's collect reads it,
/// `grown_item_size`, wherever it's called.
fn grown_asked<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    caller: DefId,
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
) -> Vec<Asked<'tcx>> {
    let tcx = recognition.tcx;
    let Some((iter, output, dest)) = crate::lower::recognition::collected_items(tcx, typing_env, callee, args) else {
        return Vec::new();
    };
    // Its types as the lowering asks them, its opaque ones revealed.
    let revealed = crate::lower::recognition::Recognition {
        typing_env: typing_env.with_post_analysis_normalized(tcx),
        ..*recognition
    };
    if !revealed.grows_by_callers_size(output, iter, dest) {
        return Vec::new();
    }
    let foreign = recognition.foreign;
    let size = asked_of(tcx, foreign, typing_env, caller, (dest, TypeFact::Size), &None);
    size.into_iter().collect()
}

/// Of generic code's `a.eq(b)`, a call of `callee`, one of `compares`,
/// with `args`, of what std may compare the lengths of first: the slot of
/// each side given as a type parameter's, whose caller says whether it's
/// `TrustedLen` and how JS counts it, `ILengthFirst` (`eq_sides`, #1182).
/// Of a body read from its THIR or its MIR alone, a library's a consumer
/// compiles.
fn eq_asks<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    caller: DefId,
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
    compares: &[DefId; 2],
) -> Vec<u32> {
    use crate::lower::recognition::EqSide;
    let [this, other, ..] = args.types().collect::<Vec<_>>()[..] else {
        return Vec::new();
    };
    if !compares.contains(&callee) {
        return Vec::new();
    }
    let recognition = crate::lower::recognition::Recognition {
        typing_env,
        ..*recognition
    };
    let Some(sides) = recognition
        .eq_sides(this, other)
        .filter(|sides| !sides.contains(&EqSide::Built))
    else {
        return Vec::new();
    };
    (sides.into_iter())
        .filter_map(|side| match side {
            EqSide::Given(param) => slot_of(recognition.tcx, recognition.foreign, caller, param),
            _ => None,
        })
        .collect()
}

/// What the bodies ask, as they're read.
#[derive(Default)]
struct Asking<'tcx> {
    asked: IndexSet<(DefId, u32, TypeFact)>,
    /// The layouts each asks of a type built of its slots (`Asked::Built`).
    built: BuiltAsks<'tcx>,
    /// What a trait's dictionaries carry of its associated type, asked of
    /// `<A as Array>::Item` (ADR 0145), and a caller's projection passed on
    /// as a callee's type parameter.
    carried: IndexSet<(DefId, TypeFact)>,
    passed_carried: Vec<(DefId, (DefId, u32))>,
    /// What a trait's dictionaries carry of a std supertrait's associated
    /// type of its `Self`, its defaults' (#990).
    carried_super: IndexSet<(DefId, DefId, TypeFact)>,
    /// A caller's type parameter passed on as a callee's: `relay<U>` calling `describe::<U>`.
    passed: Vec<((DefId, u32), (DefId, u32))>,
    /// A call of the crate's function, with its arguments: what its callee asks
    /// of a slot, instantiated, is of the caller's own where it's one of its
    /// slots, `Self::Item` of `self.map(f).counts()`'s `K` (ADR 0466).
    calls: Vec<(DefId, DefId, ty::GenericArgsRef<'tcx>)>,
}

/// Each function's built types it asks a layout of, by the type: the facts,
/// and where they're read.
type BuiltAsks<'tcx> = IndexMap<(DefId, ty::Ty<'tcx>), (std::collections::BTreeSet<TypeFact>, Gates)>;

/// `caller` asks `asked`: whether that's new.
fn ask<'tcx>(
    (asked, built): (&mut IndexSet<(DefId, u32, TypeFact)>, &mut BuiltAsks<'tcx>),
    caller: DefId,
    each: Asked<'tcx>,
) -> bool {
    match each {
        Asked::Slot(index, fact) => asked.insert((caller, index, fact)),
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

impl<'tcx> Asking<'tcx> {
    /// `default`, a default of `trait_id`, asks `fact` of `<Self as
    /// Trait>::assoc`, of a std supertrait of its `Self`: its dictionaries
    /// carry it, so a library's default compiled over `Self` reads it of
    /// the one it's given, and each of the crate's impls that copies it
    /// asks it of its own type for it, `<I as Iterator>::Item` of an
    /// `impl<I: Iterator> Tools for I`, as a collect's are (#990).
    fn ask_of_impls(
        &mut self,
        tcx: TyCtxt<'tcx>,
        foreign: &crate::lower::library::Foreign<'_, 'tcx>,
        (trait_id, default): (DefId, DefId),
        assoc: DefId,
        fact: TypeFact,
    ) {
        self.carried_super.insert((trait_id, assoc, fact));
        // A library's trait's impls here carry what its manifest says.
        if !trait_id.is_local() {
            return;
        }
        for &imp in tcx.all_local_trait_impls(()).get(&trait_id).into_iter().flatten() {
            let imp = imp.to_def_id();
            if tcx.impl_item_implementor_ids(imp).contains_key(&default) {
                continue;
            }
            let typing_env = ty::TypingEnv::non_body_analysis(tcx, imp);
            let self_ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
            let ty = super_item_of(tcx, typing_env, assoc, self_ty);
            if let Some(index) = slot_of(tcx, foreign, imp, ty) {
                self.asked.insert((imp, index, fact));
            }
        }
    }

    /// `caller` asks `asked`.
    fn ask(&mut self, caller: DefId, each: Asked<'tcx>) {
        ask((&mut self.asked, &mut self.built), caller, each);
    }

    /// What `caller` asks by naming `callee` with `args`: a std function's
    /// facts, what the impls it gives dictionaries of ask, a library's as its
    /// manifest says, and a crate function's, followed once all are read.
    #[allow(clippy::too_many_arguments)]
    fn of_call(
        &mut self,
        tcx: TyCtxt<'tcx>,
        foreign: &crate::lower::library::Foreign<'_, 'tcx>,
        given: Given<'_>,
        fns: &IndexMap<DefId, FnInfo>,
        typing_env: ty::TypingEnv<'tcx>,
        caller: DefId,
        callee: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) {
        let facts = type_facts(tcx, callee);
        for &fact in facts {
            // `size_of_val` of an unsized `T` is the value's, not the type's:
            // a size or an alignment is asked of a sized one, and its name
            // of any, a `T: ?Sized`'s too.
            let measured =
                |of: ty::Ty<'tcx>| of.is_sized(tcx, typing_env) || !matches!(fact, TypeFact::Size | TypeFact::Align);
            // A downcast's type is its last, after its `Box`'s allocator.
            // A `T: Any`'s `TypeId` is its dictionary's (ADR 0331).
            if let Some(of) = args.types().last()
                && let ty::Param(param) = of.kind()
                && measured(of)
                && !(fact == TypeFact::Id && bound_by_any(tcx, caller, param.index))
            {
                self.asked.insert((caller, param.index, fact));
            }
            // Of a type that holds them, `Cow<'_, T>`: its name, each one's.
            if fact == TypeFact::Name
                && let Some(of) = args.types().last()
                && !matches!(of.kind(), ty::Param(_))
            {
                for part in of.walk().filter_map(|part| part.as_type()) {
                    if let ty::Param(param) = part.kind() {
                        self.asked.insert((caller, param.index, fact));
                    }
                }
            }
            if let Some(assoc) = args.types().last().and_then(|of| carried_by(foreign, of)) {
                self.carried.insert((assoc, fact));
            }
            // Of a std associated type of a type parameter, `<I as
            // Iterator>::Item`, one of its slots: its caller's, as a map's
            // keys' class is (ADR 0466); of a trait's default's `Self`'s,
            // what its dictionaries carry (#990).
            if let Some(of) = args.types().last()
                && matches!(of.kind(), ty::Alias(..))
                && carried_by(foreign, of).is_none()
                && measured(of)
                && let Some(index) = slot_of(tcx, foreign, caller, of)
            {
                match default_super_item(tcx, caller, of) {
                    Some((trait_id, assoc)) => self.ask_of_impls(tcx, foreign, (trait_id, caller), assoc, fact),
                    None => _ = self.asked.insert((caller, index, fact)),
                }
            }
            // A flatten's items' constant size, of `I::Item`, a std
            // associated type of its caller's parameter (ADR 0899).
            if fact == TypeFact::ConstSize
                && let Some(of) = args.types().next()
                && !matches!(of.kind(), ty::Param(_))
                && let Some(index) = slot_of(tcx, foreign, caller, of)
            {
                self.asked.insert((caller, index, fact));
            }
        }
        if !facts.is_empty() {
            return;
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
                self.asked.insert((caller, index, TypeFact::Name));
            }
            let Some((impl_id, impl_args)) = evidence_impl(tcx, typing_env, bound) else {
                // `collect()`'s `B: FromIterator<A>` the caller's own bound
                // gives, its dictionary's `from_iter` of the iterator, which
                // the lowering calls given what its declaration carries
                // (`collected_evidence`, `std_method_facts`, #794).
                let from_iterator = std_item(tcx, StdItem::FromIterator);
                if bound.def_id == from_iterator && Some(callee) == collect_of(tcx) {
                    let from_iter = trait_method(tcx, from_iterator, "from_iter");
                    let from_args = tcx.mk_args(&[bound.args[0], bound.args[1], args[0]]);
                    for (index, fact) in crate::lower::traits::foreign_facts(tcx, foreign, from_iter) {
                        let slot =
                            crate::lower::traits::drop_slot(tcx, (foreign, &Default::default()), from_iter, index);
                        let gates = foreign_gates(foreign, from_iter, index);
                        let asked = asked_of_caller(
                            tcx,
                            foreign,
                            typing_env,
                            (caller, from_iter, from_args),
                            (slot, fact, &gates),
                        );
                        asked.into_iter().for_each(|each| self.ask(caller, each));
                    }
                }
                continue;
            };
            for (index, arg) in impl_args.iter().enumerate() {
                if let Some(ty) = arg.as_type()
                    && let ty::Param(param) = ty.kind()
                {
                    match impl_id.is_local() {
                        true => self.passed.push(((caller, param.index), (impl_id, index as u32))),
                        false => {
                            for (at, fact) in foreign.type_facts(impl_id) {
                                if at as usize == index {
                                    self.asked.insert((caller, param.index, fact));
                                }
                            }
                        }
                    }
                }
            }
            // Of one given a type of the caller's slots that isn't one of its
            // type parameters, `impl<T: Iterator> Itertools for T` of an
            // `<I as IntoIterator>::IntoIter`: what the impl asks of that,
            // as of a call's (#794).
            let of_slots = |at: usize| {
                impl_args
                    .get(at)
                    .and_then(|arg| arg.as_type())
                    .is_some_and(|ty| !matches!(ty.kind(), ty::Param(_)) && ty.has_param())
            };
            if !(0..impl_args.len()).any(of_slots) {
                continue;
            }
            match impl_id.is_local() {
                true => self.calls.push((caller, impl_id, impl_args)),
                false => {
                    for (at, fact) in foreign.type_facts(impl_id) {
                        if !of_slots(at as usize) {
                            continue;
                        }
                        let slot = crate::lower::traits::drop_slot(tcx, (foreign, &Default::default()), impl_id, at);
                        let relayed = (caller, impl_id, impl_args);
                        for each in asked_of_caller(tcx, foreign, typing_env, relayed, (slot, fact, &None)) {
                            self.ask(caller, each);
                        }
                    }
                }
            }
        }
        // A library's function, which asks what its manifest says: of a
        // caller's type parameter given it, or a std associated type of one,
        // `I::IntoIter`'s, the caller asks it too; a trait's method, of the
        // impl's method it resolves to, `Fuse<I>`'s `next_back` (ADR 0561).
        // Not a std body compiled with this crate, which is followed as the
        // crate's own are.
        if !callee.is_local() && !fns.contains_key(&callee) {
            // Of one whose impl a fact picks, each impl's, compiled here,
            // `FlattenCompat`'s of a `OneShot` or not (ADR 0899).
            if let Some(spec) = fact_specialization(tcx, typing_env, callee, args)
                && fns.contains_key(&spec.selected.0)
            {
                // The method's own arguments after its impl's, as the call
                // gives them after its trait's.
                let impl_args = spec.selected.1;
                let own = &args[tcx.generics_of(callee).parent_count..];
                let args = tcx.mk_args_from_iter(impl_args.iter().chain(own.iter().copied()));
                for method in spec.methods() {
                    self.calls.push((caller, method, args));
                }
                return;
            }
            let mut named = vec![(callee, args)];
            // Not of a struct stepped as the JS iterator it is, `Fuse<I>`'s
            // `next`, whose impl's facts its caller never passes.
            if !crate::lower::recognition::stepped_as_js_iterator(tcx, callee, args)
                && let Ok(Some(instance)) = crate::lower::items::resolve_instance_in(tcx, typing_env, callee, args)
                && instance.def_id() != callee
            {
                // One compiled here, a std body's, `FlattenCompat`'s
                // `size_hint` a `Flatten`'s calls: followed as the crate's
                // own are, not as its manifest says (ADR 0899).
                if fns.contains_key(&instance.def_id()) {
                    self.calls.push((caller, instance.def_id(), instance.args));
                    return;
                }
                named.push((instance.def_id(), instance.args));
            }
            // A std trait's required method resolved to an impl, `Vec`'s
            // `from_iter`, asks what that impl's lowering asks
            // (`collect_asks`); only one called through a dictionary
            // carries what its declaration says (#794).
            let resolved = named.len() > 1;
            for (id, args) in named {
                let facts = match resolved && id == callee {
                    true => foreign.type_facts(id),
                    false => crate::lower::traits::foreign_facts(tcx, foreign, id),
                };
                for (index, fact) in facts {
                    let slot = crate::lower::traits::drop_slot(tcx, (foreign, &Default::default()), id, index);
                    let gates = foreign_gates(foreign, id, index);
                    for each in asked_of_caller(tcx, foreign, typing_env, (caller, id, args), (slot, fact, &gates)) {
                        self.ask(caller, each);
                    }
                }
            }
            return;
        }
        // A trait's method of the crate's, called through its dictionary
        // or resolved, of its own type parameters (ADR 0466); or one of the
        // crate's functions, a compiled std impl's method among them.
        let trait_method = callee.is_local() && tcx.trait_of_assoc(callee).is_some();
        if !trait_method && !fns.contains_key(&callee) {
            return;
        }
        self.calls.push((caller, callee, args));
        for (index, arg) in args.iter().enumerate() {
            if let Some(assoc) = arg.as_type().and_then(|ty| carried_by(foreign, ty)) {
                self.passed_carried.push((assoc, (callee, index as u32)));
            }
        }
    }
}

/// The associated type of the crate's own trait or a library's `ty`
/// projects, whose facts the trait's dictionaries carry: `<A as
/// Array>::Item`. Not std's, whose facts a caller gives (ADR 0466).
fn carried_by(foreign: &crate::lower::library::Foreign<'_, '_>, ty: ty::Ty<'_>) -> Option<DefId> {
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
fn carries(foreign: &crate::lower::library::Foreign<'_, '_>, assoc: DefId, fact: TypeFact) -> bool {
    assoc.is_local() || foreign.carried(assoc).contains(&fact)
}

/// Where a library's function's slot `index` is read, as its manifest says:
/// a built slot's gates (#737).
fn foreign_gates(foreign: &crate::lower::library::Foreign<'_, '_>, id: DefId, index: u32) -> Gates {
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
fn asked_of_caller<'tcx>(
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

/// std's `Iterator::collect`.
fn collect_of(tcx: TyCtxt<'_>) -> Option<DefId> {
    Some(trait_method(tcx, std_item(tcx, StdItem::Iterator), "collect"))
}
