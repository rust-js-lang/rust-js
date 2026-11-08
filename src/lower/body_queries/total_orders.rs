//! Whether a comparison a body writes is a total order, which any sort
//! leaves in one order (#936).

use super::{strip, tail};
use crate::collections::IndexMap;
use crate::lower::fn_def;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::thir::{self, ExprId, ExprKind, LocalVarId, PatKind, Thir};
use rustc_middle::ty::{self, TyCtxt};

/// Whether the `Ordering` `thir`'s body gives, from `root`, is a total
/// order of what it compares (#936): an `Ord::cmp` of a type `total` says
/// orders totally, `total_cmp` of floats, JS's `localeCompare`, `Equal`,
/// and their `reverse()`, `then(..)`, and `then_with(f)` of a function
/// `total_fn` says is one; `then` written out, `if o != Equal { o } else
/// { p }` or `match o { Equal => p, o => o }`, through a `let` of `o`; a
/// hand-written `partial_cmp`'s `Some` of one. Not another,
/// `partial_cmp(..).unwrap_or(Equal)` of NaNs, an `if` of `<`, a constant
/// `Less`, whose answers may contradict each other, which another sort's
/// sequence of calls then tells apart.
pub(in crate::lower) fn orders_totally<'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &Thir<'tcx>,
    root: ExprId,
    total: (&dyn Fn(ty::Ty<'tcx>) -> bool, &dyn Fn(ty::Ty<'tcx>) -> bool),
) -> bool {
    // Each `let` of the body, by what it binds, read through.
    let lets: IndexMap<LocalVarId, ExprId> = (thir.stmts.iter())
        .filter_map(|stmt| match &stmt.kind {
            thir::StmtKind::Let {
                pattern,
                initializer: Some(init),
                ..
            } => match pattern.kind {
                PatKind::Binding { var, .. } => Some((var, *init)),
                _ => None,
            },
            _ => None,
        })
        .collect();
    Totals { tcx, thir, lets, total }.order(root, 0)
}

/// What `orders_totally` reads a body with.
struct Totals<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    thir: &'a Thir<'tcx>,
    lets: IndexMap<LocalVarId, ExprId>,
    total: (&'a dyn Fn(ty::Ty<'tcx>) -> bool, &'a dyn Fn(ty::Ty<'tcx>) -> bool),
}

impl<'tcx> Totals<'_, 'tcx> {
    fn order(&self, e: ExprId, depth: u32) -> bool {
        use crate::lower::recognition::{OrderingFn, is_ordering_equal, ordering_fn};
        let (tcx, thir) = (self.tcx, self.thir);
        let (total_ty, total_fn) = self.total;
        if depth > 32 {
            return false;
        }
        let e = tail(thir, e);
        match &thir[e].kind {
            ExprKind::VarRef { id } => self.lets.get(id).is_some_and(|&init| self.order(init, depth + 1)),
            // `Some(order)`, a hand-written `partial_cmp`'s.
            ExprKind::Adt(adt)
                if tcx.is_lang_item(adt.adt_def.variant(adt.variant_index).def_id, LangItem::OptionSome)
                    && adt.fields.len() == 1 =>
            {
                self.order(adt.fields[0].expr, depth + 1)
            }
            // `Ordering::Equal`, of all alike.
            ExprKind::Adt(adt) => is_ordering_equal(tcx, adt.adt_def, adt.variant_index),
            // `if o != Equal { o } else { p }`, `o.then(p)`.
            ExprKind::If {
                cond,
                then,
                else_opt: Some(otherwise),
                ..
            } => {
                let Some((negated, o)) = self.compared_with_equal(*cond) else {
                    return false;
                };
                let (kept, next) = if negated {
                    (*then, *otherwise)
                } else {
                    (*otherwise, *then)
                };
                self.same_var(o, kept) && self.order(o, depth + 1) && self.order(next, depth + 1)
            }
            // `match o { Equal => p, o => o }`, `o.then(p)`.
            ExprKind::Match { scrutinee, arms, .. } => {
                let [first, second] = arms[..] else {
                    return false;
                };
                let (first, second) = (&thir[first], &thir[second]);
                let equal = matches!(&first.pattern.kind,
                    PatKind::Variant { adt_def, variant_index, .. } if is_ordering_equal(tcx, *adt_def, *variant_index));
                let kept = match &second.pattern.kind {
                    PatKind::Binding {
                        var, subpattern: None, ..
                    } => {
                        matches!(thir[tail(thir, second.body)].kind, ExprKind::VarRef { id } if id == *var)
                    }
                    _ => false,
                };
                first.guard.is_none()
                    && second.guard.is_none()
                    && equal
                    && kept
                    && self.order(*scrutinee, depth + 1)
                    && self.order(first.body, depth + 1)
            }
            ExprKind::Call { fun, args, .. } => {
                let Some((called, generic)) = fn_def(thir[*fun].ty) else {
                    return false;
                };
                match ordering_fn(tcx, called, generic) {
                    Some(OrderingFn::Cmp) => generic.types().next().is_some_and(total_ty),
                    Some(OrderingFn::TotalCmp | OrderingFn::LocaleCompare) => true,
                    Some(OrderingFn::Reverse) => self.order(args[0], depth + 1),
                    Some(OrderingFn::Then) => self.order(args[0], depth + 1) && self.order(args[1], depth + 1),
                    Some(OrderingFn::ThenWith) => {
                        self.order(args[0], depth + 1) && total_fn(thir[strip(thir, args[1])].ty)
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// `o != Equal`, or `o == Equal`: whether it's `!=`, and `o`.
    fn compared_with_equal(&self, cond: ExprId) -> Option<(bool, ExprId)> {
        use crate::lower::recognition::{OrderingFn, is_ordering_equal, ordering_fn};
        let thir = self.thir;
        let ExprKind::Call { fun, args, .. } = &thir[tail(thir, cond)].kind else {
            return None;
        };
        let (called, generic) = fn_def(thir[*fun].ty)?;
        let Some(OrderingFn::Eq { negated }) = ordering_fn(self.tcx, called, generic) else {
            return None;
        };
        let operand = |e: ExprId| {
            let mut e = strip(thir, e);
            while let ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } = thir[e].kind {
                e = strip(thir, arg);
            }
            e
        };
        let is_equal = |e: ExprId| matches!(&thir[operand(e)].kind, ExprKind::Adt(adt) if is_ordering_equal(self.tcx, adt.adt_def, adt.variant_index));
        let [a, b] = args[..] else { return None };
        match (is_equal(a), is_equal(b)) {
            (false, true) => Some((negated, operand(a))),
            (true, false) => Some((negated, operand(b))),
            _ => None,
        }
    }

    /// Whether `a` and `b` read one variable.
    fn same_var(&self, a: ExprId, b: ExprId) -> bool {
        let var = |e: ExprId| match self.thir[tail(self.thir, e)].kind {
            ExprKind::VarRef { id } => Some(id),
            _ => None,
        };
        var(a).is_some() && var(a) == var(b)
    }
}
