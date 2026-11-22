//! A local cell its closures share (#2114's second step): a `let` of
//! `Rc::new(RefCell::new(v))`, the `let`s of its `Rc::clone`s, and the
//! closures that capture any of them, React's noop renderer's
//! `suspenseyThingCache`. Every use of each is one of a closed list: a
//! clone's argument, a closure's capture, or the receiver of the cell's
//! `borrow()`, `replace(v)`, `set(v)` or `take()`. What it holds is at hand
//! where every write of it is, read back by `borrow()`. Any other use, the
//! cell given to a function, returned, moved, lent mutably, borrowed
//! mutably, disqualifies it: the rule fails closed.

use super::slots::Write;
use crate::lower::body_queries::strip;
use crate::lower::recognition::{Recognition, Std, StdItem, is_rc_clone, is_rc_deref};
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, PatKind, StmtKind, Thir};
use rustc_span::def_id::DefId;

/// A local cell its closures share: its members, the `let` that makes it
/// and those of its clones, and its writes; `None` where a use of a member
/// isn't one of the closed list.
pub(super) struct Group<'a, 'tcx> {
    pub(super) members: Vec<LocalVarId>,
    pub(super) writes: Option<Vec<Write<'a, 'tcx>>>,
}

/// Of `bodies`, each local cell its closures share.
pub(super) fn of<'a, 'r, 'tcx>(
    in_body: &dyn Fn(&Thir<'tcx>) -> Recognition<'r, 'tcx>,
    bodies: &[(DefId, &'a Thir<'tcx>)],
) -> Vec<Group<'a, 'tcx>> {
    // Each cell made, its variable and what it's made holding.
    let mut groups: Vec<Group<'a, 'tcx>> = Vec::new();
    let mut of_var: IndexMap<LocalVarId, usize> = IndexMap::default();
    for &(_, thir) in bodies {
        let recognition = &in_body(thir);
        for stmt in thir.stmts.iter() {
            let StmtKind::Let {
                pattern,
                initializer: Some(init),
                ..
            } = &stmt.kind
            else {
                continue;
            };
            let PatKind::Binding {
                var, subpattern: None, ..
            } = pattern.kind
            else {
                continue;
            };
            if let Some(value) = made(recognition, thir, *init) {
                of_var.insert(var, groups.len());
                groups.push(Group {
                    members: vec![var],
                    writes: Some(vec![Write::Value(thir, value)]),
                });
            }
        }
    }
    // Each use of a member accounted for, by its body and its id.
    let mut accounted: IndexSet<(usize, ExprId)> = IndexSet::default();
    let key = |thir: &Thir<'tcx>, at: ExprId| (std::ptr::from_ref(thir) as usize, at);
    let member = |thir: &Thir<'tcx>, at: ExprId, of_var: &IndexMap<LocalVarId, usize>| match thir[at].kind {
        ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. } => of_var.get(&id).copied(),
        _ => None,
    };
    // A clone's `let`, a member too, until no more are.
    loop {
        let before = of_var.len();
        for &(_, thir) in bodies {
            let recognition = &in_body(thir);
            for stmt in thir.stmts.iter() {
                let StmtKind::Let {
                    pattern,
                    initializer: Some(init),
                    ..
                } = &stmt.kind
                else {
                    continue;
                };
                let PatKind::Binding {
                    var, subpattern: None, ..
                } = pattern.kind
                else {
                    continue;
                };
                let ExprKind::Call { fun, ref args, .. } = thir[strip(thir, *init)].kind else {
                    continue;
                };
                let Some((id, generic_args)) = crate::lower::fn_def(thir[fun].ty) else {
                    continue;
                };
                if !is_rc_clone(recognition.tcx, id, generic_args) {
                    continue;
                }
                let Some(at) = args.first().and_then(|&of| receiver_member(recognition, thir, of)) else {
                    continue;
                };
                if let Some(group) = member(thir, at, &of_var) {
                    accounted.insert(key(thir, at));
                    if of_var.insert(var, group).is_none() {
                        groups[group].members.push(var);
                    }
                }
            }
        }
        if of_var.len() == before {
            break;
        }
    }
    for &(_, thir) in bodies {
        let recognition = &in_body(thir);
        for e in thir.exprs.iter() {
            match &e.kind {
                // A closure's capture of a member: its uses in the closure
                // are its body's, each asked there.
                ExprKind::Closure(closure) => {
                    for &upvar in closure.upvars.iter() {
                        let mut at = strip(thir, upvar);
                        if let ExprKind::Borrow { arg, .. } = thir[at].kind {
                            at = strip(thir, arg);
                        }
                        if member(thir, at, &of_var).is_some() {
                            accounted.insert(key(thir, at));
                        }
                    }
                }
                ExprKind::Call { fun, args, .. } => {
                    let Some((id, generic_args)) = crate::lower::fn_def(thir[*fun].ty) else {
                        continue;
                    };
                    let Some(at) = args
                        .first()
                        .and_then(|&receiver| receiver_member(recognition, thir, receiver))
                    else {
                        continue;
                    };
                    let Some(group) = member(thir, at, &of_var) else {
                        continue;
                    };
                    let write = match recognition.classify(id, generic_args) {
                        Some(Std::Borrow { mutable: false, .. } | Std::CellTake) => None,
                        Some(Std::CellReplace | Std::CellSet) => match args.get(1) {
                            Some(&value) => Some(Write::Value(thir, value)),
                            None => continue,
                        },
                        _ => continue,
                    };
                    accounted.insert(key(thir, at));
                    if let (Some(write), Some(writes)) = (write, groups[group].writes.as_mut()) {
                        writes.push(write);
                    }
                }
                _ => {}
            }
        }
    }
    // A use of a member none of those accounts for disqualifies its cell.
    for &(_, thir) in bodies {
        for (at, _) in thir.exprs.iter_enumerated() {
            if let Some(group) = member(thir, at, &of_var)
                && !accounted.contains(&key(thir, at))
            {
                groups[group].writes = None;
            }
        }
    }
    groups
}

