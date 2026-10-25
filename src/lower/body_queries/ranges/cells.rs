//! A `Vec` a `RefCell` holds, by the cell (ADR 0881): its length, known
//! where its test reads it, `i < queue.borrow().len()`, as a variable's is,
//! until anything may change it. Another reference to the cell may, so
//! what's known of it goes at any call but those that only read it or
//! view it, `borrow()`, a guard's `deref`, `len()` and an index, and
//! where a value whose drop may run code is dropped: an assignment's old
//! value, or a scope's temporary or binding. One a guard on it holds is
//! the cell's no other code can borrow meanwhile.

use super::empty::drop_runs_code;
use super::known::{Below, CELL, Known, Range};
use super::{Slice, Walk, root_var, type_range, var_of};
use crate::lower::body_queries::strip;
use crate::lower::fn_def;
use crate::lower::recognition::{CellView, VecView, cell_view, is_cell_guard};
use rustc_hir::{BindingMode, ByRef};
use rustc_middle::thir::LocalVarId;
use rustc_middle::thir::visit::{Visitor, walk_expr, walk_pat, walk_stmt};
use rustc_middle::thir::{Expr, ExprId, ExprKind, Pat, PatKind, Stmt, StmtKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt};
use std::collections::HashMap;

/// Whether dropping a `ty` may run code that could change a cell's value:
/// not a guard's, which gives its borrow back.
pub(super) fn drop_may_change_cells<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> bool {
    !is_cell_guard(tcx, ty) && drop_runs_code(tcx, typing_env, ty)
}

/// Of each scope of the body at `root`, whether it may drop, where it
/// ends, a value whose drop may run code: a binding of one, a statement's
/// value, or what a call, a struct or a closure makes, wherever it goes, as
/// each may be a temporary dropped there. Once for the body, each scope
/// told by what's in it.
pub(super) fn scope_drops<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    thir: &Thir<'tcx>,
    root: ExprId,
) -> HashMap<ExprId, bool> {
    struct Drops<'a, 'tcx> {
        tcx: TyCtxt<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
        thir: &'a Thir<'tcx>,
        ids: HashMap<*const Expr<'tcx>, ExprId>,
        runs: HashMap<Ty<'tcx>, bool>,
        /// The scopes the walk is in, each whether it drops code so far.
        open: Vec<bool>,
        found: HashMap<ExprId, bool>,
    }
    impl<'a, 'tcx> Drops<'a, 'tcx> {
        fn found(&mut self, ty: Ty<'tcx>) {
            let (tcx, typing_env) = (self.tcx, self.typing_env);
            let runs = *self
                .runs
                .entry(ty)
                .or_insert_with(|| drop_may_change_cells(tcx, typing_env, ty));
            if runs && let Some(top) = self.open.last_mut() {
                *top = true;
            }
        }
    }
    impl<'a, 'tcx> Visitor<'a, 'tcx> for Drops<'a, 'tcx> {
        fn thir(&self) -> &'a Thir<'tcx> {
            self.thir
        }
        fn visit_expr(&mut self, expr: &'a Expr<'tcx>) {
            if matches!(
                expr.kind,
                ExprKind::Call { .. }
                    | ExprKind::Adt(_)
                    | ExprKind::Tuple { .. }
                    | ExprKind::Array { .. }
                    | ExprKind::Repeat { .. }
                    | ExprKind::Closure(_)
            ) {
                self.found(expr.ty);
            }
            let scope = matches!(expr.kind, ExprKind::Scope { .. });
            if scope {
                self.open.push(false);
            }
            walk_expr(self, expr);
            if scope {
                let drops = self.open.pop().expect("pushed");
                if let Some(&id) = self.ids.get(&(expr as *const _)) {
                    self.found.insert(id, drops);
                }
                if drops && let Some(top) = self.open.last_mut() {
                    *top = true;
                }
            }
        }
        fn visit_stmt(&mut self, stmt: &'a Stmt<'tcx>) {
            if let StmtKind::Expr { expr, .. } = stmt.kind {
                self.found(self.thir[expr].ty);
            }
            walk_stmt(self, stmt);
        }
        fn visit_pat(&mut self, pat: &'a Pat<'tcx>) {
            if let PatKind::Binding {
                ty,
                mode: BindingMode(ByRef::No, _),
                ..
            } = pat.kind
            {
                self.found(ty);
            }
            walk_pat(self, pat);
        }
    }
    let ids = (thir.exprs.iter_enumerated())
        .filter(|(_, e)| matches!(e.kind, ExprKind::Scope { .. }))
        .map(|(id, e)| (e as *const _, id))
        .collect();
    let mut drops = Drops {
        tcx,
        typing_env,
        thir,
        ids,
        runs: HashMap::new(),
        open: Vec::new(),
        found: HashMap::new(),
    };
    drops.visit_expr(&thir[root]);
    drops.found
}

