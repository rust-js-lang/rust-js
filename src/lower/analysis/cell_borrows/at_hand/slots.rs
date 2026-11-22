//! What a thread-local the crate alone reaches holds, its slot (#2114):
//! every write of it is one of a closed list, so a value a builtin made
//! kept in one, `RENDER_PHASE_UPDATES.set(Some(js::map::new(None)))`, is at
//! hand read back. Anything else done with what it holds disqualifies it:
//! the rule fails closed.

use crate::lower::bindings::{JsForm, creates, is_binding, js_form};
use crate::lower::body_queries::strip;
use crate::lower::recognition::{Recognition, Std, local_key_access};
use crate::lower::std_types::option::OptionPlaceOp;
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, PatKind, Thir};
use rustc_span::def_id::DefId;

/// A write of a slot.
pub(super) enum Write<'a, 'tcx> {
    /// A value written, `set(v)`'s, at hand where `AtHand::holds` says.
    Value(&'a Thir<'tcx>, ExprId),
    /// What a closure gives, `get_or_insert_with(|| ..)`'s, at hand where
    /// its value is.
    Closure(DefId),
    /// What a builtin's constructor, or one that creates its result,
    /// makes, `get_or_insert_with(js::weak_set::new)`'s.
    Made,
}

/// A closed key's writes, and the parameters of its `with_borrow` and
/// `with_borrow_mut` closures, what it holds lent to them; `None` where a
/// use of what it holds isn't one of the closed list.
pub(super) type Writes<'a, 'tcx> = Option<(Vec<Write<'a, 'tcx>>, Vec<LocalVarId>)>;

/// Of each of `closed`, the thread-locals no code but the crate's reaches
/// (`closed_keys`), its writes: its initializer's, `inits`', each `set` and
/// `replace`, and in each `with_borrow_mut`'s closure what it writes through
/// the reference it's lent.
pub(super) fn of<'a, 'r, 'tcx>(
    in_body: &dyn Fn(&Thir<'tcx>) -> Recognition<'r, 'tcx>,
    bodies: &[(DefId, &'a Thir<'tcx>)],
    (closed, initialized): (&IndexSet<DefId>, &IndexSet<DefId>),
    inits: &[(DefId, &'a Thir<'tcx>, ExprId)],
) -> IndexMap<DefId, Writes<'a, 'tcx>> {
    let thir_of = |id: DefId| bodies.iter().find(|(body, _)| *body == id).map(|(_, thir)| *thir);
    // A key whose initializer isn't known: what it makes isn't seen.
    let mut writes: IndexMap<DefId, Writes<'a, 'tcx>> = (closed.iter())
        .map(|&key| (key, initialized.contains(&key).then(|| (Vec::new(), Vec::new()))))
        .collect();
    let disqualify = |writes: &mut IndexMap<DefId, Writes<'a, 'tcx>>, key: DefId| {
        writes.insert(key, None);
    };
    for &(key, thir, root) in inits {
        if !closed.contains(&key) {
            continue;
        }
        match initial(&in_body(thir), thir, root) {
            Some(value) => {
                if let Some(Some((found, _))) = writes.get_mut(&key) {
                    found.push(Write::Value(thir, value));
                }
            }
            None => disqualify(&mut writes, key),
        }
    }
    for &(_, thir) in bodies {
        let recognition = &in_body(thir);
        let tcx = recognition.tcx;
        for e in thir.exprs.iter() {
            let ExprKind::Call { fun, args, .. } = &e.kind else {
                continue;
            };
            let Some((id, generic_args)) = crate::lower::fn_def(thir[*fun].ty) else {
                continue;
            };
            let Some(key) = closed_key(tcx, thir, (id, args), closed) else {
                continue;
            };
            let found = match recognition.classify(id, generic_args) {
                Some(Std::CellSet | Std::CellReplace) => {
                    args.get(1).map(|&value| (vec![Write::Value(thir, value)], vec![]))
                }
                // A read, or `take`'s write of `None`, `Option`'s default,
                // which holds no object.
                Some(Std::CellGet | Std::CellTake) => Some((vec![], vec![])),
                Some(Std::LocalBorrow { mutable: false }) => args
                    .get(1)
                    .and_then(|&closure| lent(thir, closure, &thir_of))
                    .map(|p| (vec![], vec![p])),
                Some(Std::LocalBorrow { mutable: true }) => args.get(1).and_then(|&closure| {
                    let param = lent(thir, closure, &thir_of)?;
                    let ExprKind::Closure(c) = &thir[strip(thir, closure)].kind else {
                        return None;
                    };
                    let closure_thir = thir_of(c.closure_id.to_def_id())?;
                    let found = written_through(&in_body(closure_thir), (closure_thir, param), bodies)?;
                    Some((found, vec![param]))
                }),
                // `closed_keys` admits no other access; were there one, what
                // it does isn't seen.
                _ => None,
            };
            match found {
                Some((found, params)) => {
                    if let Some(Some((all, lent))) = writes.get_mut(&key) {
                        all.extend(found);
                        lent.extend(params);
                    }
                }
                None => disqualify(&mut writes, key),
            }
        }
    }
    writes
}

