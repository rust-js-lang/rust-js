//! A block's tail: its own jumps, its `else`s, what it returns, and the
//! jumps threaded through it to where they end.

use super::*;

/// Whether `stmts` jump for loop `label`: an unnamed `break` or `continue`
/// of its own, or one that names it, however deep.
pub(in crate::lower::mir::tidy) fn own_jumps(stmts: &[Stmt], label: Option<&str>) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(None) | StmtKind::Continue(None) => true,
        StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) => Some(l.as_str()) == label,
        StmtKind::If(_, a, b) => own_jumps(a, label) || b.as_deref().is_some_and(|b| own_jumps(b, label)),
        StmtKind::Labeled(_, body) => own_jumps(body, label),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => own_jumps(a, label) || own_jumps(b, label),
        // In a loop of its own, one that names it.
        StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
            label.is_some_and(|l| names_jump(body, l))
        }
        _ => false,
    })
}

/// Whether a `break` or a `continue` in `stmts`, however deep, names `label`.
pub(in crate::lower::mir::tidy) fn names_jump(stmts: &[Stmt], label: &str) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) => l == label,
        StmtKind::If(_, a, b) => names_jump(a, label) || b.as_deref().is_some_and(|b| names_jump(b, label)),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => names_jump(body, label),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => names_jump(a, label) || names_jump(b, label),
        _ => false,
    })
}

/// Of each block `l`, the statements it ends after, `tail_else`'s.
pub(in crate::lower::mir::tidy) fn tail_elses(stmts: &mut [Stmt]) {
    fn tails(list: &mut Vec<Stmt>, label: &str) {
        *list = tail_else(std::mem::take(list), label);
        if let Some(Stmt {
            kind: StmtKind::If(_, then, otherwise),
            ..
        }) = list.last_mut()
        {
            tails(then, label);
            if let Some(otherwise) = otherwise {
                tails(otherwise, label);
            }
        }
    }
    for stmt in stmts {
        if let StmtKind::Labeled(label, body) = &mut stmt.kind {
            let label = label.clone();
            tails(body, &label);
        }
    }
}

/// `if (c) { s; break l; } t`, statements `l` ends after: `if (c) { s }
/// else { t }`, an `else if` chain's, `t` so too.
pub(in crate::lower::mir::tidy) fn tail_else(mut stmts: Vec<Stmt>, label: &str) -> Vec<Stmt> {
    let breaks = |list: &[Stmt]| matches!(list.last(), Some(Stmt { kind: StmtKind::Break(Some(l)), .. }) if l == label);
    if stmts.len() < 2 || !matches!(&stmts[0].kind, StmtKind::If(_, then, None) if breaks(then)) {
        return stmts;
    }
    let rest = stmts.split_off(1);
    let first = stmts.remove(0);
    let StmtKind::If(test, mut then, None) = first.kind else {
        unreachable!("matched")
    };
    then.pop();
    vec![StmtKind::If(test, then, Some(tail_else(rest, label))).at(first.span)]
}

/// `result = e; return result;` is `return e;`: nothing reads the return
/// place once it's returned, nor could a closure.
pub(in crate::lower::mir::tidy) fn returned(stmts: &mut Vec<Stmt>, result: &str) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let assigns = matches!(&stmts[i].kind, StmtKind::Assign(target, _)
            if matches!(&target.kind, ExprKind::Var(name) if name == result));
        let returns = matches!((&stmts[i].kind, &stmts[i + 1].kind), (StmtKind::Assign(target, _), StmtKind::Return(Some(value)))
            if matches!((&target.kind, &value.kind), (ExprKind::Var(a), ExprKind::Var(b)) if a == b));
        if assigns && returns {
            let StmtKind::Assign(_, value) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            stmts[i].kind = StmtKind::Return(Some(value));
        }
        i += 1;
    }
}

