//! What each function only calls reach is given at its calls, by the
//! call graph's components, callers first (#1672 M2b-2, ADR 1137).

use super::*;
use crate::lower::call_graph::{CallGraph, Callee};
use rustc_span::def_id::DefId;

/// How many times a component that calls itself is walked before what its
/// own calls still change of a parameter is forgotten: `walk(n + 1, k)`
/// tells `walk` a wider `n` each time, which nothing bounds, and the same
/// `k`, which is kept (ADR 0577).
const ROUNDS_BEFORE_FORGETTING: usize = 4;

/// What each function only calls reach is given at its calls, each of its
/// parameters' interval joined over every call (#1672 M2b-2, ADR 1137):
/// callers walked before callees, by the call graph's components, so each
/// call's arguments are known as its caller's own walk knows them. Of a
/// component that calls itself, its own calls are walked to a fixed point,
/// from what its callers outside give (#1742). None where a parameter may
/// be anything: one whose calls aren't each a call this MIR has,
/// `only_called`'s count.
pub(super) fn parameter_intervals<'tcx>(
    tcx: TyCtxt<'tcx>,
    graph: &CallGraph,
    bodies: &IndexMap<DefId, &Body<'tcx>>,
    only_called: &IndexMap<DefId, Vec<rustc_hir::HirId>>,
) -> IndexMap<DefId, Given> {
    let mut given: IndexMap<DefId, Given> = IndexMap::default();
    let mut calls_seen: IndexMap<DefId, usize> = IndexMap::default();
    for component in graph.sccs.iter().rev() {
        let members: IndexSet<DefId> = component.iter().copied().collect();
        let calls_itself = (members.iter()).any(|f| {
            (graph.calls.get(f).into_iter().flatten())
                .any(|call| matches!(call.callee, Callee::Resolved(c) if members.contains(&c)))
        });
        if !calls_itself {
            for &function in component {
                let own = given_of(function, only_called, &given, &calls_seen);
                for (callee, passed) in passes(tcx, graph, bodies, only_called, function, own) {
                    take(&mut given, &mut calls_seen, callee, passed);
                }
            }
            continue;
        }
        // Its calls from inside, each member's count, which with those
        // from outside seen are every call of it.
        let mut inside_calls: IndexMap<DefId, usize> = IndexMap::default();
        for f in &members {
            for call in graph.calls.get(f).into_iter().flatten() {
                if let Callee::Resolved(callee) = call.callee
                    && members.contains(&callee)
                {
                    *inside_calls.entry(callee).or_insert(0) += 1;
                }
            }
        }
        // What each member's parameters are of what's given from outside and
        // `inside`, its own calls': nothing known where some call is of
        // neither; and none yet, `None`, where no call has given it any,
        // so it's not walked until one does: of `even` and `odd` calling
        // each other, `odd` walked knowing nothing would tell `even`
        // nothing either.
        let own_of =
            |member: DefId, inside: &IndexMap<DefId, Given>, given: &IndexMap<DefId, Given>| -> Option<Given> {
                let total = only_called.get(&member).map_or(usize::MAX, Vec::len);
                let seen =
                    calls_seen.get(&member).copied().unwrap_or(0) + inside_calls.get(&member).copied().unwrap_or(0);
                if seen < total {
                    return Some(Given::default());
                }
                match (given.get(&member), inside.get(&member)) {
                    (Some(outside), Some(inner)) => Some(joined(outside, inner)),
                    (Some(one), None) | (None, Some(one)) => Some(one.clone()),
                    (None, None) => None,
                }
            };
        let mut inside: IndexMap<DefId, Given> = IndexMap::default();
        for round in 1.. {
            let mut next: IndexMap<DefId, Given> = IndexMap::default();
            for &member in component {
                let Some(own) = own_of(member, &inside, &given) else {
                    continue;
                };
                for (callee, passed) in passes(tcx, graph, bodies, only_called, member, own) {
                    if members.contains(&callee) {
                        match next.get_mut(&callee) {
                            Some(known) => *known = joined(known, &passed),
                            None => {
                                next.insert(callee, passed);
                            }
                        }
                    }
                }
            }
            // Past its rounds, what still changes is forgotten; and past 64,
            // all of it, which then settles.
            if round > ROUNDS_BEFORE_FORGETTING {
                for (callee, now) in next.iter_mut() {
                    let was = inside.get(callee);
                    for (at, interval) in now.intervals.iter_mut().enumerate() {
                        if round > 64 || was.is_none_or(|was| was.intervals.get(at) != Some(interval)) {
                            *interval = None;
                        }
                    }
                    if round > 64 || was.is_none_or(|was| was.below != now.below) {
                        now.below.clear();
                    }
                }
            }
            let settled = next.len() == inside.len()
                && next.iter().all(|(callee, now)| {
                    inside
                        .get(callee)
                        .is_some_and(|was| was.intervals == now.intervals && was.below == now.below)
                });
            inside = next;
            if settled {
                break;
            }
        }
        // Settled: each member given what its callers outside and its own
        // calls give, and each of its calls out passed on.
        let settled: Vec<(DefId, Given)> = (component.iter())
            .map(|&member| (member, own_of(member, &inside, &given).unwrap_or_default()))
            .collect();
        for (member, own) in settled {
            for (callee, passed) in passes(tcx, graph, bodies, only_called, member, own.clone()) {
                if !members.contains(&callee) {
                    take(&mut given, &mut calls_seen, callee, passed);
                }
            }
            let total = only_called.get(&member).map_or(0, Vec::len);
            calls_seen.insert(member, total);
            given.insert(member, own);
        }
    }
    (only_called.keys())
        .map(|&function| (function, given_of(function, only_called, &given, &calls_seen)))
        .filter(|(_, given)| given.intervals.iter().any(Option::is_some) || !given.below.is_empty())
        .collect()
}

