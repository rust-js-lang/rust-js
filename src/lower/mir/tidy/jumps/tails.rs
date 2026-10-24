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
        // Whose `break` of a loop names it (ADR 0629), its `continue` the loop's.
        StmtKind::Switch { cases, default, .. } => js::case_lists(cases, default).any(|body| own_jumps(body, label)),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => own_jumps(a, label) || own_jumps(b, label),
        // In a loop of its own, one that names it.
        StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
            label.is_some_and(|l| js::names_jump(body, l))
        }
        _ => false,
    })
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
    if !js::names_jump(&threaded, &label) {
        *body = threaded;
    } else {
        threaded_jumps(body, &label, false);
        returned_where_made(body, &label, result);
    }
    let mut jumps = !body.last().is_some_and(ends);
    js::statement_lists(body, &mut |list| jumps |= js::names_jump(list, &label));
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
            // Whose `break` of no label is its own (ADR 0629).
            StmtKind::Switch { cases, default, .. } => {
                js::case_lists_mut(cases, default).for_each(|body| broken_to(body, false, looped, to));
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
        StmtKind::Switch { cases, default, .. } => js::case_lists(cases, default).find_map(|body| first_written(body)),
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
    // Not a block the program labeled, `'desc: { .. }`, which is where
    // it's written, by the name it gave it (ADR 0742, 0745).
    if let Some(Stmt {
        kind: StmtKind::Labeled(inner, body),
        span,
    }) = stmts.last_mut()
        && span.is_none()
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
    // And `if (c) { .. if (d) { s; break l; } ..; continue; } t`, a `then`
    // that never goes on but has a way out to the block's end inside: `if
    // (c) { .. } else { t }`, whose `then` then ends where the block does, to
    // be had so too (ADR 0745). Unless no `else`'s needed, as `unelsed`
    // writes it where its `then` still never goes on.
    let left = |then: &Vec<Stmt>| then.last().is_some_and(ends) && js::names_jump(then, label);
    let Some(at) = (stmts.iter())
        .position(|s| matches!(&s.kind, StmtKind::If(_, then, None) if leaves(&ands(then).1, label) || left(then)))
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
    if !leaves(&ands(&then).1, label) {
        let (mut then, mut rest) = (then, rest);
        ending(&mut then, label);
        ending(&mut rest, label);
        stmts.push(StmtKind::If(test, then, (!rest.is_empty()).then_some(rest)).at(span));
        return;
    }
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

/// A block MIR made that ends `if (c) { a } else { b } return v;`, a
/// match's arm of `return v` that some of the `if`'s paths go on to and the
/// rest leave past with a `break` of the block, `getArrayKind`'s: the
/// `return` at the end of each path that goes on to it, and each `break` of
/// the block a path ends with gone, now where the block ends, so nothing
/// names it (ADR 0754). So too where the `if` and its `return` end a branch
/// of the `if` that ends the block, which ends where the block does, `l: {
/// if (typeof s === "string") { if (s === "a") { .. } else { break l; }
/// return; } }` (ADR 0755). Only of a `return` `returns_one_value` takes;
/// and not where a `break` of the block would be left, deeper than a path's
/// end. The `if` has an `else`: a `let … else`'s guard, `if (c) { s; break
/// l; } return v;`, has none, and stays one.
pub(in crate::lower::mir::tidy) fn returns_into_branches(stmts: &mut [Stmt]) {
    // `stmts`, which end where block `label` does: theirs, or of the
    // branches of the `if` they end with.
    fn at_ends(stmts: &mut Vec<Stmt>, label: &str) {
        if returned_into_branches(stmts, label) {
            return;
        }
        if let Some(StmtKind::If(_, then, otherwise)) = stmts.last_mut().map(|s| &mut s.kind) {
            at_ends(then, label);
            if let Some(otherwise) = otherwise {
                at_ends(otherwise, label);
            }
        }
    }
    for stmt in stmts {
        if stmt.span.is_none()
            && let StmtKind::Labeled(label, body) = &mut stmt.kind
            && crate::lower::mir::is_block_label(label)
        {
            let label = label.clone();
            at_ends(body, &label);
        }
    }
}

/// Whether `stmt` is a `return` of one value, a constant or a name, or of
/// none: one that reads as it's written wherever it's put, in each branch
/// that goes on to it (ADR 0754), or for each `break` of a block it follows
/// (ADR 0755).
pub(in crate::lower::mir::tidy) fn returns_one_value(stmt: &Stmt) -> bool {
    let one_value =
        |e: &Expr| e.is_constant() || matches!(e.kind, ExprKind::Var(_)) || matches!(e.kind, ExprKind::Undefined);
    matches!(&stmt.kind, StmtKind::Return(value) if value.as_ref().is_none_or(one_value))
}

/// `stmts`, which end where block `label` does, as `returns_into_branches`
/// has them; whether they were.
fn returned_into_branches(stmts: &mut Vec<Stmt>, label: &str) -> bool {
    // `stmts`, `exit` at the end of each path of them that goes on, and the
    // `break label` each that leaves ends with gone.
    fn into(stmts: &mut Vec<Stmt>, exit: &Stmt, label: &str) {
        let leaves = |last: &Stmt| match &last.kind {
            StmtKind::Break(Some(l)) => l != label,
            StmtKind::If(_, _, Some(_)) => false,
            _ => ends(last),
        };
        if stmts.last().is_some_and(leaves) {
            return;
        }
        let Some(last) = stmts.last_mut() else {
            stmts.push(exit.clone());
            return;
        };
        match &mut last.kind {
            StmtKind::Break(Some(l)) if l == label => {
                stmts.pop();
            }
            StmtKind::If(test, then, otherwise @ Some(_)) => {
                into(then, exit, label);
                let rest = otherwise.as_mut().expect("matched");
                into(rest, exit, label);
                // A branch of nothing left, none: `if (c) { s }`, or `if (!c)
                // { s }` of an empty `then`.
                if rest.is_empty() {
                    *otherwise = None;
                } else if then.is_empty() {
                    *test = std::mem::replace(test, Expr::undefined()).negated_test();
                    *then = otherwise.take().expect("matched");
                }
            }
            StmtKind::If(test, then, otherwise @ None) if js::names_jump(then, label) => {
                into(then, exit, label);
                match then.is_empty() {
                    true => {
                        *test = std::mem::replace(test, Expr::undefined()).negated_test();
                        *then = vec![exit.clone()];
                    }
                    false => *otherwise = Some(vec![exit.clone()]),
                }
            }
            _ => stmts.push(exit.clone()),
        }
    }
    let [.., before, last] = stmts.as_slice() else {
        return false;
    };
    if !matches!(&before.kind, StmtKind::If(_, _, Some(_))) || !returns_one_value(last) {
        return false;
    }
    let mut threaded = stmts.clone();
    let exit = threaded.pop().expect("matched");
    into(&mut threaded, &exit, label);
    if js::names_jump(&threaded, label) {
        return false;
    }
    *stmts = threaded;
    true
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
/// Not a `break` the program wrote of a block it labeled, where it's
/// written, its span (ADR 0790).
pub(in crate::lower::mir::tidy) fn drop_tail(stmts: &mut Vec<Stmt>, jump: &StmtKind) {
    let same = |stmt: &Stmt| match (&stmt.kind, jump) {
        (StmtKind::Break(Some(a)), StmtKind::Break(Some(_)))
            if !crate::lower::mir::is_block_label(a) && !stmt.span.is_none() =>
        {
            false
        }
        (StmtKind::Break(Some(a)), StmtKind::Break(Some(b)))
        | (StmtKind::Continue(Some(a)), StmtKind::Continue(Some(b))) => a == b,
        _ => false,
    };
    if stmts.last().is_some_and(same) {
        stmts.pop();
    } else if let Some(StmtKind::If(_, then, otherwise)) = stmts.last_mut().map(|s| &mut s.kind) {
        drop_tail(then, jump);
        if let Some(otherwise) = otherwise {
            drop_tail(otherwise, jump);
        }
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
/// each of the `if`'s other branch, end the list, as the `if` did. A
/// binding its condition makes, a let chain's `let Some(x) = e`, is its
/// `then`'s, where it's in scope.
pub(in crate::lower::mir::tidy) fn written_elses(stmts: &mut Vec<Stmt>, elses: &[(js::Span, js::Span, js::Span)]) {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    // Each statement written in `branch`, one at least, or in `made`, the
    // condition of the branch it's a `then` of: a binding the condition makes.
    let of = |branch: &js::Span, made: Option<&js::Span>, stmts: &[Stmt]| {
        stmts.iter().any(|s| within(branch, s.span))
            && (stmts.iter())
                .all(|s| s.span.is_none() || within(branch, s.span) || made.is_some_and(|c| within(c, s.span)))
    };
    let mut at = 0;
    while at < stmts.len() {
        if let StmtKind::If(_, exit, None) = &stmts[at].kind
            && exit.last().is_some_and(ends)
            && let Some((cond, then, otherwise)) = elses.iter().find(|(cond, ..)| within(cond, stmts[at].span))
        {
            let rest = &stmts[at + 1..];
            // A jump alone each way, `if (t) { return a; } return;`, reads as
            // it is.
            let jump = |s: &Stmt| matches!(s.kind, StmtKind::Return(_) | StmtKind::Break(_) | StmtKind::Continue(_));
            let turned = match (of(then, Some(cond), exit), of(otherwise, None, exit)) {
                _ if matches!(rest, [s] if jump(s)) && matches!(exit.as_slice(), [s] if jump(s)) => None,
                (true, false) if of(otherwise, None, rest) => Some(false),
                (false, true) if of(then, Some(cond), rest) => Some(true),
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
/// each branch, a statement; nor where `folds_into_return` says it's a
/// branch of its own. A comment before a branch's value is before it in the
/// conditional (ADR 0658).
pub(in crate::lower::mir::tidy) fn returned_values(
    stmts: &mut Vec<Stmt>,
    elses: &[(js::Span, js::Span, js::Span)],
    returns: &[js::Span],
    booleans: &HashSet<String>,
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
    {
        let folded = conditional(test.clone(), yes, no, booleans);
        let span = if_stmt.span;
        stmts.truncate(at);
        stmts.push(StmtKind::Return(Some(folded)).at(span));
    }
}
