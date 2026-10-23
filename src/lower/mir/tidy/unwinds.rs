//! The drops a panic runs as it unwinds (`mir/drops.rs`): each statement
//! that may throw while a value with a destructor is live is `try { s }
//! catch (error) { drop(x); throw error; }`, and the scope's end drops `x`
//! again. A run of them, to that end, is one `try { .. } finally { drop(x)
//! }`, as a person writes cleanup: the same drops, run in the same order on
//! either path (ADR 0437).

use crate::js::{self, Expr, ExprKind, Stmt, StmtKind};

/// `stmts`, each run of statements a value's drop ends, which a panic in
/// any of them drops too, one `try { .. } finally { .. }`. The values
/// made inside it, nested in turn, as `statement_lists` goes on into it.
pub(super) fn scoped_drops(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        if let Some(end) = scope_end(stmts, i) {
            let drop = stmts[end].clone();
            let mut body = Vec::new();
            for stmt in stmts.drain(i..end) {
                match stmt.kind {
                    StmtKind::TryCatch(inner, Some(error), mut handler) => {
                        // The drop before its `throw`, the scope's own.
                        handler.remove(handler.len() - 2);
                        match rethrows(&handler, &error) {
                            true => body.extend(inner),
                            false => body.push(StmtKind::TryCatch(inner, Some(error), handler).at(stmt.span)),
                        }
                    }
                    kind => body.push(Stmt { kind, ..stmt }),
                }
            }
            let span = drop.span;
            let rest = stmts.split_off(i + 1);
            stmts.truncate(i);
            // What a `try` of the run declared before it, `let x;`, and only
            // the run reads: in it.
            while let Some(Stmt {
                kind: StmtKind::Let(name, None),
                ..
            }) = stmts.last()
                && js::mentions_in(&rest, name) == 0
                && js::mentions_in(std::slice::from_ref(&drop), name) == 0
            {
                body.insert(0, stmts.pop().expect("a `let`"));
            }
            let hoisted = hoist_read_after(&mut body, &rest);
            stmts.extend(hoisted);
            i = stmts.len();
            stmts.push(StmtKind::Try(body, vec![drop]).at(span));
            stmts.extend(rest);
            continue;
        }
        i += 1;
    }
}

/// `let result; try { ..; result = e; } finally { .. } return result;`, what
/// a function gives made last in a `try` whose `finally` drops what's held:
/// `try { ..; return e; } finally { .. }`, which runs the `finally` once `e`
/// is made, as the original does (ADR 0588).
pub(super) fn returned_through_finally(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 2 < stmts.len() {
        let found = match (&stmts[i].kind, &stmts[i + 1].kind, &stmts[i + 2].kind) {
            (StmtKind::Let(name, None), StmtKind::Try(body, finally), StmtKind::Return(Some(value))) => matches!(&value.kind, ExprKind::Var(v) if v == name)
                && body.last().is_some_and(
                    |s| matches!(&s.kind, StmtKind::Assign(t, _) if matches!(&t.kind, ExprKind::Var(v) if v == name)),
                )
                && js::mentions_in(body, name) == 1
                && js::mentions_in(finally, name) == 0,
            _ => false,
        };
        if found {
            stmts.remove(i + 2);
            stmts.remove(i);
            let StmtKind::Try(body, _) = &mut stmts[i].kind else {
                unreachable!("matched")
            };
            let last = body.last_mut().expect("matched");
            if let StmtKind::Assign(_, value) = std::mem::replace(&mut last.kind, StmtKind::Return(None)) {
                last.kind = StmtKind::Return(Some(value));
            }
        }
        i += 1;
    }
}

/// Of a run that starts at `stmts[start]`, a `try` whose `catch` drops a
/// value last, then throws on: where the scope drops it, the statement
/// written as that drop, if every statement before it that may throw drops
/// it too and nothing leaves the run another way.
fn scope_end(stmts: &[Stmt], start: usize) -> Option<usize> {
    let drop = dropped_last(&stmts[start])?;
    for (at, stmt) in stmts.iter().enumerate().skip(start + 1) {
        if js::stmts_alike(std::slice::from_ref(stmt), std::slice::from_ref(drop)) {
            return Some(at);
        }
        match &stmt.kind {
            // Another value's drop last, one whose life crosses this one's:
            // not a scope. Lowering wraps this one's drop then, so no run is
            // found anyway; this keeps it so if that changes.
            StmtKind::TryCatch(..) => {
                let last = dropped_last(stmt)?;
                if !js::stmts_alike(std::slice::from_ref(last), std::slice::from_ref(drop)) {
                    return None;
                }
            }
            // One made of values, which can't throw, or the run ends.
            _ if !stmt.only_makes_values() => return None,
            _ => {}
        }
    }
    None
}

/// Of `try { .. } catch (error) { ..; drop; throw error; }`, `drop`.
fn dropped_last(stmt: &Stmt) -> Option<&Stmt> {
    let StmtKind::TryCatch(inner, Some(error), handler) = &stmt.kind else {
        return None;
    };
    let [.., drop, thrown] = handler.as_slice() else {
        return None;
    };
    let rethrown = matches!(&thrown.kind, StmtKind::Throw(Expr { kind: ExprKind::Var(v), .. }) if v == error);
    // One that leaves another way already dropped what it leaves; lowering
    // wraps no `return` in a `try`, but a `finally` would drop it again.
    (rethrown && !leaves(inner, 0) && js::mentions_in(std::slice::from_ref(drop), error) == 0).then_some(drop)
}

/// A `catch` that only throws on what it caught.
fn rethrows(handler: &[Stmt], error: &str) -> bool {
    matches!(handler, [Stmt { kind: StmtKind::Throw(Expr { kind: ExprKind::Var(v), .. }), .. }] if v == error)
}

/// Whether running `stmts` may leave them other than at their end or by a
/// throw: a `return`, or a `break` or `continue` of a loop or label outside
/// them, `depth` loops in.
fn leaves(stmts: &[Stmt], depth: usize) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Return(_) => true,
        StmtKind::Break(None) | StmtKind::Continue(None) => depth == 0,
        StmtKind::Break(Some(_)) | StmtKind::Continue(Some(_)) => true,
        StmtKind::If(_, a, b) => leaves(a, depth) || b.as_deref().is_some_and(|b| leaves(b, depth)),
        StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
            leaves(body, depth + 1)
        }
        StmtKind::Labeled(_, body) => leaves(body, depth),
        // Whose `break` of no label, none, would leave it alone (ADR 0629).
        StmtKind::Switch { cases, default, .. } => js::case_lists(cases, default).any(|body| leaves(body, depth)),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => leaves(a, depth) || leaves(b, depth),
        _ => false,
    })
}

/// What `body` declares that `after` reads: declared before it, `let x;`,
/// and assigned where it was made.
fn hoist_read_after(body: &mut Vec<Stmt>, after: &[Stmt]) -> Vec<Stmt> {
    let mut hoisted = Vec::new();
    for stmt in std::mem::take(body) {
        let read_after = |name: &str| js::mentions_in(after, name) > 0;
        match stmt.kind {
            StmtKind::Let(name, None) if read_after(&name) => hoisted.push(StmtKind::Let(name, None).at(stmt.span)),
            StmtKind::Const(name, value) | StmtKind::Let(name, Some(value)) if read_after(&name) => {
                hoisted.push(StmtKind::Let(name.clone(), None).at(stmt.span));
                body.push(StmtKind::Assign(Expr::var(&name), value).at(stmt.span));
            }
            kind => body.push(Stmt { kind, ..stmt }),
        }
    }
    hoisted
}
