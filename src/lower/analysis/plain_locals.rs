//! The thread-locals that are their module's `let` or `const` (ADR 0270),
//! the cells that are their function's `let` (ADR 0287), and the `&Cell`s
//! a `let` takes apart that are their value (ADR 0293).

use super::super::Body;
use super::super::bindings::is_element_builder;
use super::super::body_queries::{lent, strip};
use super::super::fn_def;
use super::super::recognition::{CellUse, StdItem, cell_use, is_cell_get, is_std_def, local_key_access};
use rustc_ast::Mutability;
use rustc_hir::{BindingMode, ByRef};
use rustc_middle::thir::visit::{self, Visitor};
use rustc_middle::thir::{Expr, ExprId, ExprKind, LocalVarId, PatKind, StmtKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::LocalDefId;
use std::collections::{HashMap, HashSet};

/// The thread-locals only read and set, by `get`, `set`, `with_borrow` and
/// `with_borrow_mut`, in their own module, and not public: nothing shares
/// their cell, so each is its module's variable, without a `{ value }`. Each
/// says whether it's `set`, a `let`, or not, a `const`.
pub(super) fn plain_thread_locals<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[&Body<'tcx>],
    keys: impl Iterator<Item = LocalDefId>,
) -> HashMap<LocalDefId, bool> {
    let mut plain: HashMap<LocalDefId, bool> = keys
        .filter(|&key| !tcx.visibility(key).is_public())
        .map(|key| (key, false))
        .collect();
    for body in bodies {
        let thir = &body.thir;
        let module = tcx.parent_module_from_def_id(body.def_id);
        // Each thread-local read or set where it's read, `KEY` of `KEY.get()`,
        // and those set.
        let mut accessed = HashSet::new();
        let mut set = HashSet::new();
        for expr in thir.exprs.iter() {
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && let Some(sets) = fn_def(thir[fun].ty).and_then(|(id, _)| local_key_access(tcx, id))
                && let Some(&first) = args.first()
                && let ExprKind::Borrow { arg, .. } = thir[strip(thir, first)].kind
            {
                accessed.insert(strip(thir, arg));
                if sets {
                    set.insert(strip(thir, arg));
                }
            }
        }
        for (id, expr) in thir.exprs.iter_enumerated() {
            if let ExprKind::NamedConst { def_id, .. } = expr.kind
                && let Some(key) = def_id.as_local()
            {
                if !accessed.contains(&id) || tcx.parent_module_from_def_id(key) != module {
                    plain.remove(&key);
                } else if set.contains(&id)
                    && let Some(sets) = plain.get_mut(&key)
                {
                    *sets = true;
                }
            }
        }
    }
    plain
}

/// The locals that are a `Cell`, or an `Rc` of one, only their function and
/// its closures read and set, by `get` and `set`, and their clones: each its
/// function's variable, without a `{ value }`, as a person keeps `let
/// timeout` that a listener sets and a cleanup clears (ADR 0287). Each maps
/// to the one it's a clone of, or to itself. One used any other way, given
/// or returned whole, `replace`d or compared, is a cell, and so is each of
/// its clones.
pub(super) fn plain_cells<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> HashMap<LocalVarId, LocalVarId> {
    let mut made: HashSet<LocalVarId> = HashSet::new();
    let mut cloned: HashMap<LocalVarId, LocalVarId> = HashMap::new();
    let mut unplain: HashSet<LocalVarId> = HashSet::new();
    for body in bodies {
        let thir = &body.thir;
        let use_of = |e: ExprId| match thir[e].kind {
            ExprKind::Call { fun, .. } => fn_def(thir[fun].ty).and_then(|(id, args)| cell_use(tcx, id, args)),
            _ => None,
        };
        // The variable `e` reads, through `&`, `*` and an `Rc`'s deref, and
        // where: what's read of it there is all that is.
        let read = |mut e: ExprId| loop {
            e = strip(thir, e);
            match thir[e].kind {
                ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = arg,
                ExprKind::Call { ref args, .. } if use_of(e) == Some(CellUse::Deref) => e = args[0],
                ExprKind::VarRef { id } => return Some((e, id)),
                ExprKind::UpvarRef { var_hir_id, .. } => return Some((e, var_hir_id)),
                _ => return None,
            }
        };
        let mut allowed: HashSet<ExprId> = HashSet::new();
        for stmt in thir.stmts.iter() {
            let StmtKind::Let {
                ref pattern,
                initializer: Some(init),
                else_block: None,
                ..
            } = stmt.kind
            else {
                continue;
            };
            let PatKind::Binding {
                var,
                mode: BindingMode(ByRef::No, Mutability::Not),
                subpattern: None,
                ..
            } = pattern.kind
            else {
                continue;
            };
            let init = strip(thir, init);
            match (use_of(init), &thir[init].kind) {
                (Some(CellUse::New), _) => {
                    made.insert(var);
                }
                (Some(CellUse::Shared), ExprKind::Call { args, .. })
                    if args.len() == 1 && use_of(strip(thir, args[0])) == Some(CellUse::New) =>
                {
                    made.insert(var);
                }
                (Some(CellUse::Cloned), ExprKind::Call { args, .. }) => {
                    if let Some((at, of)) = args.first().and_then(|&a| read(a)) {
                        allowed.insert(at);
                        cloned.insert(var, of);
                    }
                }
                _ => {}
            }
        }
        for (id, expr) in thir.exprs.iter_enumerated() {
            match expr.kind {
                ExprKind::Call { ref args, .. } if matches!(use_of(id), Some(CellUse::Get | CellUse::Set)) => {
                    allowed.extend(args.first().and_then(|&a| read(a)).map(|(at, _)| at));
                }
                // What a closure captures, which is what it reads.
                ExprKind::Closure(ref closure) => {
                    allowed.extend(closure.upvars.iter().filter_map(|&u| read(u)).map(|(at, _)| at));
                }
                _ => {}
            }
        }
        for (id, expr) in thir.exprs.iter_enumerated() {
            let var = match expr.kind {
                ExprKind::VarRef { id } => id,
                ExprKind::UpvarRef { var_hir_id, .. } => var_hir_id,
                _ => continue,
            };
            if !allowed.contains(&id) {
                unplain.insert(var);
            }
        }
    }
    // Each clone's first, and what's unplain of one is of all of them.
    let first = |mut var: LocalVarId| {
        let mut seen = 0;
        while let Some(&of) = cloned.get(&var) {
            var = of;
            seen += 1;
            if seen > cloned.len() {
                return None;
            }
        }
        made.contains(&var).then_some(var)
    };
    let groups: Vec<(LocalVarId, LocalVarId)> = made
        .iter()
        .map(|&v| (v, v))
        .chain(cloned.keys().filter_map(|&v| Some((v, first(v)?))))
        .collect();
    let spoiled: HashSet<LocalVarId> = groups
        .iter()
        .filter(|(v, _)| unplain.contains(v))
        .map(|&(_, root)| root)
        .collect();
    groups.into_iter().filter(|(_, root)| !spoiled.contains(root)).collect()
}

/// The `&Cell`s of fields a `let` binds whose every use is a `get()` that
/// comes before anything else runs, in the order the code runs: before a
/// call, a block or a loop ends, outside a loop. Building an element, as
/// `jsx!` does first, `div().class_name(..)`, runs nothing (ADR 0040). What the `let` binds can
/// be the value, as JS's destructuring has it (ADR 0293): nothing could
/// set the cell between.
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
    let mut uses: HashMap<LocalVarId, usize> = HashMap::new();
    for (id, expr) in thir.exprs.iter_enumerated() {
        match expr.kind {
            ExprKind::VarRef { id: var } => *uses.entry(var).or_default() += 1,
            ExprKind::Call { fun, ref args, .. }
                if fn_def(thir[strip(thir, fun)].ty).is_some_and(|(def_id, _)| is_cell_get(tcx, def_id))
                    && let ExprKind::VarRef { id: var } = thir[lent(thir, args[0])].kind =>
            {
                gets.entry(var).or_default().push(id);
            }
            _ => {}
        }
    }
    struct Runs<'a, 'tcx> {
        tcx: TyCtxt<'tcx>,
        thir: &'a Thir<'tcx>,
        waiting: HashSet<*const Expr<'tcx>>,
        loops: usize,
        early: bool,
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
                self.early &= self.loops == 0;
                return;
            }
            let in_loop = matches!(expr.kind, ExprKind::Loop { .. });
            self.loops += usize::from(in_loop);
            visit::walk_expr(self, expr);
            self.loops -= usize::from(in_loop);
            // What runs code: a call, and a block, which drops its own.
            let runs = match expr.kind {
                ExprKind::Call { fun, .. } => {
                    !fn_def(self.thir[fun].ty).is_some_and(|(def, _)| is_element_builder(self.tcx, def))
                }
                ExprKind::Block { .. } => true,
                _ => false,
            };
            if runs && !self.waiting.is_empty() {
                self.early = false;
            }
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
                if uses.get(&var) != Some(&reads.len()) {
                    continue;
                }
                let mut runs = Runs {
                    tcx,
                    thir,
                    waiting: reads.iter().map(|&get| &thir[get] as *const _).collect(),
                    loops: 0,
                    early: true,
                };
                for &later in &block.stmts[at + 1..] {
                    runs.visit_stmt(&thir[later]);
                }
                if let Some(tail) = block.expr {
                    runs.visit_expr(&thir[tail]);
                }
                if runs.early && runs.waiting.is_empty() {
                    found.insert(var);
                }
            }
        }
    }
}
