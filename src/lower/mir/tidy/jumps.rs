//! Tidy's jumps: `break`s, `continue`s, `return`s and labels, as a person
//! writes a loop's or a block's exits.

mod continues;
mod labels;
mod tails;
mod tried;

pub(super) use self::{continues::*, labels::*, tails::*, tried::*};

use super::*;
use crate::js::result;

/// A loop's body that ends `if (c) { a; continue l; } break;`, `l` the loop
/// or none, or any jump, or, where `nested`, an `if` that's of this shape
/// too: `if (!c) { break; } a`, as a person writes an early exit; and
/// of `if (b) { if (c) { a; continue l; } } break;`, a `while let`'s test
/// then its own, `if (!(b && c)) { break; } a`.
pub(super) fn early_breaks(stmts: &mut [Stmt], nested: bool) {
    for stmt in stmts.iter_mut() {
        let (label, body) = match &mut stmt.kind {
            StmtKind::While { label, body, .. }
            | StmtKind::ForOf { label, body, .. }
            | StmtKind::For { label, body, .. } => (label.clone(), body),
            _ => continue,
        };
        let continues = |s: &Stmt| match &s.kind {
            StmtKind::Continue(None) => true,
            StmtKind::Continue(Some(l)) => Some(l) == label.as_ref(),
            _ => false,
        };
        if broken_in_then(body, &continues) {
            continue;
        }
        // Tests each of nothing but the next: one test, `b && c`.
        let mut tests = Vec::new();
        let mut then: &[Stmt] = match body.as_slice() {
            [.., last_if, last] if matches!(&last.kind, StmtKind::Break(None)) => std::slice::from_ref(last_if),
            _ => continue,
        };
        while let [
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] = then
        {
            tests.push(test.clone());
            then = inner;
        }
        // An `if` it ends in, once its tests are read in place.
        let leaves = match nested {
            true => leaves_early(then),
            false => then.last().is_some_and(ends),
        };
        if tests.is_empty() || !leaves {
            continue;
        }
        // What it puts in the body is named apart from what's there, each
        // of its scopes inside the body's.
        if tests.len() > 1 {
            let then = then.to_vec();
            let test = (tests.into_iter())
                .reduce(|a, b| Expr::bin(Op::And, a, b))
                .expect("a test");
            let at = body.len() - 2;
            let span = body[at].span;
            body[at] = StmtKind::If(test, then, None).at(span);
        }
        body.pop();
        let Some(Stmt {
            kind: StmtKind::If(test, then, None),
            span,
        }) = body.pop()
        else {
            unreachable!("matched")
        };
        body.extend(broken_early(test, then, span, &continues));
    }
}

/// A loop's body that ends `if (c) { a } else { b; continue l; } break;`,
/// `a` going on to the `break`, MIR's of `if c { a; break; } b`: that, as
/// the program wrote it (ADR 0730), `b` the body's own where it declares
/// nothing beside it. Whether it was.
fn broken_in_then(body: &mut Vec<Stmt>, continues: &dyn Fn(&Stmt) -> bool) -> bool {
    let [.., last_if, last] = body.as_slice() else {
        return false;
    };
    let StmtKind::If(_, then, Some(otherwise)) = &last_if.kind else {
        return false;
    };
    if !matches!(last.kind, StmtKind::Break(None))
        || !otherwise.last().is_some_and(continues)
        || then.last().is_some_and(ends)
        || !fits_in_place(&body[..body.len() - 1], body.len() - 2, otherwise)
    {
        return false;
    }
    let brk = body.pop().expect("matched");
    let Some(Stmt {
        kind: StmtKind::If(test, mut then, Some(mut otherwise)),
        span,
    }) = body.pop()
    else {
        unreachable!("matched")
    };
    then.push(brk);
    otherwise.pop();
    body.push(StmtKind::If(test, then, None).at(span));
    body.extend(otherwise);
    true
}

/// Whether `then` never goes on, or ends in an `if` whose own does so.
fn leaves_early(then: &[Stmt]) -> bool {
    match then.last() {
        Some(last) if ends(last) => true,
        Some(Stmt {
            kind: StmtKind::If(_, inner, None),
            ..
        }) => leaves_early(inner),
        _ => false,
    }
}

