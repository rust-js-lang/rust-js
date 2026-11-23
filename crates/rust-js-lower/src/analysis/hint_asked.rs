//! What generic code asks the hint of, `size_hint()`, of what its callers
//! give (#1236, ADR 0170, amended): each instance a caller the crate sees
//! gives, and what it gives a library's generic function, whose body it
//! can't see, made in rust-js's form, which knows std's hint, where it's
//! made, and #1180's capacity questions can ask. The one owner of what's
//! made knowing its hint, which never says what's made knowing its back:
//! that's `back_stepped.rs`' alone, as a form that knows its back needs
//! one at every level beneath, where a hint asks none (#2209).

use crate::recognition::{
    Recognition, cloned_source, copied_source, counted_source, is_compiled_std_iterator, is_iter_adapter_def,
    stepped_by_source,
};
use crate::{Body, fn_def};
use rust_js_program::collections::IndexSet;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// What `of_crate` finds as it reads the crate: the adapters, and the
/// sources, made knowing their hint wherever they're made, `knowing`, which
/// becomes `Hinted::typed`; and what's made knowing it by the function that
/// makes it (`hinted`).
struct HintAsked<'tcx> {
    knowing: IndexSet<Ty<'tcx>>,
    hinted: Hinted<'tcx>,
}

/// The adapters made knowing their hint, which the crate's makers read, by
/// the function that makes them: of a type of no type parameter, in each
/// function whose value of it may leave it for the one that asks (#1333,
/// `may_leave`); of one generic code makes of its own parameters, only in
/// the function that makes it, as `Take<I>` is the same type of every
/// generic function's `I` (#1240): a `take(n)`'s, a `skip(n)`'s and a
/// `step_by(n)`'s made `$taking`, `$skipping` or `$stepBy` (`counted_in`),
/// and a map's and the like made knowing its hint, `$mapEnds` (`knowing_in`);
/// and those made so wherever they're made (`typed`). What a maker makes
/// of each is `made_knowing_its_hint`'s (iterators/back_facts.rs), its form
/// the in-place forms' owner's (`hint_knowing_form`), never one stepped
/// from its back for it (#2209).
// Flags, not a state: `dyn_asked` and `dyn_unseen` are two facts of the
// crate, either with the other (#1476).
#[derive(Default)]
pub struct Hinted<'tcx> {
    /// The adapters, and the sources that aren't adapters, made knowing
    /// their hint wherever they're made: each in what generic code or a
    /// library may be given where its hint is asked, of an array's items,
    /// whose JS iterator, `values()`, and JS's own adapters know none
    /// (#1236).
    pub(crate) typed: IndexSet<Ty<'tcx>>,
    /// The types of no type parameter marked, as they're found, each
    /// keyed by its makers into `counted_in` once the crate is read: empty
    /// after `of_crate` (#1333).
    pub(crate) types: IndexSet<Ty<'tcx>>,
    /// And those made knowing their hint otherwise, `$mapEnds`, keyed
    /// into `knowing_in` the same way: empty after `of_crate` (#1427).
    pub(crate) knowing_types: IndexSet<Ty<'tcx>>,
    /// Each of those made in place over an array's items with its source,
    /// made `$iter` with it: keyed by the makers of its adapter, never by
    /// its own, which another function holds in its own adapter, JS's
    /// own, `values()` (#1987): empty after `of_crate`.
    pub(crate) sources: IndexSet<(Ty<'tcx>, Ty<'tcx>)>,
    pub(crate) counted_in: IndexSet<(DefId, Ty<'tcx>)>,
    pub(crate) knowing_in: IndexSet<(DefId, Ty<'tcx>)>,
    /// What generic code grows a `Vec` by, `extend`'s iterator, by its
    /// owner, where an instance its callers give has a JS hint whose lower
    /// bound isn't std's as it's given (`given_hint_is_stds`), so
    /// std's growth by it can't be kept: the `Vec`'s capacity unknown once
    /// it grows, which `capacity()` refuses (#1180's (3)). Where the type
    /// is known, the extend is refused at compile time instead.
    pub(crate) lost: IndexSet<(DefId, Ty<'tcx>)>,
    /// Of those whose callers can't all be seen, a library's `pub`
    /// function's, those asked of each value it's given as it grows: lost
    /// where its hint meets JS's own iterator, which knows none
    /// (`$hintAsked`, #1251).
    pub(crate) by_value: IndexSet<(DefId, Ty<'tcx>)>,
    /// Whether the crate asks the hint of a `dyn Iterator`, `size_hint()`
    /// of a `Box<dyn Iterator>` (#1476): each value it makes one of is then
    /// made knowing its hint where it's made (`mark_given`), as what's
    /// given a library's function is, so the JS iterator a `dyn Iterator`
    /// is gives std's hint, its value's.
    pub dyn_asked: bool,
    /// Whether a `dyn Iterator` reaches the crate where it can't see it
    /// made: a library's or a binding's function gives one back (#1476).
    pub dyn_unseen: bool,
}

/// What `all_bodies` ask the hint of, through generic code and libraries.
pub(super) fn of_crate<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    (all_bodies, names): (&[&Body<'tcx>], &crate::call_graph::Names<'tcx>),
    keeps_capacity: bool,
) -> Hinted<'tcx> {
    let mut asked = HintAsked {
        knowing: IndexSet::default(),
        hinted: Hinted::default(),
    };
    let asks = AsksHint::of(tcx);
    let mut of_params: Vec<(DefId, Ty<'tcx>)> = Vec::new();
    let mut grown: Vec<(DefId, Ty<'tcx>)> = Vec::new();
    let iterator = crate::recognition::std_item(tcx, crate::recognition::StdItem::Iterator);
    let size_hint = crate::recognition::trait_method(tcx, iterator, "size_hint");
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            // A `dyn Iterator`'s hint asked, and one given back by what the
            // crate can't see make it (#1476).
            if callee == size_hint && args.types().next().is_some_and(|of| recognition.is_dyn_iter(of)) {
                asked.hinted.dyn_asked = true;
            }
            // A held iterator's own hint asked, of a type the crate knows
            // whole: each adapter made in place inside it over an array's
            // items, a filter's `skip(1)`, made knowing its hint where it's
            // asked, `$skipping`, as JS's own `drop(1)` keeps none (#1427).
            if callee == size_hint
                && let Some(of) = args.types().next()
                && !recognition.is_dyn_iter(of)
            {
                let of = tcx.erase_and_anonymize_regions(of.peel_refs());
                mark_in_place(tcx, recognition, tcx.typeck_root_def_id(body.def_id), of, &mut asked);
            }
            if (recognition.foreign.is_library(callee.krate) || crate::bindings::is_binding(tcx, callee))
                && (tcx.fn_sig(callee).instantiate(tcx, args).skip_binder().output().walk())
                    .filter_map(|part| part.as_type())
                    .any(|part| recognition.is_dyn_iter(part))
            {
                asked.hinted.dyn_unseen = true;
            }
            // A `Vec` generic code grows by an iterator of its type
            // parameters, `extend`'s, whose hint std asks as it fills:
            // asked of each instance its callers give, below.
            if recognition.extend_grows(callee, args)
                && let Some(iterable) = args.types().nth(2)
                && rustc_middle::ty::TypeVisitableExt::has_param(&iterable)
            {
                let typing_env = ty::TypingEnv::post_analysis(tcx, body.def_id);
                let iter_ty = crate::recognition::iterator_type_of(tcx, typing_env, iterable);
                grown.push((
                    tcx.typeck_root_def_id(body.def_id),
                    tcx.erase_and_anonymize_regions(iter_ty),
                ));
            }
            // Of a crate that keeps capacities, a `Vec` grown by a chain or
            // a zip, `collect`'s or `extend`'s, whose exact-sized parts,
            // `b.iter().skip(1)`, JS's own adapters know no hint of: each
            // made knowing it where it's made, `$skipping`, so the runtime's
            // chain sums them as std's does (#1216).
            let grown_by = (keeps_capacity)
                .then(|| recognition.vec_grown_by(callee, args, ty::TypingEnv::post_analysis(tcx, body.def_id)))
                .flatten();
            if let Some(iter_ty) = grown_by.map(|ty| tcx.erase_and_anonymize_regions(ty.peel_refs()))
                && !rustc_middle::ty::TypeVisitableExt::has_param(&iter_ty)
                && !recognition.lower_bound_is_stds(iter_ty)
            {
                // Of a chain or a zip, its parts (#1216); and of one holding an
                // adapter made in place inside another, each adapter in it,
                // as std asks its hint as `size_hint()` does: a `take(5)` of a
                // `cycle()` of a `map`'s or a `zip`'s source (#1572).
                if recognition.lower_bound_kept_by_parts(iter_ty) {
                    mark_counted(tcx, tcx.typeck_root_def_id(body.def_id), iter_ty, &mut asked.hinted);
                } else if !recognition.grows_by_exact_len(iter_ty) {
                    mark_in_place(
                        tcx,
                        recognition,
                        tcx.typeck_root_def_id(body.def_id),
                        iter_ty,
                        &mut asked,
                    );
                }
            }
            // What asks the hint of a type of its type parameters, asked of
            // what its callers give, each instance below (`AsksHint`).
            if let Some(of) = asks.of_params(callee, args) {
                of_params.push((tcx.typeck_root_def_id(body.def_id), tcx.erase_and_anonymize_regions(of)));
            }
            // Not what's given a library's function, whose hint its body asks
            // of each value its consumers give it as it grows, `$hintAsked`
            // (`by_value`, #1251): marking it here changed no JS (#2277).
        }
    }
    // Of a crate that asks a `dyn Iterator`'s hint, each value it makes
    // one of, made knowing its hint where it's made (#1476).
    if asked.hinted.dyn_asked {
        for body in all_bodies {
            for expr in body.thir.exprs.iter() {
                if let ExprKind::PointerCoercion {
                    cast: rustc_middle::ty::adjustment::PointerCoercion::Unsize,
                    source,
                    ..
                } = expr.kind
                    && recognition.is_dyn_iter(expr.ty)
                {
                    let from = body.thir[source].ty.peel_refs();
                    let from = from.boxed_ty().unwrap_or(from);
                    _ = mark_given(
                        tcx,
                        recognition,
                        tcx.erase_and_anonymize_regions(from),
                        keeps_capacity,
                        &mut asked,
                    );
                }
            }
        }
    }
    let callers =
        (!of_params.is_empty() || !grown.is_empty()).then(|| crate::instances::callers_of(tcx, names, all_bodies));
    // Each `Vec` generic code grows by what one of its callers gives whose
    // JS hint's lower bound isn't std's (#1180's (3)). Where callers can't
    // be seen, a library's `pub` function's, asked of each value its
    // consumers give, as it grows (#1251): lost by its type, every one of
    // its `extend`s would be, and what's then extended again by the
    // crate's own hint would throw.
    if let Some(callers) = &callers {
        for (owner, iter_ty) in grown {
            match crate::instances::instances_of(tcx, (callers, crate::instances::Fact::Shared), iter_ty, owner) {
                Some(instances) => {
                    if (instances.into_iter()).any(|instance| !recognition.given_growth_kept(instance)) {
                        asked.hinted.lost.insert((owner, iter_ty));
                    }
                }
                None => _ = asked.hinted.by_value.insert((owner, iter_ty)),
            }
        }
    }
    // Each instance a caller the crate sees gives, through generic callers
    // (the callers' instances' carrier, `Fact::Marked`, ADR 1115); and what
    // a generic caller makes of its own parameters on the way, `i.take(2)`
    // of `taken<I>`'s, one JS function of every instance, made knowing
    // where one of them steps an array's items, or where callers the crate
    // can't see may give one (#1240).
    if let Some(callers) = callers.filter(|_| !of_params.is_empty()) {
        for (owner, of) in of_params {
            let (instances, made) = crate::instances::seen_instances_of(tcx, &callers, of, owner);
            // Of every caller where the crate sees them all; one it can't
            // see, a library's `pub` function's, asks each value it's
            // given as it grows, `$hintAsked` (`by_value`, #1251), so what
            // it makes of them is marked by none (#2277).
            let mut of_items = false;
            for instance in instances {
                of_items |= mark_given(tcx, recognition, instance, keeps_capacity, &mut asked);
            }
            if of_items {
                for (maker, made) in made {
                    mark_made_in(tcx, maker, made, &mut asked.hinted);
                }
            }
        }
    }
    // A type of no type parameter marked so is made knowing its hint by
    // each function that makes one whose value may leave it (`may_leave`),
    // which the one that asks may be given, as values of one type move
    // between functions (#721); one made and stepped where it's made is
    // JS's own adapter there (#1333).
    let counted = std::mem::take(&mut asked.hinted.types).into_iter().map(|ty| (ty, true));
    let knowing = std::mem::take(&mut asked.hinted.knowing_types)
        .into_iter()
        .map(|ty| (ty, false));
    let sources = std::mem::take(&mut asked.hinted.sources);
    for (ty, counts) in counted.chain(knowing).collect::<Vec<_>>() {
        for body in all_bodies {
            let makes = body
                .thir
                .exprs
                .iter()
                .any(|expr| tcx.erase_and_anonymize_regions(expr.ty) == ty);
            if makes && may_leave(tcx, body, ty) {
                let maker = tcx.typeck_root_def_id(body.def_id);
                match counts {
                    true => _ = asked.hinted.counted_in.insert((maker, ty)),
                    false => _ = asked.hinted.knowing_in.insert((maker, ty)),
                }
                for &(_, source) in sources.iter().filter(|&&(part, _)| part == ty) {
                    asked.hinted.knowing_in.insert((maker, source));
                }
            }
        }
    }
    asked.hinted.typed = asked.knowing;
    asked.hinted
}

/// Whether a value of type `ty` made in `body` may leave it for another
/// function: `ty` held in what it returns, what it's given or captures
/// (`holds`), in an argument of a call that runs what isn't std's own code,
/// a std trait's method the crate's impl answers, `Hinted::from_iter(..)`,
/// or a std consumer that gives it the crate's impl (`crate_impl_called_by`),
/// in what a closure it makes captures, or unsized into a `dyn` (#1333,
/// #2376). Of one whose signature can't be read, an opaque type
/// unrevealed, it may.
fn may_leave<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>, ty: Ty<'tcx>) -> bool {
    use rustc_middle::ty::adjustment::PointerCoercion;
    let typing_env = ty::TypingEnv::post_analysis(tcx, body.def_id);
    let holds = |held: Ty<'tcx>| holds(tcx, typing_env, held, ty, &mut Vec::new());
    let own = tcx.type_of(body.def_id).instantiate_identity().skip_normalization();
    let signature: Vec<Ty<'tcx>> = match own.kind() {
        ty::Closure(_, args) => {
            let closure = args.as_closure();
            let sig = tcx.instantiate_bound_regions_with_erased(closure.sig());
            (sig.inputs().iter().copied())
                .chain([sig.output()])
                .chain(closure.upvar_tys())
                .collect()
        }
        ty::FnDef(..) => {
            let sig = tcx.fn_sig(body.def_id).instantiate_identity().skip_normalization();
            let sig = tcx.instantiate_bound_regions_with_erased(sig);
            sig.inputs().iter().copied().chain([sig.output()]).collect()
        }
        _ => return true,
    };
    if signature.into_iter().any(holds) {
        return true;
    }
    let thir = &body.thir;
    thir.exprs.iter().any(|expr| match &expr.kind {
        // Of std's own code alone: not a std trait's method the crate's impl
        // answers, nor a std consumer that gives what it steps on to one,
        // `collect()`'s `from_iter` (#2376).
        ExprKind::Call { fun, args, .. } => {
            let std = fn_def(thir[*fun].ty).is_some_and(|(callee, given)| {
                let runs = (crate::traits::resolve(tcx, typing_env, callee, given).ok().flatten())
                    .map_or(callee, |instance| instance.def_id());
                crate::recognition::is_std_crate(tcx, runs)
                    && crate::recognition::crate_impl_called_by(tcx, typing_env, callee, given).is_none()
            });
            !std && args.iter().any(|&arg| holds(thir[arg].ty))
        }
        ExprKind::Closure(closure) => closure.upvars.iter().any(|&upvar| holds(thir[upvar].ty)),
        ExprKind::PointerCoercion {
            cast: PointerCoercion::Unsize,
            source,
            ..
        } => holds(thir[*source].ty),
        _ => false,
    })
}

