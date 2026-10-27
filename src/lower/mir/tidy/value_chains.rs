//! Tidy's chains of arms a variable's value is given by: each arm its own,
//! as the chain is written, but where every arm is a value alone, one
//! nested conditional (ADR 0969, amended).

use super::*;

/// Of `x = t ? q : r`, the rest of a chain of arms a variable's value is
/// given by, folded, its arms as the chain writes them: `if (t) { x = q; }
/// else ..`, the chain going on where `r` is another, `else if (c) { x = r;
/// } else { x = s; }`, and of `!t ? e : r`, MIR's arms the other way, `if
/// (t) { x = r; } else { x = e; }`. Each arm a value alone, or the
/// conditional of an `if` in it, `wasHydrated ? a : b`; `None` of what's no
/// conditional.
pub(super) fn chained_arms(x: &str, value: &Expr, at: js::Span) -> Option<Vec<Stmt>> {
    let (t, q, r) = as_written(value)?;
    let arm_value = |e: &Expr| small(e) || matches!(&e.kind, ExprKind::Cond(_, a, b) if small(a) && small(b));
    let arm = |value: &Expr| vec![StmtKind::Assign(Expr::var(x), value.clone()).at(at)];
    let rest = match chained_arms(x, r, at) {
        Some(rest) => rest,
        None if arm_value(r) => arm(r),
        None => return None,
    };
    arm_value(q).then(|| vec![StmtKind::If(t.clone(), arm(q), Some(rest)).at(at)])
}

/// Of the rest of a chain of arms, folded, `x = b ? q : c ? r : s`, whose
/// every arm is a plain value (`plain`): that conditional, its arms as the
/// chain writes them, `!t ? e : r` of MIR's the other way round `t ? r :
/// e`, as upstream React writes its colors, `const color = a ? 'x' : b ?
/// 'y' : 'z';`. `None` where an arm is more, a call, an object or a
/// conditional of its own, whose chain stays `else if` (`chained_arms`).
pub(super) fn plain_chain(value: &Expr) -> Option<Expr> {
    let Some((t, q, r)) = as_written(value) else {
        return plain(value).then(|| value.clone());
    };
    let rest = plain_chain(r)?;
    plain(q).then(|| Expr {
        kind: ExprKind::Cond(Box::new(t.clone()), Box::new(q.clone()), Box::new(rest)),
        span: value.span,
        facts: value.facts,
    })
}

/// Of `if (t) { x = yes; } else { x = no; }`, one arm a plain value and the
/// other the rest of a chain of plain values, folded (`plain_chain`): the
/// one conditional, the plain value first, `t ? yes : b ? q : r`, and of
/// the chain in `yes`, `!t ? no : ..`, as `match o { None => .., Some(A)
/// => .., .. }` is `o == null ? .. : o === "A" ? .. : ..`. `None` where
/// neither arm or both are conditionals, a tree and no chain.
pub(super) fn plain_chain_of(test: &Expr, yes: Expr, no: Expr) -> Option<Expr> {
    let chain = |e: &Expr| matches!(e.kind, ExprKind::Cond(..));
    match (chain(&yes), chain(&no)) {
        (false, true) if plain(&yes) => Some(Expr::cond(test.clone(), yes, plain_chain(&no)?)),
        (true, false) if plain(&no) => Some(Expr::cond(test.clone().negated_test(), no, plain_chain(&yes)?)),
        _ => None,
    }
}

/// Of a chain's conditional, `t ? q : r`, its test, its arm and the rest
/// of the chain, as the chain writes them: of `!t ? e : r`, MIR's arms the
/// other way round, `t`, `r` and `e`. `None` of what's no conditional.
fn as_written(value: &Expr) -> Option<(&Expr, &Expr, &Expr)> {
    let ExprKind::Cond(t, q, r) = &value.kind else {
        return None;
    };
    Some(match &t.kind {
        ExprKind::Unary(UnaryOp::Not, inner) => (&**inner, r, q),
        _ => (&**t, q, r),
    })
}

/// Whether `e` is a value alone, as a chain of plain values' arms are: a
/// string or a number literal, `-1` too, or a name.
pub(super) fn plain(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Str(_) | ExprKind::Num(_) | ExprKind::Var(_) => true,
        ExprKind::Unary(UnaryOp::Neg, n) => matches!(n.kind, ExprKind::Num(_)),
        _ => false,
    }
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
