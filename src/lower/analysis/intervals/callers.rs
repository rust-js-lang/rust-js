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
) -> IndexMap<rustc_span::def_id::DefId, Given> {
    use crate::lower::call_graph::Callee;
    let mut given: IndexMap<rustc_span::def_id::DefId, Given> = IndexMap::default();
    let mut calls_seen: IndexMap<rustc_span::def_id::DefId, usize> = IndexMap::default();
    for component in graph.sccs.iter().rev() {
        for &function in component {
            let Some(&body) = bodies.get(&function) else { continue };
            // Its calls of a function only calls reach, by their blocks: a
            // body with none isn't walked, and of one that has, what's known
            // is kept at those blocks alone, serde's derived bodies of
            // thousands of blocks walked and kept whole taking 6 GB (#1672).
            let reaching: IndexSet<BasicBlock> = (graph.calls.get(&function).into_iter().flatten())
                .filter(|call| matches!(call.callee, Callee::Resolved(callee) if only_called.contains_key(&callee)))
                .map(|call| call.block)
                .collect();
            if reaching.is_empty() {
                continue;
            }
            let own = given_of(function, only_called, &given, &calls_seen);
            let typing_env = ty::TypingEnv::post_analysis(tcx, function);
            let walk = walk(tcx, typing_env, body, own);
            let ends = walk.ends(&reaching);
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
                // Each argument's interval, and each argument below another
                // slice argument's length.
                let roots: Vec<Option<Local>> = (args.iter())
                    .map(|arg| arg.node.place().and_then(|p| p.as_local()).map(|l| state.root(l)))
                    .collect();
                let mut below: Vec<(usize, usize, bool)> = Vec::new();
                for (index, x) in roots.iter().enumerate() {
                    for (of, slice) in roots.iter().enumerate() {
                        let (Some(x), Some(slice)) = (*x, *slice) else { continue };
                        let strict = match (
                            state.below.contains(&(x, Term::Len(slice), true)),
                            state.below.contains(&(x, Term::Len(slice), false)),
                        ) {
                            (true, _) => true,
                            (false, true) => false,
                            _ => continue,
                        };
                        below.push((index, of, strict));
                    }
                }
                let passed = Given {
                    intervals: args.iter().map(|arg| walk.operand(&arg.node, state)).collect(),
                    below,
                };
                match given.get_mut(&callee) {
                    Some(known) => {
                        for (known, passed) in known.intervals.iter_mut().zip(passed.intervals) {
                            *known = known.zip(passed).map(|(a, b)| a.join(b));
                        }
                        known.below = (known.below.iter())
                            .filter_map(|&(index, of, strict)| {
                                let there = passed.below.iter().find(|&&(i, o, _)| (i, o) == (index, of))?.2;
                                Some((index, of, strict && there))
                            })
                            .collect();
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
        .filter(|(_, given)| given.intervals.iter().any(Option::is_some) || !given.below.is_empty())
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
    given: &IndexMap<rustc_span::def_id::DefId, Given>,
    calls_seen: &IndexMap<rustc_span::def_id::DefId, usize>,
) -> Given {
    let every_call_seen = only_called
        .get(&function)
        .is_some_and(|calls| calls_seen.get(&function).copied().unwrap_or(0) >= calls.len());
    match every_call_seen {
        true => given.get(&function).cloned().unwrap_or_default(),
        false => Given::default(),
    }
}
