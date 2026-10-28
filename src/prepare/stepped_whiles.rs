//! A `while` that counts its variable last, as a `for` that steps it (ADR
//! 0974).

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind};

/// `let i = 0; while (i < n) { ..; i++; }`: `for (let i = 0; i < n; i++)
/// { .. }`, as React counts. Each runs the same: the start, then the test,
/// the body and the step, in turn. A walk, `child = child.sibling`, stays
/// the `while` React writes it as (ADR 0974, amended). Only where:
///
/// - the `let` is right before the loop, and nothing after the loop in its
///   block names the variable, which the `for` keeps to itself;
/// - the test reads it, and the body's last statement alone counts it,
///   `i++`, `at -= 2`, of no label's `continue` of the loop before it,
///   which in a `for` would run the step it skips in the `while`; and by
///   what the body doesn't declare, which the step, after the body's
///   block, can't see;
/// - no function in the loop names it: a `for`'s `let` is one variable for
///   each turn, which a closure made in a turn keeps, where the `while`'s
///   is one for all.
pub(super) fn stepped_whiles(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        if let Some(looped) = stepped(&stmts[i], &stmts[i + 1], &stmts[i + 2..]) {
            stmts[i].kind = looped;
            stmts.remove(i + 1);
        }
        i += 1;
    }
}

/// The `for` of `declared` and `looped`, a `let` and a `while`, followed in
/// their block by `after`.
fn stepped(declared: &Stmt, looped: &Stmt, after: &[Stmt]) -> Option<StmtKind> {
    let StmtKind::Let(name, Some(start)) = &declared.kind else {
        return None;
    };
    let StmtKind::While { label, cond, body } = &looped.kind else {
        return None;
    };
    let (last, rest) = body.split_last()?;
    // Its count: `name += value` and `name -= value`, or `name++` and
    // `name--`, as the printer writes a statement of a step of one; not a
    // walk's, `child = child.sibling`, nor a bit's, `rest &= rest - 1`,
    // which React writes as `while`s.
    let one = |e: &Expr| matches!(e.kind, ExprKind::Num(n) if n == 1.0);
    let (target, step) = match &last.kind {
        StmtKind::Compound(Op::Add, target, by) if one(by) => (target, js::Step::Up),
        StmtKind::Compound(Op::Sub, target, by) if one(by) => (target, js::Step::Down),
        StmtKind::Compound(op @ (Op::Add | Op::Sub), target, by) => (target, js::Step::By(*op, by.clone())),
        _ => return None,
    };
    if !matches!(&target.kind, ExprKind::Var(v) if v == name)
        || !cond.mentions_var(name)
        || js::mentions_in(after, name) > 0
        || js::continues(rest, label.as_deref())
        || sets(rest, name)
        || in_function(body, name)
        || function_names(cond, name)
        || step
            .value()
            .is_some_and(|value| function_names(value, name) || reads_declared(value, rest))
    {
        return None;
    }
    Some(StmtKind::For {
        label: label.clone(),
        name: name.clone(),
        start: start.clone(),
        test: cond.clone(),
        step,
        body: rest.to_vec(),
    })
}

/// Whether `value` reads a name `stmts` declare: the body's, which a
/// `for`'s step, after the body's block, can't see, `i += stride` of a
/// `const stride = ..;` in it.
fn reads_declared(value: &Expr, stmts: &[Stmt]) -> bool {
    let mut declared = false;
    value.visit_vars(&mut |v| declared |= js::declares(stmts, v));
    declared
}

/// Whether `stmts` set `name`, or may: `name = ..`, `name += ..`, a handle
/// on it (`js::Writes`), or in an expression, `(name = ..)`, `name++`,
/// however deep.
fn sets(stmts: &[Stmt], name: &str) -> bool {
    let mut set = js::Writes::of(stmts).may_replace(name);
    js::each_expr(stmts, &mut |e| {
        set |= matches!(&e.kind, ExprKind::Assigned(t, _) | ExprKind::Stepped(_, t)
            if matches!(&t.kind, ExprKind::Var(v) if v == name));
    });
    set
}

/// Whether a function in `stmts`, declared or a value, names `name`.
fn in_function(stmts: &[Stmt], name: &str) -> bool {
    let mut named = false;
    js::each_list(stmts, &mut |list| {
        named |= (list.iter()).any(|s| matches!(&s.kind, StmtKind::Function(f) if js::mentions_in(&f.body, name) > 0));
    });
    js::each_expr(stmts, &mut |e| named |= function_names(e, name));
    named
}

/// Whether `e` is, or holds, a function that names `name`.
fn function_names(e: &Expr, name: &str) -> bool {
    let mut named = false;
    e.each(&mut |e| {
        named |= match &e.kind {
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => js::mentions_in(body, name) > 0,
            ExprKind::Function(function) => js::mentions_in(&function.body, name) > 0,
            _ => false,
        };
    });
    named
}
