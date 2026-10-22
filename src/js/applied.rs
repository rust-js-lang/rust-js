//! A closure called as a statement where it's made, `((q) => { .. })(q);`,
//! whose value nothing reads: its body in its place (ADR 0581).

use super::*;
use std::collections::HashSet;

/// `((taskQueue) => { .. })(taskQueue);`, a closure called where it's made,
/// as a statement whose value nothing reads: its body in its place, its
/// parameter the variable it's given, as a plain thread-local's
/// `with_borrow_mut(|task_queue| ..)` reads its variable (ADR 0581). A
/// `return;` in it leaves the closure, so it's in place only where the
/// call ends the function's body; and what its body declares, where no
/// other name of the function is the same.
pub fn applied_blocks(body: &mut Vec<Stmt>) {
    loop {
        let whole = body.clone();
        if let Some(last) = body.len().checked_sub(1)
            && let Some(inlined) = block_of(&body[last], &whole, true)
        {
            body.splice(last..=last, inlined);
            continue;
        }
        let mut done = false;
        statement_lists(body, &mut |stmts| {
            if done {
                return;
            }
            if let Some(i) = (0..stmts.len()).find(|&i| block_of(&stmts[i], &whole, false).is_some()) {
                let inlined = block_of(&stmts[i], &whole, false).expect("found");
                stmts.splice(i..=i, inlined);
                done = true;
            }
        });
        if !done {
            break;
        }
    }
}

/// `f(arg);`, a call as a statement in `whole`, the function's body, of a
/// function of `params` and `body`: its body in its place, where a closure
/// of them called there would be; a `Drop` impl's, at its one drop (ADR
/// 0588).
pub fn applied_call(stmt: &Stmt, whole: &[Stmt], params: &[Pattern], body: &[Stmt]) -> Option<Vec<Stmt>> {
    let StmtKind::Expr(e) = &stmt.kind else { return None };
    let ExprKind::Call(_, args) = &e.kind else {
        return None;
    };
    let call = StmtKind::Expr(Expr::call(Expr::arrow(params.to_vec(), body.to_vec()), args.clone())).at(stmt.span);
    block_of(&call, whole, false)
}

/// The statements `stmt`, a call of a closure made there, is, in `whole`,
/// the function's body; `tail`, the body's last, which a `return;` ends too.
fn block_of(stmt: &Stmt, whole: &[Stmt], tail: bool) -> Option<Vec<Stmt>> {
    let StmtKind::Expr(e) = &stmt.kind else { return None };
    let ExprKind::Call(callee, args) = &e.kind else {
        return None;
    };
    let ExprKind::Arrow(params, body) = &callee.kind else {
        return None;
    };
    let ([Pattern::Name(param)], [arg]) = (params.as_slice(), args.as_slice()) else {
        return None;
    };
    // What it's given, read the same wherever it's read: a variable, or a
    // field of one, a thread-local's `value`.
    let given = match &arg.kind {
        ExprKind::Var(name) => name,
        ExprKind::Member(of, _)
            if arg.reads_same()
                && let ExprKind::Var(name) = &of.kind =>
        {
            name
        }
        _ => return None,
    };
    // The closure's `return`s are its own, the function's only at its end,
    // and of no value, which nothing read; elsewhere, each in the loop its
    // body ends with, that loop's `break`.
    let (any, valued) = returns(body);
    let mut body = body.clone();
    if valued || any && !tail && !returns_broken(&mut body) {
        return None;
    }
    // What the body declares at its top is declared beside `stmt`: no other
    // name of the function may be it, nor the variable it's given.
    let mut declared = HashSet::new();
    for s in &body {
        match &s.kind {
            StmtKind::Const(name, _) | StmtKind::Let(name, _) => _ = declared.insert(name.clone()),
            StmtKind::Destructure { pattern, .. } => declared.extend(pattern.names().into_iter().map(str::to_string)),
            StmtKind::Function(function) => _ = declared.insert(function.name.clone()),
            _ => {}
        }
    }
    if !declared.is_empty() {
        let mut others = whole.to_vec();
        statement_lists(&mut others, &mut |stmts| stmts.retain(|s| s != stmt));
        let mut names = HashSet::new();
        names_in(&others, &mut names);
        if declared.contains(given) || declared.iter().any(|name| names.contains(name)) {
            return None;
        }
    }
    if !matches!(&arg.kind, ExprKind::Var(v) if v == param) {
        // Read as what it's given: a parameter never set, of a body with no
        // closure of its own, whose names could shadow it.
        let mut set = false;
        each_block_mut(&mut body, &mut |stmts| {
            set |= stmts.iter().any(|s| {
                matches!(&s.kind, StmtKind::Assign(t, _) | StmtKind::Compound(_, t, _)
                if matches!(&t.kind, ExprKind::Var(v) if v == param))
            });
        });
        let whole_body = substituted(&mut body, &|v| (v == param).then(|| arg.clone()));
        if set || !whole_body {
            return None;
        }
    }
    Some(body)
}

