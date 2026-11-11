//! What generic code asks the hint of, `size_hint()`, of what its callers
//! give (#1236, ADR 0170, amended): each instance a caller the crate sees
//! gives, and what it gives a library's generic function, whose body it
//! can't see, made in rust-js's form, which knows std's hint, where it's
//! made. The fact the "made knowing" machinery reads beside a back step's
//! (`back_stepped.rs`), and #1180's capacity questions can ask.

use crate::collections::IndexSet;
use crate::lower::recognition::{
    Recognition, chained_sources, cloned_source, copied_source, counted_source, enumerated_source,
    filter_mapped_source, filtered_source, flattened_source, inspected_source, is_iter_adapter_def, mapped_source,
    reversed_source, stepped_by_source,
};
use crate::lower::{Body, fn_def};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// What `of_crate` finds: the adapters, and the sources, whose makers make
/// them knowing their back, which knows their hint too, to join a back
/// step's types (`BackStepped::typed`); and the `take(n)`s, `skip(n)`s and
/// `step_by(n)`s made `$taking`, `$skipping` or `$stepBy` where they're
/// made (`hinted`).
pub(in crate::lower) struct HintAsked<'tcx> {
    pub(in crate::lower) knowing: IndexSet<Ty<'tcx>>,
    pub(in crate::lower) hinted: Hinted<'tcx>,
}

