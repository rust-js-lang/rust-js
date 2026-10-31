//! A `while (true)` each of whose turns ends giving its variable its next
//! value and testing it, or ends testing whether to leave, as a `do { .. }
//! while (test)` (ADR 0978).

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
    let ours = |l: &Option<String>| l.is_none() || l == label;
    if let Some(looped) = tested_last(label, body, looped.span) {
        return Some(looped);
    }
    let mut body = body.clone();
    ended_by_advance(&mut body, label.as_deref());
    let (last, turn) = body.split_last()?;
    if !matches!(&last.kind, StmtKind::Break(l) if ours(l)) {
        return None;
    }
    let mut turn = turn.to_vec();
    let mut advances = Vec::new();
    advanced(&mut turn, label.as_deref(), &mut advances)?;
    // An advance before a way's end, `if (T(e)) { x = e; continue; }
    // break;`: `x = e; continue;`, whose `continue` goes on to the `do`'s
    // test, the advance's (ADR 0978, amended).
    turned(&mut turn, label.as_deref(), &mut advances)?;
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
    js::statement_lists(&mut turn, &mut |list| {
        for stmt in list.iter_mut() {
            if matches!(&stmt.kind, StmtKind::Directive(d) if d == ADVANCED) {
                stmt.kind = StmtKind::Continue(None);
            }
        }
    });
    Some(StmtKind::DoWhile {
        label: label.clone(),
        body: turn,
        cond: first.test.clone(),
    })
}

/// `while (true) { ..; if (done) { break; } }`, a body that ends testing
/// whether to leave: `do { .. } while (!done);` (ADR 0978, amended). Each
/// turn runs the same, its statements, then the test. Only where no other
/// `continue` of the loop is in it, which in a `do` would go on to its
/// test, not its start; and the test reads nothing the body declares,
/// which is out of the `do`'s test's scope. Comments above the `if`, or
/// before its `break`, are the body's last, above the test, `// Past the
/// end.\n } while (..)`, as a block that ends in comments (ADR 0753).
fn tested_last(label: &Option<String>, body: &[Stmt], looped: js::Span) -> Option<StmtKind> {
    let ours = |l: &Option<String>| l.is_none() || l == label;
    let (last, turn) = body.split_last()?;
    let StmtKind::If(done, then, None) = &last.kind else {
        return None;
    };
    if !matches!(&then[..], [Stmt { kind: StmtKind::Break(l), .. }] if ours(l))
        || js::continues(turn, label.as_deref())
        || js::declared_names(turn).iter().any(|name| done.mentions_var(name))
    {
        return None;
    }
    let mut body = turn.to_vec();
    // Where the `if` and its `break` were, in the loop, past what's before:
    // the comments before each, there.
    let past = (turn.iter().filter(|s| !s.span.is_none()).map(|s| s.span.lo)).max();
    let within = |s: &js::Span| !s.is_none() && s.lo > looped.lo && s.lo < looped.hi && past.is_none_or(|p| s.lo > p);
    let ends = [last.span]
        .into_iter()
        .chain(then.iter().map(|s| s.span))
        .filter(within);
    body.extend(ends.map(|s| StmtKind::Comments.at(js::Span { lo: s.lo, hi: s.lo + 1 })));
    Some(StmtKind::DoWhile {
        label: label.clone(),
        body,
        cond: done.clone().negated_test(),
    })
}

/// Where an advance before a way's end goes on to the `do`'s test, while
/// the others' `continue`s are checked: its `continue`, once they are.
const ADVANCED: &str = "an advance's continue";

/// A loop's body that ends advancing past a test of what it leaves on,
/// `if (next == null || next === first) { break; } x = next;`: `if (next
/// != null && next !== first) { x = next; continue; } break;`, as each way
/// that advances inside it is written (ADR 0978, amended).
fn ended_by_advance(body: &mut Vec<Stmt>, label: Option<&str>) {
    let [.., left, given] = &body[..] else {
        return;
    };
    let StmtKind::If(test, then, None) = &left.kind else {
        return;
    };
    let ours = |l: &Option<String>| l.is_none() || l.as_deref() == label;
    if !matches!(&then[..], [Stmt { kind: StmtKind::Break(l), .. }] if ours(l))
        || !matches!(&given.kind, StmtKind::Assign(target, _) if matches!(target.kind, ExprKind::Var(_)))
    {
        return;
    }
    let (test, span) = (test.clone().negated_test(), left.span);
    let given = body.pop().expect("its advance");
    let last = body.len() - 1;
    let advance = vec![given, StmtKind::Continue(None).at(js::Span::NONE)];
    body[last] = StmtKind::If(test, advance, None).at(span);
    body.push(StmtKind::Break(None).at(js::Span::NONE));
}

/// Each advance of `stmts` before a way's end, `if (T(e)) { x = e;
/// continue; } break;`, in their branches, not a loop's or a `switch`'s of
/// their own, whose `break` is theirs: `x = e;` and the `continue` it goes
/// on to the test by, `ADVANCED` until the others are checked.
fn turned(stmts: &mut Vec<Stmt>, label: Option<&str>, advances: &mut Vec<Advance>) -> Option<()> {
    let ours = |l: &Option<String>| l.is_none() || l.as_deref() == label;
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::If(_, then, otherwise) = &mut stmts[i].kind {
            turned(then, label, advances)?;
            if let Some(otherwise) = otherwise {
                turned(otherwise, label, advances)?;
            }
        }
        let advancing = matches!(&stmts[i].kind, StmtKind::If(_, then, None)
            if matches!(&then[..], [Stmt { kind: StmtKind::Assign(..), .. }, Stmt { kind: StmtKind::Continue(l), .. }] if ours(l)));
        if advancing && matches!(stmts.get(i + 1).map(|s| &s.kind), Some(StmtKind::Break(l)) if ours(l)) {
            let mut way = stmts[..=i].to_vec();
            advanced(&mut way, label, advances)?;
            let at = way.len();
            way.push(StmtKind::Directive(ADVANCED.to_string()).at(js::Span::NONE));
            stmts.splice(..i + 2, way);
            i = at;
        }
        i += 1;
    }
    Some(())
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
