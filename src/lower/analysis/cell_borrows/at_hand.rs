//! What of the crate is a value at hand, one a builtin or the program
//! made, whose property reads meet no getter of the program's (#2106, ADR
//! 1173). Split from runs_nothing.rs by concept.

use crate::collections::IndexSet;
use crate::lower::bindings::{JsForm, creates, is_binding, js_form};
use crate::lower::recognition::{Recognition, StdItem};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId, PatKind, StmtKind, Thir};
use rustc_span::def_id::DefId;

/// The crate's variables whose value is one a builtin made, at hand (#2106):
/// what a constructor makes, `new X`, or a binding that `creates` its
/// result, `regExp.exec(s)`; through `unwrap`, `expect`, `clone` and `?`,
/// a `let`, an `if let` or a `match` of one, and the parameter of an
/// `Option`'s or a `Result`'s combinator's closure given one's payload,
/// `.and_then(|matched| matched.get(1))`, a closure's capture of one too.
/// Never what a builtin hands back from what a JS caller stored, `Map.get`'s.
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
        let bind = |pattern: &rustc_middle::thir::Pat<'tcx>, found: &mut Vec<LocalVarId>| {
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
