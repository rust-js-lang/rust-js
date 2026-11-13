//! What each function only calls reach is given at its calls, by the
//! call graph's components, callers first (#1672 M2b-2, ADR 1137).

use super::*;

/// What each function only calls reach is given at its calls, each of its
/// parameters' interval joined over every call (#1672 M2b-2, ADR 1137):
/// callers walked before callees, by the call graph's components, so each
/// call's arguments are known as its caller's own walk knows them. None
/// where a parameter may be anything: a function that may call itself, or
/// one whose calls aren't each a call this MIR has, `only_called`'s count.
pub(super) fn parameter_intervals<'tcx>(
    tcx: TyCtxt<'tcx>,
    graph: &crate::lower::call_graph::CallGraph,
    bodies: &IndexMap<rustc_span::def_id::DefId, &Body<'tcx>>,
    only_called: &IndexMap<rustc_span::def_id::DefId, Vec<rustc_hir::HirId>>,
) -> IndexMap<rustc_span::def_id::DefId, Vec<Option<Interval>>> {
    use crate::lower::call_graph::Callee;
    let mut given: IndexMap<rustc_span::def_id::DefId, Vec<Option<Interval>>> = IndexMap::default();
    let mut calls_seen: IndexMap<rustc_span::def_id::DefId, usize> = IndexMap::default();
    for component in graph.sccs.iter().rev() {
        for &function in component {
            let Some(&body) = bodies.get(&function) else { continue };
            let own = given_of(function, only_called, &given, &calls_seen);
            let typing_env = ty::TypingEnv::post_analysis(tcx, function);
            let walk = walk(tcx, typing_env, body, own);
            let ends = walk.ends();
            for call in graph.calls.get(&function).into_iter().flatten() {
                let Callee::Resolved(callee) = call.callee else {
                    continue;
                };
                if !only_called.contains_key(&callee) {
                    continue;
                }
                let (Some(state), TerminatorKind::Call { args, .. }) =
                    (ends.get(&call.block), &body.basic_blocks[call.block].terminator().kind)
                else {
                    continue;
                };
                *calls_seen.entry(callee).or_insert(0) += 1;
                let passed: Vec<Option<Interval>> = args.iter().map(|arg| walk.operand(&arg.node, state)).collect();
                match given.get_mut(&callee) {
                    Some(known) => {
                        for (known, passed) in known.iter_mut().zip(passed) {
                            *known = known.zip(passed).map(|(a, b)| a.join(b));
                        }
                    }
                    None => {
                        given.insert(callee, passed);
                    }
                }
            }
        }
    }
    (only_called.keys())
        .map(|&function| (function, given_of(function, only_called, &given, &calls_seen)))
        .filter(|(_, intervals)| intervals.iter().any(Option::is_some))
        .collect()
}

/// What `function`'s parameters are given, as far as its calls are seen:
/// nothing known of one some of whose calls aren't walked yet, its own of
/// itself as it's walked, or are calls this MIR doesn't have. So one that
/// calls itself is walked knowing nothing of its parameters, and what it
/// passes itself joins what it's given.
fn given_of(
    function: rustc_span::def_id::DefId,
    only_called: &IndexMap<rustc_span::def_id::DefId, Vec<rustc_hir::HirId>>,
    given: &IndexMap<rustc_span::def_id::DefId, Vec<Option<Interval>>>,
    calls_seen: &IndexMap<rustc_span::def_id::DefId, usize>,
) -> Vec<Option<Interval>> {
    let every_call_seen = only_called
        .get(&function)
        .is_some_and(|calls| calls_seen.get(&function).copied().unwrap_or(0) >= calls.len());
    match every_call_seen {
        true => given.get(&function).cloned().unwrap_or_default(),
        false => Vec::new(),
    }
}
