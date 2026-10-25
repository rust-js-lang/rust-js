//! The drops a panic runs as it unwinds (`mir/drops.rs`): each statement
//! that may throw while a value with a destructor is live is `try { s }
//! catch (error) { drop(x); throw error; }`, and the scope's end drops `x`
//! again. A run of them, to that end, is one `try { .. } finally { drop(x)
//! }`, as a person writes cleanup: the same drops, run in the same order on
//! either path (ADR 0437); its `if`s and loops too, each way out of them
//! that drops `x` first, `drop(x); return v;`, a way out of the `try`,
//! whose `finally` drops it (ADR 0693).

use std::collections::HashSet;

use crate::js::{self, Expr, ExprKind, Stmt, StmtKind};

/// `stmts`, each run of statements a value's drop ends, which a panic in
/// any of them drops too, one `try { .. } finally { .. }`. The values
/// made inside it, nested in turn, as `statement_lists` goes on into it.
/// Whether it made one.
pub(super) fn scoped_drops(stmts: &mut Vec<Stmt>) -> bool {
    let mut scoped = false;
    let mut i = 0;
    while i < stmts.len() {
        if let Some(end) = scope_end(stmts, i) {
            let drop = stmts[end].clone();
            let mut body = unwound(stmts.drain(i..end).collect(), &drop);
            // From where its first statement is, as the Rust has nothing of
            // its own there, a guard's `let` printing nothing, to its drop:
            // what's before it, a blank line or a comment, is before the
            // `try` (ADR 0865, amended).
            let span = match body.first().and_then(written_at) {
                Some(lo) if !drop.span.is_none() && lo < drop.span.lo => js::Span { lo, hi: drop.span.hi },
                _ => drop.span,
            };
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
            scoped = true;
            continue;
        }
        i += 1;
    }
    scoped
}

/// `stmts`, each `try` a scope's `finally` didn't take whose body can't
/// throw, `try { result = $catchUnwind(f); } catch (error) { drop(x); throw
/// error; }`, its body: `catch_unwind` catches whatever unwinds through it,
/// so its call never unwinds to the drops of what's held around it (ADR
/// 0873). Whether it found any.
pub(super) fn unthrown_catches(stmts: &mut Vec<Stmt>) -> bool {
    let unthrown = |stmt: &Stmt| {
        matches!(&stmt.kind, StmtKind::TryCatch(body, Some(error), handler)
            if body.iter().all(Stmt::cannot_throw) && handler.last().is_some_and(|last| rethrows(std::slice::from_ref(last), error)))
    };
    if !stmts.iter().any(unthrown) {
        return false;
    }
    let mut out = Vec::with_capacity(stmts.len());
    for stmt in std::mem::take(stmts) {
        if unthrown(&stmt)
            && let StmtKind::TryCatch(body, ..) = stmt.kind
        {
            out.extend(body);
        } else {
            out.push(stmt);
        }
    }
    *stmts = out;
    true
}

