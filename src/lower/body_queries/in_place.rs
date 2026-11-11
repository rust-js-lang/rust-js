//! Which of `HELD_COMPILED`'s adapters, `rev()`, `enumerate()`, `map()`,
//! `flatten()`, a body makes in place, what the recognizers make of them,
//! and which it holds, std's struct (ADR 0871): `in_place_adapters`, and
//! across the crate's bodies, a `flat_map`'s closure's (`results_in_place`).
//! Split from body_queries.rs by concept, as a file is at most 1500 lines.

use super::{strip, tail};
use crate::collections::{IndexMap, IndexSet};
use rustc_hir::{BindingMode, ByRef};
use rustc_middle::thir::{self, ExprId, ExprKind, LocalVarId, PatKind, Thir};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// Each closure the crate gives a `flat_map` made in place, which steps
/// what it gives there as a JS iterator: what it gives is made in place
/// (`in_place_adapters`), its facts made again so. Not one held, std's
/// compiled `FlatMap`, which steps what it gives as std's struct, its
/// `next_back` too (#726). Before lowering, of the crate's bodies.
pub(super) fn results_in_place<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &mut [crate::lower::Body<'tcx>],
    cloned: &crate::lower::analysis::Cloned<'tcx>,
) {
    let mut given = IndexSet::default();
    for body in bodies.iter() {
        for e in body.thir.exprs.iter() {
            if let ExprKind::Call {
                fun, ref args, fn_span, ..
            } = e.kind
                && matches!(*body.thir[fun].ty.kind(), ty::FnDef(id, _) if crate::lower::recognition::is_flat_map(tcx, id))
                && body.facts.in_place_adapters.contains(&fn_span)
                && let Some(&f) = args.get(1)
                && let ExprKind::Closure(ref closure) = body.thir[strip(&body.thir, f)].kind
            {
                given.insert(closure.closure_id.to_def_id());
            }
        }
    }
    // And each that makes an adapter the crate clones apart, `chain()`'s
    // of a `cycle()`, whose parts its maker holds (ADR 0915).
    let clones_apart = |body: &crate::lower::Body<'tcx>| {
        body.thir.exprs.iter().any(|e| {
            matches!(e.kind, ExprKind::Call { .. })
                && crate::lower::iterators::cloned_apart(
                    (tcx, ty::TypingEnv::post_analysis(tcx, body.def_id)),
                    cloned,
                    e.ty,
                    body.def_id,
                )
        })
    };
    // And each that gives one to a function that asks its hint, or to a
    // library's, which may (#1323).
    let hint_asked = crate::lower::analysis::asked_of_own(tcx, bodies);
    let gives_to_hint_asked = |body: &crate::lower::Body<'tcx>| {
        body.thir.exprs.iter().any(|e| {
            matches!(e.kind, ExprKind::ZstLiteral { .. })
                && crate::lower::fn_def(e.ty).is_some_and(|(callee, _)| {
                    hint_asked.contains(&callee)
                        || !callee.is_local() && !crate::lower::recognition::is_std_crate(tcx, callee)
                })
        })
    };
    for body in bodies.iter_mut() {
        let flat_mapped = given.contains(&body.def_id);
        if !flat_mapped && !clones_apart(body) && !gives_to_hint_asked(body) {
            continue;
        }
        let stepped_result = flat_mapped.then_some(body.expr);
        let owner = body.def_id;
        let made_in = (tcx, ty::TypingEnv::post_analysis(tcx, owner));
        let cloned_apart = |ty| crate::lower::iterators::cloned_apart(made_in, cloned, ty, owner);
        body.facts.in_place_adapters = in_place_adapters(tcx, &body.thir, stepped_result, cloned_apart, &hint_asked);
    }
}

