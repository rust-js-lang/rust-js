//! A `match` of a `Result` of what may throw as JS's `try` and `catch`: a
//! JS call's (ADRs 0035, 0637) or `catch_unwind`'s (ADR 0660), and what
//! the `catch` reads of what was thrown.

use crate::js::{self, Expr, ExprKind, Stmt, StmtKind};

pub(super) fn try_catches(body: &mut Vec<Stmt>) {
    // `$try(() => f())` whose result is ignored, `let _ = f();`, is `try {
    // f(); } catch {}`: the call, what it threw dropped.
    for stmt in body.iter_mut() {
        if let StmtKind::Expr(e) = &stmt.kind
            && let Some((call, _)) = e.caught()
        {
            let tried = vec![StmtKind::Expr(call).at(stmt.span)];
            stmt.kind = StmtKind::TryCatch(js::applied_tried(tried), None, Vec::new());
        }
        // Whether it threw, returned, `match f() { Ok(_) => false, Err(_)
        // => true }` or `f().is_err()`, is `try { f(); return false; }
        // catch { return true; }` (ADR 0639).
        if let StmtKind::Return(Some(e)) = &stmt.kind
            && let Some((call, of_throw)) = threw(e)
        {
            let span = stmt.span;
            let tried = vec![
                StmtKind::Expr(call).at(span),
                StmtKind::Return(Some(Expr::bool(!of_throw))).at(span),
            ];
            let caught = vec![StmtKind::Return(Some(Expr::bool(of_throw))).at(span)];
            stmt.kind = StmtKind::TryCatch(js::applied_tried(tried), None, caught);
        }
    }
    let mut i = 0;
    while i + 1 < body.len() {
        // Past what's declared for it to set, `let text;`, as MIR's lowering
        // declares a variable where the program does.
        let declared = (body[i + 1..].iter())
            .take_while(|s| matches!(s.kind, StmtKind::Let(_, None)))
            .count();
        let at = i + 1 + declared;
        if at < body.len()
            && let Some((caught, taken)) = try_catch(&body[i], &body[at], &body[..i], &body[at + 1..])
        {
            body.splice(at..at + 1 + taken, caught);
            body.remove(i);
        }
        i += 1;
    }
}

