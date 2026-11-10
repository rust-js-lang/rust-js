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

/// What `of_crate` finds: the adapters, and the sources, whose makers make
/// them knowing their back, which knows their hint too, to join a back
/// step's types (`BackStepped::typed`); and the `take(n)`s, `skip(n)`s and
/// `step_by(n)`s made `$taking`, `$skipping` or `$stepBy` where they're
/// made (`hinted`).
pub(in crate::lower) struct HintAsked<'tcx> {
    pub(in crate::lower) knowing: IndexSet<Ty<'tcx>>,
    pub(in crate::lower) hinted: IndexSet<Ty<'tcx>>,
}

/// What `all_bodies` ask the hint of, through generic code and libraries.
pub(super) fn of_crate<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
) -> HintAsked<'tcx> {
    let mut asked = HintAsked {
        knowing: IndexSet::default(),
        hinted: IndexSet::default(),
    };
    let size_hint = crate::lower::recognition::trait_method(
        tcx,
        crate::lower::recognition::std_item(tcx, crate::lower::recognition::StdItem::Iterator),
        "size_hint",
    );
    let mut of_params: Vec<(rustc_span::def_id::DefId, Ty<'tcx>)> = Vec::new();
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            // `size_hint()` of a type of its type parameters, asked of what
            // its callers give, each instance below.
            if callee == size_hint
                && let Some(of) = args.types().next().map(|ty| ty.peel_refs())
                && rustc_middle::ty::TypeVisitableExt::has_param(&of)
            {
                of_params.push((tcx.typeck_root_def_id(body.def_id), tcx.erase_and_anonymize_regions(of)));
            }
            // A library's function's iterators, whose hint its body, which
            // the crate can't see, may ask.
            if recognition.foreign.is_library(callee.krate) {
                for given in args.types() {
                    mark_hint_knowing(
                        tcx,
                        recognition,
                        given.peel_refs(),
                        (&mut asked.knowing, &mut asked.hinted),
                    );
                }
            }
        }
    }
    // Each instance a caller the crate sees gives, through generic callers
    // (the callers' instances' carrier, `Fact::Marked`, ADR 1115).
    if !of_params.is_empty() {
        let callers = crate::lower::instances::callers_of(tcx, all_bodies);
        for (owner, of) in of_params {
            for instance in crate::lower::instances::seen_instances_of(tcx, &callers, of, owner) {
                mark_hint_knowing(tcx, recognition, instance, (&mut asked.knowing, &mut asked.hinted));
            }
        }
    }
    asked
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
fn mark_hint_knowing<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    instance: Ty<'tcx>,
    (typed, hinted): (&mut IndexSet<Ty<'tcx>>, &mut IndexSet<Ty<'tcx>>),
) {
    let adapter =
        |held: Ty<'tcx>| matches!(held.kind(), ty::Adt(adapter, _) if is_iter_adapter_def(tcx, adapter.did()));
    let of_items = (instance.walk().filter_map(|part| part.as_type()))
        .any(|held| !adapter(held) && (recognition.array_source(held).is_some() || recognition.iterates_items(held)));
    if !of_items {
        return;
    }
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
            if !adapter(source) {
                typed.insert(source);
            }
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