/// Whether a `held` value holds a `ty`: it's one, or one is among its
/// type's arguments, or a field of a struct or enum of the crate's; an
/// opaque type revealed, or, unrevealed, taken to.
fn holds<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    held: Ty<'tcx>,
    ty: Ty<'tcx>,
    seen: &mut Vec<Ty<'tcx>>,
) -> bool {
    let held = tcx.erase_and_anonymize_regions(held);
    let held = (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(held))).unwrap_or(held);
    if seen.contains(&held) {
        return false;
    }
    seen.push(held);
    held.walk().filter_map(|part| part.as_type()).any(|part| {
        let part = tcx.erase_and_anonymize_regions(part);
        match part.kind() {
            _ if part == ty => true,
            ty::Alias(..) | ty::Coroutine(..) | ty::CoroutineWitness(..) => true,
            ty::Adt(adt, args) if !crate::recognition::is_std_crate(tcx, adt.did()) => (adt.all_fields())
                .any(|field| holds(tcx, typing_env, field.ty(tcx, args).skip_normalization(), ty, seen)),
            _ => false,
        }
    })
}

/// Each function of the crate's that asks the hint of what it's given,
/// `AsksHint`'s, `hint<I:
/// Iterator>(it: I)`'s, or one that gives them on to such a function, to
/// the fixed point of the crate's bodies (#1323). By the function, not by
/// its parameter: what it's given may reach the hint through what it makes
/// of it, `metamorph_is(it)`'s `I` of an `impl MetamorphIs<I>`, so each
/// iterator it's given is, where `in_place_adapters` holds a flatten,
/// std's struct, which knows its hint. Before lowering, of the crate's
/// bodies.
pub(crate) fn asked_of_own<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[Body<'tcx>]) -> IndexSet<DefId> {
    let asks_hint = AsksHint::of(tcx);
    let mut asked: IndexSet<DefId> = IndexSet::default();
    loop {
        let before = asked.len();
        for body in bodies {
            let owner = tcx.typeck_root_def_id(body.def_id);
            let asks = body.thir.exprs.iter().any(|expr| {
                let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                    return false;
                };
                asks_hint.of_params(callee, args).is_some()
                    || asked.contains(&callee)
                        && args
                            .types()
                            .any(|given| rustc_middle::ty::TypeVisitableExt::has_param(&given))
            });
            if asks {
                asked.insert(owner);
            }
        }
        if asked.len() == before {
            return asked;
        }
    }
}

