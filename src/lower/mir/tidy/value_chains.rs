//! Tidy's chains of arms a variable's value is given by: each arm its own,
//! as the chain is written (ADR 0969).

use super::*;

/// Of `x = t ? q : r`, the rest of a chain of arms a variable's value is
/// given by, folded, its arms as the chain writes them: `if (t) { x = q; }
/// else ..`, the chain going on where `r` is another, `else if (c) { x = r;
/// } else { x = s; }`, and of `!t ? e : r`, MIR's arms the other way, `if
/// (t) { x = r; } else { x = e; }`. Each arm a value alone, or the
/// conditional of an `if` in it, `wasHydrated ? a : b`; `None` of what's no
/// conditional.
pub(super) fn chained_arms(x: &str, value: &Expr, at: js::Span) -> Option<Vec<Stmt>> {
    let ExprKind::Cond(t, q, r) = &value.kind else {
        return None;
    };
    let (t, q, r) = match &t.kind {
        ExprKind::Unary(UnaryOp::Not, inner) => (&**inner, r, q),
        _ => (&**t, q, r),
    };
    let arm_value = |e: &Expr| small(e) || matches!(&e.kind, ExprKind::Cond(_, a, b) if small(a) && small(b));
    let arm = |value: &Expr| vec![StmtKind::Assign(Expr::var(x), value.clone()).at(at)];
    let rest = match chained_arms(x, r, at) {
        Some(rest) => rest,
        None if arm_value(r) => arm(r),
        None => return None,
    };
    arm_value(q).then(|| vec![StmtKind::If(t.clone(), arm(q), Some(rest)).at(at)])
}

/// Of `let t; t = v; x = t;`, a temporary MIR gives a variable's value
/// through, declared there and read once: `x` and `v`, as `x = v;` is.
pub(super) fn assigned_through(stmts: &[Stmt]) -> Option<(&str, &Expr)> {
    let [
        Stmt {
            kind: StmtKind::Let(t, None),
            ..
        },
        Stmt {
            kind: StmtKind::Assign(made, value),
            ..
        },
        Stmt {
            kind: StmtKind::Assign(target, read),
            ..
        },
    ] = stmts
    else {
        return None;
    };
    match (&made.kind, &target.kind, &read.kind) {
        (ExprKind::Var(m), ExprKind::Var(x), ExprKind::Var(r)) if is_temporary(t) && m == t && r == t => {
            Some((x.as_str(), value))
        }
        _ => None,
    }
}
