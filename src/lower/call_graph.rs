//! One call graph of the crate, from the MIR the emitter reads (#1671
//! change 4 M3; ADR 1137): each body's calls, resolved where rustc can,
//! and of one through a `dyn` or a function pointer, anything; and the
//! graph's strongly connected components, each callee's before its
//! callers'. A graph and a table, no query engine: each analysis that
//! walks the crate's calls asks it. Of the MIR the emitter reads, not the
//! optimized one, where a call rustc inlined is no call, though the JS
//! makes it.
//!
//! And beside it, each body's names (`Crate::names_of`): each function it
//! calls or names as a value, of its generic arguments there, and where, as
//! its THIR names it, or of a body of MIR alone its MIR. Two tables of one
//! owner: who calls whom at run time, resolved, and who names whom as
//! written, of what types.

use crate::collections::{IndexMap, IndexSet};
use crate::lower::Body;
use rustc_hir::def::DefKind;
use rustc_middle::mir::{self, BasicBlock, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// The crate's call graph, built once of what its analyses walk: each of
/// its functions' MIR the emitter reads, `bodies`, and each constant's, a
/// `const`'s, a `static`'s or an inline `const { .. }`'s, by the MIR rustc
/// evaluates it by, whose calls are a function's callers too (#1742): a
/// `const fn` only constants and patterns call. The bodies it's of, and it;
/// and the names of each body the analyses read.
pub(in crate::lower) struct Crate<'a, 'tcx> {
    pub(in crate::lower) bodies: IndexMap<DefId, &'a mir::Body<'tcx>>,
    pub(in crate::lower) graph: CallGraph,
    names: Names<'tcx>,
}

/// Each body's names, of every body the analyses read (`Names::of`): made
/// with what the analysis stands on, before the bodies are told which
/// adapters each makes in place, which changes nothing a body names, and
/// before the instances' callers are asked; then the call graph's.
pub(in crate::lower) struct Names<'tcx>(IndexMap<DefId, Vec<Named<'tcx>>>);

impl<'tcx> Names<'tcx> {
    /// The names of each of `all_bodies`, each once. A library's closure
    /// may be among them twice, made by an instance and by what rustc
    /// inlined into one (#1029), its one MIR each time: whose names are
    /// alike, or it's a bug of rust-js's.
    pub(in crate::lower) fn of(all_bodies: &[&Body<'tcx>]) -> Self {
        Names(IndexMap::default()).and(all_bodies.iter().copied())
    }

    /// And the names of each of `more`, the statics' and constants'
    /// initializers, which the analysis reads beside the bodies (`of`).
    pub(in crate::lower) fn and<'b>(mut self, more: impl Iterator<Item = &'b Body<'tcx>>) -> Self
    where
        'tcx: 'b,
    {
        for body in more {
            let named = named_by(body);
            match self.0.entry(body.def_id) {
                indexmap::map::Entry::Occupied(had) => {
                    assert!(
                        *had.get() == named,
                        "two bodies of one item that name apart, a bug of rust-js's"
                    );
                }
                indexmap::map::Entry::Vacant(vacant) => {
                    vacant.insert(named);
                }
            }
        }
        self
    }

    /// Each function the body of `id`, one the analyses read, names, called
    /// or as a value, as `named_by` gives them.
    pub(in crate::lower) fn names_of(&self, id: DefId) -> &[Named<'tcx>] {
        &self.0[&id]
    }
}

/// A function a body names, called or as a value: it, its generic
/// arguments there, and where.
pub(in crate::lower) type Named<'tcx> = (DefId, ty::GenericArgsRef<'tcx>, Span);

impl<'tcx> Crate<'_, 'tcx> {
    /// Each function the body of `id`, one the analyses read, names, called
    /// or as a value (`Names::names_of`).
    pub(in crate::lower) fn names_of(&self, id: DefId) -> &[Named<'tcx>] {
        self.names.names_of(id)
    }

    /// Each body's names.
    pub(in crate::lower) fn names(&self) -> &Names<'tcx> {
        &self.names
    }
}

/// The crate's one call graph (`Crate`), of the MIR the emitter reads of
/// `all_bodies`' own functions and of the crate's constants', and `names`,
/// those of each of `all_bodies`.
pub(in crate::lower) fn crate_graph<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&'a Body<'tcx>],
    names: Names<'tcx>,
) -> Crate<'a, 'tcx> {
    let mut callers: IndexMap<DefId, &'a mir::Body<'tcx>> = (all_bodies.iter())
        .filter(|body| body.def_id.is_local())
        .filter_map(|body| Some((body.def_id, &body.mir.as_ref()?.body)))
        .collect();
    for owner in tcx.hir_body_owners() {
        let owner = owner.to_def_id();
        if !callers.contains_key(&owner)
            && matches!(
                tcx.def_kind(owner),
                DefKind::Const | DefKind::AssocConst | DefKind::AnonConst | DefKind::Static { .. }
            )
        {
            callers.insert(owner, tcx.mir_for_ctfe(owner));
        }
    }
    let listed: Vec<(DefId, &mir::Body<'tcx>)> = callers.iter().map(|(&id, &body)| (id, body)).collect();
    let graph = call_graph(tcx, &listed);
    Crate {
        bodies: callers,
        graph,
        names,
    }
}

/// Each function `body` names, called or as a value, of its generic
/// arguments there, and where: as its THIR names it, or, of a body of MIR
/// alone, a library's the crate compiles (ADR 0914), its MIR. The graph's
/// scan, `Names::of`'s, its one caller.
fn named_by<'tcx>(body: &Body<'tcx>) -> Vec<Named<'tcx>> {
    use rustc_middle::thir::ExprKind;
    if body.root().is_some() {
        return (body.thir.exprs.iter())
            .filter_map(|expr| match (&expr.kind, crate::lower::fn_def(expr.ty)) {
                (ExprKind::ZstLiteral { .. }, Some((callee, args))) => Some((callee, args, expr.span)),
                _ => None,
            })
            .collect();
    }
    struct Called<'tcx>(Vec<Named<'tcx>>);
    impl<'tcx> mir::visit::Visitor<'tcx> for Called<'tcx> {
        // And each one a closure it names as a constant holds, which optimized
        // MIR makes of one that captures only functions, std's inlined
        // `filter_map_fold` of `into_item` and `last`'s `some` (#645).
        // A reference to one too, a promoted `&FnDef` (#1210).
        fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, _: mir::Location) {
            if let Some((callee, args)) = crate::lower::fn_def(constant.ty().peel_refs()) {
                self.0.push((callee, args, constant.span));
            } else if constant.ty().is_closure() {
                for part in constant.ty().walk() {
                    if let Some(ty) = part.as_type()
                        && let Some((callee, args)) = crate::lower::fn_def(ty)
                    {
                        self.0.push((callee, args, constant.span));
                    }
                }
            }
        }
    }
    let mut called = Called(Vec::new());
    if let Some(mir) = &body.mir {
        mir::visit::Visitor::visit_body(&mut called, &mir.body);
    }
    called.0
}

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