/// The calls that ask the hint of an iterator of their function's type
/// parameters: `size_hint()`, and a `collect()`, which std's `SpecFromIter`
/// asks as the `Vec` it makes grows (#1239). The one answer, of what each
/// instance is made knowing (`of_crate`) and of which functions ask the
/// hint of what they're given (`asked_of_own`, #1327).
struct AsksHint {
    size_hint: DefId,
    collect: DefId,
}

impl AsksHint {
    fn of(tcx: TyCtxt<'_>) -> Self {
        let iterator = crate::recognition::std_item(tcx, crate::recognition::StdItem::Iterator);
        AsksHint {
            size_hint: crate::recognition::trait_method(tcx, iterator, "size_hint"),
            collect: crate::recognition::trait_method(tcx, iterator, "collect"),
        }
    }

    /// Of a call of `callee` with `args` that asks it, the iterator's type,
    /// where it's of its function's type parameters.
    fn of_params<'tcx>(&self, callee: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<Ty<'tcx>> {
        let of = args.types().next()?.peel_refs();
        ((callee == self.size_hint || callee == self.collect) && rustc_middle::ty::TypeVisitableExt::has_param(&of))
            .then_some(of)
    }
}

/// `instance`, an iterator given where its hint is asked, generic code's or a
/// library's, made knowing it where it's made (`mark_hint_knowing`); and, of
/// a crate that keeps capacities, of an array JS counts (`is_array_iter`) or
/// std's compiled struct (`is_compiled_std_iterator`),
/// where JS's own adapters lose its lower bound as it's given
/// (`given_hint_is_stds`), a set's `iter().enumerate()`,
/// `entries()`, as where it's collected knowing its type (`grown_by`,
/// #1263): of a chain or a zip, whose runtime one sums its parts' hints,
/// its parts' counted adapters (`counted_parts`). Whether it was marked.
fn mark_given<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    instance: Ty<'tcx>,
    keeps_capacity: bool,
    asked: &mut HintAsked<'tcx>,
) -> bool {
    if mark_hint_knowing(
        tcx,
        recognition,
        instance,
        (&mut asked.knowing, &mut asked.hinted.types),
    ) {
        return true;
    }
    // Of an array JS counts, or of std's compiled struct, which answers
    // std's `size_hint` and `len` itself (ADR 0871), not an adapter JS
    // makes in place of either (#2226).
    let of_arrays = (instance.walk().filter_map(|part| part.as_type())).any(|held| {
        !adapter(tcx, held)
            && (recognition.is_array_iter(held.peel_refs()) || is_compiled_std_iterator(tcx, held.peel_refs()))
    });
    if !keeps_capacity || !of_arrays || recognition.given_hint_is_stds(instance) {
        return false;
    }
    match recognition.lower_bound_kept_by_parts(instance) {
        true => {
            let (counted, knowing) = counted_parts(tcx, instance);
            asked.hinted.types.extend(counted);
            asked.knowing.extend(knowing);
        }
        false => mark_each(tcx, instance, (&mut asked.knowing, &mut asked.hinted.types)),
    }
    true
}

