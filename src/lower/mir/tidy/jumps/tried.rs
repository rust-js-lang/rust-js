//! A binding's jumps: a `let else`'s and a `?`'s first, and the steps a
//! `while` writes in order.

use super::*;

/// What a `for` binds of `item`, read by the first statements of `body`:
/// `const x = item;`, or `const a = item[0]; const b = item[1];`, its
/// pattern, whether it's a `let`, and how many statements; `_` where nothing
/// reads it. And each part read again by a `const`, `const c = item[1];`
/// after `b`'s, MIR's of a pattern's two bindings of one element: an alias,
/// `c` of `b`, which the body reads by the name the pattern gives it.
pub(in crate::lower::mir::tidy) type Binding = (js::Pattern, bool, usize, Vec<(String, String)>);
pub(in crate::lower::mir::tidy) fn binding(body: &[Stmt], item: &str) -> Option<Binding> {
    let reads = |value: &Expr| -> Option<Result<Option<usize>, String>> {
        match &value.kind {
            ExprKind::Var(v) if v == item => Some(Ok(None)),
            ExprKind::Index(of, index)
                if matches!(&of.kind, ExprKind::Var(v) if v == item)
                    && let ExprKind::Num(k) = index.kind
                    && k >= 0.0
                    && k.fract() == 0.0 =>
            {
                Some(Ok(Some(k as usize)))
            }
            ExprKind::Member(of, key) if matches!(&of.kind, ExprKind::Var(v) if v == item) => Some(Err(key.clone())),
            _ => None,
        }
    };
    let mut parts: Vec<(Result<Option<usize>, String>, String, bool)> = Vec::new();
    let mut aliases = Vec::new();
    let mut taken = 0;
    for stmt in body {
        let (bound, value, mutable) = match &stmt.kind {
            StmtKind::Const(bound, value) => (bound, value, false),
            StmtKind::Let(bound, Some(value)) => (bound, value, true),
            // A `mut` binding MIR declares before the loop, `row = item`: the
            // loop's own, its outer `let` then read by nothing.
            StmtKind::Assign(target, value)
                if let ExprKind::Var(bound) = &target.kind
                    && parts.is_empty() =>
            {
                (bound, value, assigns(&body[1..], bound))
            }
            _ => break,
        };
        let Some(part) = reads(value) else { break };
        match parts.iter().find(|(p, ..)| *p == part) {
            None => parts.push((part, bound.clone(), mutable)),
            Some((_, first, false)) if !mutable => aliases.push((bound.clone(), first.clone())),
            Some(_) => break,
        }
        taken += 1;
    }
    let mutable = parts.iter().any(|&(_, _, m)| m);
    let pattern = match parts.as_slice() {
        [] if js::mentions_in(body, item) == 0 => js::Pattern::Name("_".to_string()),
        [(Ok(None), bound, _)] => js::Pattern::Name(bound.clone()),
        _ if !parts.is_empty() && parts.iter().all(|(p, ..)| matches!(p, Ok(Some(_)))) => {
            let last = parts.iter().filter_map(|(p, ..)| p.clone().ok().flatten()).max()?;
            let mut items = vec![None; last + 1];
            for (part, bound, _) in &parts {
                items[part.clone().ok().flatten()?] = Some(bound.clone());
            }
            js::Pattern::names_array(items)
        }
        _ if !parts.is_empty() && parts.iter().all(|(p, ..)| p.is_err()) => {
            let fields = (parts.iter())
                .map(|(p, bound, _)| (p.clone().expect_err("keyed"), js::Pattern::Name(bound.clone()), None))
                .collect();
            js::Pattern::Object(fields, None)
        }
        _ => return None,
    };
    Some((pattern, mutable, taken, aliases))
}

