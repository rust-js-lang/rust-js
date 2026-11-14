//! Which of `HELD_COMPILED`'s adapters, `rev()`, `enumerate()`, `map()`,
//! `flatten()`, a body makes in place, what the recognizers make of them,
//! and which it holds, std's struct (ADR 0871): `in_place_adapters`, of
//! each body, decided across the crate's once its `Recognition` exists
//! (`decide`).
//! Split from body_queries.rs by concept, as a file is at most 1500 lines.

use super::{strip, tail};
use crate::collections::{IndexMap, IndexSet};
use rustc_hir::{BindingMode, ByRef};
use rustc_middle::thir::{self, ExprId, ExprKind, LocalVarId, PatKind, Thir};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// A call's callee, its generic arguments and the index of what it's given.
type Called<'tcx> = (DefId, ty::GenericArgsRef<'tcx>, usize);

/// Which of `HELD_COMPILED`'s adapters each body makes in place, decided
/// once the crate's `Recognition` exists, before anything reads it
/// (`InPlace`): of each body, `in_place_adapters` given what the crate's
/// other bodies say, which functions ask the hint of what they're given
/// (`asked_of_own`) and which adapters the crate clones apart (ADR 0915);
/// then each closure the crate gives a `flat_map` made in place, which
/// steps what it gives there as a JS iterator, so what it gives is made in
/// place too; not one held, std's compiled `FlatMap`, which steps what it
/// gives as std's struct, its `next_back` too (#726). A collect or an
/// extend asks the hint of what it's given, std's, which the program sees
/// where it reaches a `size_hint` of the crate's own or a library's
/// (`collect_hint_seen`): a flatten given one there is held, std's struct,
/// which knows its items' constant size, which its callers or its trait's
/// dictionaries give (#1327, #1341); and of a crate that keeps capacities,
/// a `Vec`'s collect or extend, whose capacity is its iterator's hint's
/// lower bound, which JS's own `flatMap` hasn't (`grows_vec_by`, #1528). Of
/// the build of std's iterator structs, none (ADR 0908).
pub(in crate::lower) fn decide<'tcx>(
    recognition: crate::lower::recognition::Recognition<'_, 'tcx>,
    bodies: &mut [crate::lower::Body<'tcx>],
    (cloned, keeps_capacity): (&crate::lower::analysis::Cloned<'tcx>, bool),
    std_iterators: bool,
) {
    let tcx = recognition.tcx;
    // Which chains are transparent, counted where decisions are asked
    // (#1673): a measurement, which decides nothing here.
    if crate::decisions::path().is_some() && !std_iterators {
        for body in bodies.iter() {
            crate::lower::iterators::count_chains(&recognition, (&body.thir, body.def_id), keeps_capacity);
        }
    }
    if std_iterators {
        for body in bodies.iter_mut() {
            body.facts.in_place_adapters = super::InPlace::decided(IndexSet::default());
        }
        return;
    }
    let hint_asked = crate::lower::analysis::asked_of_own(tcx, bodies);
    let made = |body: &crate::lower::Body<'tcx>, stepped_result: Option<ExprId>| {
        let owner = body.def_id;
        let made_in = (tcx, ty::TypingEnv::post_analysis(tcx, owner));
        let cloned_apart = |ty| crate::lower::iterators::cloned_apart(made_in, cloned, ty, owner);
        let observed = |(callee, args, index): Called<'tcx>, given: ty::Ty<'tcx>| {
            recognition.collect_hint_seen((callee, index), given)
                || keeps_capacity && recognition.grows_vec_by((callee, args, index), made_in.1)
        };
        let hint_lost = |made: ty::Ty<'tcx>| recognition.in_place_hint_lost(made);
        in_place_adapters(
            tcx,
            &body.thir,
            stepped_result,
            cloned_apart,
            (&hint_asked, &observed, &hint_lost),
        )
    };
    let mut sets: Vec<IndexSet<Span>> = bodies.iter().map(|body| made(body, None)).collect();
    // Each closure given a `flat_map` made in place.
    let mut given = IndexSet::default();
    for (body, set) in bodies.iter().zip(&sets) {
        for e in body.thir.exprs.iter() {
            if let ExprKind::Call {
                fun, ref args, fn_span, ..
            } = e.kind
                && matches!(*body.thir[fun].ty.kind(), ty::FnDef(id, _) if crate::lower::recognition::is_flat_map(tcx, id))
                && set.contains(&fn_span)
                && let Some(&f) = args.get(1)
                && let ExprKind::Closure(ref closure) = body.thir[strip(&body.thir, f)].kind
            {
                given.insert(closure.closure_id.to_def_id());
            }
        }
    }
    for (body, set) in bodies.iter().zip(sets.iter_mut()) {
        if given.contains(&body.def_id) {
            *set = made(body, Some(body.expr));
        }
    }
    for (body, set) in bodies.iter_mut().zip(sets) {
        body.facts.in_place_adapters = super::InPlace::decided(set);
    }
}