/// What `Rc::new(RefCell::new(v))` (or of a `Cell`), `init` in `thir`, is
/// made holding, `v`; `None` of anything else.
fn made<'tcx>(recognition: &Recognition<'_, 'tcx>, thir: &Thir<'tcx>, init: ExprId) -> Option<ExprId> {
    let at = strip(thir, init);
    let ExprKind::Call { fun, ref args, .. } = thir[at].kind else {
        return None;
    };
    let (id, generic_args) = crate::lower::fn_def(thir[fun].ty)?;
    if recognition.classify(id, generic_args) != Some(Std::Same) || !recognition.is_std_type(thir[at].ty, StdItem::Rc) {
        return None;
    }
    let cell = strip(thir, *args.first()?);
    let ExprKind::Call { fun, ref args, .. } = thir[cell].kind else {
        return None;
    };
    let (id, generic_args) = crate::lower::fn_def(thir[fun].ty)?;
    (recognition.classify(id, generic_args) == Some(Std::CellNew))
        .then(|| args.first().copied())
        .flatten()
}

/// The member a receiver, `receiver` in `thir`, is the cell of: through
/// borrows, derefs and an `Rc`'s `Deref`.
pub(super) fn receiver_member<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    thir: &Thir<'tcx>,
    receiver: ExprId,
) -> Option<ExprId> {
    let mut at = strip(thir, receiver);
    loop {
        match &thir[at].kind {
            ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => at = strip(thir, *arg),
            ExprKind::Call { fun, args, .. }
                if let Some((id, generic_args)) = crate::lower::fn_def(thir[*fun].ty)
                    && is_rc_deref(recognition.tcx, id, generic_args)
                    && let Some(&of) = args.first() =>
            {
                at = strip(thir, of);
            }
            ExprKind::VarRef { .. } | ExprKind::UpvarRef { .. } => return Some(at),
            _ => return None,
        }
    }
}