/// `if (c) { s } else { e }` of an `e` that always leaves, written before
/// `s`, a `let .. else`'s: `if (!c) { e } s`, as the program wrote it, the
/// guard first, where nothing `s` declares is declared beside it. And `if
/// (c) { s } e` of an `s` that never goes on, `e` the rest, which leaves,
/// written in a `let … else`'s `else` (`let_elses`) where `s` isn't: `let
/// Some(x) = .. else { return a; }; return f(x);`, whose two `return`s MIR
/// joins after the `if` (ADR 0879, amended). Not of a guard already
/// first, `if (r.TAG === "Err") { return .. } const x = r._0; ..` of a
/// `?`, whose rest's `let` is written before its exit. And of `else`s that
/// each `break` a loop, whose `break`s MIR joins into one after the `if`,
/// `if (c) { s } break;` or `if (c) { s } else { e } break;`: each its
/// guard, `if (!c) { break; }`, `if (!c) { e break; }`, its test marked so
/// (`Known::Guard`), which no loop's head takes (ADR 0879, amended).
pub(in crate::lower::mir::tidy) fn let_else_first(stmts: &mut Vec<Stmt>, let_elses: &[js::Span]) {
    // Where a statement is written: a `return`'s where its value is, as each
    // way of a `?` returns at the `?` (q1).
    let place = |s: &Stmt| match &s.kind {
        StmtKind::Return(Some(v)) if !v.span.is_none() => v.span,
        _ => s.span,
    };
    let first = |list: &[Stmt]| match list.first() {
        Some(
            s @ Stmt {
                kind: StmtKind::Return(_),
                ..
            },
        ) if !place(s).is_none() => Some(place(s).lo),
        _ => first_written(list),
    };
    let written_first = |e: &[Stmt], s: &[Stmt]| matches!((first(e), first(s)), (Some(e), Some(s)) if e < s);
    // The `let … else`'s `else` each of `exit`'s statements is written in,
    // and none of `then`'s; but its last, a jump MIR placed where the loop
    // or the body ends, which each `else` it ends shares.
    let let_else_exit = |exit: &[Stmt], then: &[Stmt]| {
        let written = |s: &&Stmt| !place(s).is_none();
        let within = |e: &js::Span, s: &Stmt| e.lo <= place(s).lo && place(s).hi <= e.hi;
        let exit = match exit.split_last() {
            Some((last, before))
                if matches!(
                    last.kind,
                    StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Return(_)
                ) =>
            {
                match let_elses.iter().any(|e| within(e, last)) {
                    true => exit,
                    false => before,
                }
            }
            _ => exit,
        };
        (exit.iter().any(|s| written(&s)))
            .then(|| {
                let_elses.iter().copied().find(|e| {
                    exit.iter().filter(written).all(|s| within(e, s))
                        && !then.iter().filter(written).any(|s| within(e, s))
                })
            })
            .flatten()
    };
    // The `let … else` whose test `test` is, its `else` written right after
    // it and before `then`, the rest.
    let else_after = |test: &Expr, then: &[Stmt]| {
        (!test.span.is_none())
            .then(|| {
                (let_elses.iter().copied())
                    .filter(|e| e.lo >= test.span.hi)
                    .min_by_key(|e| e.lo)
                    .filter(|e| first_written(then).is_none_or(|at| at >= e.hi))
            })
            .flatten()
    };
    // Whether `stmt` is written in `at`, an `else`.
    let within = |at: js::Span, stmt: &Stmt| !stmt.span.is_none() && at.lo <= stmt.span.lo && stmt.span.hi <= at.hi;
    // A `let … else`'s test negated, its guard's, as written (`Known::Guard`).
    let guard = |test: Expr| {
        let test = test.negated_test();
        Expr {
            facts: test.facts.with(js::Known::Guard),
            ..test
        }
    };
    let mut i = 0;
    while i < stmts.len() {
        // `if (c) { s } e` of a lone jump `e` MIR joined the `else`s after it
        // into, not written in the `else` of the `let … else` whose test `c`
        // is: its guard first, `if (!c) { e }`, then `s`, and `e` where `s`
        // goes on to it.
        if let StmtKind::If(test, then, None) = &stmts[i].kind
            && let [exit] = &stmts[i + 1..]
            && matches!(
                exit.kind,
                StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Return(_)
            )
            && let Some(written) = else_after(test, then)
            && !within(written, exit)
            && fits_in_place(stmts, i, then)
        {
            let exit = stmts.pop().expect("matched");
            let span = stmts[i].span;
            let StmtKind::If(test, then, _) = stmts.pop().expect("matched").kind else {
                unreachable!("matched")
            };
            let goes_on = !then.last().is_some_and(ends);
            stmts.push(StmtKind::If(guard(test), vec![exit.clone()], None).at(span));
            stmts.extend(then);
            if goes_on {
                stmts.push(exit);
            }
            i += 1;
            continue;
        }
        // `if (c) { e } else { s } return;` of an `e` written in a `let …
        // else`'s `else`, first, and the `return;` a unit function ends
        // with, which `e` goes on to: `if (c) { e return; } s` (ADR 0879,
        // amended).
        if let StmtKind::If(_, then, Some(otherwise)) = &stmts[i].kind
            && let [exit] = &stmts[i + 1..]
            && matches!(exit.kind, StmtKind::Return(None))
            && !then.last().is_some_and(ends)
            && let_else_exit(then, otherwise).is_some_and(|written| !within(written, exit))
            && written_first(then, otherwise)
            && fits_in_place(stmts, i, otherwise)
        {
            let exit = stmts.pop().expect("matched");
            let span = stmts[i].span;
            let StmtKind::If(test, mut then, Some(otherwise)) = stmts.pop().expect("matched").kind else {
                unreachable!("matched")
            };
            let goes_on = !otherwise.last().is_some_and(ends);
            then.push(exit.clone());
            let test = Expr {
                facts: test.facts.with(js::Known::Guard),
                ..test
            };
            stmts.push(StmtKind::If(test, then, None).at(span));
            stmts.extend(otherwise);
            if goes_on {
                stmts.push(exit);
            }
            i += 1;
            continue;
        }
        let rest = &stmts[i + 1..];
        // A lone jump after an `else` that goes on to it, MIR's, which the
        // `else` ends with.
        let mut joined = false;
        let first = match &stmts[i].kind {
            StmtKind::If(_, then, Some(otherwise)) if otherwise.last().is_some_and(ends) => {
                written_first(otherwise, then) && fits_in_place(stmts, i, then)
            }
            // `if (c) { s } else { e } j` of an `s` that never goes on and
            // an `e` written in a `let … else`'s `else` that goes on to `j`,
            // a jump MIR joined the `else`s into: `if (!c) { e j } s`.
            StmtKind::If(_, then, Some(otherwise)) => {
                joined = true;
                then.last().is_some_and(ends)
                    && matches!(rest, [exit] if matches!(exit.kind, StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Return(_)))
                    && let_else_exit(otherwise, then).is_some_and(|written| !within(written, &rest[0]))
                    && written_first(otherwise, then)
                    && fits_in_place(stmts, i, then)
            }
            StmtKind::If(_, then, None) => {
                then.last().is_some_and(ends)
                    && rest.last().is_some_and(ends)
                    && let_else_exit(rest, then).is_some()
                    && written_first(rest, then)
                    && fits_in_place(stmts, i, then)
            }
            _ => false,
        };
        if !first {
            i += 1;
            continue;
        }
        let span = stmts[i].span;
        let StmtKind::If(test, then, otherwise) = std::mem::replace(&mut stmts[i].kind, StmtKind::Break(None)) else {
            unreachable!("matched")
        };
        let otherwise = match (otherwise, joined) {
            (Some(mut otherwise), true) => {
                otherwise.extend(stmts.pop());
                otherwise
            }
            (Some(otherwise), false) => otherwise,
            (None, _) => stmts.split_off(i + 1),
        };
        stmts.splice(
            i..=i,
            std::iter::once(StmtKind::If(guard(test), otherwise, None).at(span)).chain(then),
        );
        i += 1;
    }
}

