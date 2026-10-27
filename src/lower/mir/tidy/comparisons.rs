//! Tidy's comparisons: a test of numbers as a person writes it, `b < 48`
//! where it's never `NaN`, and a float's order its comparison.

use super::*;

/// `!(b >= 48)`, a comparison of what's never `NaN`: `b < 48`, as a person
/// writes it. An integer is never `NaN`, nor what a bitwise operator, a
/// `length` or an `indexOf` gives; a string compared, to a string literal as Rust's types
/// say, is ordered too. A float may be `NaN`, neither less nor more (ADR
/// 0371).
pub(super) fn turned(e: &mut Expr, numbers: &HashSet<String>) {
    let ExprKind::Unary(UnaryOp::Not, inner) = &e.kind else {
        return;
    };
    let ExprKind::Binary(op, a, b) = &inner.kind else {
        return;
    };
    let op = match op {
        Op::Lt => Op::Ge,
        Op::Le => Op::Gt,
        Op::Gt => Op::Le,
        Op::Ge => Op::Lt,
        _ => return,
    };
    let never_nan = |x: &Expr| match &x.kind {
        ExprKind::Var(v) => numbers.contains(v),
        ExprKind::Num(n) => n.fract() == 0.0,
        ExprKind::Binary(Op::BitOr | Op::BitAnd | Op::BitXor | Op::Shl | Op::Shr | Op::UShr, ..) => true,
        ExprKind::Member(_, m) => m == "length",
        // Where an array's `indexOf` finds it, `-1` of none (ADR 0966).
        ExprKind::Call(f, args) => args.len() == 1 && matches!(&f.kind, ExprKind::Member(_, m) if m == "indexOf"),
        _ => false,
    };
    let text = |x: &Expr| matches!(x.kind, ExprKind::Str(_));
    if text(a) || text(b) || (never_nan(a) && never_nan(b)) {
        *e = Expr {
            span: e.span,
            ..Expr::bin(op, (**a).clone(), (**b).clone())
        };
    }
}

/// `$partialCmp(a, b) !== -1`, a float's `partial_cmp(&b) != Some(Less)`,
/// compared with an `Ordering`: `!(a < b)`, as a person writes it, and `===
/// -1` `a < b`, `1` `a > b`, `0` `a === b`. `$partialCmp` is `a < b ? -1 : a
/// > b ? 1 : a === b ? 0 : undefined`, so each is its value, where `a` and
/// `b` are floats or number literals, whose comparisons run nothing; not of
/// a value of any shape, whose `valueOf` the helper may run twice (ADR 0966).
pub(super) fn ordered_floats(e: &mut Expr, floats: &HashSet<String>) {
    let ExprKind::Binary(eq @ (Op::Eq | Op::Ne), compared, ordering) = &e.kind else {
        return;
    };
    let ExprKind::Call(callee, args) = &compared.kind else {
        return;
    };
    let ordering = match &ordering.kind {
        ExprKind::Unary(UnaryOp::Neg, n) => n.as_int().map(|n| -n),
        _ => ordering.as_int(),
    };
    let op = match ordering {
        Some(-1) => Op::Lt,
        Some(1) => Op::Gt,
        Some(0) => Op::Eq,
        _ => return,
    };
    let number = |x: &Expr| match &x.kind {
        ExprKind::Var(v) => floats.contains(v),
        ExprKind::Num(_) => true,
        _ => false,
    };
    let [a, b] = args.as_slice() else { return };
    if callee.helper_name() != Some("$partialCmp") || !number(a) || !number(b) {
        return;
    }
    let tested = Expr::bin(op, a.clone(), b.clone());
    let tested = match (eq, op) {
        (Op::Eq, _) => tested,
        (_, Op::Eq) => Expr::bin(Op::Ne, a.clone(), b.clone()),
        _ => Expr::unary(UnaryOp::Not, tested),
    };
    *e = Expr { span: e.span, ..tested };
}