/// Each `rev()`, `enumerate()` or `map()`, `HELD_COMPILED`'s, whose adapter
/// is given, as what's called or an argument, where it's stepped as a JS
/// iterator, or a variable holds that it's stepped through alone, or by
/// another of them made in place of it: to another of them made in place,
/// to a std function or method not
/// compiled, `v.iter().rev().filter(f)`'s `filter`, an iterator trait's
/// method on it, `rfold`, a `for` loop's `into_iter`,
/// `zip(a, v.iter().rev())`, or to a type parameter of the crate's
/// function given a JS iterator, not as it is (ADR 0061): its `fn_span`,
/// and that call's. No one else holds it, so it's what the recognizers
/// make of it, `v.toReversed()`; one held, by a `let`, a field, a
/// parameter of its own type or one given as it is, or given to a compiled
/// std body, `fuse()`'s, is std's struct, `{ iter }` (ADR 0871).
pub(super) fn in_place_adapters<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &Thir<'tcx>,
    stepped_result: Option<ExprId>,
    cloned_apart: impl Fn(ty::Ty<'tcx>) -> bool,
    hint_asked: &IndexSet<DefId>,
) -> IndexSet<Span> {
    use crate::lower::recognition::{StdItem, std_item};
    let iterator_traits = [std_item(tcx, StdItem::Iterator), std_item(tcx, StdItem::IntoIterator)];
    // `cycle()`, which holds what it's given and clones it, as std's
    // `Cycle` does (ADR 0915).
    let cycle = crate::lower::recognition::trait_method(tcx, iterator_traits[0], "cycle");
    // `rev()`, and what steps from the back alone, `DoubleEndedIterator`'s
    // methods: given a zip that folds apart, each reaches its `fold` only as
    // its `rfold`, which `$zipEnds` folds as std does (#812).
    let rev = crate::lower::recognition::trait_method(tcx, iterator_traits[0], "rev");
    let from_its_back = |callee: DefId| {
        callee == rev
            || (tcx.trait_of_assoc(callee)).is_some_and(|tr| crate::lower::recognition::is_double_ended(tcx, tr))
    };
    // Each call of a `rev()`, an `enumerate()` or a `map()` given, as what's called or
    // an argument, borrowed for it or not, to another call: its `fn_span`,
    // and the call's, its callee, and the argument's place.
    let mut given = Vec::new();
    for e in thir.exprs.indices() {
        let ExprKind::Call {
            fun,
            ref args,
            fn_span: at,
            ..
        } = thir[e].kind
        else {
            continue;
        };
        let ty::FnDef(callee, _) = *thir[fun].ty.kind() else {
            continue;
        };
        for (index, &arg) in args.iter().enumerate() {
            // Or borrowed for it, `rev().nth(n)`'s `&mut`, which it steps
            // there too.
            let arg = match thir[strip(thir, arg)].kind {
                ExprKind::Borrow { arg, .. } => strip(thir, arg),
                _ => strip(thir, arg),
            };
            let ExprKind::Call { fun, fn_span, .. } = thir[arg].kind else {
                continue;
            };
            // Not one held to be cloned, by `cycle()` or as a part of an
            // adapter cloned apart, which its maker holds: std's struct,
            // which its compiled `Clone` clones, but one cloned apart
            // itself, a `map()`'s, made by its own maker (ADRs 0525, 0915).
            let held_to_clone = (callee == cycle || cloned_apart(thir[e].ty)) && !cloned_apart(thir[arg].ty);
            // Not a zip std folds otherwise than it steps (ADR 0997), but
            // where what it's given to steps it from its back (#812).
            if let ty::FnDef(maker, _) = *thir[fun].ty.kind()
                && crate::lower::recognition::makes_held_compiled(tcx, maker)
                && !held_to_clone
                && (!crate::lower::recognition::folds_apart(tcx, thir[arg].ty) || from_its_back(callee))
            {
                given.push((fn_span, at, callee, index, maker));
            }
        }
    }
    // `size_hint()`, which a JS iterator, once made, can't answer as Rust
    // does: of what the recognizers size where it's made, `rev()`'s, or of
    // std's struct, which knows it, a variable's or a `flatten()`'s (ADR
    // 0899).
    let size_hint = crate::lower::recognition::trait_method(tcx, iterator_traits[0], "size_hint");
    // One is sized in place where its maker's form knows its hint; a
    // `rev()` of a zip is, `$rev($zipEnds(..))`, whatever its source is
    // (#400, #740).
    let sized = |maker: DefId| crate::lower::recognition::sized_in_place(tcx, maker);
    let flattens = |maker: DefId| {
        let made = tcx.fn_sig(maker).instantiate_identity().skip_binder().output();
        crate::lower::recognition::flattened_source(tcx, made).is_some()
    };
    // What's given where its hint is asked: to `size_hint()`, to a function
    // of the crate's that asks the hint of what it's given (`asked_of_own`,
    // the hint's owner's), or to a library's, whose body, which the crate
    // can't see, may ask it, as `hint_asked.rs` takes it; or as the
    // receiver of an iterator's method whose own call is, which gives its
    // hint of it, `flat_map(f).filter(g)`'s: to the fixed point (#1323),
    // each call's `fn_span`.
    let library = |callee: DefId| !callee.is_local() && !crate::lower::recognition::is_std_crate(tcx, callee);
    let mut wanted: IndexSet<Span> = IndexSet::default();
    loop {
        let before = wanted.len();
        for e in thir.exprs.indices() {
            let ExprKind::Call {
                fun,
                ref args,
                fn_span: at,
                ..
            } = thir[e].kind
            else {
                continue;
            };
            let ty::FnDef(callee, _) = *thir[fun].ty.kind() else {
                continue;
            };
            let inputs = tcx.fn_sig(callee).instantiate_identity().skip_binder().inputs();
            let adapts = callee != size_hint && tcx.trait_of_assoc(callee) == Some(iterator_traits[0]);
            for (index, &arg) in args.iter().enumerate() {
                let arg = match thir[strip(thir, arg)].kind {
                    ExprKind::Borrow { arg, .. } => strip(thir, arg),
                    _ => strip(thir, arg),
                };
                let ExprKind::Call { fn_span: made, .. } = thir[arg].kind else {
                    continue;
                };
                let asks = match inputs.get(index) {
                    _ if adapts && index == 0 => wanted.contains(&at),
                    Some(_) => callee == size_hint || hint_asked.contains(&callee) || library(callee),
                    None => false,
                };
                if asks {
                    wanted.insert(made);
                }
            }
        }
        if wanted.len() == before {
            break;
        }
    }
    let steps_as_js =
        |callee: DefId, index: usize, in_place: &IndexSet<Span>, at: Span, (made, maker): (Span, DefId)| {
            // A flatten whose hint is asked where it's given (`wanted`): held,
            // std's struct, which knows its items' constant size, as of
            // `size_hint()` itself; JS's own `flatMap` knows none (ADR 0899,
            // #1323).
            if flattens(maker) && wanted.contains(&made) {
                return false;
            }
            // Another of them, itself made in place, `enumerate().rev()`'s: in
            // place too.
            if crate::lower::recognition::makes_held_compiled(tcx, callee) {
                return in_place.contains(&at);
            }
            if callee == size_hint && !sized(maker) {
                return false;
            }
            let std = crate::lower::recognition::is_std_crate(tcx, callee)
                && !crate::lower::recognition::is_compiled_std(tcx, callee);
            // What it's given for, a type parameter bound by `Iterator` or
            // `IntoIterator`, which steps it, `filter`'s `Self`, `zip`'s `U`,
            // `next_back()`'s and `len()`'s too: not one that holds it,
            // `mem::replace`'s `T`, `Vec::push`'s, `clone()`'s or `fmt`'s
            // `Self`, its own compiled impl's, of its struct (#537).
            let inputs = tcx.fn_sig(callee).instantiate_identity().skip_binder().inputs();
            let input = inputs.get(index).map(|input| *input.peel_refs().kind());
            let Some(ty::Param(param)) = input else {
                return false;
            };
            crate::lower::iterators::bound_as_iterator(tcx, callee, param)
                && (std || callee.is_local() && !crate::lower::iterators::given_as_it_is(tcx, callee, param))
        };
    let mut found = IndexSet::default();
    // What a closure gives that's stepped where it's given, `flat_map`'s:
    // made in place, as one given to it would be.
    if let Some(result) = stepped_result.map(|e| tail(thir, e))
        && let ExprKind::Call { fun, fn_span, .. } = thir[result].kind
        && matches!(*thir[fun].ty.kind(), ty::FnDef(id, _) if crate::lower::recognition::makes_held_compiled(tcx, id))
    {
        found.insert(fn_span);
    }
    // A variable a `let` binds one to, `let mut it = s.chars().enumerate()`,
    // read only where `Iterator`'s methods step it as a JS iterator, `next()`,
    // a `for` loop's `into_iter`, a `by_ref()` whose `&mut` they step, or
    // `drop`, never given anywhere else: nothing tells what it is, so it's
    // what the recognizers make of it too, `s.entries()`.
    let made: IndexMap<LocalVarId, Span> = (thir.stmts.iter())
        .filter_map(|stmt| match stmt.kind {
            thir::StmtKind::Let {
                ref pattern,
                initializer: Some(init),
                else_block: None,
                ..
            } => match (&pattern.kind, &thir[strip(thir, init)].kind) {
                (
                    &PatKind::Binding {
                        var,
                        mode: BindingMode(ByRef::No, _),
                        subpattern: None,
                        ..
                    },
                    &ExprKind::Call { fun, fn_span, .. },
                ) if matches!(*thir[fun].ty.kind(), ty::FnDef(id, _) if crate::lower::recognition::makes_held_compiled(tcx, id)) => {
                    Some((var, fn_span))
                }
                _ => None,
            },
            _ => None,
        })
        .collect();

    let mut reads: IndexMap<LocalVarId, usize> = IndexMap::default();
    for e in thir.exprs.iter() {
        if let ExprKind::VarRef { id } = e.kind
            && made.contains_key(&id)
        {
            *reads.entry(id).or_default() += 1;
        }
    }
    // Each call that steps what it's given as a JS iterator, by `Iterator`'s
    // methods but `by_ref()`, and `mem::drop`, which closes either: what
    // it's given.
    let mut js_steps: Vec<(ExprId, Span)> = Vec::new();
    let mut lent: Vec<(ExprId, ExprId)> = Vec::new();
    let mut makers: Vec<(ExprId, Span)> = Vec::new();
    for e in thir.exprs.indices() {
        let ExprKind::Call {
            fun,
            ref args,
            fn_span: at,
            ..
        } = thir[e].kind
        else {
            continue;
        };
        let ty::FnDef(callee, _) = *thir[fun].ty.kind() else {
            continue;
        };
        let by_iterator = tcx
            .trait_of_assoc(callee)
            .is_some_and(|tr| iterator_traits.contains(&tr))
            && !crate::lower::recognition::is_compiled_std(tcx, callee)
            && callee != size_hint;
        let by_ref = by_iterator && crate::lower::recognition::is_by_ref(tcx, callee);
        for &arg in args.iter() {
            let arg = match thir[strip(thir, arg)].kind {
                ExprKind::Borrow { arg, .. } => strip(thir, arg),
                _ => strip(thir, arg),
            };
            match () {
                // `it.by_ref()`, lent where it's stepped as a JS iterator
                // too, `for c in it.by_ref()`.
                _ if by_ref => lent.push((e, arg)),
                // Another of them made of it, `it.rev()`, which steps it as
                // a JS iterator where that's made in place.
                _ if crate::lower::recognition::makes_held_compiled(tcx, callee) => makers.push((arg, at)),
                _ if by_iterator || crate::lower::recognition::is_mem_drop(tcx, callee) => js_steps.push((arg, at)),
                _ => {}
            }
        }
    }
    let stepped_at: IndexMap<ExprId, Span> = js_steps.iter().copied().collect();
    let mut stepped: IndexMap<LocalVarId, Vec<Span>> = IndexMap::default();
    let mut note = |e: ExprId, at: Span| {
        if let ExprKind::VarRef { id } = thir[e].kind
            && made.contains_key(&id)
        {
            stepped.entry(id).or_default().push(at);
        }
    };
    for &(arg, at) in &js_steps {
        note(arg, at);
    }
    for &(by_ref, arg) in &lent {
        if let Some(&at) = stepped_at.get(&by_ref) {
            note(arg, at);
        }
    }
    let mut made_of: IndexMap<LocalVarId, Vec<Span>> = IndexMap::default();
    for &(arg, at) in &makers {
        if let ExprKind::VarRef { id } = thir[arg].kind
            && made.contains_key(&id)
        {
            made_of.entry(id).or_default().push(at);
        }
    }
    // Each given to another made in place, through a chain of them,
    // `rev().rev()`'s first, and each variable whose reads are all such
    // steps, those of another made of it in place too, to the fixed point.
    loop {
        let before = found.len();
        for &(made, at, callee, index, maker) in &given {
            if steps_as_js(callee, index, &found, at, (made, maker)) {
                found.insert(made);
            }
        }
        for (&var, &at) in &made {
            let uses = stepped.get(&var).map_or(&[][..], Vec::as_slice);
            let of = made_of.get(&var).map_or(&[][..], Vec::as_slice);
            let all = uses.len() + of.len();
            if !found.contains(&at)
                && all > 0
                && reads.get(&var) == Some(&all)
                && of.iter().all(|maker| found.contains(maker))
            {
                found.insert(at);
            }
        }
        if found.len() == before {
            return found;
        }
    }
}