/// `?` of a `Result`, `if (r.TAG !== "Err") { s } else { return r; }`, or
/// `if (r.TAG !== "Err") { s } return r;` of an `s` that never goes on: its
/// `return` first, `if (r.TAG === "Err") { return r; } s`, where nothing
/// `s` declares is declared beside it.
pub(in crate::lower::mir::tidy) fn tried_first(stmts: &mut Vec<Stmt>) {
    let returns = |s: Option<&Stmt>, test: &Expr| {
        matches!(s, Some(Stmt { kind: StmtKind::Return(Some(returned)), .. })
            if matches!(result::tested(test), Some((of, false, false)) if js::same_path(of, returned)
                && matches!(returned.kind, ExprKind::Var(_))))
    };
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::If(test, then, otherwise) = &stmts[i].kind else {
            i += 1;
            continue;
        };
        let tried = match otherwise {
            Some(otherwise) => otherwise.len() == 1 && returns(otherwise.first(), test),
            None => then.last().is_some_and(ends) && returns(stmts.get(i + 1), test),
        };
        if !tried || !fits_in_place(stmts, i, then) {
            i += 1;
            continue;
        }
        let span = stmts[i].span;
        let StmtKind::If(test, then, otherwise) = std::mem::replace(&mut stmts[i].kind, StmtKind::Break(None)) else {
            unreachable!("matched")
        };
        let exit = match otherwise {
            Some(otherwise) => otherwise,
            None => vec![stmts.remove(i + 1)],
        };
        // What it held, here now, a `?` of its own too: looked at next.
        stmts.splice(
            i..=i,
            std::iter::once(StmtKind::If(test.negated_test(), exit, None).at(span)).chain(then),
        );
        i += 1;
    }
}