/// `if (t) { a } break;`, of an `a` `leaves_early` takes: `if (!t) { break;
/// } a`, an `if` it ends in so too, and a `continue` it ends in, which goes
/// round anyway, gone.
fn broken_early(test: Expr, mut then: Vec<Stmt>, span: js::Span, continues: &dyn Fn(&Stmt) -> bool) -> Vec<Stmt> {
    let mut out = vec![StmtKind::If(test.negated_test(), vec![StmtKind::Break(None).at(span)], None).at(span)];
    match then.pop() {
        Some(last) if continues(&last) => out.extend(then),
        Some(Stmt {
            kind: StmtKind::If(inner_test, inner, None),
            span: inner_span,
        }) => {
            out.extend(then);
            out.extend(broken_early(inner_test, inner, inner_span, continues));
        }
        Some(last) => {
            out.extend(then);
            out.push(last);
        }
        None => {}
    }
    out
}

/// A `while (true)` whose body ends `if (c) { a } return x;`, MIR's loop
/// that returns where it ends, `while let Some(p) = it.next()` of a
/// function that gives `None` after it, as a person writes an early exit:
/// - `a` going on, `if (c) { a; continue; }`: `if (!c) { break; } a` and
///   the `return x;` after the loop, as `early_breaks` makes of one that
///   breaks; not where something else leaves the loop, nor of what it
///   declares, which isn't seen after it.
/// - `a` never going on past its own end: `if (!c) { return x; } a` (ADR 0370).
pub(super) fn early_returns(stmts: &mut Vec<Stmt>, elses: &[(js::Span, js::Span, js::Span)]) {
    let mut at = 0;
    while at < stmts.len() {
        while let Some(after) = early_return(&mut stmts[at], elses) {
            if let Some(after) = after {
                stmts.insert(at + 1, after);
                break;
            }
        }
        at += 1;
    }
}

/// One of `early_returns`'s steps of `stmt`, if it takes one: the `return`
/// it leaves after the loop, if it does. Its body's end, `if (c) { a }
/// return x;`, or `if (c) { a } else { e }` of a lone exit `e`, `return;` as
/// MIR's `?` is, or `break;`: `if (!c) { e } a`, as `a` goes on to the next
/// time round where the body ends. Not of an `else` the program wrote,
/// `if let Some(c) = c { a } else { break }`, which reads as written (ADR
/// 0879).
pub(super) fn early_return(stmt: &mut Stmt, elses: &[(js::Span, js::Span, js::Span)]) -> Option<Option<Stmt>> {
    let StmtKind::While { label, cond, body } = &mut stmt.kind else {
        return None;
    };
    if !matches!(cond.kind, ExprKind::Bool(true)) {
        return None;
    }
    let continues = |s: &Stmt| match &s.kind {
        StmtKind::Continue(None) => true,
        StmtKind::Continue(Some(l)) => Some(l) == label.as_ref(),
        _ => false,
    };
    let is_return = |s: &Stmt| matches!(s.kind, StmtKind::Return(_));
    let exits = |s: &Stmt| matches!(s.kind, StmtKind::Return(_) | StmtKind::Break(_) | StmtKind::Continue(_));
    // Where the `if` is, and whether the exit is its `else`.
    let (at, in_else) = match body.as_slice() {
        [
            ..,
            Stmt {
                kind: StmtKind::If(_, _, Some(otherwise)),
                ..
            },
        ] if matches!(otherwise.as_slice(), [e] if exits(e)) => (body.len() - 1, true),
        [
            ..,
            Stmt {
                kind: StmtKind::If(_, _, None),
                ..
            },
            last,
        ] if is_return(last) => (body.len() - 2, false),
        _ => return None,
    };
    let rest = &body[at..];
    if in_else && written_else(&rest[0], elses) {
        return None;
    }
    let StmtKind::If(_, then, otherwise) = &rest[0].kind else {
        unreachable!("matched")
    };
    let exit = match otherwise {
        Some(otherwise) => &otherwise[0],
        None => &rest[1],
    };
    let mut inside = false;
    if let StmtKind::Return(Some(value)) = &exit.kind {
        value.visit_vars(&mut |v| inside |= declared(body).any(|d| d == v));
    }
    let goes_on = then.last().is_some_and(continues);
    // The `return` after the loop: of no other way out of it, so nothing
    // after it ran, nor of what it declares.
    let after = goes_on && !in_else;
    if after && (breaks(body, label.as_deref()) || inside) {
        return None;
    }
    if !in_else && !goes_on && !then.last().is_some_and(ends) {
        return None;
    }
    // `if (c) { return 1; } return 2;`, an exit each way: an early return
    // already, which turned would be `if (!c) { return 2; } return 1;`, and
    // turned again this, without end (a generated program's, seed 40074).
    if !in_else && matches!(then.as_slice(), [e] if exits(e)) {
        return None;
    }
    let mut tail = body.split_off(at).into_iter();
    let Some(Stmt {
        kind: StmtKind::If(test, mut then, otherwise),
        span,
    }) = tail.next()
    else {
        unreachable!("matched")
    };
    let exit = match otherwise {
        Some(mut otherwise) => otherwise.pop(),
        None => tail.next(),
    }
    .expect("matched");
    if goes_on {
        then.pop();
    }
    if after {
        body.push(StmtKind::If(test.negated_test(), vec![StmtKind::Break(None).at(span)], None).at(span));
        body.extend(then);
        return Some(Some(exit));
    }
    body.push(StmtKind::If(test.negated_test(), vec![exit], None).at(span));
    body.extend(then);
    Some(None)
}