/// Whether `e`, which reads without changing anything, reads through a
/// `RefCell`, `queue.borrow().len()`, which other code may change.
pub(super) fn reads_a_cell<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, e: ExprId) -> bool {
    match thir[e].kind {
        ExprKind::Scope { value: inner, .. }
        | ExprKind::Use { source: inner }
        | ExprKind::Deref { arg: inner }
        | ExprKind::Field { lhs: inner, .. }
        | ExprKind::Unary { arg: inner, .. }
        | ExprKind::Cast { source: inner }
        | ExprKind::Borrow { arg: inner, .. } => reads_a_cell(tcx, thir, inner),
        ExprKind::Binary { lhs, rhs, .. } => reads_a_cell(tcx, thir, lhs) || reads_a_cell(tcx, thir, rhs),
        ExprKind::Call { fun, ref args, .. } => {
            fn_def(thir[strip(thir, fun)].ty).is_some_and(|(def_id, args)| cell_view(tcx, def_id, args).is_some())
                || args.iter().any(|&arg| reads_a_cell(tcx, thir, arg))
        }
        _ => false,
    }
}

impl<'a, 'tcx> Walk<'a, 'tcx> {
    /// The slice a place is, and whether it's reached through a `RefCell`
    /// or its guard, `queue.borrow()`'s, whose change no variable's says.
    pub(super) fn slice_through(&self, mut e: ExprId) -> Option<(Slice, bool)> {
        let thir = self.thir;
        let mut path = Vec::new();
        let mut celled = false;
        loop {
            e = strip(thir, e);
            match thir[e].kind {
                // What a `RefCell` holds, through its guard (ADR 0881).
                ExprKind::Call { fun, ref args, .. } if let Some(view) = self.cell_view(fun) => {
                    if matches!(view, CellView::Guard { .. }) {
                        path.push(CELL);
                    }
                    celled = true;
                    e = args[0];
                }
                ExprKind::Borrow { arg, .. } => e = arg,
                ExprKind::Deref { arg } if matches!(thir[arg].ty.kind(), ty::Ref(..)) || thir[arg].ty.is_box() => {
                    e = arg
                }
                // A `Vec`'s slice, `deref` or `deref_mut`, is as long.
                ExprKind::Call { fun, ref args, .. } if self.vec_view(fun) == Some(VecView::Slice) => e = args[0],
                ExprKind::Field { lhs, name, .. } => {
                    path.push(name.as_usize());
                    e = lhs;
                }
                ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } if !self.volatile.contains(&id) => {
                    path.reverse();
                    return Some(((path, id), celled));
                }
                _ => return None,
            }
        }
    }

    /// What a `RefCell`'s call `fun` makes of what it holds.
    pub(super) fn cell_view(&self, fun: ExprId) -> Option<CellView> {
        let (def_id, generic_args) = fn_def(self.thir[strip(self.thir, fun)].ty)?;
        cell_view(self.tcx, def_id, generic_args)
    }

    /// The variable whose value a change of `place` changes: its root, or
    /// of a slice through a `RefCell` or its guard, what holds the cell
    /// (ADR 0881).
    pub(super) fn changed_place(&self, place: ExprId) -> Option<LocalVarId> {
        root_var(self.thir, place).or_else(|| {
            (self.slice_through(place))
                .filter(|&(_, celled)| celled)
                .map(|((_, var), _)| var)
        })
    }

    /// The slice in a `RefCell` whose item or whole `place` is: of
    /// `queue.borrow_mut()[i]`, the queue's.
    pub(super) fn cell_of(&self, place: ExprId) -> Option<Slice> {
        let thir = self.thir;
        let mut e = strip(thir, place);
        if let ExprKind::Deref { arg } = thir[e].kind
            && let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, arg)].kind
            && self.vec_view(fun) == Some(VecView::Item)
        {
            e = args[0];
        }
        (self.slice_through(e))
            .filter(|((path, _), _)| path.contains(&CELL))
            .map(|(slice, _)| slice)
    }

    /// What's known once `items[index]` gave its item: a variable `index`
    /// is below the length, `items.len() - 1` at most.
    pub(super) fn indexed(&self, items: ExprId, index: Option<ExprId>, known: &mut Option<Known>) {
        let thir = self.thir;
        let (Some(slice), Some(index)) = (self.slice(items), index) else {
            return;
        };
        let (Some(var), Some(k)) = (var_of(thir, index), known.as_mut()) else {
            return;
        };
        let Some((lo, hi)) = type_range(thir[index].ty) else {
            return;
        };
        if self.volatile.contains(&var) {
            return;
        }
        let range = k.vars.entry(var).or_insert_with(|| Range::of(lo, hi));
        range.lo = range.lo.max(0);
        if range.below.as_ref().is_none_or(|(s, _)| *s == slice) {
            range.below = Some((slice, Below::LENGTH.offset(-1)));
        }
    }
}