/// `instance`, an iterator given where its hint is asked, made knowing it
/// where it's made, only of what steps an array's items, which JS's own
/// iterator, `values()`, and its own adapters know no hint of: a range's,
/// the crate's own iterator's and std's compiled ones are `$Steps`, whose
/// own `map`, `filter` and `take` know it as std's (ADRs 0542, 0918). Each
/// adapter in it whose maker makes it knowing (`typed`), a map's or a
/// filter's, `$filterEnds`; a `take(n)`'s, a `skip(n)`'s or a
/// `step_by(n)`'s made `$taking`, `$skipping` or `$stepBy` (`hinted`), and
/// its source, where it's no adapter, a `Vec`'s `iter()`, `$iter` (#1236).
/// Whether it steps an array's items, so was marked.
fn mark_hint_knowing<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    instance: Ty<'tcx>,
    marked: (&mut IndexSet<Ty<'tcx>>, &mut IndexSet<Ty<'tcx>>),
) -> bool {
    if steps_items(tcx, recognition, instance) {
        mark_each(tcx, instance, marked);
        return true;
    }
    // A `take(n)` of a `dyn Iterator`'s, a binding's JS iterator that knows
    // no hint: JS's own `take` knows none either, where std's `Take` bounds
    // its by `n` (#1691). It alone is made knowing, `$taking`, of its
    // source as it is.
    let (_, hinted) = marked;
    let mut takes = false;
    for held in instance.walk().filter_map(|part| part.as_type()) {
        let held = tcx.erase_and_anonymize_regions(held);
        if let Some((source, false)) = counted_source(tcx, held)
            && source
                .walk()
                .filter_map(|part| part.as_type())
                .any(|part| part.is_trait())
        {
            hinted.insert(held);
            takes = true;
        }
    }
    takes
}

