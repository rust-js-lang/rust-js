//! The thread-locals that are their module's `let` or `const` (ADR 0270),
//! the cells that are their function's `let` (ADR 0287), and the `&Cell`s
//! a `let` takes apart that are their value (ADR 0293).

use super::super::Body;
use super::super::body_queries::{lent, strip};
use super::super::fn_def;
use super::super::recognition::{StdItem, is_cell_get, is_local_borrow, is_std_def, local_key_access};
use rustc_ast::Mutability;
use rustc_middle::thir::visit::{self, Visitor};
use rustc_middle::thir::{Expr, ExprId, ExprKind, LocalVarId, PatKind, StmtKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::LocalDefId;
use std::collections::{HashMap, HashSet};

/// The thread-locals only read and set, by `get`, `set`, `with_borrow` and
/// `with_borrow_mut`, and not public: nothing shares their cell, so each
/// is its module's variable, without a `{ value }`. A `with_borrow`'s
/// closure must be `quiet`, asking nothing of borrows, as nothing counts
/// them (ADR 0328).
pub(super) fn plain_thread_locals<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[&Body<'tcx>],
    keys: impl Iterator<Item = LocalDefId>,
    quiet: &dyn Fn(LocalDefId) -> bool,
) -> HashSet<LocalDefId> {
    let mut plain: HashSet<LocalDefId> = keys.filter(|&key| !tcx.visibility(key).is_public()).collect();
    for body in bodies {
        let thir = &body.thir;
        // Each thread-local read or set where it's read, `KEY` of `KEY.get()`.
        let mut accessed = HashSet::new();
        for expr in thir.exprs.iter() {
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && let Some((id, _)) = fn_def(thir[fun].ty)
                && local_key_access(tcx, id).is_some()
                && let Some(&first) = args.first()
                && let ExprKind::Borrow { arg, .. } = thir[strip(thir, first)].kind
                && (!is_local_borrow(tcx, id)
                    || matches!(thir[strip(thir, args[1])].kind, ExprKind::Closure(ref c) if quiet(c.closure_id)))
            {
                accessed.insert(strip(thir, arg));
            }
        }
        for (id, expr) in thir.exprs.iter_enumerated() {
            if let ExprKind::NamedConst { def_id, .. } = expr.kind
                && let Some(key) = def_id.as_local()
                && !accessed.contains(&id)
            {
                plain.remove(&key);
            }
        }
    }
    plain
}

/// The `&Cell`s of fields a `let` binds whose every `get()` is in the
/// rest of its block. What the `let` binds can be the value, as JS's
/// destructuring has it (ADR 0293).
pub(super) fn read_at_once<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> HashSet<LocalVarId> {
    let mut found = HashSet::new();
    for body in bodies {
        read_at_once_in(tcx, &body.thir, &mut found);
    }
    found
}

fn read_at_once_in<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, found: &mut HashSet<LocalVarId>) {
    let cell = |ty: Ty<'tcx>| {
        matches!(ty.kind(), ty::Ref(_, inner, Mutability::Not)
            if matches!(inner.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Cell)))
    };
    let mut gets: HashMap<LocalVarId, Vec<ExprId>> = HashMap::new();
    for (id, expr) in thir.exprs.iter_enumerated() {
        if let ExprKind::Call { fun, ref args, .. } = expr.kind
            && fn_def(thir[strip(thir, fun)].ty).is_some_and(|(def_id, _)| is_cell_get(tcx, def_id))
            && let ExprKind::VarRef { id: var } = thir[lent(thir, args[0])].kind
        {
            gets.entry(var).or_default().push(id);
        }
    }
    struct Runs<'a, 'tcx> {
        thir: &'a Thir<'tcx>,
        waiting: HashSet<*const Expr<'tcx>>,
    }
    impl<'a, 'tcx> Visitor<'a, 'tcx> for Runs<'a, 'tcx> {
        fn thir(&self) -> &'a Thir<'tcx> {
            self.thir
        }
        fn visit_expr(&mut self, expr: &'a Expr<'tcx>) {
            if self.waiting.is_empty() {
                return;
            }
            if self.waiting.remove(&(expr as *const _)) {
                return;
            }
            visit::walk_expr(self, expr);
        }
    }
    for block in thir.blocks.iter() {
        for (at, &stmt) in block.stmts.iter().enumerate() {
            let StmtKind::Let {
                ref pattern,
                initializer: Some(_),
                else_block: None,
                ..
            } = thir[stmt].kind
            else {
                continue;
            };
            let mut bound = Vec::new();
            pattern.walk_always(|p| {
                if let PatKind::Binding { var, ty, .. } = p.kind
                    && cell(ty)
                {
                    bound.push(var);
                }
            });
            for var in bound {
                let Some(reads) = gets.get(&var) else { continue };
                let mut runs = Runs {
                    thir,
                    waiting: reads.iter().map(|&get| &thir[get] as *const _).collect(),
                };
                for &later in &block.stmts[at + 1..] {
                    runs.visit_stmt(&thir[later]);
                }
                if let Some(tail) = block.expr {
                    runs.visit_expr(&thir[tail]);
                }
                if runs.waiting.is_empty() {
                    found.insert(var);
                }
            }
        }
    }
}