/// `?` of an `Ok` just made, `const r = { TAG: "Ok", _0: 3 }; if (r.TAG ===
/// "Err") { return r; }`, or `if (r.TAG !== "Err") { a } else { b }`, past
/// the `let`s its branches assign, or first in a labeled block: no `Err`, so
/// the branch it takes in its place, and what it holds, a constant or a name,
/// read where `r._0` is, `r` gone once nothing reads it.
/// `serialize_field(..)?` of a hand-written `Serialize` (ADR 0459), and
/// `Ok(3)?` as it's written.
pub(in crate::lower::mir::tidy) fn made_ok_tried(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::Const(name, made) = &stmts[i].kind else {
            i += 1;
            continue;
        };
        let Some(payload) = result::made_ok_payload(made) else {
            i += 1;
            continue;
        };
        let (name, payload) = (name.clone(), payload.cloned());
        // The `if` that tests it: next, past the `let`s declared for its
        // branches, or first in a labeled block there.
        let mut at = i + 1;
        while matches!(stmts.get(at).map(|s| &s.kind), Some(StmtKind::Let(_, None))) {
            at += 1;
        }
        let folded = match stmts.get_mut(at).map(|s| &mut s.kind) {
            Some(StmtKind::Labeled(_, body)) => taken_in_place(body, 0, &name),
            Some(_) => taken_in_place(stmts, at, &name),
            None => false,
        };
        if !folded {
            i += 1;
            continue;
        }
        // What it holds, read in its place, where reading it there is the same.
        if let Some(value) = payload.filter(|value| value.is_constant() || matches!(value.kind, ExprKind::Var(_))) {
            js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
                if result::reads_held(e, &name) {
                    *e = value.clone();
                }
            });
        }
        let mut read = false;
        js::visit_stmts(&stmts[i + 1..], &mut |n| read |= n == name);
        let StmtKind::Const(_, made) = &stmts[i].kind else {
            unreachable!("matched")
        };
        if !read && !made.has_effects() {
            stmts.remove(i);
        }
        // What the branch taken holds, a `?` of its own too, looked at next.
    }
}

/// `stmts[at]`, an `if` testing whether `name`, an `Ok`, is one: the branch it
/// takes, in its place. Whether it was one.
fn taken_in_place(stmts: &mut Vec<Stmt>, at: usize, name: &str) -> bool {
    let Some(StmtKind::If(test, _, _)) = stmts.get(at).map(|s| &s.kind) else {
        return false;
    };
    let Some((of, ok, equal)) = result::tested(test) else {
        return false;
    };
    if !matches!(&of.kind, ExprKind::Var(r) if r == name) {
        return false;
    }
    let StmtKind::If(_, then, otherwise) = std::mem::replace(&mut stmts[at].kind, StmtKind::Break(None)) else {
        unreachable!("matched")
    };
    let taken = match ok == equal {
        true => then,
        false => otherwise.unwrap_or_default(),
    };
    stmts.splice(at..=at, taken);
    true
}

/// A loop's body `if (t) { a; continue; } rest`, `a` written after `rest`,
/// `a` always leaving and more than one exit, a guard's, and `rest`
/// leaving or going round again at the body's end: `if (!t) { rest } a`, in
/// the order the program wrote them, `rest` ending in a `continue` where it
/// went round; `a`'s last `continue` goes on anyway. Not of a lone exit,
/// `if (!value) { return; }` of a `?`, written where the block it shares
/// is: a guard first, as a person writes it.
pub(in crate::lower::mir::tidy) fn written_order_steps(stmts: &mut [Stmt]) {
    for stmt in stmts {
        let (StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. }) = &mut stmt.kind
        else {
            continue;
        };
        let found = (0..body.len()).find(|&i| {
            let StmtKind::If(_, then, None) = &body[i].kind else {
                return false;
            };
            let rest = &body[i + 1..];
            // Not `a` of one exit alone, a guard's: of a lone `return;`, a
            // `?`'s, as it's written; of another, `return f(x);`, a later
            // stage puts it where the program wrote it (#382).
            then.len() != 1
                && then.last().is_some_and(ends)
                && matches!((first_written(rest), first_written(then)), (Some(r), Some(a)) if r < a)
        });
        let Some(i) = found else { continue };
        let mut rest = body.split_off(i + 1);
        // What ends the body goes round again: in an `if` now, by a `continue`.
        if !rest.last().is_some_and(ends) {
            let span = rest.last().map_or(js::Span::NONE, |s| s.span);
            rest.push(StmtKind::Continue(None).at(span));
        }
        let Some(Stmt {
            kind: StmtKind::If(test, mut then, None),
            span,
        }) = body.pop()
        else {
            unreachable!("found")
        };
        if matches!(then.last().map(|l| &l.kind), Some(StmtKind::Continue(None))) {
            then.pop();
        }
        body.push(StmtKind::If(test.negated_test(), rest, None).at(span));
        body.extend(then);
    }
}

