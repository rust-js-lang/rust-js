//! The adapter types the crate steps from their back (ADR 0545): a `map` or
//! a `filter` whose `next_back`, `rfold` and the like are called, of a
//! source not an array's, whose own forms step it, or of an array's items
//! with a destructor, an owning source closed to drop them, which they
//! don't. A map of such a type is
//! made knowing its back where it's made, as the iterators draft asks of
//! one owner, before lowering: so one held in a variable before it's
//! stepped, as a library's drop flags make one, still steps from its back.

use crate::lower::recognition::{
    Recognition, back_stepped_source, copied_source, enumerated_source, filtered_source, flattened_source,
    is_double_ended, is_exact_size, is_iter_adapter_def, mapped_source,
};
use crate::lower::{Body, fn_def};
use rustc_middle::thir::{ExprId, ExprKind};
use rustc_middle::ty::{self, Ty, TyCtxt};
use std::collections::{HashMap, HashSet};

/// Each such type `all_bodies` step from its back, regions erased.
pub(super) fn back_stepped<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> HashSet<Ty<'tcx>> {
    let mut stepped = HashSet::new();
    // Each body's own, by its function: what a caller's call of it makes too.
    let mut of_body: HashMap<rustc_span::def_id::DefId, HashSet<Ty<'tcx>>> = HashMap::new();
    let rev = crate::lower::recognition::compiled_provided_method(tcx, "Iterator", "rev");
    for body in all_bodies {
        let stepped = of_body.entry(body.def_id).or_default();
        // A `rev()` held, std's compiled `Rev`, which steps what it's given
        // by its dictionary, not one a std iterator's method steps where
        // it's made, which the recognizers reverse (ADR 0871).
        let in_place = &body.facts.in_place_adapters;
        let called: HashMap<ExprId, rustc_span::Span> = (body.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                ExprKind::Call { fun, fn_span, .. } => Some((fun, fn_span)),
                _ => None,
            })
            .collect();
        for (at, expr) in body.thir.exprs.iter_enumerated() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            let held_rev = Some(callee) == rev && called.get(&at).is_none_or(|span| !in_place.contains(span));
            // What it asks to be double-ended holds such a type, as an
            // `Either` of filters does, stepped from its back by the impl
            // that asks it of what it holds; the type asked itself is its
            // call's own, below or at its `rev()`.
            for (clause, _) in tcx.clauses_of(callee).instantiate(tcx, args) {
                let Some(asked) = clause.skip_normalization().as_trait_clause() else {
                    continue;
                };
                // Or its length, of a map whose closure has a destructor,
                // held in a variable to drop it, whose form no use reads
                // (#385): made knowing both where it's made.
                let length = is_exact_size(tcx, asked.def_id());
                if !is_double_ended(tcx, asked.def_id()) && !length {
                    continue;
                }
                let hidden = asked.self_ty().skip_binder().peel_refs();
                let asked = recognition.reveal(hidden);
                if length && !closure_drops(tcx, asked, &drops) {
                    continue;
                }
                // One a function gives behind an `impl`, which its caller
                // can't make knowing its back: made so where it's made.
                let given = hidden != asked;
                // Not through a std adapter, whose own call steps what it
                // holds: `filter(p).map(f).rev()`'s is `rev()`'s.
                let mut walk = asked.walk();
                while let Some(part) = walk.next() {
                    let Some(held) = part.as_type() else { continue };
                    if matches!(held.kind(), ty::Adt(adapter, _) if is_iter_adapter_def(tcx, adapter.did())) {
                        // The type asked itself too where it's of an array's
                        // items with a destructor, which `rev()` can't step
                        // from its back once it's held in a variable.
                        // A held `Rev`'s, of an array's items too, which no
                        // form of the caller's reverses once it's held.
                        // And a map whose closure has a destructor, held in a
                        // variable too.
                        if held != asked
                            || given
                            || held_rev
                            || dropping_items(recognition, held, &drops)
                            || closure_drops(tcx, held, &drops)
                        {
                            stepped.extend(back_stepped_adapter(tcx, recognition, held, given || held_rev, &drops));
                        }
                        // Given behind an `impl`, each it holds too, made
                        // where it's made, which no call of the caller's
                        // steps.
                        if !given {
                            walk.skip_current_subtree();
                        }
                    }
                }
            }
            if !tcx.trait_of_assoc(callee).is_some_and(|tr| is_double_ended(tcx, tr)) {
                continue;
            }
            let Some(hidden) = args.types().next().map(|ty| ty.peel_refs()) else {
                continue;
            };
            let adapter = recognition.reveal(hidden);
            match hidden != adapter {
                // Given behind an `impl`: each adapter in it.
                true => stepped.extend(adapter.walk().filter_map(|part| part.as_type()).filter_map(|held| {
                    matches!(held.kind(), ty::Adt(a, _) if is_iter_adapter_def(tcx, a.did()))
                        .then(|| back_stepped_adapter(tcx, recognition, held, true, &drops))
                        .flatten()
                })),
                false => stepped.extend(back_stepped_adapter(tcx, recognition, adapter, false, &drops)),
            }
        }
    }
    stepped.extend(of_body.values().flatten().copied());
    // And of a generic function's type parameters, `Filter<I, P>` a function
    // steps from its back that it's given: each of its callers' types,
    // `Filter<Copied<Iter<u32>>, {closure}>`, which its caller makes, through
    // what calls it in turn (#388). At most 32 calls deep, as a function
    // that calls itself with a type of its own types would go on.
    for _ in 0..32 {
        let mut more = Vec::new();
        for body in all_bodies {
            for expr in body.thir.exprs.iter() {
                let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                    continue;
                };
                let Some(own) = of_body.get(&callee) else { continue };
                for &ty in own
                    .iter()
                    .filter(|ty| rustc_middle::ty::TypeVisitableExt::has_param(*ty))
                {
                    let made = ty::EarlyBinder::bind(tcx, ty)
                        .instantiate(tcx, args)
                        .skip_normalization();
                    let made = tcx.erase_and_anonymize_regions(made);
                    // Of the caller's own, which a type of its parameters may
                    // look like the callee's, `Filter<I, P>` of both.
                    if !of_body.get(&body.def_id).is_some_and(|known| known.contains(&made)) {
                        more.push((body.def_id, made));
                    }
                }
            }
        }
        if more.is_empty() {
            break;
        }
        for (owner, made) in more {
            stepped.insert(made);
            of_body.entry(owner).or_default().insert(made);
        }
    }
    with_stepped_sources(tcx, stepped)
}

