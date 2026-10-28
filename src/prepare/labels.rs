//! A loop's labels at the end of preparation: a `try` of a closure's body
//! whose `return`s leave the loop, and the labels the program wrote that
//! nothing names now (the user's decision, 2026-10-28, wl4).

use crate::js::{self, ExprKind, Stmt, StmtKind};

/// Of `stmts`, each loop's: its `try`s' closures' jumps, then its label
/// where nothing names it, as tidy keeps the program's to here.
pub(super) fn loops(stmts: &mut [Stmt]) {
    tried_jumps(stmts);
    for stmt in stmts.iter_mut() {
        if let StmtKind::While { label, body, .. }
        | StmtKind::DoWhile { label, body, .. }
        | StmtKind::ForOf { label, body, .. }
        | StmtKind::For { label, body, .. } = &mut stmt.kind
            && label.as_deref().is_some_and(|l| !js::names_jump(body, l))
        {
            *label = None;
        }
    }
}

/// A loop's `try` whose body is a closure of `catch_unwind(|| ..)` called
/// where it's made, then the jump of its `Ok` arm, `try { (() => { ..;
/// return; .. })(); break; } catch ..`: the closure's body, each `return;`
/// of it the jump, `try { ..; break outer; .. break; } catch ..`, as
/// React's renderRootSync writes it (ADR 0660, amended, wl4). Each jump as
/// it is where nothing between its `return` and the `try` would take it,
/// a `switch` a `break`, a loop either; else of the loop's label, the
/// program's; and where the loop has none, the closure stays, as JS can't
/// say where it goes without a label the program didn't write.
fn tried_jumps(stmts: &mut [Stmt]) {
    for stmt in stmts.iter_mut() {
        if let StmtKind::While { label, body, .. }
        | StmtKind::DoWhile { label, body, .. }
        | StmtKind::ForOf { label, body, .. }
        | StmtKind::For { label, body, .. } = &mut stmt.kind
        {
            // A `try` that ends the body: what its closure falls to, and
            // so each `return;` of it, is the loop's next turn.
            if let Some(Stmt {
                kind: StmtKind::TryCatch(tried, ..),
                span,
            }) = body.last_mut()
                && let [
                    Stmt {
                        kind: StmtKind::Expr(_),
                        ..
                    },
                ] = &tried[..]
            {
                let mut next = tried.clone();
                next.push(StmtKind::Continue(None).at(*span));
                if let Some(mut inlined) = jumped(&next, label.as_deref()) {
                    if matches!(inlined.last().map(|s| &s.kind), Some(StmtKind::Continue(None))) {
                        inlined.pop();
                    }
                    *tried = inlined;
                }
            }
            in_loop(body, label.as_deref());
        }
    }
}

/// Of `stmts`, a loop's body, its `label`'s: each `try` an unlabeled jump
/// in it leaves the loop by.
fn in_loop(stmts: &mut [Stmt], label: Option<&str>) {
    for stmt in stmts.iter_mut() {
        match &mut stmt.kind {
            StmtKind::TryCatch(tried, ..) => {
                if let Some(body) = jumped(tried, label) {
                    *tried = body;
                }
            }
            StmtKind::If(_, then, otherwise) => {
                in_loop(then, label);
                if let Some(otherwise) = otherwise {
                    in_loop(otherwise, label);
                }
            }
            StmtKind::Labeled(_, body) => in_loop(body, label),
            _ => {}
        }
    }
}

/// `tried`'s body, `(() => { .. })(); jump;`, a closure of no value whose
/// each `return;` is `jump`, where `label` says each where it must.
fn jumped(tried: &[Stmt], label: Option<&str>) -> Option<Vec<Stmt>> {
    let [
        Stmt {
            kind: StmtKind::Expr(call),
            ..
        },
        jump,
    ] = tried
    else {
        return None;
    };
    if !matches!(jump.kind, StmtKind::Break(_) | StmtKind::Continue(_)) {
        return None;
    }
    let ExprKind::Call(callee, args) = &call.kind else {
        return None;
    };
    let ExprKind::Arrow(params, body) = &callee.kind else {
        return None;
    };
    if !args.is_empty() || !params.is_empty() || js::returns(body) != (true, false) {
        return None;
    }
    // Not where it labels something the loop's label, which JS refuses
    // inside the loop.
    if label.is_some_and(|label| labels(body, label)) {
        return None;
    }
    let mut body = body.clone();
    rejumped(&mut body, &jump.kind, label, (false, false))?;
    if !body.last().is_some_and(js::ends) {
        body.push(jump.clone());
    }
    Some(body)
}

/// Each `return;` of `stmts`, a closure's, `jump`: of `label` where what's
/// around it, `(a switch, a loop)`, would take it as it is. `None` where
/// one needs a label and the loop has none.
fn rejumped(stmts: &mut [Stmt], jump: &StmtKind, label: Option<&str>, around: (bool, bool)) -> Option<()> {
    for stmt in stmts.iter_mut() {
        match &mut stmt.kind {
            StmtKind::Return(None) => {
                stmt.kind = match jump {
                    StmtKind::Break(None) if around.0 || around.1 => StmtKind::Break(Some(label?.to_string())),
                    StmtKind::Continue(None) if around.1 => StmtKind::Continue(Some(label?.to_string())),
                    _ => jump.clone(),
                };
            }
            StmtKind::If(_, then, otherwise) => {
                rejumped(then, jump, label, around)?;
                if let Some(otherwise) = otherwise {
                    rejumped(otherwise, jump, label, around)?;
                }
            }
            StmtKind::Labeled(_, body) => rejumped(body, jump, label, around)?,
            StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::For { body, .. } => rejumped(body, jump, label, (around.0, true))?,
            StmtKind::Switch { cases, default, .. } => {
                for body in cases.iter_mut().map(|case| &mut case.body).chain(default.iter_mut()) {
                    rejumped(body, jump, label, (true, around.1))?;
                }
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                rejumped(a, jump, label, around)?;
                rejumped(b, jump, label, around)?;
            }
            _ => {}
        }
    }
    Some(())
}

/// Whether `stmts` label a block or a loop `name`, which JS refuses inside
/// another of that label.
fn labels(stmts: &[Stmt], name: &str) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Labeled(l, body) => l == name || labels(body, name),
        StmtKind::While { label, body, .. }
        | StmtKind::DoWhile { label, body, .. }
        | StmtKind::ForOf { label, body, .. }
        | StmtKind::For { label, body, .. } => label.as_deref() == Some(name) || labels(body, name),
        StmtKind::If(_, a, b) => labels(a, name) || b.as_deref().is_some_and(|b| labels(b, name)),
        StmtKind::Switch { cases, default, .. } => {
            cases.iter().any(|case| labels(&case.body, name)) || default.as_deref().is_some_and(|d| labels(d, name))
        }
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => labels(a, name) || labels(b, name),
        _ => false,
    })
}
