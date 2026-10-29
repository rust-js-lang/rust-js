//! A `switch` of an `Option`'s value, which takes `None` where no case
//! is `null`'s: the `default`, or past the `switch` (ADR 0629, amended).

use crate::js::{self, Expr, ExprKind, Stmt, StmtKind};
use crate::lower::mir::hints::Switch;

use super::does_nothing;

/// A block's `if (x != null) { switch (x) { .. } }` of an `Option`'s
/// value, then what MIR placed after it that the `_` arm does, `hit =
/// false;`, which `None` and each value no case takes reach: `switch (x)
/// { .. default: hit = false; }`, as JS's `switch` takes `null` to the
/// `default` where no case is `null`'s (#221). Not where a `break;` of no
/// label in a case leaves the `switch` for that rest. A rest that breaks
/// its loop never comes here: its `Some`s' way past it makes the `switch`
/// a labeled block's, whose `default` jumps, or one of `continue`s with no
/// `if` of its `null` (ADR 0629, amended).
pub(super) fn some_switch_defaults(body: &mut Vec<Stmt>, switches: &[Switch]) {
    let Some(at) = body.iter().position(|s| match &s.kind {
        StmtKind::If(test, then, None) => match &then[..] {
            [
                switch @ Stmt {
                    kind: StmtKind::Switch { cases, default, .. },
                    ..
                },
            ] => {
                default.as_deref().is_none_or(|d| d.is_empty())
                    && js::null_test(test, true).is_some_and(|x| switch_of(switch, x, switches))
                    && !cases.iter().any(|case| js::breaks(&case.body, None))
            }
            _ => false,
        },
        _ => false,
    }) else {
        return;
    };
    if at + 1 == body.len() {
        return;
    }
    let rest = body.split_off(at + 1);
    let StmtKind::If(_, then, _) = &mut body[at].kind else {
        unreachable!("found")
    };
    let mut switch = then.pop().expect("found");
    if let StmtKind::Switch { default, .. } = &mut switch.kind {
        *default = Some(rest);
    }
    body[at] = switch;
}

/// Whether `stmt` is a `switch` of the variable `x`, each case a literal,
/// or a constant of a `match` whose each is a number, a string or a
/// registry symbol (`Switch::valued`), which `null` and `undefined`
/// aren't, and a `default` that does nothing: one that does nothing of
/// what's nullish.
pub(super) fn switch_of(stmt: &Stmt, x: &Expr, switches: &[Switch]) -> bool {
    let StmtKind::Switch {
        discriminant,
        cases,
        default,
    } = &stmt.kind
    else {
        return false;
    };
    let valued = (switches.iter()).any(|s| s.span == stmt.span && !stmt.span.is_none() && s.valued);
    let literal = |v: &Expr| {
        matches!(
            v.kind,
            ExprKind::Str(_) | ExprKind::Num(_) | ExprKind::Bool(_) | ExprKind::BigInt(_) | ExprKind::BigUint(_)
        )
    };
    matches!(&x.kind, ExprKind::Var(_))
        && js::same_path(discriminant, x)
        && default.as_deref().is_none_or(does_nothing)
        && (valued || (cases.iter()).all(|case| case.values.iter().all(literal)))
}