/// What `function`, walked knowing `own` of its parameters, passes each
/// function only calls reach it calls: each call's arguments' intervals,
/// and each argument below another slice argument's length. Of its calls'
/// blocks alone what's known is kept, serde's derived bodies of thousands
/// of blocks walked and kept whole taking 6 GB (#1672).
fn passes<'tcx>(
    tcx: TyCtxt<'tcx>,
    graph: &CallGraph,
    bodies: &IndexMap<DefId, &Body<'tcx>>,
    only_called: &IndexMap<DefId, Vec<rustc_hir::HirId>>,
    function: DefId,
    own: Given,
) -> Vec<(DefId, Given)> {
    let Some(&body) = bodies.get(&function) else {
        return Vec::new();
    };
    let reaching: IndexSet<BasicBlock> = (graph.calls.get(&function).into_iter().flatten())
        .filter(|call| matches!(call.callee, Callee::Resolved(callee) if only_called.contains_key(&callee)))
        .map(|call| call.block)
        .collect();
    if reaching.is_empty() {
        return Vec::new();
    }
    let typing_env = ty::TypingEnv::post_analysis(tcx, function);
    let walk = walk(tcx, typing_env, body, own);
    let ends = walk.ends(&reaching);
    let mut found = Vec::new();
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
        found.push((
            callee,
            Given {
                intervals: args.iter().map(|arg| walk.operand(&arg.node, state)).collect(),
                below,
            },
        ));
    }
    found
}

/// `passed`, a call's, joined into what `callee` is given, the call seen.
fn take(given: &mut IndexMap<DefId, Given>, calls_seen: &mut IndexMap<DefId, usize>, callee: DefId, passed: Given) {
    *calls_seen.entry(callee).or_insert(0) += 1;
    match given.get_mut(&callee) {
        Some(known) => *known = joined(known, &passed),
        None => {
            given.insert(callee, passed);
        }
    }
}

/// What two calls give a function's parameters, together: each interval's
/// join, unknown where either is, and each relation both hold, strictly
/// where both are.
fn joined(a: &Given, b: &Given) -> Given {
    Given {
        intervals: (a.intervals.iter().zip(&b.intervals))
            .map(|(a, b)| a.zip(*b).map(|(a, b)| a.join(b)))
            .collect(),
        below: (a.below.iter())
            .filter_map(|&(index, of, strict)| {
                let there = b.below.iter().find(|&&(i, o, _)| (i, o) == (index, of))?.2;
                Some((index, of, strict && there))
            })
            .collect(),
    }
}

/// What `function`'s parameters are given, as far as its calls are seen:
/// nothing known of one some of whose calls aren't walked yet, or are calls
/// this MIR doesn't have.
fn given_of(
    function: DefId,
    only_called: &IndexMap<DefId, Vec<rustc_hir::HirId>>,
    given: &IndexMap<DefId, Given>,
    calls_seen: &IndexMap<DefId, usize>,
) -> Given {
    let every_call_seen = only_called
        .get(&function)
        .is_some_and(|calls| calls_seen.get(&function).copied().unwrap_or(0) >= calls.len());
    match every_call_seen {
        true => given.get(&function).cloned().unwrap_or_default(),
        false => Given::default(),
    }
}
