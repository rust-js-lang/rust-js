//! A `while (true)` each of whose turns ends giving its variable its next
//! value and testing it, as a `do { .. } while (test)` (ADR 0978).

use crate::js::{self, Expr, ExprKind, Stmt, StmtKind};

/// `while (true) { if (a) { ..; if (T(e)) { x = e; continue; } } else {
/// ..; const next = e2; if (T(next)) { x = next; continue; } } break; }`,
/// Rust's `loop` of a circular list, each arm advancing to the next item
/// where it isn't the first, `match next { Some(n) if !ptr_eq(..) => update
/// = n, _ => break }`: `do { if (a) { ..; x = e; } else { ..; x = e2; } }
/// while (T(x));`, as React's updateReducerImpl walks its queue. Each turn
/// runs the same: what's before the advance, the value, the test, as the
/// test given the value first is the test of the variable given it. Only
/// where:
///
/// - each way through the body ends in such an advance, `T` the same of
///   each, reading the variable nowhere but in what it's given;
/// - each advance's value reads the same twice (`reads_same`), or is a
///   `const` made right before it, read nowhere else;
/// - no other `continue` of the loop is in it, which in a `do` would go
///   on to its test, not its start; a `break` leaves either alike;
/// - the variable is its block's `let`, before the loop, which nothing
///   after the loop names, nor a function in it: the `do`'s last advance
///   gives it the value the test then refuses, which Rust's never does.
pub(super) fn do_whiles(stmts: &mut [Stmt]) {
    for i in 0..stmts.len() {
        let (before, rest) = stmts.split_at(i);
        if let Some(looped) = do_while(before, &rest[0], &rest[1..]) {
            stmts[i].kind = looped;
        }
    }
}

/// An advance at the end of a way through the loop: what's tested, given
/// the variable, and the statements it's written as.
struct Advance {
    variable: String,
    test: Expr,
}

fn do_while(before: &[Stmt], looped: &Stmt, after: &[Stmt]) -> Option<StmtKind> {
    let StmtKind::While { label, cond, body } = &looped.kind else {
        return None;
    };
    if !matches!(cond.kind, ExprKind::Bool(true)) {
        return None;
    }
    let (last, turn) = body.split_last()?;
    let ours = |l: &Option<String>| l.is_none() || l == label;
    if !matches!(&last.kind, StmtKind::Break(l) if ours(l)) {
        return None;
    }
    let mut turn = turn.to_vec();
    let mut advances = Vec::new();
    advanced(&mut turn, label.as_deref(), &mut advances)?;
    let (first, others) = advances.split_first()?;
    let variable = &first.variable;
    let same = |a: &Expr, b: &Expr| {
        js::stmts_alike(
            &[StmtKind::Expr(a.clone()).at(a.span)],
            &[StmtKind::Expr(b.clone()).at(b.span)],
        )
    };
    let declared = before
        .iter()
        .any(|s| matches!(&s.kind, StmtKind::Let(n, _) if n == variable));
    if !declared
        || others
            .iter()
            .any(|a| a.variable != *variable || !same(&a.test, &first.test))
        || js::continues(&turn, label.as_deref())
        || js::mentions_in(after, variable) > 0
        || in_function(&turn, variable)
    {
        return None;
    }
    Some(StmtKind::DoWhile {
        label: label.clone(),
        body: turn,
        cond: first.test.clone(),
    })
}

/// Of `stmts`, a way through the loop, each advance it ends in, written as
/// its assignment alone, `x = e;`: `None` where one way doesn't end in one.
fn advanced(stmts: &mut Vec<Stmt>, label: Option<&str>, advances: &mut Vec<Advance>) -> Option<()> {
    let last = stmts.last_mut()?;
    match &mut last.kind {
        StmtKind::If(_, then, Some(otherwise)) => {
            advanced(then, label, advances)?;
            advanced(otherwise, label, advances)
        }
        StmtKind::If(test, then, None) => {
            let [
                Stmt {
                    kind: StmtKind::Assign(target, value),
                    ..
                },
                Stmt {
                    kind: StmtKind::Continue(l),
                    ..
                },
            ] = then.as_slice()
            else {
                return None;
            };
            let ExprKind::Var(variable) = &target.kind else {
                return None;
            };
            if !(l.is_none() || l.as_deref() == label) {
                return None;
            }
            let (variable, value, test) = (variable.clone(), value.clone(), test.clone());
            // Its value a `const` made right before it, read nowhere else:
            // what made it, given in its place.
            let at = stmts.len() - 1;
            let made = match &value.kind {
                ExprKind::Var(name) if at > 0 => match &stmts[at - 1].kind {
                    StmtKind::Const(n, made) if n == name => Some(made.clone()),
                    _ => None,
                },
                _ => None,
            };
            if made.is_none() && !value.reads_same() {
                return None;
            }
            let test = given(&test, &value, &variable)?;
            let span = stmts[at].span;
            let given = made.clone().unwrap_or(value);
            stmts[at] = StmtKind::Assign(Expr::var(&variable), given).at(span);
            if made.is_some() {
                stmts.remove(at - 1);
            }
            advances.push(Advance { variable, test });
            Some(())
        }
        _ => None,
    }
}

/// `T(x)` of `test`, `T(value)`: each read of `value` in it, `variable`.
/// `None` where it reads `variable` otherwise, which the `do`'s test would
/// read after it's given its next value, where the advance read it before.
fn given(test: &Expr, value: &Expr, variable: &str) -> Option<Expr> {
    const HOLE: &str = "$given";
    let mut holed = test.clone();
    holed.each_mut(&mut |e| {
        if js::same_path(e, value) {
            *e = Expr::var(HOLE).or_at(e.span);
        }
    });
    if holed.mentions_var(variable) || !holed.mentions_var(HOLE) {
        return None;
    }
    holed.each_mut(&mut |e| {
        if matches!(&e.kind, ExprKind::Var(v) if v == HOLE) {
            *e = Expr::var(variable).or_at(e.span);
        }
    });
    Some(holed)
}

/// Whether a function in `stmts`, declared or a value, names `name`.
fn in_function(stmts: &[Stmt], name: &str) -> bool {
    let mut named = false;
    js::each_list(stmts, &mut |list| {
        named |= (list.iter()).any(|s| matches!(&s.kind, StmtKind::Function(f) if js::mentions_in(&f.body, name) > 0));
    });
    js::each_expr(stmts, &mut |e| {
        named |= match &e.kind {
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => js::mentions_in(body, name) > 0,
            ExprKind::Function(function) => js::mentions_in(&function.body, name) > 0,
            _ => false,
        };
    });
    named
}
