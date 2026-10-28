//! Tidy's `Option`s: one made where it's tested is its test (ADR 0910).

use super::*;

/// An `Option` made where it's tested, `const x = t ? v : undefined; if
/// (x) { a } else { b }`, of a value `v` that's there (`Known::Defined`):
/// `if (t) { a } else { b }`, `x` read as `v` in `a`, as `if let Some(x) =
/// kind.object()` is upstream's `if (typeof value === "object" && value
/// !== null)` (ADR 0910). Its test is the `Option`'s presence, `!!x`, true
/// exactly where it's set (`Known::Presence`), which is where `t` holds,
/// `v` being there; or `!x`, of a branch that ends, `x` read past it. `v`
/// is a variable nothing changes, so it's what `x` was wherever `x` is
/// read, a closure's later too; `x` is read nowhere else, and neither is
/// declared in what reads `x`, which would read another.
/// Past what's only declared between, `let typeofNextType;` of what the
/// test's branches set.
pub(super) fn tested_options(stmts: &mut Vec<Stmt>, changing: &HashSet<String>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let declared = |s: &Stmt| matches!(s.kind, StmtKind::Let(_, None));
        let at = i + 1 + stmts[i + 1..].iter().take_while(|s| declared(s)).count();
        if at < stmts.len()
            && let Some((tested, rest)) = tested_option(&stmts[i], &stmts[at], &stmts[at + 1..], changing)
        {
            stmts[at].kind = tested;
            if let Some(rest) = rest {
                stmts.splice(at + 1.., rest);
            }
            stmts.remove(i);
        }
        i += 1;
    }
}

/// `tested_options`'s rewrite of `made`, the `Option`'s `const`, and
/// `tested`, its test, before `rest`: the `if` of where `t` holds, and
/// `rest`, where it read `x`, of `v` in its place.
fn tested_option(
    made: &Stmt,
    tested: &Stmt,
    rest: &[Stmt],
    changing: &HashSet<String>,
) -> Option<(StmtKind, Option<Vec<Stmt>>)> {
    let StmtKind::Const(x, made) = &made.kind else {
        return None;
    };
    let ExprKind::Cond(test, value, none) = &made.kind else {
        return None;
    };
    let ExprKind::Var(v) = &value.kind else { return None };
    if !matches!(none.kind, ExprKind::Undefined) || !value.facts.has(js::Known::Defined) || changing.contains(v) {
        return None;
    }
    let StmtKind::If(presence, then, otherwise) = &tested.kind else {
        return None;
    };
    let of_x = |e: &Expr| matches!(&e.kind, ExprKind::Var(o) if o == x);
    // `x != null`, where it's set, or `x == null`, where it isn't.
    let set = match (js::null_test(presence, true), js::null_test(presence, false)) {
        (Some(option), _) if of_x(option) => true,
        (_, Some(option)) if of_x(option) => false,
        _ => return None,
    };
    // What reads `x`, and what mustn't: of `if (!x) { b } rest`, `rest`,
    // past a `b` that ends, as a `let .. else`'s does.
    let (reads, unread): (&[Stmt], [&[Stmt]; 2]) = match (set, otherwise) {
        (true, _) => (then, [otherwise.as_deref().unwrap_or_default(), rest]),
        (false, None) if then.last().is_some_and(ends) => (rest, [then, &[]]),
        (false, _) => return None,
    };
    let read_elsewhere = unread.iter().any(|stmts| js::mentions_in(stmts, x) > 0) || test.mentions_var(x);
    if read_elsewhere || js::declares(reads, x) || js::declares(reads, v) {
        return None;
    }
    let mut reads = reads.to_vec();
    js::each_expr_mut(&mut reads, &mut |e| {
        if of_x(e) {
            *e = (**value).clone();
        }
    });
    Some(match set {
        true => (StmtKind::If((**test).clone(), reads, otherwise.clone()), None),
        false => (
            StmtKind::If(test.clone().negated_test(), then.clone(), None),
            Some(reads),
        ),
    })
}