/// `l: { .. result = a; break l; .. result = b; } return result;`, a
/// function's returns MIR jumps out of a block for, a loop's too, in any
/// list of statements: each a `return` where it's made, `return a;`, and
/// the block's statements where nothing jumps to its end. Each is the
/// `return` it jumps to, whatever else reads `result`.
pub(in crate::lower::mir::tidy) fn returned_from_block(stmts: &mut Vec<Stmt>, result: &str) {
    let [.., block, last] = stmts.as_mut_slice() else {
        return;
    };
    if !matches!(&last.kind, StmtKind::Return(Some(v)) if matches!(&v.kind, ExprKind::Var(r) if r == result)) {
        return;
    }
    let StmtKind::Labeled(label, body) = &mut block.kind else {
        return;
    };
    let label = label.clone();
    // Its jumps threaded to where it ends, where that leaves none there.
    let mut threaded = body.clone();
    threaded_jumps(&mut threaded, &label, true);
    returned_where_made(&mut threaded, &label, result);
    if !targets(&threaded, &label) {
        *body = threaded;
    } else {
        threaded_jumps(body, &label, false);
        returned_where_made(body, &label, result);
    }
    let mut jumps = !body.last().is_some_and(ends);
    js::statement_lists(body, &mut |list| jumps |= targets(list, &label));
    // Where nothing goes on to its end, and its statements fit in its place.
    let at = stmts.len() - 2;
    if jumps || !matches!(&stmts[at].kind, StmtKind::Labeled(_, body) if fits_in_place(stmts, at, body)) {
        return;
    }
    stmts.pop();
    let Some(StmtKind::Labeled(_, body)) = stmts.pop().map(|s| s.kind) else {
        unreachable!("matched")
    };
    stmts.extend(body);
}

/// Of block `label`, followed by `return result`: each `break label` once
/// the result is made, a `return` where it's made.
fn returned_where_made(body: &mut Vec<Stmt>, label: &str, result: &str) {
    js::statement_lists(body, &mut |list| {
        let mut i = 1;
        while i < list.len() {
            if matches!(&list[i].kind, StmtKind::Break(Some(l)) if l == label) && made_at_ends(&list[..i], result) {
                returned_at_ends(&mut list[..i]);
                list.remove(i);
            } else {
                i += 1;
            }
        }
    });
    if made_at_ends(body, result) {
        returned_at_ends(body);
    }
}

/// A `break label` after a block: its statements in its place, its own
/// `break`s that one; and where `branches`, after a loop, each of the
/// loop's own `break`s that one, after an `if` of two branches, the end of
/// each that goes on.
fn threaded_jumps(body: &mut Vec<Stmt>, label: &str, branches: bool) {
    js::statement_lists(body, &mut |list| {
        for i in (0..list.len().saturating_sub(1)).rev() {
            if !matches!(&list[i + 1].kind, StmtKind::Break(Some(l)) if l == label) {
                continue;
            }
            if matches!(&list[i].kind, StmtKind::Labeled(_, block) if fits_in_place(list, i, block)) {
                let StmtKind::Labeled(inner, mut block) = list.remove(i).kind else {
                    unreachable!("matched")
                };
                relabeled(&mut block, &inner, label);
                list.splice(i..i, block);
                continue;
            }
            if !branches {
                continue;
            }
            let jump = list[i + 1].clone();
            match &mut list[i].kind {
                StmtKind::If(_, then, Some(otherwise)) => {
                    for branch in [then, otherwise] {
                        if !branch.last().is_some_and(ends) {
                            branch.push(jump.clone());
                        }
                    }
                }
                StmtKind::While {
                    label: looped, body, ..
                }
                | StmtKind::ForOf {
                    label: looped, body, ..
                }
                | StmtKind::For {
                    label: looped, body, ..
                } => {
                    broken_to(body, true, looped.as_deref(), label);
                }
                _ => continue,
            }
            if ends(&list[i]) {
                list.remove(i + 1);
            }
        }
    });
}

/// Each `break` in `stmts` that leaves loop `looped`, `break to`: one of no
/// label where `own`, not in a loop of their own, and one of its label.
fn broken_to(stmts: &mut [Stmt], own: bool, looped: Option<&str>, to: &str) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Break(l) if (own && l.is_none()) || (l.is_some() && l.as_deref() == looped) => {
                *l = Some(to.to_string());
            }
            StmtKind::If(_, a, b) => {
                broken_to(a, own, looped, to);
                if let Some(b) = b {
                    broken_to(b, own, looped, to);
                }
            }
            StmtKind::Labeled(_, body) => broken_to(body, own, looped, to),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                broken_to(a, own, looped, to);
                broken_to(b, own, looped, to);
            }
            StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
                broken_to(body, false, looped, to);
            }
            _ => {}
        }
    }
}