/// The adapters made knowing their hint, which the crate's makers read:
/// of a type of no type parameter, wherever it's made (`types`); and of one
/// generic code makes of its own parameters, only in the function that
/// makes it, by its owner, as `Take<I>` is the same type of every generic
/// function's `I` (#1240): a `take(n)`'s, a `skip(n)`'s and a
/// `step_by(n)`'s made `$taking`, `$skipping` or `$stepBy` (`counted_in`),
/// and a map's and the like made knowing its back (`knowing_in`).
#[derive(Default)]
pub(in crate::lower) struct Hinted<'tcx> {
    pub(in crate::lower) types: IndexSet<Ty<'tcx>>,
    pub(in crate::lower) counted_in: IndexSet<(DefId, Ty<'tcx>)>,
    pub(in crate::lower) knowing_in: IndexSet<(DefId, Ty<'tcx>)>,
    /// What generic code grows a `Vec` by, `extend`'s iterator, by its
    /// owner, where an instance its callers give has a JS hint whose lower
    /// bound isn't std's as it's given (`given_hint_is_stds`), so
    /// std's growth by it can't be kept: the `Vec`'s capacity unknown once
    /// it grows, which `capacity()` refuses (#1180's (3)). Where the type
    /// is known, the extend is refused at compile time instead.
    pub(in crate::lower) lost: IndexSet<(DefId, Ty<'tcx>)>,
    /// Of those whose callers can't all be seen, a library's `pub`
    /// function's, those asked of each value it's given as it grows: lost
    /// where its hint meets JS's own iterator, which knows none
    /// (`$hintAsked`, #1251).
    pub(in crate::lower) by_value: IndexSet<(DefId, Ty<'tcx>)>,
}

/// What `all_bodies` ask the hint of, through generic code and libraries.
pub(super) fn of_crate<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    keeps_capacity: bool,
) -> HintAsked<'tcx> {
    let mut asked = HintAsked {
        knowing: IndexSet::default(),
        hinted: Hinted::default(),
    };
    let asks = AsksHint::of(tcx);
    let collect = asks.collect;
    let mut of_params: Vec<(DefId, Ty<'tcx>)> = Vec::new();
    let mut grown: Vec<(DefId, Ty<'tcx>)> = Vec::new();
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            // A `Vec` generic code grows by an iterator of its type
            // parameters, `extend`'s, whose hint std asks as it fills:
            // asked of each instance its callers give, below.
            if recognition.extend_grows(callee, args)
                && let Some(iterable) = args.types().nth(2)
                && rustc_middle::ty::TypeVisitableExt::has_param(&iterable)
            {
                let typing_env = ty::TypingEnv::post_analysis(tcx, body.def_id);
                let iter_ty = crate::lower::recognition::iterator_type_of(tcx, typing_env, iterable);
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
            let grown_by = match () {
                _ if !keeps_capacity => None,
                _ if callee == collect && args.types().nth(1).is_some_and(|into| recognition.grows_as_vec(into)) => {
                    args.types().next()
                }
                _ if recognition.extend_grows(callee, args) => args.types().nth(2).map(|iterable| {
                    let typing_env = ty::TypingEnv::post_analysis(tcx, body.def_id);
                    crate::lower::recognition::iterator_type_of(tcx, typing_env, iterable)
                }),
                _ => None,
            };
            if let Some(iter_ty) = grown_by.map(|ty| tcx.erase_and_anonymize_regions(ty.peel_refs()))
                && !rustc_middle::ty::TypeVisitableExt::has_param(&iter_ty)
                && !recognition.lower_bound_is_stds(iter_ty)
                && recognition.lower_bound_kept_by_parts(iter_ty)
            {
                mark_counted(tcx, tcx.typeck_root_def_id(body.def_id), iter_ty, &mut asked.hinted);
            }
            // What asks the hint of a type of its type parameters, asked of
            // what its callers give, each instance below (`AsksHint`).
            if let Some(of) = asks.of_params(callee, args) {
                of_params.push((tcx.typeck_root_def_id(body.def_id), tcx.erase_and_anonymize_regions(of)));
            }
            // A library's function's iterators, whose hint its body, which
            // the crate can't see, may ask.
            if recognition.foreign.is_library(callee.krate) {
                for given in args.types() {
                    _ = mark_hint_knowing(
                        tcx,
                        recognition,
                        given.peel_refs(),
                        (&mut asked.knowing, &mut asked.hinted.types),
                    );
                }
            }
        }
    }
    let callers =
        (!of_params.is_empty() || !grown.is_empty()).then(|| crate::lower::instances::callers_of(tcx, all_bodies));
    // Each `Vec` generic code grows by what one of its callers gives whose
    // JS hint's lower bound isn't std's (#1180's (3)). Where callers can't
    // be seen, a library's `pub` function's, asked of each value its
    // consumers give, as it grows (#1251): lost by its type, every one of
    // its `extend`s would be, and what's then extended again by the
    // crate's own hint would throw.
    if let Some(callers) = &callers {
        for (owner, iter_ty) in grown {
            match crate::lower::instances::instances_of(tcx, callers, iter_ty, owner) {
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
            let (instances, made) = crate::lower::instances::seen_instances_of(tcx, &callers, of, owner);
            // Of every caller where the crate sees them all, or any it may
            // be given, a library's `pub` function's, whose callers are its
            // consumers'.
            let mut of_items = crate::lower::instances::instances_of(tcx, &callers, of, owner).is_none();
            for instance in instances {
                of_items |= mark_hint_knowing(
                    tcx,
                    recognition,
                    instance,
                    (&mut asked.knowing, &mut asked.hinted.types),
                );
            }
            if of_items {
                for (maker, made) in made {
                    mark_made_in(tcx, maker, made, &mut asked.hinted);
                }
            }
        }
    }
    asked
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
pub(in crate::lower) fn asked_of_own<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[Body<'tcx>]) -> IndexSet<DefId> {
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
        let iterator = crate::lower::recognition::std_item(tcx, crate::lower::recognition::StdItem::Iterator);
        AsksHint {
            size_hint: crate::lower::recognition::trait_method(tcx, iterator, "size_hint"),
            collect: crate::lower::recognition::trait_method(tcx, iterator, "collect"),
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
    let of_items = (instance.walk().filter_map(|part| part.as_type())).any(|held| {
        !adapter(tcx, held) && (recognition.array_source(held).is_some() || recognition.iterates_items(held))
    });
    if of_items {
        mark_each(tcx, instance, marked);
    }
    of_items
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
    for held in instance.walk().filter_map(|part| part.as_type()) {
        let held = tcx.erase_and_anonymize_regions(held);
        // A map of them, `$mapEnds`, which keeps its source's hint.
        if crate::lower::recognition::mapped_source(tcx, held).is_some() {
            hinted.counted_in.insert((maker, held));
        }
        let Some(source) = counted_source(tcx, held)
            .map(|(source, _)| source)
            .or_else(|| stepped_by_source(tcx, held))
        else {
            continue;
        };
        hinted.counted_in.insert((maker, held));
        // What it counts, made knowing its hint down to its own source: an
        // enumeration's `$enumerate`, a map's `$mapEnds`, the source's
        // `$iter`.
        for below in source.walk().filter_map(|part| part.as_type()) {
            let below = tcx.erase_and_anonymize_regions(below.peel_refs());
            if made_knowing_by_its_maker(tcx, below) || !adapter(tcx, below) && below.is_adt() {
                hinted.knowing_in.insert((maker, below));
            }
        }
    }
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

/// Whether `ty` is an adapter whose maker makes it knowing its back and its
/// hint where it's a type's question, `back_facts.rs`' own list: a map's,
/// a filter's, a `filter_map`'s, an `inspect`'s, a chain's, a copy's, an
/// enumeration's, a flatten's or a `rev()`'s. Not a zip's, a `skip(n)`'s,
/// a `take(n)`'s nor a `step_by(n)`'s, whose JS knows its hint as made, or
/// is made knowing it where it's stepped from its back.
fn made_knowing_by_its_maker<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    mapped_source(tcx, ty).is_some()
        || filtered_source(tcx, ty).is_some()
        || filter_mapped_source(tcx, ty).is_some()
        || inspected_source(tcx, ty).is_some()
        || chained_sources(tcx, ty).is_some()
        || copied_source(tcx, ty).is_some()
        || cloned_source(tcx, ty).is_some()
        || enumerated_source(tcx, ty).is_some()
        || flattened_source(tcx, ty).is_some()
        || reversed_source(tcx, ty).is_some()
}