/// Each adapter an iterator of type `ty` holds made in place inside another,
/// of a kind std's struct is where it's held, of an array's items or a
/// collection's, JS's own iterator, `values()`, or of an array made of it,
/// so JS's own adapter there, which keeps no hint: a `filter`'s `skip(1)`
/// of a `Vec`'s, a `cycle()`'s `map` of an array's, a `take(2)` of a map's
/// values (#1427, #1572). Each made knowing its hint, a `skip(1)`'s
/// `$skipping`, a `map`'s `$mapEnds`, and its source, `$iter`: in `maker`,
/// where the hint is asked, and in each function whose value of it may
/// leave it, which `maker` may be given (`may_leave`, #1333): never by
/// type across the crate, whose other values of it, another case's
/// `iter().cloned()` beside it, are JS's own. Nothing else, as what holds
/// a `$Steps`, a range's, keeps std's hint through its own adapters (ADR
/// 0542). One stepped from its back, a `rev()` in it, too: `$skipping`
/// steps from its back as std's `Skip` does (#2141, #1427).
fn mark_in_place<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    maker: DefId,
    ty: Ty<'tcx>,
    asked: &mut HintAsked<'tcx>,
) {
    let ty = ty.peel_refs();
    let parts = || ty.walk().filter_map(|part| part.as_type());
    for part in parts() {
        let part = tcx.erase_and_anonymize_regions(part.peel_refs());
        let ty::Adt(_, args) = part.kind() else { continue };
        let Some(source) = args.types().next() else { continue };
        // Or a copy made in place, never std's struct where it isn't
        // compiled, JS's own `values().map(clone)` of an array's items
        // (#1998).
        let copy_in_place = (copied_source(tcx, part).or_else(|| cloned_source(tcx, part))).is_some()
            && !crate::recognition::is_compiled_std_iterator(tcx, part);
        if part == ty
            || !(crate::recognition::is_held_compiled(tcx, part) || copy_in_place)
            || adapter(tcx, source)
            || recognition.array_source(source).is_none()
        {
            continue;
        }
        if counted_source(tcx, part).is_some() || stepped_by_source(tcx, part).is_some() {
            asked.hinted.counted_in.insert((maker, part));
            asked.hinted.types.insert(part);
        } else if made_knowing_by_its_maker(tcx, part) {
            asked.hinted.knowing_in.insert((maker, part));
            asked.hinted.knowing_types.insert(part);
        } else {
            continue;
        }
        let source = tcx.erase_and_anonymize_regions(source.peel_refs());
        asked.hinted.knowing_in.insert((maker, source));
        asked.hinted.sources.insert((part, source));
    }
}