/// Where the first of `stmts` that says is written, inside a loop's or a
/// `try`'s too.
pub(in crate::lower::mir::tidy) fn first_written(stmts: &[Stmt]) -> Option<u32> {
    stmts.iter().find_map(|stmt| match &stmt.kind {
        // An `if`'s where its branch is, a `match`'s test written before
        // each arm; but not before its test, of a binding its pattern makes.
        StmtKind::If(_, then, _) => {
            let test = (!stmt.span.is_none()).then_some(stmt.span.lo);
            first_written(then).max(test)
        }
        _ if !stmt.span.is_none() => Some(stmt.span.lo),
        StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::While { body, .. }
        | StmtKind::Labeled(_, body)
        | StmtKind::Try(body, _)
        | StmtKind::TryCatch(body, ..) => first_written(body),
        _ => None,
    })
}

/// Whether `stmts` end assigning `result`, or leaving, each branch of an
/// `if` that ends them too: a `return`, a `throw`, or a jump, which, to a
/// block's end, stays, as the block does then.
pub(in crate::lower::mir::tidy) fn made_at_ends(stmts: &[Stmt], result: &str) -> bool {
    match stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Assign(target, _)) => matches!(&target.kind, ExprKind::Var(r) if r == result),
        Some(StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Continue(_) | StmtKind::Break(_)) => true,
        // Or that never goes on: a loop it never leaves.
        Some(StmtKind::While { cond, body, label }) => {
            matches!(cond.kind, ExprKind::Bool(true)) && !breaks(body, label.as_deref())
        }
        Some(StmtKind::If(_, then, Some(otherwise))) => made_at_ends(then, result) && made_at_ends(otherwise, result),
        _ => false,
    }
}

/// `stmts`, of `made_at_ends`, returning what they assign the result.
pub(in crate::lower::mir::tidy) fn returned_at_ends(stmts: &mut [Stmt]) {
    let Some(last) = stmts.last_mut() else { return };
    match &mut last.kind {
        StmtKind::Assign(_, value) => {
            let value = std::mem::replace(value, Expr::undefined());
            last.kind = StmtKind::Return(Some(value));
        }
        StmtKind::If(_, then, Some(otherwise)) => {
            returned_at_ends(then);
            returned_at_ends(otherwise);
        }
        _ => {}
    }
}

/// What follows a `return`, a `throw`, a `break` or a `continue` in the
/// same list never runs, nor what follows an `if` each of whose branches
/// ends so, nor a `while (true)` no `break` leaves.
pub(in crate::lower::mir::tidy) fn unreachable_after(stmts: &mut Vec<Stmt>) {
    if let Some(at) = stmts.iter().position(ends) {
        stmts.truncate(at + 1);
    }
}

/// A `break l` that ends labeled block `l`, or a `continue l` that ends
/// loop `l`'s body, where it would go on anyway: in the tails of the `if`s
/// that end them too.
pub(in crate::lower::mir::tidy) fn tail_jumps(stmts: &mut [Stmt]) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Labeled(label, body) => drop_tail(body, &StmtKind::Break(Some(label.clone()))),
            StmtKind::While {
                label: Some(label),
                body,
                ..
            } => drop_tail(body, &StmtKind::Continue(Some(label.clone()))),
            _ => {}
        }
    }
}

/// A labeled block's `if (c) { s; break l; } t`, `s` leaving for where
/// `t` goes on to, its end: `if (c) { s } else { t }`, a `match`'s arms
/// as a person writes them, the `break`s then ending the branches; and a
/// block that ends it, one with it. Of `if (!a) { if (!b) { s; break l; }
/// } t`, an or-pattern's, `if (a || b) { t } else { s }`.
pub(in crate::lower::mir::tidy) fn branch_ends(stmts: &mut [Stmt]) {
    for stmt in stmts {
        if let StmtKind::Labeled(label, body) = &mut stmt.kind {
            ending(body, label);
        }
    }
}