/// `adapter`, regions erased, where it's a map or a filter of a source not
/// an array's, a filter of an array's items, which no form of its own
/// steps from its back (ADR 0990), a map of an array's items whose drop
/// runs, `drops`'s, or one that a function gives behind an `impl`,
/// `given`, which no form of its caller's steps.
fn back_stepped_adapter<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    adapter: Ty<'tcx>,
    given: bool,
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> Option<Ty<'tcx>> {
    // A flatten, of whatever source: its own forms never step from the back.
    // A chain a function gives behind an `impl` is std's `Chain`, which
    // steps from its back by its own methods (ADR 0919).
    if flattened_source(tcx, adapter).is_some() {
        return Some(tcx.erase_and_anonymize_regions(adapter));
    }
    // An enumeration of items with a destructor, held in a variable to drop
    // them, whose own `next_back()` the crate calls: std's compiled
    // `Enumerate` where it's held (ADR 0871), which steps its source by
    // its dictionaries (#383).
    if enumerated_source(tcx, adapter).is_some() {
        let item = recognition.iterator_item(adapter);
        return item
            .is_some_and(&drops)
            .then(|| tcx.erase_and_anonymize_regions(adapter));
    }
    let mapped = mapped_source(tcx, adapter);
    let copied = copied_source(tcx, adapter);
    let source = (mapped.or_else(|| filtered_source(tcx, adapter)).or(copied))?.peel_refs();
    // A map of an array's items steps from its back by its own form, its
    // source's last item mapped, and a copy of them by its items'; a
    // filter's has none. A copy of another, a type parameter's, `Copied<I>`
    // generic code steps from its back, made knowing it (#385).
    let native = (mapped.is_some() || copied.is_some())
        && !given
        && recognition.array_source(source).is_some()
        && !dropping_items(recognition, adapter, &drops)
        && !closure_drops(tcx, adapter, &drops);
    (!native).then(|| tcx.erase_and_anonymize_regions(adapter))
}

/// `stepped`, the types stepped from their back, and each map or filter
/// they step from its back in turn, their sources', through what steps
/// from its back where its source does: made knowing its back where it's
/// made, held in a variable or not, as what steps it asks (#218).
fn with_stepped_sources<'tcx>(tcx: TyCtxt<'tcx>, stepped: HashSet<Ty<'tcx>>) -> HashSet<Ty<'tcx>> {
    let mut all = stepped.clone();
    for ty in stepped {
        let mut source = back_stepped_source(tcx, ty);
        while let Some(held) = source.map(|held| held.peel_refs()) {
            if mapped_source(tcx, held).is_some()
                || filtered_source(tcx, held).is_some()
                || copied_source(tcx, held).is_some()
            {
                all.insert(tcx.erase_and_anonymize_regions(held));
            }
            source = back_stepped_source(tcx, held);
        }
    }
    all
}

/// Whether `adapter` is a map whose closure has a destructor, `drops`'s:
/// closed to drop it and held in a variable, so no form of its own steps
/// it from its back or counts it (#385).
fn closure_drops<'tcx>(tcx: TyCtxt<'tcx>, adapter: Ty<'tcx>, drops: impl Fn(Ty<'tcx>) -> bool) -> bool {
    mapped_source(tcx, adapter).is_some() && matches!(adapter.kind(), ty::Adt(_, args) if drops(args.type_at(1)))
}

/// Whether `adapter`, a map or a filter, is of an array's items whose drop
/// runs, `drops`'s: an owning source closed to drop them (ADR 0465).
fn dropping_items<'tcx>(
    recognition: Recognition<'_, 'tcx>,
    adapter: Ty<'tcx>,
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> bool {
    let tcx = recognition.tcx;
    mapped_source(tcx, adapter)
        .or_else(|| filtered_source(tcx, adapter))
        .map(|source| source.peel_refs())
        .is_some_and(|source| {
            recognition.array_source(source).is_some() && recognition.iterator_item(source).is_some_and(drops)
        })
}