/// `while (true) { if (c) { body; continue; } rest }`, a loop MIR tests at
/// its head whose `rest`, what's after it, always leaves elsewhere: `while
/// (c) { body } rest`, where nothing else jumps in it
/// to the loop, as `rest`'s jumps would go to another once it's outside.
/// Of a `while` the program wrote, of `written`, its `body` may go round
/// by a `continue` inside it too: a `body` that goes on with the loop,
/// never by a `break`, which would run `rest` once it's outside. React's
/// `findFirstSuspended` walks up while a fiber has no sibling, its `rest`
/// a `continue` of the loop around (ADR 0702, amended). No `body` here goes
/// round where it ends, its `rest` an `else`, `while (true) { if (c) { body
/// } else { rest } }`: the stages before take such an `else` apart.
/// Its `rest`'s own `break`, which left the loop to what follows it,
/// falls out to there (`fallen`).
pub(in crate::lower::mir::tidy) fn while_heads(stmts: &mut Vec<Stmt>, written: &[(js::Span, bool, Option<String>)]) {
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::While { label, cond, body } = &stmts[i].kind
            && matches!(cond.kind, ExprKind::Bool(true))
            && let Some((test, then, rest)) = tested_at_head(body, label.as_deref(), {
                written_loop(written, stmts[i].span).is_some_and(|&(_, w, _)| w)
            })
            && rest.last().is_some_and(ends)
            && let Some(rest) = fallen(rest, label.as_deref())
        {
            let span = stmts[i].span;
            let looped = StmtKind::While {
                label: label.clone(),
                cond: test.clone(),
                body: then.to_vec(),
            };
            stmts.splice(i..=i, std::iter::once(looped.at(span)).chain(rest));
        }
        i += 1;
    }
}

/// A `while`'s `rest`, `while_heads`', after the loop: its first jump of
/// the loop, a `break`, which leaves the loop for what follows it, gone
/// with what's after it, which never runs, as it falls out to there once
/// `rest` is after the loop (ADR 0702, amended). None where it jumps to
/// the loop otherwise.
fn fallen(rest: &[Stmt], label: Option<&str>) -> Option<Vec<Stmt>> {
    let own_break = |s: &Stmt| matches!(&s.kind, StmtKind::Break(l) if l.as_deref().is_none_or(|l| Some(l) == label));
    let Some(at) = rest.iter().position(|s| own_jumps(std::slice::from_ref(s), label)) else {
        return Some(rest.to_vec());
    };
    own_break(&rest[at]).then(|| rest[..at].to_vec())
}

/// Of a `while (true)`'s `body`, `while_heads`': its test, what each turn
/// does then, without its `continue` at the end, and what's after.
fn tested_at_head<'s>(
    body: &'s [Stmt],
    label: Option<&str>,
    written: bool,
) -> Option<(&'s Expr, &'s [Stmt], &'s [Stmt])> {
    let own_continue =
        |s: &Stmt| matches!(&s.kind, StmtKind::Continue(l) if l.as_deref().is_none_or(|l| Some(l) == label));
    let [
        Stmt {
            kind: StmtKind::If(test, then, None),
            ..
        },
        rest @ ..,
    ] = body
    else {
        return None;
    };
    let (last, before) = then.split_last()?;
    if own_continue(last) {
        let kept = !own_jumps(before, label) || (written && !breaks(before, label));
        return kept.then_some((test, before, rest));
    }
    // Going round by a `continue` inside, never on to `rest`.
    (written && ends(last) && js::continues(then, label) && !breaks(then, label)).then_some((
        test,
        then.as_slice(),
        rest,
    ))
}