/// The closed key a call of `id`, a thread-local's access, is of.
pub(super) fn closed_key<'tcx>(
    tcx: rustc_middle::ty::TyCtxt<'tcx>,
    thir: &Thir<'tcx>,
    (id, args): (DefId, &[ExprId]),
    closed: &IndexSet<DefId>,
) -> Option<DefId> {
    local_key_access(tcx, id)?;
    let ExprKind::Borrow { arg, .. } = thir[strip(thir, *args.first()?)].kind else {
        return None;
    };
    let ExprKind::NamedConst { def_id: key, .. } = thir[strip(thir, arg)].kind else {
        return None;
    };
    closed.contains(&key).then_some(key)
}

/// What a thread-local's initializer, `RefCell::new(None)`'s, makes it hold:
/// the value its cell's `new` is given; `None` of any other initializer.
fn initial<'tcx>(recognition: &Recognition<'_, 'tcx>, thir: &Thir<'tcx>, root: ExprId) -> Option<ExprId> {
    let mut at = strip(thir, root);
    while let ExprKind::Block { block } = thir[at].kind {
        at = strip(thir, thir[block].expr?);
    }
    let ExprKind::Call { fun, ref args, .. } = thir[at].kind else {
        return None;
    };
    let (id, generic_args) = crate::lower::fn_def(thir[fun].ty)?;
    let made_cell = recognition.classify(id, generic_args) == Some(Std::CellNew);
    made_cell.then(|| args.first().copied()).flatten()
}

/// The variable `closure`, a call's argument in `thir`, binds its parameter
/// to, what the thread-local holds lent to it.
fn lent<'a, 'tcx>(
    thir: &Thir<'tcx>,
    closure: ExprId,
    thir_of: &dyn Fn(DefId) -> Option<&'a Thir<'tcx>>,
) -> Option<LocalVarId> {
    let ExprKind::Closure(c) = &thir[strip(thir, closure)].kind else {
        return None;
    };
    let closure_thir = thir_of(c.closure_id.to_def_id())?;
    let param = (closure_thir.params.iter()).find(|param| param.self_kind.is_none() && param.pat.is_some())?;
    match param.pat.as_deref()?.kind {
        PatKind::Binding {
            var, subpattern: None, ..
        } => Some(var),
        _ => None,
    }
}