/// Whether an iterator of type `instance` steps an array's items, or a
/// string's, which JS's own iterator, `values()`, and its own adapters
/// know no hint of (#1236).
fn steps_items<'tcx>(tcx: TyCtxt<'tcx>, recognition: Recognition<'_, 'tcx>, instance: Ty<'tcx>) -> bool {
    (instance.walk().filter_map(|part| part.as_type())).any(|held| {
        !adapter(tcx, held) && (recognition.array_source(held).is_some() || recognition.iterates_items(held))
    })
}

fn adapter<'tcx>(tcx: TyCtxt<'tcx>, held: Ty<'tcx>) -> bool {
    matches!(held.kind(), ty::Adt(adapter, _) if is_iter_adapter_def(tcx, adapter.did()))
}

/// Each adapter `instance` holds made knowing its hint, and its source.
fn mark_each<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Ty<'tcx>,
    (typed, hinted): (&mut IndexSet<Ty<'tcx>>, &mut IndexSet<Ty<'tcx>>),
) {
    for held in instance.walk().filter_map(|part| part.as_type()) {
        let held = tcx.erase_and_anonymize_regions(held);
        if made_knowing_by_its_maker(tcx, held) {
            typed.insert(held);
        }
        if let Some(source) = counted_source(tcx, held)
            .map(|(source, _)| source)
            .or_else(|| stepped_by_source(tcx, held))
        {
            hinted.insert(held);
            let source = tcx.erase_and_anonymize_regions(source.peel_refs());
            if !adapter(tcx, source) {
                typed.insert(source);
            }
        }
    }
}