/// A `match` of a JS call's `Result` (ADR 0035), `made`, its `$try`, then
/// `tested`, the `if` of its `Ok` or its `Err`, and of `rest`, what follows
/// it: `try { .. } catch { .. }` of the call (ADR 0637), and how many of
/// `rest` it takes. The `Err` arm reads no error but by its binding, the
/// `catch`'s. Only the call is in the `try`, as only it is in Rust: what
/// the `Ok` arm does is too where it can't throw, `try { return f(); }`;
/// else it's after it, where the `Err` arm leaves, the `Ok`'s value a `let`
/// the `try` sets, `let v; try { v = f(); } catch { return; } g(v);`.
/// `before`, what's before it, which that `let` mustn't clash with.
fn try_catch(made: &Stmt, tested: &Stmt, before: &[Stmt], rest: &[Stmt]) -> Option<(Vec<Stmt>, usize)> {
    let StmtKind::Const(result, value) = &made.kind else {
        return None;
    };
    let (call, caught) = value.caught()?;
    // A `?:` of it returned, `unwrap_or(d)`'s, is the `if` of its arms,
    // each returned (ADR 0660).
    let arms;
    let tested = match &tested.kind {
        StmtKind::Return(Some(Expr {
            kind: ExprKind::Cond(test, then, otherwise),
            ..
        })) => {
            let returned = |e: &Expr| vec![StmtKind::Return(Some(e.clone())).at(tested.span)];
            arms = StmtKind::If((**test).clone(), returned(then), Some(returned(otherwise))).at(tested.span);
            &arms
        }
        _ => tested,
    };
    let StmtKind::If(test, then, otherwise) = &tested.kind else {
        return None;
    };
    // Whether the `if` is of the `Ok`: `TAG === "Ok"`, or `!== "Err"`.
    let (tested, ok, eq) = js::result::tested(test)?;
    if !matches!(&tested.kind, ExprKind::Var(v) if v == result) {
        return None;
    }
    let of_ok = ok == eq;
    // Each arm's statements, and how many of `rest` are one's: an `if`'s
    // and its `else`'s; or, after an arm that leaves, the other's, `rest`,
    // which leaves too, where it's the `Err`'s, the `catch`'s; or, after an
    // `Ok` arm that leaves where `rest` goes on, none, the `catch`'s, what
    // follows the `try` run where the call threw, as it ran after the `if`
    // (ADR 0637, amended).
    let leaves = |stmts: &[Stmt]| stmts.last().is_some_and(js::ends);
    let (ok, err, taken): (&[Stmt], &[Stmt], usize) = match (of_ok, otherwise) {
        (true, Some(otherwise)) => (then, otherwise, 0),
        (false, Some(otherwise)) => (otherwise, then, 0),
        (true, None) if !leaves(then) => (then, &[], 0),
        (true, None) if leaves(rest) => (then, rest, rest.len()),
        (false, None) if leaves(then) => (rest, then, rest.len()),
        (false, None) => (&[], then, 0),
        (true, None) => (then, &[], 0),
    };
    let after = &rest[taken..];
    // Each arm's binding, `const v = result._0;`: the `Ok`'s value, and the
    // `Err`'s error, the `catch`'s.
    // `let` where the arm sets it again, a shadow that's the binding it
    // shadows (ADR 0650), `let x = result._0; x = ..`.
    let bound = |stmts: &[Stmt]| match stmts.first().map(|s| &s.kind) {
        Some(StmtKind::Const(name, value)) if js::result::reads_held(value, result) => Some((name.clone(), false)),
        Some(StmtKind::Let(name, Some(value))) if js::result::reads_held(value, result) => Some((name.clone(), true)),
        _ => None,
    };
    let (value_name, error) = (bound(ok).map(|(name, _)| name), bound(err));
    let (error, set_again) = match error {
        Some((name, set_again)) => (Some(name), set_again),
        None => (None, false),
    };
    let ok = &ok[usize::from(value_name.is_some())..];
    let err = &err[usize::from(error.is_some())..];
    // The `Ok`'s value kept as it is, `return result._0;` or `x =
    // result._0;`, or its binding's: the call's.
    let kept = |stmt: &Stmt| -> Option<StmtKind> {
        let read = |e: &Expr| match &value_name {
            Some(name) => matches!(&e.kind, ExprKind::Var(v) if v == name),
            None => js::result::reads_held(e, result),
        };
        match &stmt.kind {
            StmtKind::Return(Some(e)) if read(e) => Some(StmtKind::Return(Some(call.clone()))),
            StmtKind::Assign(to, e) if read(e) && matches!(to.kind, ExprKind::Var(_)) => {
                Some(StmtKind::Assign(to.clone(), call.clone()))
            }
            _ => None,
        }
    };
    let held = match ok {
        [stmt] => kept(stmt).map(|kind| kind.at(stmt.span)),
        _ => None,
    };
    // Nothing else reads the `Result`.
    let reads = js::mentions_in(ok, result) + js::mentions_in(err, result) + js::mentions_in(after, result);
    if reads != usize::from(held.is_some() && value_name.is_none()) {
        return None;
    }
    let span = made.span;
    // What unwound is the payload `$unwound` makes of what was thrown: the
    // `Err`'s binding, of the `catch`'s, `thrown` (ADR 0660).
    let (binding, in_catch) = match (caught, &error) {
        (js::Caught::Unwound, Some(payload)) => {
            const THROWN: &str = "thrown";
            if payload == THROWN || js::mentions_in(err, THROWN) > 0 {
                return None;
            }
            let unwound = Expr::call_helper("$unwound", vec![Expr::var(THROWN)]);
            let made = match set_again {
                true => StmtKind::Let(payload.clone(), Some(unwound)),
                false => StmtKind::Const(payload.clone(), unwound),
            }
            .at(span);
            (
                Some(THROWN.to_string()),
                std::iter::once(made).chain(err.iter().cloned()).collect(),
            )
        }
        _ => (error.clone(), err.to_vec()),
    };
    // A closure's body, of `catch_unwind(|| ..)`, its value returned: in
    // the `try`, its `return`s the function's (ADR 0660).
    let caught =
        |tried: Vec<Stmt>| StmtKind::TryCatch(js::applied_tried(tried), binding.clone(), in_catch.clone()).at(span);
    let alone = StmtKind::Expr(call.clone()).at(span);
    // What can't throw: what makes values, and what leaves with one.
    let cannot_throw = |stmt: &Stmt| match &stmt.kind {
        StmtKind::Return(value) => value.as_ref().is_none_or(|v| !v.may_throw()),
        StmtKind::Break(_) | StmtKind::Continue(_) => true,
        _ => stmt.only_makes_values(),
    };
    if let Some(held) = held {
        return Some((vec![caught(vec![held])], taken));
    }
    // What the `Ok` arm does can't throw: in the `try`.
    if ok.iter().all(cannot_throw) {
        let first = match &value_name {
            Some(name) => StmtKind::Const(name.clone(), call.clone()).at(span),
            None => alone,
        };
        let tried = std::iter::once(first).chain(ok.iter().cloned()).collect();
        return Some((vec![caught(tried)], taken));
    }
    // Else after it, where the `Err` arm leaves, its value set by the `try`:
    // a `let` of a name nothing else here reads or declares.
    let unread =
        |name: &String| js::mentions_in(before, name) + js::mentions_in(err, name) + js::mentions_in(after, name) == 0;
    if !leaves(err) || !value_name.as_ref().is_none_or(unread) {
        return None;
    }
    let mut out = Vec::new();
    let tried = match &value_name {
        Some(name) => {
            out.push(StmtKind::Let(name.clone(), None).at(span));
            StmtKind::Assign(Expr::var(name), call.clone()).at(span)
        }
        None => alone,
    };
    out.push(caught(vec![tried]));
    out.extend(ok.iter().cloned());
    Some((out, taken))
}

/// `$try(() => call).TAG === "Err"`, or `!== "Ok"`: whether a JS call
/// threw, a `match` of its `Result` that gives `true` of the one and
/// `false` of the other, or its `is_err()`, `is_ok()`. The call, and the
/// test's value where it threw (ADR 0639).
fn threw(e: &Expr) -> Option<(Expr, bool)> {
    let (of, ok, eq) = js::result::tested(e)?;
    Some((of.caught()?.0, eq != ok))
}
