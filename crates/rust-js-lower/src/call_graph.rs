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

use crate::Body;
use rust_js_program::collections::{IndexMap, IndexSet};
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
pub struct Crate<'a, 'tcx> {
    pub(crate) bodies: IndexMap<DefId, &'a mir::Body<'tcx>>,
    pub(crate) graph: CallGraph,
    names: Names<'tcx>,
}

/// Each body's names, of every body the analyses read (`Names::of`): made
/// with what the analysis stands on, before the bodies are told which
/// adapters each makes in place, which changes nothing a body names, and
/// before the instances' callers are asked; then the call graph's.
pub(crate) struct Names<'tcx>(TyCtxt<'tcx>, IndexMap<DefId, Vec<Named<'tcx>>>);

impl<'tcx> Names<'tcx> {
    /// The names of each of `all_bodies`, each once. A library's closure
    /// may be among them twice, made by an instance and by what rustc
    /// inlined into one (#1029), its one MIR each time: whose names are
    /// alike, or it's a bug of rust-js's.
    pub(crate) fn of(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> Self {
        Names(tcx, IndexMap::default()).and(all_bodies.iter().copied())
    }

    /// And the names of each of `more`, the statics' and constants'
    /// initializers, which the analysis reads beside the bodies (`of`).
    pub(crate) fn and<'b>(mut self, more: impl Iterator<Item = &'b Body<'tcx>>) -> Self
    where
        'tcx: 'b,
    {
        for body in more {
            let named = named_by(self.0, body);
            match self.1.entry(body.def_id) {
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
    pub(crate) fn names_of(&self, id: DefId) -> &[Named<'tcx>] {
        &self.1[&id]
    }

    /// Each body and what it names (`only_called`'s question, ADR 1148).
    pub(crate) fn iter(&self) -> impl Iterator<Item = (DefId, &[Named<'tcx>])> + '_ {
        self.1.iter().map(|(&id, named)| (id, named.as_slice()))
    }
}

/// A function a body names, called or as a value: it, its generic
/// arguments there, where, and how.
pub(crate) type Named<'tcx> = (DefId, ty::GenericArgsRef<'tcx>, Span, Naming);

/// How a body names a function: as what a call calls, `f(x)`, `x.f()`,
/// whose arguments are what's given there; as a value, `let g = f;`, a
/// function pointer or a callback, which anything may call with anything;
/// or as what a `function::bind` binds each parameter of (`bound_all`),
/// whose every call gives it what's bound (rd632, #1675).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Naming {
    Called,
    Value,
    Bound,
}

impl<'tcx> Crate<'_, 'tcx> {
    /// Each function the body of `id`, one the analyses read, names, called
    /// or as a value (`Names::names_of`).
    pub(crate) fn names_of(&self, id: DefId) -> &[Named<'tcx>] {
        self.names.names_of(id)
    }

    /// Each body's names.
    pub(crate) fn names(&self) -> &Names<'tcx> {
        &self.names
    }
}

/// The crate's one call graph (`Crate`), of the MIR the emitter reads of
/// `all_bodies`' own functions and of the crate's constants', and `names`,
/// those of each of `all_bodies`.
pub(crate) fn crate_graph<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&'a Body<'tcx>],
    names: Names<'tcx>,
) -> Crate<'a, 'tcx> {
    // That each crate impl std's own code reaches with an iterator is one
    // the callers' carrier is told of, in the build every test runs (#2376).
    #[cfg(debug_assertions)]
    crate::instances::check_std_reach(tcx, &names, all_bodies);
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
fn named_by<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> Vec<Named<'tcx>> {
    use rustc_middle::thir::ExprKind;
    if body.root().is_some() {
        let thir = &body.thir;
        let unscoped = |mut at: rustc_middle::thir::ExprId| {
            while let ExprKind::Scope { value, .. } = thir[at].kind {
                at = value;
            }
            at
        };
        // What each call calls, beneath its scopes; and what a
        // `function::bind` of each parameter binds (`bound_all`).
        let mut callees: IndexSet<rustc_middle::thir::ExprId> = IndexSet::default();
        let mut bound: IndexSet<rustc_middle::thir::ExprId> = IndexSet::default();
        for expr in thir.exprs.iter() {
            if let ExprKind::Call { fun, ref args, .. } = expr.kind {
                let fun = unscoped(fun);
                callees.insert(fun);
                if let Some((bind, generic)) = crate::fn_def(thir[fun].ty)
                    && bound_all(tcx, bind, generic).is_some()
                    && let Some(&first) = args.first()
                {
                    bound.insert(unscoped(first));
                }
            }
        }
        return (thir.exprs.iter_enumerated())
            .filter_map(|(at, expr)| match (&expr.kind, crate::fn_def(expr.ty)) {
                (ExprKind::ZstLiteral { .. }, Some((callee, args))) => {
                    let naming = match (callees.contains(&at), bound.contains(&at)) {
                        (true, _) => Naming::Called,
                        (false, true) => Naming::Bound,
                        (false, false) => Naming::Value,
                    };
                    Some((callee, args, expr.span, naming))
                }
                _ => None,
            })
            .collect();
    }
    struct Called<'tcx>(TyCtxt<'tcx>, Vec<Named<'tcx>>);
    impl<'tcx> mir::visit::Visitor<'tcx> for Called<'tcx> {
        // What a call calls, a function's constant: called; what it's given,
        // as any operand.
        fn visit_terminator(&mut self, terminator: &mir::Terminator<'tcx>, location: mir::Location) {
            match &terminator.kind {
                TerminatorKind::Call { func, args, .. } | TerminatorKind::TailCall { func, args, .. } => {
                    let called = func.constant().and_then(|constant| {
                        crate::fn_def(constant.ty()).map(|(callee, generic)| (callee, generic, constant.span))
                    });
                    match called {
                        Some((callee, generic, span)) => self.1.push((callee, generic, span, Naming::Called)),
                        None => self.visit_operand(func, location),
                    }
                    let mut args = args.iter();
                    // What a `function::bind` of each parameter binds.
                    if let Some((bind, generic, _)) = called
                        && bound_all(self.0, bind, generic).is_some()
                        && let Some(first) = args.next()
                    {
                        match first.node.constant().and_then(|constant| crate::fn_def(constant.ty())) {
                            Some((f, of)) => self.1.push((f, of, first.span, Naming::Bound)),
                            None => self.visit_operand(&first.node, location),
                        }
                    }
                    for arg in args {
                        self.visit_operand(&arg.node, location);
                    }
                }
                _ => self.super_terminator(terminator, location),
            }
        }
        // And each one a closure it names as a constant holds, which optimized
        // MIR makes of one that captures only functions, std's inlined
        // `filter_map_fold` of `into_item` and `last`'s `some` (#645).
        // A reference to one too, a promoted `&FnDef` (#1210).
        fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, _: mir::Location) {
            if let Some((callee, args)) = crate::fn_def(constant.ty().peel_refs()) {
                self.1.push((callee, args, constant.span, Naming::Value));
            } else if constant.ty().is_closure() {
                for part in constant.ty().walk() {
                    if let Some(ty) = part.as_type()
                        && let Some((callee, args)) = crate::fn_def(ty)
                    {
                        self.1.push((callee, args, constant.span, Naming::Value));
                    }
                }
            }
        }
    }
    let mut called = Called(tcx, Vec::new());
    if let Some(mir) = &body.mir {
        mir::visit::Visitor::visit_body(&mut called, &mir.body);
    }
    called.1
}