/// Where a function ends, a `return;` the program didn't write, `written`
/// where those it did are: nothing, as it falls off the end there; and
/// `if (c) { s; return; } rest`, `if (c) { s } else { rest }`, as the
/// program wrote it, `if (!c) { rest }` of no `s`. So too in the branches
/// of an `if` that ends the function, and of one whose `then` ends so each
/// way: a function's last
/// `match` or `if` of arms that give `()`, its arms as written, with no
/// `return;` MIR made (ADR 0879, amended, the user's decision,
/// 2026-10-27).
pub(super) fn tail_returns(stmts: &mut Vec<Stmt>, returns: &[js::Span]) {
    ending_returns(stmts, returns, false);
}

/// `tail_returns` of `stmts`, a branch of the `if` that ends the function
/// where `branch`: there a `return undefined;` MIR made of a `None` or a
/// `()` is nothing too, as falling off the end gives `undefined` (ADR 0237,
/// amended); not a body's own, a constant's value.
fn ending_returns(stmts: &mut Vec<Stmt>, returns: &[js::Span], branch: bool) {
    let unwritten = |s: &Stmt| match &s.kind {
        StmtKind::Return(None) => !written(returns, s.span),
        StmtKind::Return(Some(v)) => branch && matches!(v.kind, ExprKind::Undefined) && !written(returns, s.span),
        _ => false,
    };
    // A copy MIR made of a `return;` the program wrote before it in the
    // list, a `let … else`'s, its span that `return`'s, is one too: the
    // program wrote it once, there (rd73).
    let copied = |stmts: &[Stmt]| match stmts.split_last() {
        Some((
            last @ Stmt {
                kind: StmtKind::Return(None),
                ..
            },
            before,
        )) => !last.span.is_none() && returns_at(before, last.span),
        _ => false,
    };
    while stmts.last().is_some_and(unwritten) || copied(stmts) {
        stmts.pop();
    }
    if let Some(StmtKind::If(_, then, otherwise)) = stmts.last_mut().map(|last| &mut last.kind) {
        ending_returns(then, returns, true);
        if let Some(otherwise) = otherwise {
            ending_returns(otherwise, returns, true);
        }
    }
    if let Some(at) = (stmts.iter())
        .position(|s| matches!(&s.kind, StmtKind::If(_, then, None) if then.last().is_some_and(unwritten)))
    {
        let rest = stmts.split_off(at + 1);
        let Some(Stmt {
            kind: StmtKind::If(test, mut then, None),
            span,
        }) = stmts.pop()
        else {
            unreachable!("found")
        };
        then.pop();
        ending_returns(&mut then, returns, branch);
        let mut rest = rest;
        ending_returns(&mut rest, returns, branch);
        match (then.is_empty(), rest.is_empty()) {
            (true, true) => {}
            (true, false) => stmts.push(StmtKind::If(test.negated_test(), rest, None).at(span)),
            (false, true) => stmts.push(StmtKind::If(test, then, None).at(span)),
            (false, false) => stmts.push(StmtKind::If(test, then, Some(rest)).at(span)),
        }
    }
    // `if (c) { s } else { return; }`, a `let … else`'s `return` the program
    // wrote: `if (!c) { return; } s`, its binding first, `const b = x; if (b
    // == null) { return; }`.
    if let Some(Stmt {
        kind: StmtKind::If(_, then, Some(otherwise)),
        ..
    }) = stmts.last()
        && matches!(otherwise.as_slice(), [r] if matches!(r.kind, StmtKind::Return(None)) && written(returns, r.span))
        && !then.last().is_some_and(ends)
    {
        let Some(Stmt {
            kind: StmtKind::If(test, then, Some(otherwise)),
            span,
        }) = stmts.pop()
        else {
            unreachable!("matched")
        };
        let mut then = then.into_iter().peekable();
        // The binding of what it tests is set, first, tested itself.
        let bound = match (&test.kind, then.peek().map(|s| &s.kind)) {
            (ExprKind::Binary(Op::LooseNe, x, null), Some(StmtKind::Const(b, value)))
                if matches!(null.kind, ExprKind::Null | ExprKind::Undefined) && js::same_path(x, value) =>
            {
                Some(b.clone())
            }
            _ => None,
        };
        let test = match &bound {
            Some(b) => {
                stmts.push(then.next().expect("the binding"));
                Expr::bin(Op::LooseEq, Expr::var(b), Expr::null())
            }
            None => test.negated_test(),
        };
        stmts.push(StmtKind::If(test, otherwise, None).at(span));
        stmts.extend(then);
    }
    match stmts.last_mut().map(|s| &mut s.kind) {
        // Each branch's, and a branch that leaves nothing, as
        // `emptied_branch` writes it: `if (!c) {} else { s }`, a macro's
        // function's `if let` (MIR's `return` there is where the macro is
        // called), is `if (c) { s }`.
        Some(StmtKind::If(_, then, otherwise)) => {
            ending_returns(then, returns, true);
            if let Some(otherwise) = otherwise {
                ending_returns(otherwise, returns, true);
            }
            let last = stmts.pop().expect("matched");
            if let Some(kind) = emptied_branch(last.kind) {
                stmts.push(kind.at(last.span));
            }
        }
        Some(StmtKind::Labeled(_, body)) => ending_returns(body, returns, branch),
        _ => {}
    }
}

