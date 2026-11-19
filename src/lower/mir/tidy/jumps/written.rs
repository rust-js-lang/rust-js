//! An `if`'s branches as the program wrote them (ADR 0879): its `else`, and
//! which way round its branches are where MIR turned them, a value's too,
//! told from MIR's guards and the `&&` and `||` it makes of the same
//! branches.

use super::*;

/// Whether `stmt` is `if (t) { a } else { e }` of an `if` the program wrote
/// with an `else`, `a` its `then` and `e` none of it, its `else` or the jump
/// MIR made of it, `break` where the loop goes on to what's after it: the
/// `else` it wrote, which an early exit of MIR's, `if (!t) { e } a`, isn't,
/// nor a `while`'s test (ADR 0879).
pub(super) fn written_else(stmt: &Stmt, elses: &[(js::Span, js::Span, js::Span)]) -> bool {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    let StmtKind::If(_, first, Some(second)) = &stmt.kind else {
        return false;
    };
    elses.iter().any(|(cond, then, otherwise)| {
        // A `while`'s desugaring's, its `else` the whole loop, isn't one.
        otherwise.lo >= then.hi
            && within(cond, stmt.span)
            && first.iter().any(|s| within(then, s.span))
            && (first.iter()).all(|s| s.span.is_none() || within(then, s.span) || within(cond, s.span))
            && (second.iter()).all(|s| !within(then, s.span) && !within(cond, s.span))
    })
}

/// Whether the `if` at `span` of branches `a` and `b` is one the program
/// wrote with an `else`, each branch's statements written in one of its
/// branches, `a` in its `then` and `b` in its `else` or the other way
/// round, MIR's turned: a value written with an `if`, not with the `&&` or
/// `||` MIR makes the same branches of (ADR 0879).
pub(in crate::lower::mir::tidy) fn written_branches(
    span: js::Span,
    a: &[Stmt],
    b: &[Stmt],
    elses: &[(js::Span, js::Span, js::Span)],
) -> bool {
    written_order(span, a, b, elses).is_some()
}

/// `written_branches`, and which way round: `Some(false)` where `a` is
/// written in the `then` and `b` in the `else`, `Some(true)` where MIR
/// turned them, `a` the `else`'s.
pub(in crate::lower::mir::tidy) fn written_order(
    span: js::Span,
    a: &[Stmt],
    b: &[Stmt],
    elses: &[(js::Span, js::Span, js::Span)],
) -> Option<bool> {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    // Where it's written: a `return` MIR made of a branch's value, where
    // the function ends, where its value is.
    let at = |branch: &js::Span, s: &Stmt| {
        within(branch, s.span) || matches!(&s.kind, StmtKind::Return(Some(v)) if within(branch, v.span))
    };
    let of = |branch: &js::Span, stmts: &[Stmt]| {
        stmts.iter().any(|s| at(branch, s)) && stmts.iter().all(|s| s.span.is_none() || at(branch, s))
    };
    elses.iter().find_map(|(cond, then, otherwise)| {
        if otherwise.lo < then.hi || !within(cond, span) {
            return None;
        }
        match (of(then, a) && of(otherwise, b), of(then, b) && of(otherwise, a)) {
            (true, _) => Some(false),
            (_, true) => Some(true),
            _ => None,
        }
    })
}

/// `written_branches` of a chain's values, `yes` where its tests hold and
/// `no` where one doesn't, each where it's written: `if a && b { x } else {
/// true }`'s, whose branches MIR nests in the tests (ADR 0879).
pub(super) fn written_values(
    span: js::Span,
    (yes, no): (&Expr, &Expr),
    elses: &[(js::Span, js::Span, js::Span)],
) -> bool {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    elses.iter().any(|(cond, then, otherwise)| {
        otherwise.lo >= then.hi
            && within(cond, span)
            && ((within(then, yes.span) && within(otherwise, no.span))
                || (within(then, no.span) && within(otherwise, yes.span)))
    })
}

/// `if (t) { exit } rest`, of an `if` the program wrote with an `else`
/// (`Written::elses`), as it wrote it: `if (t) { exit } else { rest }` where
/// `exit` is its `then`, and `if (!t) { rest } else { exit }` where it's its
/// `else`. MIR's structure makes a guard of it where `exit` never goes on,
/// a `return`, a `continue` or a `break`; `rest`, the statements after it,
/// each of the `if`'s other branch, end the list, as the `if` did. A
/// binding its condition makes, a let chain's `let Some(x) = e`, is its
/// `then`'s, where it's in scope.
/// Where statements the program wrote after the `if` follow its other
/// branch's, the branch is those before them, `if (t) { exit } else {
/// rest } more`. Its test says so (`Known::Else`), as each `if`'s does of
/// both branches the program wrote, so preparation keeps the `else`
/// (ADR 0237, amended, the user's decision, 2026-10-28).
/// And `if (t) { b } else { a }` of `if !t { a } else { b }`, its branches
/// MIR turned, a `switchInt` testing `t`: `if (!t) { a } else { b }`, as
/// written (ADR 0879).
pub(in crate::lower::mir::tidy) fn written_elses(
    stmts: &mut Vec<Stmt>,
    written: &crate::lower::mir::hints::Written,
    returns: &[js::Span],
) {
    linked_elses(stmts, (&written.elses, &written.else_ifs), returns, false);
}