/// Each `take(n)`, `skip(n)` or `step_by(n)` `instance` holds made knowing
/// its hint, `$taking`, `$skipping` or `$stepBy`, its source, where it's
/// no adapter, `$iter`, and each other adapter its maker makes knowing it,
/// in `maker` alone, as `mark_made_in`'s are (#1240): not the chain or the
/// zip that holds them, whose runtime one sums their hints or takes the
/// fewer, as it's made (#1216).
fn mark_counted<'tcx>(tcx: TyCtxt<'tcx>, maker: DefId, instance: Ty<'tcx>, hinted: &mut Hinted<'tcx>) {
    let (counted, knowing) = counted_parts(tcx, instance);
    hinted.counted_in.extend(counted.into_iter().map(|held| (maker, held)));
    hinted
        .knowing_in
        .extend(knowing.into_iter().map(|below| (maker, below)));
}

/// Of `instance`, the adapters made knowing their hint where it's made,
/// `mark_counted`'s, and what's below each made knowing its back: each
/// `take(n)`, `skip(n)` or `step_by(n)`, `$taking`, `$skipping` or
/// `$stepBy`, and each map, `$mapEnds`, which keeps its source's hint; and
/// below a counted one, an enumeration's `$enumerate`, a map's `$mapEnds`,
/// the source's `$iter`. Not the chain or the zip that holds them, whose
/// runtime one sums their hints or takes the fewer, as it's made (#1216).
fn counted_parts<'tcx>(tcx: TyCtxt<'tcx>, instance: Ty<'tcx>) -> (Vec<Ty<'tcx>>, Vec<Ty<'tcx>>) {
    let (mut counted, mut knowing) = (Vec::new(), Vec::new());
    for held in instance.walk().filter_map(|part| part.as_type()) {
        let held = tcx.erase_and_anonymize_regions(held);
        if crate::recognition::mapped_source(tcx, held).is_some() {
            counted.push(held);
        }
        let Some(source) = counted_source(tcx, held)
            .map(|(source, _)| source)
            .or_else(|| stepped_by_source(tcx, held))
        else {
            continue;
        };
        counted.push(held);
        for below in source.walk().filter_map(|part| part.as_type()) {
            let below = tcx.erase_and_anonymize_regions(below.peel_refs());
            if made_knowing_by_its_maker(tcx, below) || !adapter(tcx, below) && below.is_adt() {
                knowing.push(below);
            }
        }
    }
    (counted, knowing)
}

