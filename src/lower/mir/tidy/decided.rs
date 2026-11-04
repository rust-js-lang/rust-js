//! Tidy's decided tests: a test an earlier one decides, its branch taken
//! in its place, and a value it says is there read as it is (ADR 0564,
//! ADR 0620).

use super::*;

/// A test an earlier one decided, of what nothing changes between: in `if
/// (t) { a } else { b }`, `t` holds through `a` and fails through `b`, and
/// where `a` leaves, fails through what follows the `if` (where `b` leaves,
/// holds); so an `if (t)` there is its `then`, or its `else`, and an `if
/// (!t)` the other way. A std body's drop of what its early return gave
/// away, `if (option != null) { return option; } .. if (option != null) {
/// drop }`, and a drop's test inside its own (ADR 0564). `t` of what nothing
/// assigns, `changing`'s; one that reads through a field or a helper,
/// `x.value`, `$someValue(x)`, only up to the next call or assignment
/// through a field, which may change it. Not an `if` the program wrote, of
/// a condition in `written`: what it says (ADR 0564, amended).
pub(super) fn decided_tests(stmts: &mut Vec<Stmt>, changing: &HashSet<String>, written: &[js::Span]) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::If(test, then, otherwise) = &stmts[i].kind else {
            i += 1;
            continue;
        };
        if !decidable(test, changing) {
            i += 1;
            continue;
        }
        let test = test.clone();
        let then_leaves = then.last().is_some_and(ends);
        let else_leaves = otherwise.as_ref().is_some_and(|o| o.last().is_some_and(ends));
        let StmtKind::If(_, then, otherwise) = &mut stmts[i].kind else {
            unreachable!("matched")
        };
        decided_in(then, 0, (&test, written), true, true);
        if let Some(otherwise) = otherwise {
            decided_in(otherwise, 0, (&test, written), false, true);
        }
        match (then_leaves, else_leaves) {
            (true, false) => decided_in(stmts, i + 1, (&test, written), false, false),
            (false, true) => decided_in(stmts, i + 1, (&test, written), true, false),
            _ => {}
        }
        i += 1;
    }
}

/// Whether `test` can be decided by an earlier reading of it: it does
/// nothing, isn't a constant, and reads no variable something assigns; or
/// reads through a field, which anything that does something disturbs
/// (`disturbs_any`), a closure's assignment too, as only a call runs it.
fn decidable(test: &Expr, changing: &HashSet<String>) -> bool {
    let mut stable = !test.has_effects() && !test.is_constant();
    if test.reads_only_vars() {
        test.visit_vars(&mut |v| stable &= !changing.contains(v));
    }
    stable
}

/// `stmts` from `from` on, where `test` is known to hold, or to fail: each
/// `if` of it, or of its negation, its branch taken, up to what may change
/// what `test` reads; and, in a branch of the `if` of `test`, its `parts`,
/// each part of a chain it decides left out (ADR 0743). Not past the `if`,
/// where a `match`'s arms are tested one after another, each test whole,
/// which a `switch` of them reads (ADR 0668).
fn decided_in(stmts: &mut Vec<Stmt>, from: usize, (test, written): (&Expr, &[js::Span]), holds: bool, parts: bool) {
    let wrote = |t: &Expr| !t.span.is_none() && written.iter().any(|c| c.lo < t.span.hi && t.span.lo < c.hi);
    let mut j = from;
    while j < stmts.len() {
        match &mut stmts[j].kind {
            StmtKind::If(t, ..) if disturbs(t, test) => return,
            StmtKind::If(t, ..) if let Some(taken) = decided(t, test).filter(|_| !wrote(t)) => {
                let StmtKind::If(_, then, otherwise) = stmts.remove(j).kind else {
                    unreachable!("matched")
                };
                let branch = match taken == holds {
                    true => then,
                    false => otherwise.unwrap_or_default(),
                };
                stmts.splice(j..j, branch);
                // What the branch had is where it was, known the same there.
            }
            StmtKind::If(t, ..) if parts && let Some(folded) = decided_part(t, test, holds) => {
                // A part of its chain decided, `a || x` where `x` fails: `a`,
                // looked at again (ADR 0743).
                let StmtKind::If(t, ..) = &mut stmts[j].kind else {
                    unreachable!("matched")
                };
                *t = folded;
            }
            StmtKind::If(_, then, otherwise) => {
                decided_in(then, 0, (test, written), holds, parts);
                let mut disturbed = disturbs_any(then, test);
                if let Some(otherwise) = otherwise {
                    decided_in(otherwise, 0, (test, written), holds, parts);
                    disturbed |= disturbs_any(otherwise, test);
                }
                if disturbed {
                    return;
                }
                j += 1;
            }
            StmtKind::Labeled(_, body) => {
                decided_in(body, 0, (test, written), holds, parts);
                if disturbs_any(body, test) {
                    return;
                }
                j += 1;
            }
            _ => match disturbs_any(std::slice::from_ref(&stmts[j]), test) {
                // What it reads first, before what it does, `const sibling =
                // $unwrap(node.sibling)`, read where the test is known.
                true => {
                    present_in(&mut stmts[j], test, holds, true);
                    return;
                }
                false => {
                    present_in(&mut stmts[j], test, holds, false);
                    j += 1;
                }
            },
        }
    }
}