/// `written_elses` of `stmts`, whose first is the next link of an `else if`
/// chain where `link`: the chain's, each link as written, a value's too.
fn linked_elses(
    stmts: &mut Vec<Stmt>,
    (elses, else_ifs): (&[(js::Span, js::Span, js::Span)], &[js::Span]),
    returns: &[js::Span],
    link: bool,
) {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    // Where a statement after the guard is written: a `return` MIR made of
    // a value, where the function ends, where its value is.
    let place = |s: &Stmt| match &s.kind {
        StmtKind::Return(Some(v)) if s.span.is_none() => v.span,
        _ => s.span,
    };
    // Each statement written in `branch`, one at least, or in `made`, the
    // condition of the branch it's a `then` of: a binding the condition
    // makes. Of what's after the guard, `placed`, each where `place` says.
    let of = |branch: &js::Span, made: Option<&js::Span>, stmts: &[Stmt], placed: bool| {
        let at = |s: &Stmt| if placed { place(s) } else { s.span };
        stmts.iter().any(|s| within(branch, at(s)))
            && (stmts.iter())
                .all(|s| at(s).is_none() || within(branch, at(s)) || made.is_some_and(|c| within(c, at(s))))
    };
    // A jump MIR made alone each way, `if (t) { return a; } return b;` of a
    // value the program wrote with an `if`, reads as it is, a conditional;
    // and an `else` of only the `return;` MIR made of a `None` or a `()` is
    // no `else` (ADR 0237, amended). Not the next link of a chain, which is
    // one.
    let made = |s: &Stmt| match s.kind {
        StmtKind::Return(_) => !super::written(returns, s.span),
        StmtKind::Break(_) | StmtKind::Continue(_) => s.span.is_none(),
        _ => false,
    };
    let jumps = |s: &Stmt| matches!(s.kind, StmtKind::Break(_) | StmtKind::Continue(_));
    let nothing = |s: &Stmt| match &s.kind {
        StmtKind::Return(None) => made(s),
        StmtKind::Return(Some(v)) => matches!(v.kind, ExprKind::Undefined) && made(s),
        _ => false,
    };
    let plain = |exit: &[Stmt], rest: &[Stmt], linked: bool| matches!(rest, [s] if nothing(s) || !linked && made(s) && matches!(exit, [e] if made(e)));
    // An `else if let`'s, written at `otherwise`, whose binding, a `const`,
    // comes first: no `else if` of JS's, its links each an `if` after the
    // last, where each leaves (ADR 0237, amended).
    let binds_link = |list: &[Stmt], otherwise: &js::Span| {
        matches!(
            list.first(),
            Some(Stmt {
                kind: StmtKind::Const(..),
                ..
            })
        ) && else_ifs.contains(otherwise)
    };
    let mut at = 0;
    while at < stmts.len() {
        // The chain's next link, after what an `else if let` binds.
        let linked = link && stmts[..at].iter().all(|s| matches!(s.kind, StmtKind::Const(..)));
        // Its branches MIR turned, `if !c { a } else { b }`'s `if (c) { b }
        // else { a }` of a `switchInt` that tests `c`: as written, `a`
        // first (ADR 0879).
        if let StmtKind::If(_, first, Some(second)) = &stmts[at].kind
            && let Some((cond, then, otherwise)) = elses.iter().find(|(cond, ..)| within(cond, stmts[at].span))
            && otherwise.lo >= then.hi
            && of(otherwise, None, first, false)
            && of(then, Some(cond), second, false)
        {
            let StmtKind::If(test, first, Some(second)) =
                std::mem::replace(&mut stmts[at].kind, StmtKind::Expr(Expr::js_undefined()))
            else {
                unreachable!("matched")
            };
            stmts[at].kind = StmtKind::If(test.negated_test(), second, Some(first));
        }
        if let StmtKind::If(_, exit, None) = &stmts[at].kind
            && exit.last().is_some_and(ends)
            && let Some((cond, then, otherwise)) = elses.iter().find(|(cond, ..)| within(cond, stmts[at].span))
        {
            let rest = &stmts[at + 1..];
            // The statements after the guard that are the other branch's:
            // those up to the last written in it, where what follows is
            // written after the whole `if`, the statements the program
            // wrote after it (ADR 0879, amended); or, where none follows,
            // all of them.
            let branch = |b: &js::Span, made: Option<&js::Span>| {
                let end = otherwise.hi.max(then.hi);
                let after = |s: &Stmt| place(s).is_none() || place(s).lo >= end;
                let k = rest.iter().rposition(|s| within(b, place(s))).map_or(0, |k| k + 1);
                let more = rest[k..].iter().any(|s| !place(s).is_none());
                match more {
                    true => (of(b, made, &rest[..k], true) && rest[k..].iter().all(after)).then_some(k),
                    false => of(b, made, rest, true).then_some(rest.len()),
                }
            };
            let turned = match (of(then, Some(cond), exit, true), of(otherwise, None, exit, true)) {
                _ if plain(exit, rest, linked) => None,
                (true, false) => branch(otherwise, None).map(|k| (false, k)),
                (false, true) => branch(then, Some(cond)).map(|k| (true, k)),
                // A jump alone MIR made, or put where the test is, `if (!b)
                // { break; }` of a written `else { break; }` (`early
                // breaks`): the branch the rest isn't written in. Not a
                // `while`'s, whose `else` is the whole loop. The `return;`
                // MIR made of a `break` of the loop a function ends with
                // too, which `kept_jumps` makes it again (ADR 0684, amended).
                (false, false)
                    if otherwise.lo >= then.hi
                        && matches!(&exit[..], [e] if jumps(e) && (e.span.is_none() || within(cond, e.span))
                            || matches!(e.kind, StmtKind::Return(None)) && made(e)) =>
                {
                    (branch(then, Some(cond)).map(|k| (true, k)))
                        .or_else(|| branch(otherwise, None).map(|k| (false, k)))
                }
                _ => None,
            };
            // What's made the branch declares nothing what's after it reads,
            // which a block would put out of its scope: a late `let x;` of
            // `let x; if c { return; } else if let Some(p) = o { x = p; }
            // else { x = 7; } f(x)` stays before the `if` (rd578).
            let turned = turned.and_then(|(turned, k)| {
                let (moved, after) = stmts[at + 1..].split_at(k);
                bindings::kept_declarations(moved, after).map(|kept| (turned, k, kept))
            });
            if let Some((turned, k, kept)) = turned {
                let after = stmts.split_off(at + 1 + k);
                let mut rest = stmts.split_off(at + 1);
                stmts.extend(after);
                at += bindings::declared_before(stmts, at, &mut rest, &kept);
                let StmtKind::If(test, mut exit, None) =
                    std::mem::replace(&mut stmts[at].kind, StmtKind::Expr(Expr::js_undefined()))
                else {
                    unreachable!("matched")
                };
                // The jump `early breaks` put where the test is: written in
                // neither branch, so no `let … else`'s, written first.
                if let [e] = &mut exit[..]
                    && jumps(e)
                    && within(cond, e.span)
                {
                    e.span = js::Span::NONE;
                }
                let linking = binds_link(if turned { &exit } else { &rest }, otherwise);
                // A guard of its own in what's now the `else`, an `else if`'s,
                // which the walk has passed: its chain's next link, where the
                // chain holds.
                linked_elses(&mut rest, (elses, else_ifs), returns, !turned && !linking);
                stmts[at].kind = match turned {
                    false => StmtKind::If(test, exit, Some(rest)),
                    true => StmtKind::If(test.negated_test(), rest, Some(exit)),
                };
                if !linking {
                    stmts[at].facts = stmts[at].facts.with(js::Known::Else);
                }
            }
        }
        // An `if` with both the branches the program wrote, as MIR left it:
        // its test says so, which preparation keeps the `else` of (ADR 0237,
        // amended).
        // Where its `then` leaves and MIR's `else` goes on to the statements
        // the program wrote after the `if`, `if (a) { return; } else if (b)
        // { return 7; } return 9;`, those follow it, as written.
        let span = stmts[at].span;
        if written_else(&stmts[at], elses)
            && let Some((_, branch, other)) = elses.iter().find(|(cond, ..)| within(cond, span))
            && let StmtKind::If(_, then, Some(otherwise)) = &mut stmts[at].kind
            && !plain(then, otherwise, linked)
            && !binds_link(otherwise, other)
        {
            let end = other.hi.max(branch.hi);
            let k = otherwise
                .iter()
                .rposition(|s| within(other, place(s)))
                .map_or(0, |k| k + 1);
            if k > 0
                && then.last().is_some_and(ends)
                && otherwise[k..].iter().any(|s| !place(s).is_none())
                && otherwise[k..].iter().all(|s| place(s).is_none() || place(s).lo >= end)
                && let Some(kept) = bindings::kept_declarations(&otherwise[..k], &otherwise[k..])
            {
                let after = otherwise.split_off(k);
                let mut kept_in = std::mem::take(otherwise);
                stmts.splice(at + 1..at + 1, after);
                let declared = bindings::declared_before(stmts, at, &mut kept_in, &kept);
                let StmtKind::If(_, _, Some(otherwise)) = &mut stmts[at + declared].kind else {
                    unreachable!("matched")
                };
                *otherwise = kept_in;
                at += declared;
            }
            stmts[at].facts = stmts[at].facts.with(js::Known::Else);
        }
        at += 1;
    }
}