/// What a `with_borrow_mut` closure, `closure_thir`, writes through its
/// parameter `param`: each use of it one of a closed list, `*p = v`, an
/// `Option`'s `insert`, `get_or_insert`, `get_or_insert_with`, `replace`,
/// or a read; `None` where one isn't, or a closure in it captures it.
fn written_through<'a, 'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    (closure_thir, param): (&'a Thir<'tcx>, LocalVarId),
    bodies: &[(DefId, &'a Thir<'tcx>)],
) -> Option<Vec<Write<'a, 'tcx>>> {
    let tcx = recognition.tcx;
    let captured = bodies.iter().any(|&(_, thir)| {
        (thir.exprs.iter()).any(|e| matches!(e.kind, ExprKind::UpvarRef { var_hir_id, .. } if var_hir_id == param))
    });
    if captured {
        return None;
    }
    let thir = closure_thir;
    // The variable's place, through the derefs and borrows of a receiver.
    let of_param = |e: ExprId| {
        let mut at = strip(thir, e);
        loop {
            match thir[at].kind {
                ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => at = strip(thir, arg),
                ExprKind::VarRef { id } if id == param => return Some(at),
                _ => return None,
            }
        }
    };
    let mut lent_mutably = IndexSet::default();
    for e in thir.exprs.iter() {
        match e.kind {
            ExprKind::Borrow {
                borrow_kind: BorrowKind::Mut { .. },
                arg,
            }
            | ExprKind::RawBorrow {
                mutability: rustc_ast::Mutability::Mut,
                arg,
            } => _ = lent_mutably.insert(strip(thir, arg)),
            _ => {}
        }
    }
    let mut found = Vec::new();
    let mut accounted = IndexSet::default();
    for (at, e) in thir.exprs.iter_enumerated() {
        match &e.kind {
            ExprKind::Assign { lhs, rhs } => {
                if let ExprKind::Deref { arg } = thir[strip(thir, *lhs)].kind
                    && let Some(var) = of_param(arg)
                {
                    accounted.insert(var);
                    found.push(Write::Value(thir, *rhs));
                }
            }
            ExprKind::Call { fun, args, .. } => {
                let Some((id, generic_args)) = crate::lower::fn_def(thir[*fun].ty) else {
                    continue;
                };
                let Some(var) = args.first().and_then(|&receiver| of_param(receiver)) else {
                    continue;
                };
                if !recognition.is_lang_adt(thir[var].ty.peel_refs(), LangItem::Option) {
                    continue;
                }
                let write = match recognition.classify(id, generic_args) {
                    Some(Std::OptionPlace(OptionPlaceOp::Insert | OptionPlaceOp::GetOrInsert) | Std::OptionReplace) => {
                        args.get(1).map(|&value| Some(Write::Value(thir, value)))
                    }
                    Some(Std::OptionPlace(OptionPlaceOp::GetOrInsertWith)) => {
                        args.get(1).map(|&made| match &thir[strip(thir, made)].kind {
                            ExprKind::Closure(c) => Some(Write::Closure(c.closure_id.to_def_id())),
                            _ => match crate::lower::fn_def(thir[made].ty) {
                                Some((maker, _))
                                    if is_binding(tcx, maker)
                                        && (matches!(js_form(tcx, maker), JsForm::New(_)) || creates(tcx, maker)) =>
                                {
                                    Some(Write::Made)
                                }
                                _ => None,
                            },
                        })
                    }
                    // Reads, and `take`'s write of `None`.
                    Some(Std::OptionTake | Std::IsNone | Std::IsSome) => Some(None),
                    _ => continue,
                };
                match write {
                    Some(Some(write)) => {
                        accounted.insert(var);
                        found.push(write);
                    }
                    Some(None) => _ = accounted.insert(var),
                    // A maker that isn't a closure nor a builtin's
                    // constructor: what it makes isn't seen.
                    None => return None,
                }
            }
            // `*p`, read, not lent mutably.
            ExprKind::Deref { arg } if !lent_mutably.contains(&at) => {
                if let ExprKind::VarRef { id } = thir[strip(thir, *arg)].kind
                    && id == param
                {
                    accounted.insert(strip(thir, *arg));
                }
            }
            _ => {}
        }
    }
    let every_use_accounted = (thir.exprs.iter_enumerated())
        .filter(|(_, e)| matches!(e.kind, ExprKind::VarRef { id } if id == param))
        .all(|(at, _)| accounted.contains(&at));
    every_use_accounted.then_some(found)
}