/// `stmts`, which end where block `label` does, as `branch_ends` has them.
pub(in crate::lower::mir::tidy) fn ending(stmts: &mut Vec<Stmt>, label: &str) {
    if let Some(Stmt {
        kind: StmtKind::Labeled(inner, body),
        ..
    }) = stmts.last_mut()
    {
        let (inner, mut body) = (inner.clone(), std::mem::take(body));
        js::statement_lists(&mut body, &mut |stmts| {
            for stmt in stmts {
                if matches!(&stmt.kind, StmtKind::Break(Some(l)) if *l == inner) {
                    stmt.kind = StmtKind::Break(Some(label.to_string()));
                }
            }
        });
        stmts.pop();
        stmts.extend(body);
    }
    // Not a `let … else`'s, `if (c) { s; break l; } throw e;`: a guard.
    let exits = matches!(
        stmts.last().map(|s| &s.kind),
        Some(StmtKind::Return(_) | StmtKind::Throw(_))
    );
    let Some(at) = (stmts.iter())
        .position(|s| matches!(&s.kind, StmtKind::If(_, then, None) if leaves(&ands(then).1, label)))
        .filter(|_| !exits)
    else {
        if let Some(StmtKind::If(_, then, otherwise)) = stmts.last_mut().map(|s| &mut s.kind) {
            ending(then, label);
            if let Some(otherwise) = otherwise {
                ending(otherwise, label);
            }
        }
        return;
    };
    let rest = stmts.split_off(at + 1);
    let Some(Stmt {
        kind: StmtKind::If(test, then, None),
        span,
    }) = stmts.pop()
    else {
        unreachable!("found")
    };
    let (tests, mut then) = ands(&then);
    let mut rest = rest;
    ending(&mut then, label);
    ending(&mut rest, label);
    let tests: Vec<Expr> = std::iter::once(test).chain(tests).collect();
    let negated = tests.len() > 1
        && tests
            .iter()
            .all(|t| matches!(t.kind, ExprKind::Unary(UnaryOp::Not, _) | ExprKind::Binary(Op::Ne, ..)));
    let test = (tests.into_iter())
        .rev()
        .reduce(|b, a| Expr::bin(Op::And, a, b))
        .expect("one");
    stmts.push(match (rest.is_empty(), negated) {
        (true, _) => StmtKind::If(test, then, None).at(span),
        (false, true) => StmtKind::If(test.negated_test(), rest, Some(then)).at(span),
        (false, false) => StmtKind::If(test, then, Some(rest)).at(span),
    });
}

/// Whether `stmts` always end with a `break label`.
pub(in crate::lower::mir::tidy) fn leaves(stmts: &[Stmt], label: &str) -> bool {
    match stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Break(Some(l))) => l == label,
        Some(StmtKind::If(_, then, Some(otherwise))) => leaves(then, label) && leaves(otherwise, label),
        _ => false,
    }
}

/// `jump` where it ends `stmts`, or a branch of the `if` that ends them.
pub(in crate::lower::mir::tidy) fn drop_tail(stmts: &mut Vec<Stmt>, jump: &StmtKind) {
    let same = |kind: &StmtKind| match (kind, jump) {
        (StmtKind::Break(Some(a)), StmtKind::Break(Some(b)))
        | (StmtKind::Continue(Some(a)), StmtKind::Continue(Some(b))) => a == b,
        _ => false,
    };
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(kind) if same(kind) => {
            stmts.pop();
        }
        Some(StmtKind::If(_, then, otherwise)) => {
            drop_tail(then, jump);
            if let Some(otherwise) = otherwise {
                drop_tail(otherwise, jump);
            }
        }
        _ => {}
    }
}

/// Each `break from` and `continue from` in `stmts`, however deep, `to`'s.
pub(in crate::lower::mir::tidy) fn relabeled(stmts: &mut Vec<Stmt>, from: &str, to: &str) {
    js::statement_lists(stmts, &mut |list| {
        for stmt in list.iter_mut() {
            if let StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) = &mut stmt.kind
                && l == from
            {
                *l = to.to_string();
            }
        }
    });
}

/// A `while (true)` nothing `break`s, what the program wrote after it MIR
/// moved into its one way out, `if (t) { e = f(e); continue outer; }`: that
/// way out a `break`, and what it does, after the loop, as written. Not of
/// what reads a name the loop declares; what it declares is its Rust
/// block's, named apart from what's beside the loop already.
pub(in crate::lower::mir::tidy) fn exits_after(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        if let Some(after) = exit_after(stmts, i) {
            stmts.splice(i + 1..i + 1, after);
        }
        i += 1;
    }
}

