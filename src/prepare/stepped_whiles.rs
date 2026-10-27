//! A `while` that gives its variable its next value last, as a `for` that
//! steps it (ADR 0974).

use crate::js::{self, Expr, ExprKind, Op, Stmt, StmtKind};

/// `let child = fiber.child; while (child != null) { ..; child =
/// child.sibling; }`, Rust's `while let Some(c) = child { ..; child =
/// c.sibling.clone(); }`: `for (let child = fiber.child; child != null;
/// child = child.sibling) { .. }`, as React walks a fiber's children. Each
/// runs the same: the start, then the test, the body and the step, in
/// turn. Only where:
///
/// - the `let` is right before the loop, and nothing after the loop in its
///   block names the variable, which the `for` keeps to itself;
/// - the test reads it, and the body's last statement alone sets it, of
///   its own value, `child.sibling` of `child`, `i + 1` of `i`, of no
///   label's `continue` of the loop before it, which in a `for` would run
///   the step it skips in the `while`; and of what the body doesn't
///   declare, which the step, after the body's block, can't see;
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
    // Its step: `name = value` and `name -= value`, or `name++` and
    // `name--`, as the printer writes a statement of a step of one; not a
    // bit's, `rest &= rest - 1`, which React steps in a `while`.
    let one = |e: &Expr| matches!(e.kind, ExprKind::Num(n) if n == 1.0);
    let (target, step) = match &last.kind {
        StmtKind::Assign(target, value) => (target, js::Step::To(value.clone())),
        StmtKind::Compound(Op::Add, target, by) if one(by) => (target, js::Step::Up),
        StmtKind::Compound(Op::Sub, target, by) if one(by) => (target, js::Step::Down),
        StmtKind::Compound(op @ (Op::Add | Op::Sub), target, by) => (target, js::Step::By(*op, by.clone())),
        _ => return None,
    };
    // Of the variable, its next value of itself, `child = child.sibling`,
    // not of another, `parent = node.return`, which upstream React walks
    // with a `while`.
    let stepped = matches!(&target.kind, ExprKind::Var(v) if v == name)
        && !matches!(&step, js::Step::To(value) if !value.mentions_var(name));
    if !stepped
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
/// `for`'s step, after the body's block, can't see, `child = current.sibling`
/// of a `const current = child;` in it.
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
