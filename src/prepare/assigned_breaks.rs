//! A loop's way out that gives a variable declared before it what a test
//! read, as JS writes it: `if ((next = node.nextSibling) !== null) {
//! break; }` (rd148).

use crate::js::{self, Expr, ExprKind, Stmt, StmtKind};

/// `let v; while (true) { ..; const t = e; if (t !== null) { v = t; break;
/// } .. }`, the loop's value given at its one way out, `let next = loop {
/// .. if let Some(next) = node.next_sibling() { break next; } .. }`: `if
/// ((v = e) !== null) { break; }`, as React's getModernOffsetsFromPoints
/// walks to a next sibling. Each turn reads `e` once, in the same place;
/// `v` is given the value the test refuses too, which nothing reads: only
/// where `v` is a `let` of no value before the loop that nothing in it
/// names but the way out, so what follows reads it only where Rust's
/// `break` gave it, as rustc holds; and `t` is read first in the test,
/// once, and nowhere else (ADR 0702, amended).
pub(super) fn assigned_breaks(stmts: &mut [Stmt]) {
    for i in 0..stmts.len() {
        let (before, rest) = stmts.split_at_mut(i);
        let StmtKind::While { label, cond, body } = &mut rest[0].kind else {
            continue;
        };
        if !matches!(cond.kind, ExprKind::Bool(true)) {
            continue;
        }
        // The loop without its way out, where it was.
        let mut others = body.clone();
        let Some((variable, test, span)) = taken(&mut others, label.as_deref()) else {
            continue;
        };
        let declared = (before.iter()).any(|s| matches!(&s.kind, StmtKind::Let(n, None) if *n == variable));
        if !declared || js::mentions_in(&others, &variable) > 0 {
            continue;
        }
        let way_out = StmtKind::If(test, vec![StmtKind::Break(None).at(js::Span::NONE)], None).at(span);
        js::statement_lists(&mut others, &mut |list| {
            for stmt in list.iter_mut() {
                if matches!(&stmt.kind, StmtKind::Directive(d) if d == WAY_OUT) {
                    *stmt = way_out.clone();
                }
            }
        });
        *body = others;
    }
}

/// Where the way out `taken` took was, until it's put back.
const WAY_OUT: &str = "an assigned break";

/// The first way out of `list`, a loop's body, `assigned_at`'s, in it or
/// in its `if`s' branches, taken out, `WAY_OUT` in its place: its
/// variable, its test and where it's written.
fn taken(list: &mut Vec<Stmt>, label: Option<&str>) -> Option<(String, Expr, js::Span)> {
    for k in 0..list.len() {
        if k + 1 < list.len()
            && let Some((variable, test)) = assigned_at(list, k, label)
        {
            let span = list[k + 1].span;
            list.splice(k..k + 2, [StmtKind::Directive(WAY_OUT.to_string()).at(js::Span::NONE)]);
            return Some((variable, test, span));
        }
        if let StmtKind::If(_, then, otherwise) = &mut list[k].kind {
            if let Some(found) = taken(then, label) {
                return Some(found);
            }
            if let Some(found) = otherwise.as_mut().and_then(|o| taken(o, label)) {
                return Some(found);
            }
        }
    }
    None
}

/// Of `body`'s `const t = e; if (T(t)) { v = t; break; }` at `k`: `v`, and
/// `T((v = e))`, where `T` reads `t` first, once, and nothing after reads
/// it.
fn assigned_at(body: &[Stmt], k: usize, label: Option<&str>) -> Option<(String, Expr)> {
    let [
        Stmt {
            kind: StmtKind::Const(made, value),
            ..
        },
        Stmt {
            kind: StmtKind::If(test, then, None),
            ..
        },
    ] = &body[k..k + 2]
    else {
        return None;
    };
    let [
        Stmt {
            kind: StmtKind::Assign(target, given),
            ..
        },
        Stmt {
            kind: StmtKind::Break(l),
            ..
        },
    ] = then.as_slice()
    else {
        return None;
    };
    let (ExprKind::Var(variable), ExprKind::Var(read)) = (&target.kind, &given.kind) else {
        return None;
    };
    let tested_first = matches!(&test.kind, ExprKind::Binary(_, a, b)
        if matches!(&a.kind, ExprKind::Var(t) if t == made) && !b.mentions_var(made));
    if read != made
        || !(l.is_none() || l.as_deref() == label)
        || !tested_first
        || js::mentions_in(&body[k + 2..], made) > 0
    {
        return None;
    }
    let assigned = Expr::assigned(Expr::var(variable), value.clone());
    let test = test.substitute(&|n: &str| (n == made).then(|| assigned.clone()))?;
    Some((variable.clone(), test))
}