/// Whether `stmts` return, but in a closure of their own, and whether a
/// `return` gives a value.
pub fn returns(stmts: &[Stmt]) -> (bool, bool) {
    let mut found = (false, false);
    for s in stmts {
        let (any, valued) = match &s.kind {
            StmtKind::Return(value) => (true, value.is_some()),
            StmtKind::If(_, a, b) => {
                let (x, y) = returns(a);
                let (z, w) = b.as_deref().map(returns).unwrap_or_default();
                (x || z, y || w)
            }
            StmtKind::While { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::Labeled(_, body) => returns(body),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                let (x, y) = returns(a);
                let (z, w) = returns(b);
                (x || z, y || w)
            }
            _ => (false, false),
        };
        found = (found.0 || any, found.1 || valued);
    }
    found
}

/// `stmts` with each variable `with` names read as its expression, each of
/// their own expressions once; whether each could be, none with a closure
/// whose names could shadow one.
fn substituted(stmts: &mut [Stmt], with: &dyn Fn(&str) -> Option<Expr>) -> bool {
    let one = |e: &mut Expr| match e.substitute(with) {
        Some(read) => {
            *e = read;
            true
        }
        None => false,
    };
    stmts.iter_mut().all(|stmt| match &mut stmt.kind {
        StmtKind::Const(_, e)
        | StmtKind::Let(_, Some(e))
        | StmtKind::Destructure { value: e, .. }
        | StmtKind::Expr(e)
        | StmtKind::Return(Some(e))
        | StmtKind::Throw(e) => one(e),
        StmtKind::Assign(a, b) | StmtKind::Compound(_, a, b) => one(a) && one(b),
        StmtKind::If(test, a, b) => {
            one(test) && substituted(a, with) && b.as_deref_mut().is_none_or(|b| substituted(b, with))
        }
        StmtKind::While { cond, body, .. } => one(cond) && substituted(body, with),
        StmtKind::ForOf { iterable, body, .. } => one(iterable) && substituted(body, with),
        StmtKind::For { start, test, body, .. } => one(start) && one(test) && substituted(body, with),
        StmtKind::Labeled(_, body) => substituted(body, with),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => substituted(a, with) && substituted(b, with),
        StmtKind::Let(_, None)
        | StmtKind::Break(_)
        | StmtKind::Continue(_)
        | StmtKind::Return(None)
        | StmtKind::Directive(_) => true,
        StmtKind::Function(_) => false,
    })
}

/// A closure's body that ends with a loop, each `return;` in it in that
/// loop and in no other: each that loop's `break;`, as leaving the loop
/// ends the body. Whether it is.
fn returns_broken(body: &mut [Stmt]) -> bool {
    let Some((last, before)) = body.split_last_mut() else {
        return false;
    };
    if returns(before).0 {
        return false;
    }
    let (StmtKind::While {
        label: None,
        body: inner,
        ..
    }
    | StmtKind::ForOf {
        label: None,
        body: inner,
        ..
    }
    | StmtKind::For {
        label: None,
        body: inner,
        ..
    }) = &mut last.kind
    else {
        return false;
    };
    // Each `return;` outside any inner loop, whose `break` would be its own.
    fn broken(stmts: &mut [Stmt]) -> bool {
        stmts.iter_mut().all(|stmt| match &mut stmt.kind {
            StmtKind::Return(None) => {
                stmt.kind = StmtKind::Break(None);
                true
            }
            StmtKind::If(_, a, b) => broken(a) && b.as_deref_mut().is_none_or(broken),
            StmtKind::Labeled(_, a) => broken(a),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => broken(a) && broken(b),
            StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
                !returns(body).0
            }
            _ => true,
        })
    }
    broken(inner)
}