/// `stmts`, each `try` a scope's `finally` didn't take of one `$unwrap`
/// read in place, `let tmp; try { tmp = $unwrap(f); task$live = false; }
/// catch (error) { $dispose(task); throw error; } tmp(task);`: its test and
/// panic, as an `assert!`'s are, the drops before the throw, `if (f ==
/// null) { $dispose(task); throw new Error(..); } tmp = f;`, which reads
/// `f(task)` once its temporary is read in place (ADR 0873, amended). The
/// same drops, then the same panic: a panic writes nothing as it's thrown
/// (ADR 0660). Whether it found any.
pub(super) fn tested_unwraps(stmts: &mut Vec<Stmt>) -> bool {
    let tested = |stmt: &Stmt| {
        let StmtKind::TryCatch(body, Some(error), handler) = &stmt.kind else {
            return None;
        };
        let [first, rest @ ..] = body.as_slice() else {
            return None;
        };
        let (StmtKind::Assign(_, value) | StmtKind::Expr(value)) = &first.kind else {
            return None;
        };
        let (test, option, panic) = crate::lower::std_types::option::unwrap_tested(value)?;
        let (last, drops) = handler.split_last()?;
        let given = option.reads_same()
            && rest.iter().all(Stmt::cannot_throw)
            && rethrows(std::slice::from_ref(last), error)
            && js::mentions_in(drops, error) == 0;
        given.then(|| (test, option.clone(), panic))
    };
    if !stmts.iter().any(|stmt| tested(stmt).is_some()) {
        return false;
    }
    let mut out = Vec::with_capacity(stmts.len());
    for stmt in std::mem::take(stmts) {
        let Some((test, option, panic)) = tested(&stmt) else {
            out.push(stmt);
            continue;
        };
        let StmtKind::TryCatch(mut body, _, mut handler) = stmt.kind else {
            unreachable!("tested")
        };
        let last = handler.pop().expect("tested");
        handler.push(StmtKind::Throw(panic).at(last.span));
        out.push(StmtKind::If(test, handler, None).at(stmt.span));
        match &mut body[0].kind {
            StmtKind::Assign(_, value) => *value = option,
            _ => {
                body.remove(0);
            }
        }
        out.extend(body);
    }
    *stmts = out;
    true
}

/// Where `stmt` is written: its own span's start, or of one placed nowhere,
/// a temporary's `result = f()`, its value's, and a `try` a call's drops
/// make, its first statement's.
fn written_at(stmt: &Stmt) -> Option<u32> {
    if !stmt.span.is_none() {
        return Some(stmt.span.lo);
    }
    match &stmt.kind {
        StmtKind::Try(body, _) | StmtKind::TryCatch(body, ..) => body.first().and_then(written_at),
        StmtKind::Assign(_, value)
        | StmtKind::Const(_, value)
        | StmtKind::Let(_, Some(value))
        | StmtKind::Expr(value)
        | StmtKind::Return(Some(value))
            if !value.span.is_none() =>
        {
            Some(value.span.lo)
        }
        _ => None,
    }
}

/// `let result; try { ..; result = e; } finally { .. } return result;`, what
/// a function gives made last in a `try` whose `finally` drops what's held:
/// `try { ..; return e; } finally { .. }`, which runs the `finally` once `e`
/// is made, as the original does (ADR 0588). Of an `e` the same whenever
/// it's made, a constant or a function, `noop` (`Expr::is_fixed`), which
/// the drop can't change: `try { .. } finally { .. } return e;`, as a
/// person writes it, after (ADR 0693, amended).
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
            let last = body.pop().expect("matched");
            let StmtKind::Assign(_, value) = last.kind else {
                unreachable!("matched")
            };
            match value.is_fixed() {
                true => stmts.insert(i + 1, StmtKind::Return(Some(value)).at(last.span)),
                false => body.push(StmtKind::Return(Some(value)).at(last.span)),
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
    let drop = first_drop(std::slice::from_ref(&stmts[start]))?;
    let run = Run::of(&stmts[start..], drop);
    for (at, stmt) in stmts.iter().enumerate().skip(start) {
        if at > start && alike(stmt, drop) {
            return Some(at);
        }
        if !run.covers(std::slice::from_ref(stmt), 0, &mut Vec::new()) {
            return None;
        }
    }
    None
}

/// The first `try` in `stmts` whose `catch` drops a value last, then throws
/// on, its drop: in them, or in an `if`'s or a loop's, as `Run::covers` goes.
fn first_drop(stmts: &[Stmt]) -> Option<&Stmt> {
    stmts.iter().find_map(|stmt| match &stmt.kind {
        StmtKind::TryCatch(..) => dropped_last(stmt),
        StmtKind::If(_, a, b) => first_drop(a).or_else(|| b.as_deref().and_then(first_drop)),
        StmtKind::While { body, .. } | StmtKind::Labeled(_, body) => first_drop(body),
        _ => None,
    })
}

