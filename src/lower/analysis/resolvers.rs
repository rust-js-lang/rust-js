//! A promise's resolve function, which JS gives the executor of a binding
//! that makes the promise, `promise::new(move |resolve, reject| ..)`'s
//! `resolve` (rd654; ADR 0898, amended).
//!
//! JS calls the executor at once with the promise's resolving functions
//! (ECMA-262's `Promise` constructor, CreateResolvingFunctions). Its
//! resolve, given what's no object, fulfils the promise (FulfillPromise),
//! which only queues the promise's reactions as jobs
//! (TriggerPromiseReactions): it runs no code. Given an object, it reads
//! its `then` first, which a getter or a proxy's trap of runs there and
//! then, and a `then` it calls only in a job of its own, later. So given
//! only what runs nothing as JS reads it, it runs nothing, as a binding
//! that runs only what it's given does (`cell_borrows`'s
//! `resolve_runs_nothing`).

use rustc_hir as hir;
use rustc_middle::thir::{ExprId, ExprKind, Thir};
use rustc_middle::ty::{self, TyCtxt};

/// Whether `callee`, in `thir`, is a promise's resolve function: the first
/// parameter, bound by value and never `mut`, so never another value, of a
/// closure written in place as the first argument of a binding that calls
/// it with a new promise's resolving functions,
/// `#[rust_js::calls_with_resolvers]`'s (`effects::calls_with_resolvers`),
/// which nothing else can call; read where it's bound or where a closure
/// it's captured by reads it, through borrows of it.
pub(in crate::lower) fn is_resolve(tcx: TyCtxt<'_>, thir: &Thir<'_>, mut callee: ExprId) -> bool {
    let var = loop {
        match thir[crate::lower::body_queries::strip(thir, callee)].kind {
            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => callee = arg,
            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => break id.0,
            _ => return false,
        }
    };
    let hir::Node::Pat(pat) = tcx.hir_node(var) else {
        return false;
    };
    let hir::PatKind::Binding(hir::BindingMode(hir::ByRef::No, hir::Mutability::Not), _, _, None) = pat.kind else {
        return false;
    };
    let hir::Node::Param(param) = tcx.parent_hir_node(var) else {
        return false;
    };
    let hir::Node::Expr(executor) = tcx.parent_hir_node(param.hir_id) else {
        return false;
    };
    let hir::ExprKind::Closure(closure) = executor.kind else {
        return false;
    };
    if tcx.hir_body(closure.body).params.first().map(|first| first.hir_id) != Some(param.hir_id) {
        return false;
    }
    let hir::Node::Expr(call) = tcx.parent_hir_node(executor.hir_id) else {
        return false;
    };
    let hir::ExprKind::Call(fun, [given, ..]) = call.kind else {
        return false;
    };
    let typeck = tcx.typeck(fun.hir_id.owner.def_id);
    given.hir_id == executor.hir_id
        && matches!(*typeck.node_type(fun.hir_id).kind(),
            ty::FnDef(id, _) if crate::lower::bindings::calls_with_resolvers(tcx, id))
}
