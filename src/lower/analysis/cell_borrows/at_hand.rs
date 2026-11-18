//! What of the crate is a value at hand, one a builtin or the program
//! made, whose property reads meet no getter of the program's (#2106, ADR
//! 1173). Split from runs_nothing.rs by concept.

use crate::collections::{IndexMap, IndexSet};
use crate::lower::bindings::{JsForm, creates, is_binding, js_form};
use crate::lower::recognition::{Recognition, StdItem};
use rustc_ast::Mutability;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::mir::BorrowKind;
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, Pat, PatKind, StmtKind, Thir};
use rustc_span::def_id::DefId;

/// The crate's variables whose value is one a builtin made, at hand (#2106):
/// what a constructor makes, `new X`, or a binding that `creates` its
/// result, `regExp.exec(s)`; through `unwrap`, `expect`, `clone` and `?`,
/// a `let`, an `if let` or a `match` of one, and the parameter of an
/// `Option`'s or a `Result`'s combinator's closure given one's payload,
/// `.and_then(|matched| matched.get(1))`, a closure's capture of one too.
/// Never what a builtin hands back from what a JS caller stored, `Map.get`'s.
/// A variable is at hand only where every write of it is: its binding's
/// and each assignment's, to it or to a closure's capture of it; one lent
/// `&mut` or bound `ref mut`, whose writes aren't seen, never is.
#[derive(Default)]
pub(in crate::lower) struct AtHand {
    vars: IndexSet<LocalVarId>,
}

/// The combinators whose closure is given the payload of what they're of.
const PAYLOAD_COMBINATORS: [&str; 9] = [
    "and_then",
    "map",
    "map_or",
    "map_or_else",
    "is_some_and",
    "is_none_or",
    "is_ok_and",
    "filter",
    "inspect",
];

impl AtHand {
    /// Of each of the crate's bodies, `(def_id, thir)`, until it holds.
    pub(in crate::lower) fn of_crate<'tcx>(
        recognition: &Recognition<'_, 'tcx>,
        bodies: &[(DefId, &Thir<'tcx>)],
    ) -> Self {
        let mut at_hand = AtHand::default();
        let thir_of = |id: DefId| bodies.iter().find(|(body, _)| *body == id).map(|(_, thir)| *thir);
        let (assigned, lent) = writes(bodies);
        let bind = |pattern: &Pat<'tcx>, found: &mut Vec<LocalVarId>| {
            pattern.walk_always(|p| {
                if let PatKind::Binding { var, .. } = p.kind {
                    found.push(var);
                }
            });
        };
        loop {
            let mut found = Vec::new();
            for &(_, thir) in bodies {
                for stmt in thir.stmts.iter() {
                    if let StmtKind::Let {
                        pattern,
                        initializer: Some(value),
                        ..
                    } = &stmt.kind
                        && at_hand.holds(recognition, thir, *value)
                    {
                        bind(pattern, &mut found);
                    }
                }
                for e in thir.exprs.iter() {
                    match &e.kind {
                        ExprKind::Let { expr, pat } if at_hand.holds(recognition, thir, *expr) => bind(pat, &mut found),
                        ExprKind::Match { scrutinee, arms, .. } if at_hand.holds(recognition, thir, *scrutinee) => {
                            for &arm in arms.iter() {
                                bind(&thir[arm].pattern, &mut found);
                            }
                        }
                        ExprKind::Call { fun, args, .. } => {
                            if let Some(pattern) = at_hand.payload_parameter(recognition, thir, (*fun, args), &thir_of)
                            {
                                bind(pattern, &mut found);
                            }
                        }
                        _ => {}
                    }
                }
            }
            // Each condition holds of more as more is at hand, so this least
            // fixed point is sound: a variable a cycle of writes alone gives
            // is never at hand.
            found.retain(|var| {
                !lent.contains(var)
                    && (assigned.get(var).into_iter().flatten())
                        .all(|&(thir, value)| at_hand.holds(recognition, thir, value))
            });
            let before = at_hand.vars.len();
            at_hand.vars.extend(found);
            if at_hand.vars.len() == before {
                return at_hand;
            }
        }
    }

    /// Of a call, in `thir`, of an `Option`'s or a `Result`'s combinator
    /// whose subject is at hand: its closure's parameter, given the payload.
    fn payload_parameter<'a, 'tcx>(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        thir: &Thir<'tcx>,
        (fun, args): (ExprId, &[ExprId]),
        thir_of: &dyn Fn(DefId) -> Option<&'a Thir<'tcx>>,
    ) -> Option<&'a rustc_middle::thir::Pat<'tcx>> {
        let (callee, _) = crate::lower::fn_def(thir[fun].ty)?;
        let [subject, .., last] = args else {
            return None;
        };
        let of = thir[*subject].ty.peel_refs();
        let combinator = PAYLOAD_COMBINATORS.contains(&recognition.tcx.item_name(callee).as_str())
            && (recognition.is_lang_adt(of, LangItem::Option) || recognition.is_std_type(of, StdItem::Result));
        if !combinator || !self.holds(recognition, thir, *subject) {
            return None;
        }
        let ExprKind::Closure(closure) = &thir[crate::lower::body_queries::strip(thir, *last)].kind else {
            return None;
        };
        let closure_thir = thir_of(closure.closure_id.to_def_id())?;
        (closure_thir.params.iter())
            .find(|param| param.self_kind.is_none() && param.pat.is_some())
            .and_then(|param| param.pat.as_deref())
    }