/// The adapters the held flip makes std's compiled struct, `HELD_COMPILED`'s
/// `enumerate()` and `map()` (#1681's step iii). Not `rev()`: `back_stepped`,
/// which decides the flip, reads whether a `rev()` is made in place (its
/// held-`rev()` rule marks what a held one is given made knowing its back),
/// so flipping one would change what decides it (ADR 1136, amended).
pub(in crate::lower) const HELD_FLIPPED: [&str; 2] = ["enumerate", "map"];

/// The second stage of which adapters each body makes in place
/// (`InPlace`), once the analysis knows what's stepped from its back: what
/// it takes out of the ground. A chain that starts on a variable holding an
/// iterator apart, `held`'s (`Transparency::Held`), whose value is std's
/// struct, not one made knowing its back or its length
/// (`BackStepped::made_knowing`), is std's compiled structs: its
/// `HELD_FLIPPED` adapters are taken out, std's body the meaning where the
/// hand-written form isn't proved to be it (#1681's step iii). Of each body,
/// the crate's and its initializers', the harness's too, as each is
/// lowered; none of the build of std's structs, which makes none in place.
pub(in crate::lower) fn decide_held<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    bodies: &[crate::lower::Body<'tcx>],
    (back_stepped, keeps_capacity): (&crate::lower::analysis::BackStepped<'tcx>, bool),
) {
    let tcx = recognition.tcx;
    let iterator = crate::lower::recognition::std_item(tcx, crate::lower::recognition::StdItem::Iterator);
    let flipped: Vec<DefId> = (HELD_FLIPPED.iter())
        .map(|&method| crate::lower::recognition::trait_method(tcx, iterator, method))
        .collect();
    let meaning = crate::lower::session::caches(tcx).meaning.get();
    for body in bodies {
        let held = match (meaning, body.mir.as_ref()) {
            (false, Some(mir)) => held_links(recognition, body, (&mir.body, &flipped), (back_stepped, keeps_capacity)),
            _ => IndexSet::default(),
        };
        body.facts.in_place_adapters.decide_held(held);
    }
}

/// The `fn_span` of each `flipped` link of `body`'s chains that start on a
/// variable holding std's struct apart (`decide_held`), from the source
/// outward while each is one: what a compiled adapter is given is std's
/// struct, the variable's or another flipped one's, never a hand-written
/// form, as ADR 0871 says of what's given to a compiled body. Past a link
/// that isn't flipped, `src.chain(v).map(f)`'s `chain`, a `$lazyChain` in
/// place, none is: the compiled `Map` stepped it as std's `Chain` struct,
/// which it isn't, and folded nothing (#1780).
fn held_links<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    body: &crate::lower::Body<'tcx>,
    (mir, flipped): (&rustc_middle::mir::Body<'tcx>, &[DefId]),
    (back_stepped, keeps_capacity): (&crate::lower::analysis::BackStepped<'tcx>, bool),
) -> IndexSet<Span> {
    let thir = &body.thir;
    let held_apart = |source: ExprId| {
        matches!(thir[strip(thir, source)].kind, ExprKind::VarRef { .. })
            && !back_stepped.made_knowing(*recognition, (body.def_id, mir), thir[source].ty)
    };
    let flipped_span = |link: ExprId| match thir[link].kind {
        ExprKind::Call { fun, fn_span, .. } if matches!(*thir[fun].ty.kind(), ty::FnDef(callee, _) if flipped.contains(&callee)) => {
            Some(fn_span)
        }
        _ => None,
    };
    let chains = crate::lower::iterators::chains(recognition, (thir, body.def_id), keeps_capacity);
    (chains.into_iter())
        .filter(|chain| chain.answer == crate::lower::iterators::Transparency::Held)
        .filter(|chain| chain.source.is_some_and(held_apart))
        // Its links are from what's called last to what's called on the
        // source.
        .flat_map(|chain| chain.links.into_iter().rev().map_while(flipped_span))
        .collect()
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
    (hint_asked, observed, hint_lost): (
        &IndexSet<DefId>,
        &impl Fn(Called<'tcx>, ty::Ty<'tcx>) -> bool,
        &impl Fn(ty::Ty<'tcx>) -> bool,
    ),
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
    let into_iter = crate::lower::recognition::trait_method(tcx, iterator_traits[1], "into_iter");
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
                && crate::lower::recognition::makes_held_compiled(tcx, maker, thir[arg].ty)
                && !held_to_clone
                && (!crate::lower::recognition::folds_apart(tcx, thir[arg].ty) || from_its_back(callee))
            {
                given.push((fn_span, at, (callee, thir[e].ty), index, (maker, thir[arg].ty)));
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
    let sized = |maker: DefId, made: ty::Ty<'tcx>| crate::lower::recognition::sized_in_place(tcx, maker, made);
    let flattens = |maker: DefId| {
        let made = tcx.fn_sig(maker).instantiate_identity().skip_binder().output();
        crate::lower::recognition::flattened_source(tcx, made).is_some()
    };
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
            } => match (&pattern.kind, &thir[strip(thir, init)].kind, thir[strip(thir, init)].ty) {
                (
                    &PatKind::Binding {
                        var,
                        mode: BindingMode(ByRef::No, _),
                        subpattern: None,
                        ..
                    },
                    &ExprKind::Call { fun, fn_span, .. },
                    made,
                ) if matches!(*thir[fun].ty.kind(), ty::FnDef(id, _) if crate::lower::recognition::makes_held_compiled(tcx, id, made)) => {
                    Some((var, fn_span))
                }
                _ => None,
            },
            _ => None,
        })
        .collect();
    // A variable that holds a flatten, or an adapter of one, `rev()`'s,
    // given where its hint is asked, `let mut it = v.iter().flat_map(f);
    // it.next(); it.collect()` of a crate that keeps capacities
    // (`observed`): the variable isn't made in place, so neither is the
    // flatten it holds, std's struct, as one given there at once is
    // (`hint_seen`, #1528).
    let holds_flatten = |held: ty::Ty<'tcx>| {
        (held.peel_refs().walk().filter_map(|part| part.as_type()))
            .any(|part| crate::lower::recognition::flattened_source(tcx, part).is_some())
    };
    // What's given where its hint is asked: to `size_hint()`, to a function
    // of the crate's that asks the hint of what it's given (`asked_of_own`,
    // the hint's owner's), or to a library's, whose body, which the crate
    // can't see, may ask it, as `hint_asked.rs` takes it; to a collect or
    // an extend, where the program sees what it asks (`collect_hint_seen`,
    // hint_asks.rs', #1327); or
    // as an iterator an iterator's method whose own call is holds, which
    // gives its hint of it: its receiver, `flat_map(f).filter(g)`'s, or
    // another it's given, a `chain`'s or a `zip`'s, whose hint sums or takes
    // the fewer of both (#1528): to the fixed point (#1323), each call's
    // `fn_span`, and each variable `hint_seen`.
    let library = |callee: DefId| !callee.is_local() && !crate::lower::recognition::is_std_crate(tcx, callee);
    // The type parameter `callee` is given what's at `index` for, `zip`'s
    // `U`, whatever it's bound by.
    let param_at = |callee: DefId, index: usize| {
        let inputs = tcx.fn_sig(callee).instantiate_identity().skip_binder().inputs();
        match inputs.get(index).map(|input| *input.peel_refs().kind()) {
            Some(ty::Param(param)) => Some(param),
            _ => None,
        }
    };
    let mut wanted: IndexSet<Span> = IndexSet::default();
    let mut hint_seen: IndexSet<LocalVarId> = IndexSet::default();
    loop {
        let before = wanted.len() + hint_seen.len();
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
            let Some((callee, generic_args)) = crate::lower::fn_def(thir[fun].ty) else {
                continue;
            };
            let inputs = tcx.fn_sig(callee).instantiate_identity().skip_binder().inputs();
            let adapts = callee != size_hint && tcx.trait_of_assoc(callee) == Some(iterator_traits[0]);
            for (index, &arg) in args.iter().enumerate() {
                let arg = match thir[strip(thir, arg)].kind {
                    ExprKind::Borrow { arg, .. } => strip(thir, arg),
                    _ => strip(thir, arg),
                };
                let held_iterator = index == 0
                    || param_at(callee, index)
                        .is_some_and(|param| crate::lower::iterators::bound_as_iterator(tcx, callee, param));
                let asks = match inputs.get(index) {
                    _ if observed((callee, generic_args, index), thir[arg].ty) => true,
                    _ if adapts && held_iterator => wanted.contains(&at),
                    Some(_) => callee == size_hint || hint_asked.contains(&callee) || library(callee),
                    None => false,
                };
                match thir[arg].kind {
                    ExprKind::Call { fn_span: given, .. } if asks => _ = wanted.insert(given),
                    ExprKind::VarRef { id } if asks && made.contains_key(&id) && holds_flatten(thir[arg].ty) => {
                        _ = hint_seen.insert(id)
                    }
                    _ => {}
                }
            }
        }
        if wanted.len() + hint_seen.len() == before {
            break;
        }
    }
    let steps_as_js = |(callee, called): (DefId, ty::Ty<'tcx>),
                       index: usize,
                       in_place: &IndexSet<Span>,
                       at: Span,
                       (made, maker, made_ty): (Span, DefId, ty::Ty<'tcx>)| {
        // A flatten whose hint is asked where it's given (`wanted`): held,
        // std's struct, which knows its items' constant size, as of
        // `size_hint()` itself; JS's own `flatMap` knows none (ADR 0899,
        // #1323).
        if flattens(maker) && wanted.contains(&made) {
            return false;
        }
        // One a `cycle()` holds, whose JS made in place loses std's hint, a
        // `map` of a string's `chars()`: held, std's struct, whose hint asks
        // its source's, as the cycle's own asks it, wherever its hint is
        // asked, a `String`'s collect's reserve, a crate's iterator that
        // holds the cycle (`in_place_hint_lost`, #1367).
        if callee == cycle && hint_lost(made_ty) {
            return false;
        }
        // Another of them, itself made in place, `enumerate().rev()`'s: in
        // place too.
        if crate::lower::recognition::makes_held_compiled(tcx, callee, called) {
            return in_place.contains(&at);
        }
        if callee == size_hint && !sized(maker, made_ty) {
            return false;
        }
        // `IntoIterator::into_iter` of an iterator gives it itself on,
        // unstepped, `iproduct!`'s to `cartesian_product`, which may hold
        // and clone it: in place only where a `for` loop's steps it there
        // (#1409).
        if callee == into_iter && !at.is_desugaring(rustc_span::DesugaringKind::ForLoop) {
            return false;
        }
        let std = crate::lower::recognition::is_std_crate(tcx, callee)
            && !crate::lower::recognition::is_compiled_std(tcx, callee);
        // What it's given for, a type parameter bound by `Iterator` or
        // `IntoIterator`, which steps it, `filter`'s `Self`, `zip`'s `U`,
        // `next_back()`'s and `len()`'s too: not one that holds it,
        // `mem::replace`'s `T`, `Vec::push`'s, `clone()`'s or `fmt`'s
        // `Self`, its own compiled impl's, of its struct (#537).
        let Some(param) = param_at(callee, index) else {
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
        && matches!(*thir[fun].ty.kind(), ty::FnDef(id, _) if crate::lower::recognition::makes_held_compiled(tcx, id, thir[result].ty))
    {
        found.insert(fn_span);
    }

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
                _ if crate::lower::recognition::makes_held_compiled(tcx, callee, thir[e].ty) => makers.push((arg, at)),
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
        for &(made, at, callee, index, (maker, made_ty)) in &given {
            if steps_as_js(callee, index, &found, at, (made, maker, made_ty)) {
                found.insert(made);
            }
        }
        for (&var, &at) in &made {
            let uses = stepped.get(&var).map_or(&[][..], Vec::as_slice);
            let of = made_of.get(&var).map_or(&[][..], Vec::as_slice);
            let all = uses.len() + of.len();
            if !found.contains(&at)
                && !hint_seen.contains(&var)
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