/// Whether `stmts` hold a `return;` at `span`, however deep.
fn returns_at(stmts: &[Stmt], span: js::Span) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Return(None) => stmt.span == span,
        StmtKind::If(_, a, b) => returns_at(a, span) || b.as_deref().is_some_and(|b| returns_at(b, span)),
        StmtKind::Labeled(_, body)
        | StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. } => returns_at(body, span),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => returns_at(a, span) || returns_at(b, span),
        StmtKind::Switch { cases, default, .. } => js::case_lists(cases, default).any(|body| returns_at(body, span)),
        _ => false,
    })
}

/// Whether `stmts` hold a `return` the program wrote, of `returns`.
pub(super) fn writes_return(stmts: &[Stmt], returns: &[js::Span]) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Return(_) => written(returns, stmt.span),
        StmtKind::If(_, a, b) => writes_return(a, returns) || b.as_deref().is_some_and(|b| writes_return(b, returns)),
        _ => false,
    })
}

/// Whether a statement at `span` is a `return` the program wrote, of
/// `returns`, where those are.
pub(super) fn written(returns: &[js::Span], span: js::Span) -> bool {
    !span.is_none() && returns.iter().any(|w| w.lo <= span.lo && span.hi <= w.hi)
}

/// `l: { if (a) { break l; } if (b) { break l; } rest } t`, of `let … else`
/// and checks that go on to one end, `t`, a `throw` or a `return`, and
/// `rest` never going on past its own: `if (a || b) { t } rest`, as a
/// person writes a guard (`break_test`).
pub(super) fn guarded(stmts: &mut Vec<Stmt>) {
    for i in 0..stmts.len() {
        let StmtKind::Labeled(label, body) = &stmts[i].kind else {
            continue;
        };
        let tail = &stmts[i + 1..];
        let guards = (body.iter()).take_while(|s| break_test(s, label).is_some()).count();
        let rest = &body[guards..];
        if !tail.last().is_some_and(ends) {
            continue;
        }
        if guards == 0 || js::names_jump(rest, label) || !rest.last().is_some_and(ends) {
            continue;
        }
        // What `rest` declares, now beside what's before it.
        if declared(rest).any(|name| declared(&stmts[..i]).any(|n| n == name)) {
            continue;
        }
        let test = (body[..guards].iter())
            .map(|s| break_test(s, label).expect("a guard"))
            .reduce(|a, b| Expr::bin(Op::Or, a, b))
            .expect("one");
        // Where the block is, or of one MIR made, where its first guard is.
        let span = made_where(stmts[i].span, body);
        let rest = rest.to_vec();
        let tail = stmts.split_off(i + 1);
        stmts.pop();
        stmts.push(StmtKind::If(test, tail, None).at(span));
        stmts.extend(rest);
        return;
    }
}

/// When `stmt` breaks out to `label`, `if (c) { break l; }`: its test,
/// `c`. One that tests on, `if (k) { if (m) { break l; } }`, or breaks in
/// its `else`, is `skipped_tests`' (ADR 0946), whose one test is the same.
pub(super) fn break_test(stmt: &Stmt, label: &str) -> Option<Expr> {
    match &stmt.kind {
        StmtKind::If(test, then, None) if matches!(then.as_slice(), [Stmt { kind: StmtKind::Break(Some(l)), .. }] if l == label) => {
            Some(test.clone())
        }
        _ => None,
    }
}