/// A run's statements, as a `finally` that drops `drop` would end them:
/// what they declare, which a way out may give, and what closures in them
/// set, which the drop might too.
struct Run<'a> {
    drop: &'a Stmt,
    declared: HashSet<String>,
    set_by_closures: HashSet<String>,
}

impl<'a> Run<'a> {
    fn of(stmts: &[Stmt], drop: &'a Stmt) -> Run<'a> {
        let mut declared = HashSet::new();
        js::each_list(stmts, &mut |list| {
            for stmt in list {
                if let StmtKind::Const(name, _) | StmtKind::Let(name, _) = &stmt.kind {
                    declared.insert(name.clone());
                }
            }
        });
        Run {
            drop,
            declared,
            set_by_closures: js::Writes::of(stmts).by_closures(),
        }
    }

    /// Whether `stmts`, `loops` loops into the run, inside `labels` of its
    /// own, are what a `finally` of the drop may end: each that may throw a
    /// `try` that drops it last, or an `if` or a loop whose test can't
    /// throw, of such statements; and each way out of the run, a `return`,
    /// or a `break` or a `continue` of what's outside it, after the drop.
    fn covers(&self, stmts: &[Stmt], loops: usize, labels: &mut Vec<String>) -> bool {
        let mut i = 0;
        while i < stmts.len() {
            let stmt = &stmts[i];
            let covered = match &stmt.kind {
                _ if alike(stmt, self.drop) => {
                    i += 1;
                    stmts
                        .get(i)
                        .is_some_and(|out| self.leaves_after_drop(out, loops, labels))
                }
                // Another value's drop last, one whose life crosses this
                // one's: not a scope. Lowering wraps this one's drop then, so
                // no run is found anyway; this keeps it so if that changes.
                StmtKind::TryCatch(..) => dropped_last(stmt).is_some_and(|last| alike(last, self.drop)),
                StmtKind::If(test, a, b) => {
                    !test.may_throw()
                        && self.covers(a, loops, labels)
                        && b.as_deref().is_none_or(|b| self.covers(b, loops, labels))
                }
                StmtKind::While { label, cond, body } => {
                    labels.extend(label.clone());
                    let covered = !cond.may_throw() && self.covers(body, loops + 1, labels);
                    labels.truncate(labels.len() - usize::from(label.is_some()));
                    covered
                }
                StmtKind::Labeled(label, body) => {
                    labels.push(label.clone());
                    let covered = self.covers(body, loops, labels);
                    labels.pop();
                    covered
                }
                // A jump that stays in the run.
                StmtKind::Break(None) | StmtKind::Continue(None) => loops > 0,
                StmtKind::Break(Some(label)) | StmtKind::Continue(Some(label)) => labels.contains(label),
                // One made of values, which can't throw, or the run ends.
                _ => stmt.only_makes_values(),
            };
            if !covered {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Whether `out`, after the drop, leaves the run: a `return` of what the
    /// drop can't change, a variable of the run's own no closure sets, a
    /// constant or nothing; or a jump out of it.
    fn leaves_after_drop(&self, out: &Stmt, loops: usize, labels: &[String]) -> bool {
        match &out.kind {
            StmtKind::Return(None) => true,
            StmtKind::Return(Some(value)) => match &value.kind {
                ExprKind::Var(name) => self.declared.contains(name) && !self.set_by_closures.contains(name),
                _ => value.is_constant(),
            },
            StmtKind::Break(None) | StmtKind::Continue(None) => loops == 0,
            StmtKind::Break(Some(label)) | StmtKind::Continue(Some(label)) => !labels.contains(label),
            _ => false,
        }
    }
}

/// `stmts` of a run `Run::covers`, in a `finally` that drops `drop`: each
/// `catch`'s drop of it gone, and a `catch` that only throws again with
/// it; and the drop before each way out, which the `finally` runs, `const
/// v = e; drop; return v;` given as it's made, `return e;`.
fn unwound(stmts: Vec<Stmt>, drop: &Stmt) -> Vec<Stmt> {
    let all = stmts.clone();
    let mut out: Vec<Stmt> = Vec::new();
    let mut stmts = stmts.into_iter().peekable();
    while let Some(stmt) = stmts.next() {
        let leaves = |s: &Stmt| matches!(s.kind, StmtKind::Return(_) | StmtKind::Break(_) | StmtKind::Continue(_));
        if alike(&stmt, drop)
            && stmts.peek().is_some_and(leaves)
            && let Some(mut leaving) = stmts.next()
        {
            if let StmtKind::Return(Some(Expr {
                kind: ExprKind::Var(name),
                ..
            })) = &leaving.kind
                && let Some(made) = out.last().and_then(made_here)
                && made == name
                && js::mentions_in(&all, name) == 2 + usize::from(declared_unset(&all, name))
                && js::mentions_in(std::slice::from_ref(drop), name) == 0
                && let Some(Stmt {
                    kind: StmtKind::Const(_, value) | StmtKind::Assign(_, value),
                    ..
                }) = out.pop()
            {
                // And its `let v;`, which nothing's left to set.
                out.retain(|s| !matches!(&s.kind, StmtKind::Let(v, None) if v == name));
                leaving.kind = StmtKind::Return(Some(value));
            }
            out.push(leaving);
            continue;
        }
        let kind = match stmt.kind {
            StmtKind::TryCatch(inner, Some(error), mut handler) => {
                // The drop before its `throw`, the scope's own.
                handler.remove(handler.len() - 2);
                if rethrows(&handler, &error) {
                    out.extend(inner);
                    continue;
                }
                StmtKind::TryCatch(inner, Some(error), handler)
            }
            StmtKind::If(test, a, b) => StmtKind::If(test, unwound(a, drop), b.map(|b| unwound(b, drop))),
            StmtKind::While { label, cond, body } => StmtKind::While {
                label,
                cond,
                body: unwound(body, drop),
            },
            StmtKind::Labeled(label, body) => StmtKind::Labeled(label, unwound(body, drop)),
            kind => kind,
        };
        out.push(Stmt { kind, ..stmt });
    }
    out
}

/// Whether `stmts` declare `name` and set it nowhere there, `let v;`.
fn declared_unset(stmts: &[Stmt], name: &str) -> bool {
    stmts
        .iter()
        .any(|s| matches!(&s.kind, StmtKind::Let(v, None) if v == name))
}

/// The variable `stmt` makes, `const v = e;` or `v = e;`.
fn made_here(stmt: &Stmt) -> Option<&str> {
    match &stmt.kind {
        StmtKind::Const(name, _) => Some(name),
        StmtKind::Assign(
            Expr {
                kind: ExprKind::Var(name),
                ..
            },
            _,
        ) => Some(name),
        _ => None,
    }
}

/// Whether `a` is the same JS as `b`, a drop: `js::stmts_alike`'s.
fn alike(a: &Stmt, b: &Stmt) -> bool {
    js::stmts_alike(std::slice::from_ref(a), std::slice::from_ref(b))
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
    // Not a declaration, a drop function's whose call a tidy pass found
    // never runs, `const dropMany = ..`: it does nothing, and in a
    // `finally` its name would be the `finally`'s alone, where what's after
    // calls the one the scope's end declares (ADR 0901).
    let declares = matches!(drop.kind, StmtKind::Const(..) | StmtKind::Let(..));
    // One that leaves another way already dropped what it leaves; lowering
    // wraps no `return` in a `try`, but a `finally` would drop it again.
    (rethrown && !declares && !leaves(inner, 0) && js::mentions_in(std::slice::from_ref(drop), error) == 0)
        .then_some(drop)
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
