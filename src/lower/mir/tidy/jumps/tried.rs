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
/// `?`, whose rest's `let` is written before its exit.
pub(in crate::lower::mir::tidy) fn let_else_first(stmts: &mut Vec<Stmt>, let_elses: &[js::Span]) {
    let written_first =
        |e: &[Stmt], s: &[Stmt]| matches!((first_written(e), first_written(s)), (Some(e), Some(s)) if e < s);
    // Each of `exit`'s statements written in one `let … else`'s `else`, and
    // none of `then`'s.
    let let_else_exit = |exit: &[Stmt], then: &[Stmt]| {
        let written = |s: &&Stmt| !s.span.is_none();
        let within = |e: &js::Span, s: &Stmt| e.lo <= s.span.lo && s.span.hi <= e.hi;
        exit.iter().any(|s| written(&s))
            && let_elses.iter().any(|e| {
                exit.iter().filter(written).all(|s| within(e, s)) && !then.iter().filter(written).any(|s| within(e, s))
            })
    };
    let mut i = 0;
    while i < stmts.len() {
        let rest = &stmts[i + 1..];
        let first = match &stmts[i].kind {
            StmtKind::If(_, then, Some(otherwise)) => {
                otherwise.last().is_some_and(ends) && written_first(otherwise, then) && fits_in_place(stmts, i, then)
            }
            StmtKind::If(_, then, None) => {
                then.last().is_some_and(ends)
                    && rest.last().is_some_and(ends)
                    && let_else_exit(rest, then)
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
        let otherwise = otherwise.unwrap_or_else(|| stmts.split_off(i + 1));
        stmts.splice(
            i..=i,
            std::iter::once(StmtKind::If(test.negated_test(), otherwise, None).at(span)).chain(then),
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
/// `a` always leaving, and `rest` leaving, or going round again at the
/// body's end where `a` is more than one exit, a guard's: `if (!t) { rest
/// } a`, in the order the program wrote them,
/// `rest` ending in a `continue` where it went round; `a`'s last
/// `continue` goes on anyway. Not of a lone
/// exit, `if (!value) { return; }` of a `?`, written where the block it
/// shares is: a guard first, as a person writes it.
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
            let lone = match then.as_slice() {
                [
                    Stmt {
                        kind: StmtKind::Return(value),
                        ..
                    },
                ] => value.as_ref().is_none_or(Expr::is_constant),
                [
                    Stmt {
                        kind: StmtKind::Break(_) | StmtKind::Continue(_),
                        ..
                    },
                ] => true,
                _ => false,
            };
            // Where `rest` goes round, `a` more than a guard's exit.
            let goes_round = !rest.last().is_some_and(ends);
            !lone
                && !(goes_round && then.len() == 1)
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
/// by a `continue` inside it too, or where it ends, its `rest` an `else`,
/// `while (true) { if (c) { body } else { rest } }`: a `body` that goes on
/// with the loop, never by a `break`, which would run `rest` once it's
/// outside. React's `findFirstSuspended` walks up while a fiber has no
/// sibling, its `rest` a `continue` of the loop around (ADR 0702, amended).
pub(in crate::lower::mir::tidy) fn while_heads(stmts: &mut Vec<Stmt>, written: &[(js::Span, bool, Option<String>)]) {
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::While { label, cond, body } = &stmts[i].kind
            && matches!(cond.kind, ExprKind::Bool(true))
            && let Some((test, then, rest)) = tested_at_head(body, label.as_deref(), {
                written_loop(written, stmts[i].span).is_some_and(|&(_, w, _)| w)
            })
            && !own_jumps(rest, label.as_deref())
            && rest.last().is_some_and(ends)
        {
            let span = stmts[i].span;
            let looped = StmtKind::While {
                label: label.clone(),
                cond: test.clone(),
                body: then.to_vec(),
            };
            let rest = rest.to_vec();
            stmts.splice(i..=i, std::iter::once(looped.at(span)).chain(rest));
        }
        i += 1;
    }
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
    let (test, then, rest, otherwise) = match body {
        [
            Stmt {
                kind: StmtKind::If(test, then, None),
                ..
            },
            rest @ ..,
        ] => (test, then.as_slice(), rest, false),
        [
            Stmt {
                kind: StmtKind::If(test, then, Some(rest)),
                ..
            },
        ] if written => (test, then.as_slice(), rest.as_slice(), true),
        _ => return None,
    };
    let (last, before) = then.split_last()?;
    if own_continue(last) {
        let kept = !own_jumps(before, label) || (written && !breaks(before, label));
        return kept.then_some((test, before, rest));
    }
    // Going round where it ends, as an `else`'s `then` does, never on to
    // `rest`; or by a `continue` inside.
    let falls = !ends(last);
    let goes_round = (falls && otherwise) || js::continues(then, label);
    (written && goes_round && (otherwise || !falls) && !breaks(then, label)).then_some((test, then, rest))
}
