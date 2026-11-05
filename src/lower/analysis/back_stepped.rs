//! The adapter types the crate steps from their back (ADR 0545): a `map` or
//! a `filter` whose `next_back`, `rfold` and the like are called, of a
//! source not an array's, whose own forms step it, or of an array's items
//! with a destructor, an owning source closed to drop them, which they
//! don't. A map of such a type is
//! made knowing its back where it's made, as the iterators draft asks of
//! one owner, before lowering: so one held in a variable before it's
//! stepped, as a library's drop flags make one, still steps from its back.
//! So are a chain's sides (#718), and a map whose closure has a destructor
//! that a zip read by index asks the length of (#775).

use crate::lower::body_queries::strip;
use crate::lower::recognition::{
    Recognition, back_stepped_source, chained_sources, cloned_source, copied_source, enumerated_source,
    enumerates_a_compiled_struct, filter_mapped_source, filtered_source, flattened_source, inspected_source,
    is_double_ended, is_exact_size, is_iter_adapter_def, mapped_source, reversed_source, zipped_sources,
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
    let zip = crate::lower::recognition::compiled_provided_method(tcx, "Iterator", "zip");
    for body in all_bodies {
        let typing_env = ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(body.def_id));
        let stepped = of_body.entry(body.def_id).or_default();
        // A `rev()` held, std's compiled `Rev`, which steps what it's given
        // by its dictionary, not one a std iterator's method steps where
        // it's made, which the recognizers reverse (ADR 0871).
        let in_place = &body.facts.in_place_adapters;
        let called: HashMap<ExprId, (rustc_span::Span, &[ExprId])> = (body.thir.exprs.iter())
            .filter_map(|e| match e.kind {
                ExprKind::Call {
                    fun, fn_span, ref args, ..
                } => Some((strip(&body.thir, fun), (fn_span, &**args))),
                _ => None,
            })
            .collect();
        for (at, expr) in body.thir.exprs.iter_enumerated() {
            let (ExprKind::ZstLiteral { .. }, Some((callee, args))) = (&expr.kind, fn_def(expr.ty)) else {
                continue;
            };
            // A zip that reads its sides by index (ADR 0973), which asks the
            // length of `b`, a map whose closure has a destructor, held in a
            // variable to drop it, whose form no use reads: made knowing its
            // length where it's made, as one a bound asks it of is (#775).
            if Some(callee) == zip
                && let [a, b] = args.types().collect::<Vec<_>>()[..]
                && !rustc_middle::ty::TypeVisitableExt::has_param(&(a, b))
                && crate::lower::recognition::zips_by_index(tcx, typing_env, a, b)
            {
                let b = recognition.reveal(crate::lower::recognition::iterator_type_of(tcx, typing_env, b));
                if closure_drops(tcx, b, &drops) {
                    stepped.extend(back_stepped_adapter(tcx, recognition, b, false, &drops));
                }
            }
            let held_rev = Some(callee) == rev && called.get(&at).is_none_or(|(span, _)| !in_place.contains(span));
            // `DoubleEndedIterator`'s own method, `next_back()`, which asks
            // it of what it's called on: below, of each adapter.
            let own_method = tcx.trait_of_assoc(callee).is_some_and(|tr| is_double_ended(tcx, tr));
            // Whether what asks its back, `rev()` or generic code, is given
            // a value of type `asked` that a variable, a field or a place
            // holds, not one made where it's given: `kept.rev()` of a `let
            // kept = v.iter().filter_map(f)`, which no form of the call's
            // steps from its back (#484). Through its borrows: what a call
            // gives, `it.by_ref()`'s, is that call's.
            let through = |mut e: ExprId| loop {
                e = strip(&body.thir, e);
                match body.thir[e].kind {
                    ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = arg,
                    _ => break &body.thir[e],
                }
            };
            let given_held = |asked: Ty<'tcx>| {
                !own_method
                    && called.get(&at).is_some_and(|(_, args)| {
                        (args.iter()).any(|&arg| {
                            body.thir[arg].ty.peel_refs() == asked
                                && !matches!(through(arg).kind, ExprKind::Call { .. })
                        })
                    })
            };
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
                // One held where it's given, made knowing its back where
                // it's made, as one given behind an `impl` is: asked its
                // back, not its length alone.
                let held_arg = !length && given_held(hidden);
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
                        // variable too. Not a flatten the `rev()` asks itself,
                        // which stays made in place up to its `rev()`, past
                        // calls that unwind to the same cleanup (#723), and is
                        // made `$flattenEnds` there.
                        // And an enumeration of std's struct, `$enumerate` of
                        // it, which no form of its own steps from the back
                        // (#712).
                        if held != asked
                            || given
                            || held_rev
                            || held_arg
                            || dropping_items(recognition, held, &drops)
                            || closure_drops(tcx, held, &drops)
                            || enumerates_a_compiled_struct(tcx, held)
                        {
                            let made_apart = given || held_rev || held_arg;
                            stepped.extend(back_stepped_adapter(tcx, recognition, held, made_apart, &drops));
                        }
                        // A chain of items with a destructor a `rev()` or
                        // generic code asks to be double-ended, which MIR
                        // holds in a variable to drop them before it's
                        // asked: made knowing its back where it's made, as no
                        // form is left to read where it's stepped (#718).
                        if held == asked
                            && !given
                            && chained_sources(tcx, held).is_some()
                            && recognition.iterator_item(held).is_some_and(&drops)
                        {
                            stepped.insert(tcx.erase_and_anonymize_regions(held));
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
            if !own_method {
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
    // A `rev()`, whose forms, a range stepped from its end or an array's
    // items' chain reversed, step only forward: made knowing its back where
    // it's made, as MIR may hold it in a variable before its `rfold` or
    // `nth_back`, past a call, a cleanup that drops it, or a `&mut` (#702).
    if reversed_source(tcx, adapter).is_some() {
        return Some(tcx.erase_and_anonymize_regions(adapter));
    }
    // A zip, whose own forms, `$lazyZip` and `$trustedZip`, step only
    // forward: made knowing its back where it's made, `$zipEnds`, as MIR
    // holds it in a variable before its `next_back` (#740).
    if zipped_sources(tcx, adapter).is_some() {
        return Some(tcx.erase_and_anonymize_regions(adapter));
    }
    // An enumeration of items with a destructor, held in a variable to drop
    // them, whose own `next_back()` the crate calls: std's compiled
    // `Enumerate` where it's held (ADR 0871), which steps its source by
    // its dictionaries (#383). Or of std's struct compiled wherever it's
    // made, `$enumerate` of it, of whatever items (#712).
    // And of a map or a filter of an array's items with a destructor, held
    // in a variable to drop them though what it gives has none (#764).
    if let Some(source) = enumerated_source(tcx, adapter) {
        let item = recognition.iterator_item(adapter);
        return (item.is_some_and(&drops)
            || dropping_items(recognition, source.peel_refs(), &drops)
            || enumerates_a_compiled_struct(tcx, adapter))
        .then(|| tcx.erase_and_anonymize_regions(adapter));
    }
    // An `inspect(f)` is made as a map, stepped from its back as one is.
    let mapped = mapped_source(tcx, adapter).or_else(|| inspected_source(tcx, adapter));
    let copied = copied_source(tcx, adapter);
    // A `filter_map`, made as a map filtered, steps from its back as a
    // filter does, by no form of its own (#473).
    let filtered = filtered_source(tcx, adapter).or_else(|| filter_mapped_source(tcx, adapter));
    let source = (mapped.or(filtered).or(copied))?.peel_refs();
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
/// made, held in a variable or not, as what steps it asks (#218). And a
/// chain one steps, the one runtime `Chain`, which MIR may hold in a
/// variable before its map is made, its sides made knowing theirs where
/// it's made: a `map` of a chain of the crate's iterators folded from its
/// back (#739). And a chain's sides in turn, which `$lazyChain` steps from
/// their back, each made knowing it where it's made: MIR holds side `a` in
/// a variable before the chain is made, where `b`'s drop flag must be
/// cleared, so no form of `a` is left to read (#718). And a zip's sides,
/// which `$zipEnds` steps from their back and counts, a `cloned()`'s too,
/// as MIR holds a side in a variable before the zip is made (#792).
fn with_stepped_sources<'tcx>(tcx: TyCtxt<'tcx>, stepped: HashSet<Ty<'tcx>>) -> HashSet<Ty<'tcx>> {
    let mut all = stepped.clone();
    let mut sources: Vec<Ty<'tcx>> = stepped.into_iter().collect();
    while let Some(ty) = sources.pop() {
        let held = match chained_sources(tcx, ty).or_else(|| zipped_sources(tcx, ty)) {
            Some((a, b)) => vec![a, b],
            None => back_stepped_source(tcx, ty).into_iter().collect(),
        };
        for held in held.into_iter().map(|held| held.peel_refs()) {
            if mapped_source(tcx, held).is_some()
                || filtered_source(tcx, held).is_some()
                || filter_mapped_source(tcx, held).is_some()
                || copied_source(tcx, held).is_some()
                || cloned_source(tcx, held).is_some()
                || inspected_source(tcx, held).is_some()
                || chained_sources(tcx, held).is_some()
            {
                all.insert(tcx.erase_and_anonymize_regions(held));
            }
            sources.push(held);
        }
    }
    all
}

/// Whether `adapter` is a map whose closure has a destructor, `drops`'s:
/// closed to drop it and held in a variable, so no form of its own steps
/// it from its back or counts it (#385). An `inspect(f)`'s too, made as a
/// map, `F` its second type as `Map`'s is (#780).
fn closure_drops<'tcx>(tcx: TyCtxt<'tcx>, adapter: Ty<'tcx>, drops: impl Fn(Ty<'tcx>) -> bool) -> bool {
    (mapped_source(tcx, adapter).is_some() || inspected_source(tcx, adapter).is_some())
        && matches!(adapter.kind(), ty::Adt(_, args) if drops(args.type_at(1)))
}

/// Whether `adapter`, a map, an inspect or a filter, is of an array's
/// items whose drop runs, `drops`'s: an owning source closed to drop them
/// (ADR 0465), or a `rev()` of one, which steps them from the other end
/// (#646).
fn dropping_items<'tcx>(
    recognition: Recognition<'_, 'tcx>,
    adapter: Ty<'tcx>,
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> bool {
    let tcx = recognition.tcx;
    let unreversed = |mut source: Ty<'tcx>| {
        while let Some(reversed) = crate::lower::recognition::reversed_source(tcx, source) {
            source = reversed.peel_refs();
        }
        source
    };
    // An `inspect(f)`'s too, made as a map (#751).
    mapped_source(tcx, adapter)
        .or_else(|| inspected_source(tcx, adapter))
        .or_else(|| filtered_source(tcx, adapter))
        .map(|source| unreversed(source.peel_refs()))
        .is_some_and(|source| {
            recognition.array_source(source).is_some() && recognition.iterator_item(source).is_some_and(drops)
        })
}
