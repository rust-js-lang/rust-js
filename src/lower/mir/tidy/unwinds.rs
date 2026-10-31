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
/// `result` is the function's return place; `guards`, where each drop
/// guard's `let` is written and where the block it's dropped at the end of
/// ends. Whether it made one.
pub(super) fn scoped_drops(stmts: &mut Vec<Stmt>, result: &str, guards: &[(u32, u32)]) -> bool {
    let mut scoped = false;
    let mut i = 0;
    while i < stmts.len() {
        if let Some(end) = scope_end(stmts, i, result) {
            let drop = stmts[end].clone();
            // A guard's `let` past each statement before the run, and what
            // their blocks hold: the run's, not an earlier value's.
            let mut written = 0;
            js::each_list(&stmts[..i], &mut |list| {
                written = (list.iter().filter_map(written_at)).fold(written, u32::max);
            });
            let mut body = unwound(stmts.drain(i..end).collect(), &drop);
            // From where its first statement is, as the Rust has nothing of
            // its own there, a guard's `let` printing nothing, to its drop:
            // what's before it, a blank line or a comment, is before the
            // `try` (ADR 0865, amended); or from where that guard's `let`
            // is, past the statement before, so what's between the `let`
            // and the first statement, a comment, is in the `try`, as
            // upstream's `try { // Invoke the function an extra time ..`
            // (ADR 0865, amended, the user's decision, 2026-10-27).
            let first = body.first().and_then(written_at);
            let first = first.map(|lo| {
                (guards.iter())
                    .filter(|&&(guard, end)| end == drop.span.hi && written < guard && guard < lo)
                    .map(|&(guard, _)| guard)
                    .max()
                    .unwrap_or(lo)
            });
            let span = match first {
                Some(lo) if !drop.span.is_none() && lo < drop.span.lo => js::Span { lo, hi: drop.span.hi },
                _ => drop.span,
            };
            let rest = stmts.split_off(i + 1);
            stmts.truncate(i);
            gathered_lets(stmts, &mut body, &rest, &drop);
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
fn scope_end(stmts: &[Stmt], start: usize, result: &str) -> Option<usize> {
    let drop = first_drop(std::slice::from_ref(&stmts[start]))?;
    let run = Run::of(&stmts[start..], drop, result);
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
/// set, which the drop might too; and the function's return place,
/// `result`, which only its own assignments set, as neither a closure nor
/// a drop names Rust's return place: given wherever it's declared, as a
/// `return` in the `try` gives it, `try { if (wrap) { return callback(); }
/// .. } finally { .. }` (ADR 0693, amended).
struct Run<'a> {
    drop: &'a Stmt,
    declared: HashSet<String>,
    set_by_closures: HashSet<String>,
    result: &'a str,
}

impl<'a> Run<'a> {
    fn of(stmts: &[Stmt], drop: &'a Stmt, result: &'a str) -> Run<'a> {
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
            result,
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
    /// drop can't change, a variable of the run's own no closure sets, the
    /// function's return place, a constant or nothing; or a jump out of it.
    fn leaves_after_drop(&self, out: &Stmt, loops: usize, labels: &[String]) -> bool {
        match &out.kind {
            StmtKind::Return(None) => true,
            StmtKind::Return(Some(value)) => match &value.kind {
                ExprKind::Var(name) => {
                    name == self.result || self.declared.contains(name) && !self.set_by_closures.contains(name)
                }
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
    // A drop is never a declaration: a function lowered from its MIR
    // declares each drop function once, at its top (ADR 0098, amended),
    // not in each `catch` that calls it (ADR 0901, amended).
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

/// `stmts`, each `while (true)` loop that drops one value before each way
/// out of it, `$unborrow(borrow); break;`, `$unborrow(borrow); return
/// result;`, and in each `catch` of what may throw in it, `try { tmp =
/// $next(iter); } catch (error) { $unborrow(borrow); throw error; }`, with
/// those `try`s before it from where the value's first dropped so: one `try
/// { .. } finally { $unborrow(borrow); }` of them, each of its drops gone,
/// as a `for` over a `RefCell`'s borrow drops it at the loop's end, or as
/// it's left (rd191). Each run is `tidied`, its loop made a `for` of what
/// it steps as MIR's would be, and the `break` that left the loop, now
/// after it, at the `try`'s end, gone. Where `for_made`, as the early
/// "drop scopes" stage asks, before `jumps` has read MIR's blocks: only of
/// a loop that steps what `loops` makes a `for` of, its blocks and its own
/// label read (rd200). Whether it made one.
pub(super) fn looped_drops(stmts: &mut Vec<Stmt>, for_made: bool, tidied: &mut dyn FnMut(&mut Vec<Stmt>)) -> bool {
    let mut made = false;
    let mut k = 0;
    while k < stmts.len() {
        let StmtKind::While { cond, label: own, .. } = &stmts[k].kind else {
            k += 1;
            continue;
        };
        let own = own.clone();
        let Some(drop) = (matches!(cond.kind, ExprKind::Bool(true)))
            .then(|| first_drop(std::slice::from_ref(&stmts[k])).cloned())
            .flatten()
        else {
            k += 1;
            continue;
        };
        // From the first of the `try`s before it that drop it, past what
        // can't throw between.
        // And what can't throw that only it reads, a range's `iter = {
        // start: 0, end: nodes.length }`, which `loops` makes a `for` of.
        let only_it_reads = |stmt: &Stmt| match &stmt.kind {
            StmtKind::Assign(target, _) => matches!(&target.kind, ExprKind::Var(name)
                if stmt.cannot_throw() && js::mentions_in(&stmts[k + 1..], name) == 0),
            _ => false,
        };
        let mut start = k;
        while start > 0
            && (dropping_catch(&stmts[start - 1], &drop).is_some()
                || matches!(stmts[start - 1].kind, StmtKind::Let(_, None))
                || only_it_reads(&stmts[start - 1]))
        {
            start -= 1;
        }
        // A `let x;` of them only they read is theirs, in the `try`, as a
        // scope's (`scoped_drops`).
        while start < k
            && matches!(&stmts[start].kind, StmtKind::Let(name, None) if js::mentions_in(&stmts[k + 1..], name) > 0)
        {
            start += 1;
        }
        // What's before it, of no way out, and its body, whose `break`s
        // are its ways out.
        let mut run: Vec<Stmt> = stmts[start..k].to_vec();
        let mut looped = stmts[k].clone();
        let StmtKind::While { body, .. } = &mut looped.kind else {
            unreachable!("a loop")
        };
        // The blocks it labels, whose `break`s go on in it, and the loops
        // in it, whose `continue`s and `break`s go on in it too, `continue
        // bb10;` of an inner loop of a value given each turn (P0).
        let mut labels = Vec::new();
        js::statement_lists(body, &mut |list| {
            labels.extend(list.iter().filter_map(|stmt| match &stmt.kind {
                StmtKind::Labeled(label, _) => Some(label.clone()),
                StmtKind::While { label, .. }
                | StmtKind::DoWhile { label, .. }
                | StmtKind::For { label, .. }
                | StmtKind::ForOf { label, .. } => label.clone(),
                _ => None,
            }))
        });
        // What follows a block that never falls out, MIR's `bb34: { ..
        // continue bb5; } return result;`, never runs: one whose last
        // statement leaves and that nothing in it breaks.
        js::statement_lists(body, &mut super::jumps::unreachable_after);
        js::statement_lists(body, &mut |list| {
            let left = list.iter().position(|stmt| {
                matches!(&stmt.kind, StmtKind::Labeled(label, inner)
                    if inner.last().is_some_and(js::ends) && !js::breaks(inner, Some(label)))
            });
            if let Some(at) = left {
                list.truncate(at + 1);
            }
        });
        // Its own label, MIR's `bb5: while (true)`: its `break bb5` is a way
        // out, its `continue bb5` a turn's, as unlabeled.
        if let Some(own) = &own {
            js::statement_lists(body, &mut |list| {
                for stmt in list.iter_mut() {
                    match &stmt.kind {
                        StmtKind::Break(Some(l)) if l == own => stmt.kind = StmtKind::Break(None),
                        StmtKind::Continue(Some(l)) if l == own => stmt.kind = StmtKind::Continue(None),
                        _ => {}
                    }
                }
            });
        }
        if !dropped_at_each_exit(&mut run, &drop, (0, false, &[]))
            || !dropped_at_each_exit(body, &drop, (0, false, &labels))
        {
            k += 1;
            continue;
        }
        // Where it's asked, `for_made`, only of a loop whose first statement
        // past `let`s is a step `loops` makes a `for` of, `const t =
        // $rangeNext(iter)`, or as MIR holds it in a `try` of the drop, `try
        // { t = $next(iter); } ..`: what reading it early is for. Another's
        // left to the scope it's in.
        let step =
            |value: &Expr| matches!(&value.kind, ExprKind::Call(callee, _) if super::loops::step_of(callee).is_some());
        let steps = |stmt: &Stmt| match &stmt.kind {
            StmtKind::Const(_, value) | StmtKind::Let(_, Some(value)) => step(value),
            StmtKind::TryCatch(inner, ..) => {
                matches!(inner.as_slice(), [Stmt { kind: StmtKind::Assign(_, value), .. }] if step(value))
            }
            _ => false,
        };
        let first = body.iter().find(|stmt| !matches!(stmt.kind, StmtKind::Let(_, None)));
        if for_made && !first.is_some_and(steps) {
            k += 1;
            continue;
        }
        let span = looped.span;
        run.push(looped);
        let rest = stmts.split_off(k + 1);
        stmts.truncate(start);
        gathered_lets(stmts, &mut run, &rest, &drop);
        tidied(&mut run);
        if let [
            ..,
            last_loop,
            Stmt {
                kind: StmtKind::Break(None),
                ..
            },
        ] = run.as_slice()
            && matches!(last_loop.kind, StmtKind::For { .. } | StmtKind::ForOf { .. })
        {
            run.pop();
        }
        stmts.push(StmtKind::Try(run, vec![drop]).at(span));
        k = stmts.len();
        stmts.extend(rest);
        made = true;
    }
    made
}

/// Each `let x;` of `before` that the run `body` a `try` is made of alone
/// reads: neither what's after it, `rest`, nor its `drop`, nor the rest of
/// `before`, as MIR declares a body's variables at its top: in the `try`,
/// first, where `declarations` gives it its value (`scoped_drops`,
/// `looped_drops`).
fn gathered_lets(before: &mut Vec<Stmt>, body: &mut Vec<Stmt>, rest: &[Stmt], drop: &Stmt) {
    let mut taken = Vec::new();
    let mut i = before.len();
    while i > 0 {
        i -= 1;
        let StmtKind::Let(name, None) = &before[i].kind else {
            continue;
        };
        let elsewhere = js::mentions_in(&before[..i], name) + js::mentions_in(&before[i + 1..], name);
        if elsewhere == 0 && js::mentions_in(rest, name) == 0 && js::mentions_in(std::slice::from_ref(drop), name) == 0
        {
            taken.push(before.remove(i));
        }
    }
    taken.reverse();
    body.splice(0..0, taken);
}

/// Of `try { .. } catch (error) { drop; throw error; }`, its body.
fn dropping_catch<'s>(stmt: &'s Stmt, drop: &Stmt) -> Option<&'s Vec<Stmt>> {
    match &stmt.kind {
        StmtKind::TryCatch(inner, Some(error), handler)
            if matches!(handler.as_slice(), [dropped, thrown]
                if alike(dropped, drop) && rethrows(std::slice::from_ref(thrown), error)) =>
        {
            Some(inner)
        }
        _ => None,
    }
}

/// `stmts`, `depth` loops into the loop that drops `drop` before each way
/// out of it, with each of those drops gone and each `try` that drops it
/// only as it throws its body; whether each way out dropped it, each
/// `return`, `throw` and `break` of the loop, and nothing else may throw,
/// but in such a `try`, `caught`, or drop it, nor leave it by a label.
fn dropped_at_each_exit(stmts: &mut Vec<Stmt>, drop: &Stmt, (depth, caught, labels): (usize, bool, &[String])) -> bool {
    let mut out = Vec::with_capacity(stmts.len());
    let mut dropped = false;
    for mut stmt in std::mem::take(stmts) {
        if alike(&stmt, drop) {
            if dropped {
                return false;
            }
            dropped = true;
            continue;
        }
        if let Some(inner) = dropping_catch(&stmt, drop) {
            let mut inner = inner.clone();
            // What it holds as it throws, the drop: its body as it runs,
            // which a `finally` drops; but none of its own ways out.
            if leaves(&inner, depth) || js::stmts_alike(&inner, std::slice::from_ref(drop)) {
                return false;
            }
            if !dropped_at_each_exit(&mut inner, drop, (depth, true, labels)) {
                return false;
            }
            if dropped {
                return false;
            }
            out.extend(inner);
            continue;
        }
        let exit = match &stmt.kind {
            StmtKind::Return(_) | StmtKind::Throw(_) => true,
            StmtKind::Break(None) => depth == 0,
            // A block's of its own, `bb25: { .. break bb25; }`, goes on in
            // it, as a loop's in it does, `continue bb10;`; another label's
            // leaves it, as a `return` does, the program's `break
            // 'get_instance` out of the block the loop's in (rd200).
            StmtKind::Break(Some(label)) | StmtKind::Continue(Some(label)) => !labels.contains(label),
            _ => false,
        };
        if exit {
            if !dropped {
                return false;
            }
            dropped = false;
            out.push(stmt);
            continue;
        }
        // A drop, then what isn't a way out: not this shape.
        if dropped && !stmt.cannot_throw() {
            return false;
        }
        let ok = match &mut stmt.kind {
            StmtKind::If(test, a, b) => {
                (caught || !test.may_throw())
                    && dropped_at_each_exit(a, drop, (depth, caught, labels))
                    && b.as_mut()
                        .is_none_or(|b| dropped_at_each_exit(b, drop, (depth, caught, labels)))
            }
            StmtKind::While { cond, body, .. } => {
                (caught || !cond.may_throw()) && dropped_at_each_exit(body, drop, (depth + 1, caught, labels))
            }
            // What goes on in it, a block's own `break`, a turn's `continue`.
            StmtKind::Break(_) | StmtKind::Continue(_) => true,
            StmtKind::Labeled(label, body) if labels.contains(label) => {
                dropped_at_each_exit(body, drop, (depth, caught, labels))
            }
            StmtKind::Labeled(..)
            | StmtKind::Switch { .. }
            | StmtKind::Try(..)
            | StmtKind::TryCatch(..)
            | StmtKind::For { .. }
            | StmtKind::ForOf { .. } => false,
            _ => caught || stmt.cannot_throw(),
        };
        if !ok {
            return false;
        }
        out.push(stmt);
    }
    *stmts = out;
    !dropped
}