    /// Whether `e`, in `thir`, is a value at hand.
    pub(in crate::lower) fn holds<'tcx>(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        thir: &Thir<'tcx>,
        e: ExprId,
    ) -> bool {
        let tcx = recognition.tcx;
        let e = crate::lower::body_queries::strip(thir, e);
        match &thir[e].kind {
            ExprKind::Deref { arg } | ExprKind::Borrow { arg, .. } => self.holds(recognition, thir, *arg),
            ExprKind::VarRef { id } => self.vars.contains(id),
            ExprKind::UpvarRef { var_hir_id, .. } => self.vars.contains(var_hir_id),
            ExprKind::Call { fun, args, .. } => match crate::lower::fn_def(thir[*fun].ty) {
                Some((callee, _)) if is_binding(tcx, callee) => {
                    matches!(js_form(tcx, callee), JsForm::New(_)) || creates(tcx, callee)
                }
                Some((callee, _)) => {
                    matches!(
                        tcx.item_name(callee).as_str(),
                        "unwrap" | "expect" | "unwrap_unchecked" | "clone" | "branch"
                    ) && args.first().is_some_and(|&of| self.holds(recognition, thir, of))
                }
                None => false,
            },
            _ => false,
        }
    }
}

/// Of each variable, each value assigned to it, in its body.
type Assigned<'a, 'tcx> = IndexMap<LocalVarId, Vec<(&'a Thir<'tcx>, ExprId)>>;

/// Of each variable of `bodies`, the values assigned to it, `x = value`'s,
/// a closure's to its capture of it too; and the variables lent `&mut`,
/// `&raw mut` or bound `ref mut` in a pattern of them, whose writes through
/// the loan aren't seen.
fn writes<'a, 'tcx>(bodies: &[(DefId, &'a Thir<'tcx>)]) -> (Assigned<'a, 'tcx>, IndexSet<LocalVarId>) {
    let mut assigned = Assigned::default();
    let mut lent = IndexSet::default();
    for &(_, thir) in bodies {
        let lends_mut = |pattern: &Pat<'tcx>| {
            let mut lends = false;
            pattern.walk_always(|p| {
                lends |= matches!(
                    p.kind,
                    PatKind::Binding {
                        mode: rustc_hir::BindingMode(rustc_hir::ByRef::Yes(_, Mutability::Mut), _),
                        ..
                    }
                );
            });
            lends
        };
        for stmt in thir.stmts.iter() {
            if let StmtKind::Let {
                pattern,
                initializer: Some(value),
                ..
            } = &stmt.kind
                && lends_mut(pattern)
            {
                lent.extend(root(thir, *value));
            }
        }
        for e in thir.exprs.iter() {
            match &e.kind {
                ExprKind::Assign { lhs, rhs } => match root_written(thir, *lhs) {
                    Some((var, true)) => assigned.entry(var).or_default().push((thir, *rhs)),
                    Some((var, false)) => {
                        lent.insert(var);
                    }
                    None => {}
                },
                ExprKind::AssignOp { lhs: arg, .. }
                | ExprKind::Borrow {
                    borrow_kind: BorrowKind::Mut { .. },
                    arg,
                }
                | ExprKind::RawBorrow {
                    mutability: Mutability::Mut,
                    arg,
                } => lent.extend(root(thir, *arg)),
                ExprKind::Let { expr, pat } if lends_mut(pat) => lent.extend(root(thir, *expr)),
                ExprKind::Match { scrutinee, arms, .. } if arms.iter().any(|&arm| lends_mut(&thir[arm].pattern)) => {
                    lent.extend(root(thir, *scrutinee));
                }
                _ => {}
            }
        }
    }
    (assigned, lent)
}

/// The variable the place `e`, in `thir`, is part of, through its fields,
/// elements and derefs: `x`, of `x.0[i]` or `*x`.
fn root(thir: &Thir<'_>, e: ExprId) -> Option<LocalVarId> {
    root_written(thir, e).map(|(var, _)| var)
}

/// `root`'s, and whether the place is the variable itself, `x`, not a part
/// of it or what it points to.
fn root_written(thir: &Thir<'_>, e: ExprId) -> Option<(LocalVarId, bool)> {
    let e = crate::lower::body_queries::strip(thir, e);
    match &thir[e].kind {
        ExprKind::VarRef { id } => Some((*id, true)),
        ExprKind::UpvarRef { var_hir_id, .. } => Some((*var_hir_id, true)),
        ExprKind::Field { lhs: of, .. } | ExprKind::Index { lhs: of, .. } | ExprKind::Deref { arg: of } => {
            root(thir, *of).map(|var| (var, false))
        }
        _ => None,
    }
}
