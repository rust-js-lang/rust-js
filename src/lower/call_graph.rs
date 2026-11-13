//! One call graph of the crate, from the MIR the emitter reads (#1671
//! change 4 M3; ADR 1137): each body's calls, resolved where rustc can,
//! and of one through a `dyn` or a function pointer, anything; and the
//! graph's strongly connected components, each callee's before its
//! callers'. A graph and a table, no query engine: each analysis that
//! walks the crate's calls asks it. Of the MIR the emitter reads, not the
//! optimized one, where a call rustc inlined is no call, though the JS
//! makes it.

use crate::collections::{IndexMap, IndexSet};
use rustc_middle::mir::{self, BasicBlock, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;

/// What a call calls: a function, as rustc resolves it, an impl's method
/// of a trait's; or anything, of a call through a `dyn` or a function
/// pointer.
#[derive(Clone, Copy)]
pub(in crate::lower) enum Callee {
    Resolved(DefId),
    Unknown,
}

/// A call, by the block its terminator ends.
pub(in crate::lower) struct Call {
    pub(in crate::lower) block: BasicBlock,
    pub(in crate::lower) callee: Callee,
}

/// The crate's calls, by the function each is of, and its strongly
/// connected components, each callee's before its callers'.
pub(in crate::lower) struct CallGraph {
    pub(in crate::lower) calls: IndexMap<DefId, Vec<Call>>,
    pub(in crate::lower) sccs: Vec<Vec<DefId>>,
}

/// The call graph of `bodies`, each a function's MIR, the one the emitter
/// reads.
pub(in crate::lower) fn call_graph<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[(DefId, &mir::Body<'tcx>)]) -> CallGraph {
    let mut calls: IndexMap<DefId, Vec<Call>> = IndexMap::default();
    for &(caller, body) in bodies {
        let typing_env = ty::TypingEnv::post_analysis(tcx, caller);
        let found = (body.basic_blocks.iter_enumerated()).filter_map(|(block, data)| {
            let TerminatorKind::Call { func, .. } = &data.terminator().kind else {
                return None;
            };
            let callee = match func.const_fn_def() {
                Some((id, args)) => match ty::Instance::try_resolve(tcx, typing_env, id, args) {
                    Ok(Some(instance)) if matches!(instance.def, ty::InstanceKind::Item(_)) => {
                        Callee::Resolved(instance.def_id())
                    }
                    Ok(Some(instance)) if matches!(instance.def, ty::InstanceKind::Virtual(..)) => Callee::Unknown,
                    _ => Callee::Resolved(id),
                },
                None => Callee::Unknown,
            };
            Some(Call { block, callee })
        });
        calls.entry(caller).or_default().extend(found);
    }
    let sccs = components(&calls);
    CallGraph { calls, sccs }
}

/// The strongly connected components of the graph `calls` makes of the
/// functions it has calls of, each callee's before its callers': Tarjan's,
/// by a stack, not recursion.
fn components(calls: &IndexMap<DefId, Vec<Call>>) -> Vec<Vec<DefId>> {
    let nodes: Vec<DefId> = calls.keys().copied().collect();
    let edges = |node: DefId| -> Vec<DefId> {
        (calls.get(&node).into_iter().flatten())
            .filter_map(|call| match call.callee {
                Callee::Resolved(callee) if calls.contains_key(&callee) => Some(callee),
                _ => None,
            })
            .collect()
    };
    let mut index: IndexMap<DefId, (usize, usize)> = IndexMap::default();
    let mut on_stack: IndexSet<DefId> = IndexSet::default();
    let mut stack: Vec<DefId> = Vec::new();
    let mut sccs: Vec<Vec<DefId>> = Vec::new();
    for &root in &nodes {
        if index.contains_key(&root) {
            continue;
        }
        // Each frame: a node, and the edges of it still to visit.
        let mut work: Vec<(DefId, Vec<DefId>)> = vec![(root, edges(root))];
        let next = index.len();
        index.insert(root, (next, next));
        stack.push(root);
        on_stack.insert(root);
        while let Some((node, pending)) = work.last_mut() {
            let node = *node;
            if let Some(to) = pending.pop() {
                if !index.contains_key(&to) {
                    let next = index.len();
                    index.insert(to, (next, next));
                    stack.push(to);
                    on_stack.insert(to);
                    work.push((to, edges(to)));
                } else if on_stack.contains(&to) {
                    let low = index[&node].1.min(index[&to].0);
                    index[&node].1 = low;
                }
                continue;
            }
            work.pop();
            if let Some((parent, _)) = work.last() {
                let low = index[parent].1.min(index[&node].1);
                index[parent].1 = low;
            }
            if index[&node].0 == index[&node].1 {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    on_stack.swap_remove(&member);
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                sccs.push(component);
            }
        }
    }
    sccs
}
