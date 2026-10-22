//! Tidy's logicals: a value the program wrote with `&&` and `||`, which MIR
//! made branches of, returned as one value as it was written.

use super::*;

/// `if (t) { return a; } return b;`, MIR's of a value the program wrote
/// with `&&` or `||` where `t` is tested: one value returned, `t ? a : b`,
/// as the program wrote it, each test before it folded in, `s !== "en" &&
/// (t ? a : b)`; not where it wrote a `return`. An `if`'s condition is one
/// too, as `returned_values` makes of one with an `else`.
pub(super) fn logical_returns(
    stmts: &mut Vec<Stmt>,
    logicals: &[js::Span],
    returns: &[js::Span],
    booleans: &HashSet<String>,
) {
    let within = |s: js::Span| !s.is_none() && logicals.iter().any(|l| l.lo <= s.lo && s.hi <= l.hi);
    while let [.., last_if, last] = stmts.as_slice()
        && let StmtKind::Return(Some(no)) = &last.kind
        && let StmtKind::If(test, then, None) = &last_if.kind
        && let [
            Stmt {
                kind: StmtKind::Return(Some(yes)),
                span: yes_span,
            },
        ] = then.as_slice()
        && within(last_if.span)
        && !written(returns, last.span)
        && !written(returns, *yes_span)
    {
        let returned = StmtKind::Return(Some(conditional(test.clone(), yes.clone(), no.clone(), booleans)));
        let span = last_if.span;
        stmts.splice(stmts.len() - 2.., [returned.at(span)]);
    }
}

/// `l: { if (a) { if (b) { break l; } return true; } } return d;`, MIR's
/// of a value the program wrote with `&&` and `||` mixed, `(a && !b) || d`:
/// the one value returned, each `return` and `break` of the block a value
/// of what follows it, folded as `conditional` folds one, and a value that
/// both sides end with taken out, `c ? (p || d) : d` as `(c && p) || d`.
/// Not where what's returned would be written twice, nor where the
/// program wrote a `return` (ADR 0613).
pub(super) fn logical_blocks(
    stmts: &mut Vec<Stmt>,
    logicals: &[js::Span],
    returns: &[js::Span],
    booleans: &HashSet<String>,
) {
    let within = |s: js::Span| !s.is_none() && logicals.iter().any(|l| l.lo <= s.lo && s.hi <= l.hi);
    let [.., block, last] = stmts.as_slice() else { return };
    let (StmtKind::Labeled(label, body), StmtKind::Return(Some(after))) = (&block.kind, &last.kind) else {
        return;
    };
    // Its tests a logical's the program wrote, the block's own span MIR's.
    if !body.first().is_some_and(|s| within(s.span)) || written(returns, last.span) {
        return;
    }
    // The value `stmts` give, falling through to `fall`, a `break` of the
    // block to `after`.
    fn value(
        stmts: &[Stmt],
        fall: &Expr,
        (label, after): (&str, &Expr),
        returns: &[js::Span],
        booleans: &HashSet<String>,
    ) -> Option<Expr> {
        match stmts {
            [] => Some(fall.clone()),
            [
                Stmt {
                    kind: StmtKind::Return(Some(v)),
                    span,
                },
                ..,
            ] if !written(returns, *span) => Some(v.clone()),
            [
                Stmt {
                    kind: StmtKind::Break(Some(l)),
                    ..
                },
                ..,
            ] if l == label => Some(after.clone()),
            [
                Stmt {
                    kind: StmtKind::If(test, then, None),
                    ..
                },
                rest @ ..,
            ] => {
                let no = value(rest, fall, (label, after), returns, booleans)?;
                let yes = value(then, &no, (label, after), returns, booleans)?;
                Some(folded(test.clone(), yes, no, booleans))
            }
            _ => None,
        }
    }
    let Some(made) = value(body, after, (label, after), returns, booleans) else {
        return;
    };
    // What's returned after it, once, as the program wrote it.
    let mut once = 0;
    let mut seen = vec![StmtKind::Expr(made.clone()).at(js::Span::NONE)];
    js::each_expr_mut(&mut seen, &mut |e| once += usize::from(e == after));
    if once > 1 || has_conditional(&made) {
        return;
    }
    let span = block.span;
    stmts.splice(stmts.len() - 2.., [StmtKind::Return(Some(made)).at(span)]);
}

/// `c ? yes : no` of booleans, as `conditional` folds it, and `c ? (p ||
/// no) : no` as `(c && p) || no`, a value both end with taken out.
fn folded(test: Expr, yes: Expr, no: Expr, booleans: &HashSet<String>) -> Expr {
    // `p` of `p || no`, an `||` chain that ends with `no`.
    fn without(e: &Expr, tail: &Expr) -> Option<Expr> {
        match &e.kind {
            ExprKind::Binary(Op::Or, a, b) if **b == *tail => Some((**a).clone()),
            ExprKind::Binary(Op::Or, a, b) => Some(Expr::bin(Op::Or, (**a).clone(), without(b, tail)?)),
            _ => None,
        }
    }
    if let Some(p) = without(&yes, &no) {
        let test = conditional(test, Expr::bool(true), Expr::bool(false), booleans);
        return Expr::bin(Op::Or, Expr::bin(Op::And, test, p), no);
    }
    conditional(test, yes, no, booleans)
}

/// Whether `e` holds a `c ? a : b`, which a fold left: not what `&&` and
/// `||` were written as.
fn has_conditional(e: &Expr) -> bool {
    let mut found = false;
    let mut stmts = vec![StmtKind::Expr(e.clone()).at(js::Span::NONE)];
    js::each_expr_mut(&mut stmts, &mut |e| found |= matches!(e.kind, ExprKind::Cond(..)));
    found
}