/// Of `stmt`, where `test` is known to hold, or to fail, each `x ?? d` of
/// a variable it says is there, `typeof x === "function"` holding or `x ==
/// null` failing: `x`, which `d` never replaces (ADR 0620); and each
/// `$unwrap(p)` of a `p` it says is there that reads the same, a field read
/// again, `while (node.sibling != null) { .. $unwrap(node.sibling) }`: `p`
/// (ADR 0620, amended). Of one that `disturbs`, `first`, only the value it
/// reads before what it does, `const sibling = $unwrap(node.sibling)`, and
/// what it returns, a module variable's `return $unwrap(current)` after its
/// `if (current == null) { throw .. }` (rd443).
fn present_in(stmt: &mut Stmt, test: &Expr, holds: bool, first: bool) {
    let known = match holds {
        true => test.clone(),
        false => test.clone().negated_test(),
    };
    let there = |x: &Expr| implies(&known, &Expr::bin(Op::LooseNe, x.clone(), Expr::null()));
    let mut present = |e: &mut Expr| match &e.kind {
        ExprKind::Binary(Op::Coalesce, x, _) if !first && matches!(x.kind, ExprKind::Var(_)) && there(x) => {
            *e = (**x).clone().or_at(e.span);
        }
        _ => match crate::lower::std_types::option::unwrap_tested(e) {
            Some((_, option, _)) if option.reads_same() && there(option) => *e = option.clone().or_at(e.span),
            _ => {}
        },
    };
    match &mut stmt.kind {
        _ if !first => js::each_expr_mut(std::slice::from_mut(stmt), &mut present),
        StmtKind::Const(_, value)
        | StmtKind::Let(_, Some(value))
        | StmtKind::Expr(value)
        | StmtKind::Return(Some(value)) => present(value),
        StmtKind::Assign(target, value) if matches!(target.kind, ExprKind::Var(_)) => present(value),
        _ => {}
    }
}

/// Of `t`, a chain of `&&` and `||`, where `test` is known to hold, or to
/// fail: the chain without the parts of it `test` decides, `a ||
/// Array.isArray(v)` in an `else` of `Array.isArray(v)` its `a`; none where
/// no part of it is (ADR 0743). A part whose value is known is left out of
/// its chain only where it does nothing, as a test that's decided does not.
fn decided_part(t: &Expr, test: &Expr, holds: bool) -> Option<Expr> {
    fn fold(e: &Expr, test: &Expr, holds: bool, changed: &mut bool) -> Expr {
        let ExprKind::Binary(op @ (Op::And | Op::Or), a, b) = &e.kind else {
            return match decided(e, test) {
                Some(taken) => {
                    *changed = true;
                    Expr::bool(taken == holds).or_at(e.span)
                }
                None => e.clone(),
            };
        };
        let (a, b) = (fold(a, test, holds, changed), fold(b, test, holds, changed));
        // What `a op b` is where one side is known: `true || b` is `true`,
        // `false || b` is `b`, `a || false` is `a`; `&&` the other way.
        let known = |x: &Expr| match x.kind {
            ExprKind::Bool(v) => Some(v),
            _ => None,
        };
        let short = matches!(op, Op::Or);
        match (known(&a), known(&b)) {
            (Some(v), _) if v == short => a,
            (Some(_), _) => b,
            (_, Some(v)) if v != short => a,
            (_, Some(_)) if !a.has_effects() => b,
            _ => Expr::bin(*op, a, b).or_at(e.span),
        }
    }
    if !matches!(t.kind, ExprKind::Binary(Op::And | Op::Or, ..)) {
        return None;
    }
    let mut changed = false;
    let folded = fold(t, test, holds, &mut changed);
    changed.then_some(folded)
}

/// Whether `t` is `test`, `Some(true)`, or its negation, `Some(false)`.
fn decided(t: &Expr, test: &Expr) -> Option<bool> {
    let same = |a: &Expr, b: &Expr| same_test(a, b) || a.written_alike(b);
    if same(t, test) {
        return Some(true);
    }
    same(&t.clone().negated_test(), test).then_some(false)
}

/// Whether evaluating `e` may change what `test` reads: nothing, of a test
/// that reads only variables nothing assigns; of one that reads through a
/// field or a helper, anything that does something.
fn disturbs(e: &Expr, test: &Expr) -> bool {
    !test.reads_only_vars() && e.has_effects()
}

/// Whether running `stmts` may change what `test` reads: an assignment of a
/// variable it reads; and, of one that reads through a field or a helper,
/// an assignment through a field, or anything that does something.
fn disturbs_any(stmts: &[Stmt], test: &Expr) -> bool {
    let mut vars = Vec::new();
    test.visit_vars(&mut |v| vars.push(v.to_string()));
    if vars.iter().any(|v| js::Writes::sets(stmts, v)) {
        return true;
    }
    if test.reads_only_vars() {
        return false;
    }
    let mut stmts = stmts.to_vec();
    let mut disturbed = false;
    js::statement_lists(&mut stmts, &mut |list| {
        disturbed |= list.iter().any(|s| {
            matches!(&s.kind, StmtKind::Assign(target, _) | StmtKind::Compound(_, target, _) if !matches!(target.kind, ExprKind::Var(_)))
        });
    });
    js::each_expr_mut(&mut stmts, &mut |e| disturbed |= e.has_effects());
    disturbed
}