/// The function a `function::bind`, `bind`, of `generic_args` binds each
/// parameter of, its tuple as many as it has: `bind(complete_root, None,
/// (root, lanes))` of `complete_root(root, lanes)` (rd632).
pub fn bound_all<'tcx>(tcx: TyCtxt<'tcx>, bind: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> Option<DefId> {
    if !crate::recognition::is_function_bind(tcx, bind) {
        return None;
    }
    let mut types = generic_args.types();
    let ty::FnDef(id, _) = *types.next()?.kind() else {
        return None;
    };
    let ty::Tuple(items) = types.nth(1)?.kind() else {
        return None;
    };
    (items.len() == tcx.fn_sig(id).skip_binder().skip_binder().inputs().len()).then_some(id)
}

/// What a call calls: a function, as rustc resolves it, an impl's method
/// of a trait's; or anything, of a call through a `dyn` or a function
/// pointer.
#[derive(Clone, Copy)]
pub(crate) enum Callee {
    Resolved(DefId),
    Unknown,
}

/// A call, by the block its terminator ends.
pub(crate) struct Call {
    pub(crate) block: BasicBlock,
    pub(crate) callee: Callee,
}

/// The crate's calls, by the function each is of, and its strongly
/// connected components, each callee's before its callers'.
pub(crate) struct CallGraph {
    pub(crate) calls: IndexMap<DefId, Vec<Call>>,
    pub(crate) sccs: Vec<Vec<DefId>>,
}

/// The call graph of `bodies`, each a function's MIR, the one the emitter
/// reads.
pub(crate) fn call_graph<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[(DefId, &mir::Body<'tcx>)]) -> CallGraph {
    let mut calls: IndexMap<DefId, Vec<Call>> = IndexMap::default();
    for &(caller, body) in bodies {
        let typing_env = ty::TypingEnv::post_analysis(tcx, caller);
        let found = (body.basic_blocks.iter_enumerated()).filter_map(|(block, data)| {
            let TerminatorKind::Call { func, .. } = &data.terminator().kind else {
                return None;
            };
            let callee = match func.const_fn_def() {
                Some((id, args)) => match crate::traits::resolve(tcx, typing_env, id, args) {
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