/// Each adapter `made`, a type of `maker`'s own type parameters, holds made
/// knowing its hint in `maker` alone: neither its source, a type parameter
/// whose callers' values are their own, nor another function's of the same
/// type (#1240).
fn mark_made_in<'tcx>(tcx: TyCtxt<'tcx>, maker: DefId, made: Ty<'tcx>, hinted: &mut Hinted<'tcx>) {
    for held in made.walk().filter_map(|part| part.as_type()) {
        let held = tcx.erase_and_anonymize_regions(held);
        if made_knowing_by_its_maker(tcx, held) {
            hinted.knowing_in.insert((maker, held));
        }
        if counted_source(tcx, held).is_some() || stepped_by_source(tcx, held).is_some() {
            hinted.counted_in.insert((maker, held));
        }
    }
}

/// Whether `ty` is an adapter whose maker makes it knowing its hint where
/// it's marked: one the in-place forms' owner gives a form that knows it
/// (`hint_knowing_std`), a map's, a filter's, a flatten's and the like. Not
/// a zip's, whose JS knows its hint as made, nor a `skip(n)`'s, a
/// `take(n)`'s or a `step_by(n)`'s, marked apart (`counted_in`).
fn made_knowing_by_its_maker<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    crate::iterators::in_place_forms::hint_knowing_std(tcx, ty).is_some()
}