/// `if (t) { exit } rest`, of an `if` the program wrote with an `else`
/// (`Written::elses`), as it wrote it: `if (t) { exit } else { rest }` where
/// `exit` is its `then`, and `if (!t) { rest } else { exit }` where it's its
/// `else`. MIR's structure makes a guard of it where `exit` never goes on,
/// a `return`, a `continue` or a `break`; `rest`, the statements after it,
/// each of the `if`'s other branch, end the list, as the `if` did.
pub(in crate::lower::mir::tidy) fn written_elses(stmts: &mut Vec<Stmt>, elses: &[(js::Span, js::Span, js::Span)]) {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    // Each statement written in `branch`, one at least.
    let of = |branch: &js::Span, stmts: &[Stmt]| {
        stmts.iter().any(|s| !s.span.is_none()) && stmts.iter().all(|s| s.span.is_none() || within(branch, s.span))
    };
    let mut at = 0;
    while at < stmts.len() {
        if let StmtKind::If(_, exit, None) = &stmts[at].kind
            && exit.last().is_some_and(ends)
            && let Some((_, then, otherwise)) = elses.iter().find(|(cond, ..)| within(cond, stmts[at].span))
        {
            let rest = &stmts[at + 1..];
            // A jump alone each way, `if (t) { return a; } return;`, reads as
            // it is.
            let jump = |s: &Stmt| matches!(s.kind, StmtKind::Return(_) | StmtKind::Break(_) | StmtKind::Continue(_));
            let turned = match (of(then, exit), of(otherwise, exit)) {
                _ if matches!(rest, [s] if jump(s)) && matches!(exit.as_slice(), [s] if jump(s)) => None,
                (true, false) if of(otherwise, rest) => Some(false),
                (false, true) if of(then, rest) => Some(true),
                _ => None,
            };
            if let Some(turned) = turned {
                let rest = stmts.split_off(at + 1);
                let StmtKind::If(test, exit, None) =
                    std::mem::replace(&mut stmts[at].kind, StmtKind::Expr(Expr::undefined()))
                else {
                    unreachable!("matched")
                };
                stmts[at].kind = match turned {
                    false => StmtKind::If(test, exit, Some(rest)),
                    true => StmtKind::If(test.negated_test(), rest, Some(exit)),
                };
            }
        }
        at += 1;
    }
}

/// The value an `if` the program wrote with an `else` gives, returned:
/// `if (t) { return a; } return b;`, as each path returns, is `return t ? a
/// : b;`, as a person writes the function's value of two small ones, and as
/// `x = t ? a : b` is of a variable's (`branch_values`). Not of a guard the
/// program wrote, an `if` with no `else`, nor of a `return` it wrote in
/// each branch, a statement; nor of a branch with a comment, nor where
/// `folds_into_return` says it's a branch of its own.
pub(in crate::lower::mir::tidy) fn returned_values(
    stmts: &mut Vec<Stmt>,
    elses: &[(js::Span, js::Span, js::Span)],
    returns: &[js::Span],
    booleans: &HashSet<String>,
    comments: &[js::Span],
) {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    // A branch's value, where its statement was, of a `return` the program
    // didn't write.
    let returned = |stmts: &[Stmt]| match stmts {
        [
            Stmt {
                kind: StmtKind::Return(Some(value)),
                span,
            },
        ] if small(value) && !written(returns, *span) => Some(spanned(value, *span)),
        _ => None,
    };
    let Some(at) = stmts.len().checked_sub(2) else {
        return;
    };
    let (if_stmt, last) = (&stmts[at], &stmts[at + 1..]);
    let StmtKind::If(test, then, None) = &if_stmt.kind else {
        return;
    };
    if let (Some(yes), Some(no)) = (returned(then), returned(last))
        && folds_into_return(&yes, &no)
        && elses.iter().any(|(cond, ..)| within(cond, if_stmt.span))
        && !commented(if_stmt.span, &[then.as_slice(), last].concat(), comments)
    {
        let folded = conditional(test.clone(), yes, no, booleans);
        let span = if_stmt.span;
        stmts.truncate(at);
        stmts.push(StmtKind::Return(Some(folded)).at(span));
    }
}
