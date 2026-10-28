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
    // A block whose `break`s are `return`s now: its statements in its place.
    js::statement_lists(body, &mut unlabeled);
}

/// A `break label` after a block: its statements in its place, its own
/// `break`s that one; and where `branches`, after an `if` of two branches,
/// the end of each that goes on. A `while (true)`'s own `break`s, after
/// it, are `made_at_ends`', which returns where each makes the result.
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
                _ => continue,
            }
            if ends(&list[i]) {
                list.remove(i + 1);
            }
        }
    });
}

/// `l: { ..; while (..) { ..; break l; } ..; } return;`, a block MIR made
/// of the returns it joined, the program's early `return`s, which a loop
/// inside it leaves by: each such `break l` the `return;` it goes to, as
/// written, `while (..) { if (node.return === null) { return; } .. }`,
/// which no `break` of no label can say, its other `break`s too, and the
/// block, which nothing names then, its statements in its place, where they
/// fit (ADR 0742, amended). Only a loop with a `return` the program wrote
/// in it (`returns`), not a loop's own end; not a block the program labeled
/// (`written`), whose `break`s it wrote; nor one no such loop leaves, whose
/// `break`s other passes make an `if`'s branch, `&&`, `||` or a guard of.
/// The same jump either way: the block's end goes on to that `return;`
/// alone.
pub(in crate::lower::mir::tidy) fn returns_past_blocks(
    stmts: &mut Vec<Stmt>,
    written: &[LabeledBlock],
    returns: &[js::Span],
) {
    // Whether a `break label` is in a loop of `stmts` a written `return` is
    // in.
    fn left_by_a_return(stmts: &[Stmt], label: &str, returns: &[js::Span], in_loop: bool) -> bool {
        stmts.iter().any(|stmt| match &stmt.kind {
            StmtKind::Break(Some(l)) => in_loop && l == label,
            StmtKind::If(_, a, b) => {
                left_by_a_return(a, label, returns, in_loop)
                    || b.as_deref()
                        .is_some_and(|b| left_by_a_return(b, label, returns, in_loop))
            }
            StmtKind::Labeled(_, body) => left_by_a_return(body, label, returns, in_loop),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                left_by_a_return(a, label, returns, in_loop) || left_by_a_return(b, label, returns, in_loop)
            }
            StmtKind::Switch { cases, default, .. } => {
                js::case_lists(cases, default).any(|body| left_by_a_return(body, label, returns, in_loop))
            }
            StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
                let returning =
                    !stmt.span.is_none() && (returns.iter()).any(|r| stmt.span.lo <= r.lo && r.hi <= stmt.span.hi);
                left_by_a_return(body, label, returns, in_loop || returning)
            }
            _ => false,
        })
    }
    let mut i = 0;
    while i + 1 < stmts.len() {
        let returned = stmts[i + 1].kind.clone();
        let StmtKind::Labeled(label, body) = &mut stmts[i].kind else {
            i += 1;
            continue;
        };
        if !matches!(returned, StmtKind::Return(None))
            || written.iter().any(|block| block.label == *label)
            || !left_by_a_return(body, label, returns, false)
        {
            i += 1;
            continue;
        }
        let label = label.clone();
        js::statement_lists(body, &mut |list| {
            for stmt in list.iter_mut() {
                if matches!(&stmt.kind, StmtKind::Break(Some(l)) if *l == label) {
                    stmt.kind = returned.clone();
                }
            }
        });
        let fits = matches!(&stmts[i].kind, StmtKind::Labeled(_, body) if fits_in_place(stmts, i, body));
        if fits {
            let StmtKind::Labeled(_, body) = stmts.remove(i).kind else {
                unreachable!("matched")
            };
            stmts.splice(i..i, body);
            continue;
        }
        i += 1;
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
        // Or a `while (true)` left only once the result is made, where it
        // goes on to what they end at, or never left.
        Some(StmtKind::While { cond, body, label }) => {
            matches!(cond.kind, ExprKind::Bool(true)) && broken_made(body, true, label.as_deref(), result)
        }
        Some(StmtKind::If(_, then, Some(otherwise))) => made_at_ends(then, result) && made_at_ends(otherwise, result),
        // Or a block each of whose `break`s follows what makes it, as its
        // end does, which goes on to what they end at: its statements in
        // its place, where they fit, once those are `return`s.
        Some(StmtKind::Labeled(label, body)) => {
            made_at_ends(body, result)
                && broken_made(body, false, Some(label.as_str()), result)
                && fits_in_place(stmts, stmts.len() - 1, body)
        }
        _ => false,
    }
}

/// Whether each `break` in `stmts` that leaves loop `looped` follows what
/// makes `result`, as `made_at_ends` says: one of no label where `own`,
/// not in a loop or a `switch` of their own, and one of its label.
fn broken_made(stmts: &[Stmt], own: bool, looped: Option<&str>, result: &str) -> bool {
    stmts.iter().enumerate().all(|(i, stmt)| match &stmt.kind {
        StmtKind::Break(l) if (own && l.is_none()) || (l.is_some() && l.as_deref() == looped) => {
            made_at_ends(&stmts[..i], result)
        }
        StmtKind::If(_, a, b) => {
            broken_made(a, own, looped, result) && b.as_deref().is_none_or(|b| broken_made(b, own, looped, result))
        }
        StmtKind::Labeled(_, body) => broken_made(body, own, looped, result),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
            broken_made(a, own, looped, result) && broken_made(b, own, looped, result)
        }
        StmtKind::Switch { cases, default, .. } => {
            js::case_lists(cases, default).all(|body| broken_made(body, false, looped, result))
        }
        StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
            broken_made(body, false, looped, result)
        }
        _ => true,
    })
}

/// `stmts`, of `made_at_ends`, returning what they assign the result; of
/// a `while (true)`'s, where each of its `break`s is.
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
        StmtKind::While { body, label, .. } => returned_breaks(body, true, label.as_deref()),
        StmtKind::Labeled(label, body) => {
            returned_breaks(body, false, Some(label.as_str()));
            returned_at_ends(body);
        }
        _ => {}
    }
}

/// Each `break` of `broken_made`'s in `stmts`, the `return` of what makes
/// the result before it.
fn returned_breaks(stmts: &mut Vec<Stmt>, own: bool, looped: Option<&str>) {
    let mut i = 0;
    while i < stmts.len() {
        match &mut stmts[i].kind {
            StmtKind::Break(l) if (own && l.is_none()) || (l.is_some() && l.as_deref() == looped) => {
                returned_at_ends(&mut stmts[..i]);
                stmts.remove(i);
                continue;
            }
            StmtKind::If(_, a, b) => {
                returned_breaks(a, own, looped);
                if let Some(b) = b {
                    returned_breaks(b, own, looped);
                }
            }
            StmtKind::Labeled(_, body) => returned_breaks(body, own, looped),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                returned_breaks(a, own, looped);
                returned_breaks(b, own, looped);
            }
            StmtKind::Switch { cases, default, .. } => {
                js::case_lists_mut(cases, default).for_each(|body| returned_breaks(body, false, looped));
            }
            StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
                returned_breaks(body, false, looped)
            }
            _ => {}
        }
        i += 1;
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
/// } t`, an or-pattern's, `if (!a && !b) { s } else { t }`, which the
/// passes after turn as the program wrote it.
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
    // Nor one before a loop that never goes on, as a `return` doesn't,
    // `for (const k in props) { .. } return;` as MIR makes it (ADR 0805).
    let exits = stmts.last().is_some_and(|last| match &last.kind {
        StmtKind::Return(_) | StmtKind::Throw(_) => true,
        StmtKind::While { .. } => ends(last),
        _ => false,
    });
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
    let test = (std::iter::once(test).chain(tests))
        .rev()
        .reduce(|b, a| Expr::bin(Op::And, a, b))
        .expect("one");
    stmts.push(StmtKind::If(test, then, (!rest.is_empty()).then_some(rest)).at(span));
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
    // A function's call of such, `made(kind, mode)`, reads as written
    // wherever it's put too, React's getTag's `return
    // createFiberFromScope(type, pendingProps, mode, lanes, key);` (ADR
    // 0955).
    let call_of_values = |e: &Expr| {
        matches!(&e.kind, ExprKind::Call(callee, args)
            if matches!(callee.kind, ExprKind::Var(_)) && args.iter().all(one_value))
    };
    matches!(&stmt.kind, StmtKind::Return(value) if value.as_ref().is_none_or(|e| one_value(e) || call_of_values(e)))
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
        // A branch with nothing left, as `emptied_branch` writes it: `if
        // (!on) {} else { s }` of a guard's arm is `if (on) { s }`.
        let last = stmts.pop().expect("matched");
        if let Some(kind) = emptied_branch(last.kind) {
            stmts.push(kind.at(last.span));
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

/// Whether `stmt` is `if (t) { a } else { e }` of an `if` the program wrote
/// with an `else`, `a` its `then` and `e` none of it, its `else` or the jump
/// MIR made of it, `break` where the loop goes on to what's after it: the
/// `else` it wrote, which an early exit of MIR's, `if (!t) { e } a`, isn't,
/// nor a `while`'s test (ADR 0879).
pub(in crate::lower::mir::tidy) fn written_else(stmt: &Stmt, elses: &[(js::Span, js::Span, js::Span)]) -> bool {
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
pub(in crate::lower::mir::tidy) fn written_values(
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
                std::mem::replace(&mut stmts[at].kind, StmtKind::Expr(Expr::undefined()))
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
                // `while`'s, whose `else` is the whole loop.
                (false, false)
                    if otherwise.lo >= then.hi
                        && matches!(&exit[..], [e] if jumps(e) && (e.span.is_none() || within(cond, e.span))) =>
                {
                    (branch(then, Some(cond)).map(|k| (true, k)))
                        .or_else(|| branch(otherwise, None).map(|k| (false, k)))
                }
                _ => None,
            };
            if let Some((turned, k)) = turned {
                let after = stmts.split_off(at + 1 + k);
                let mut rest = stmts.split_off(at + 1);
                stmts.extend(after);
                let StmtKind::If(test, mut exit, None) =
                    std::mem::replace(&mut stmts[at].kind, StmtKind::Expr(Expr::undefined()))
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
                let written = |t: Expr| match linking {
                    true => t,
                    false => Expr {
                        facts: t.facts.with(js::Known::Else),
                        ..t
                    },
                };
                stmts[at].kind = match turned {
                    false => StmtKind::If(written(test), exit, Some(rest)),
                    true => StmtKind::If(written(test.negated_test()), rest, Some(exit)),
                };
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
            && let StmtKind::If(test, then, Some(otherwise)) = &mut stmts[at].kind
            && !plain(then, otherwise, linked)
            && !binds_link(otherwise, other)
        {
            test.facts = test.facts.with(js::Known::Else);
            let end = other.hi.max(branch.hi);
            let k = otherwise
                .iter()
                .rposition(|s| within(other, place(s)))
                .map_or(0, |k| k + 1);
            if k > 0
                && then.last().is_some_and(ends)
                && otherwise[k..].iter().any(|s| !place(s).is_none())
                && otherwise[k..].iter().all(|s| place(s).is_none() || place(s).lo >= end)
            {
                let after = otherwise.split_off(k);
                stmts.splice(at + 1..at + 1, after);
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
    let Some(at) = stmts.len().checked_sub(2) else {
        return;
    };
    let (if_stmt, last) = (&stmts[at], &stmts[at + 1..]);
    // A branch's value, where its statement was, of a `return` the program
    // didn't write in it: none, or the one it wrote of the `if`, `return if
    // c { a } else { b };`, whose value the `if` is.
    let of_if = |span: js::Span| returns.iter().any(|r| within(r, span) && within(r, if_stmt.span));
    let returned = |stmts: &[Stmt]| match stmts {
        [
            Stmt {
                kind: StmtKind::Return(Some(value)),
                span,
            },
        ] if small(value) && (!written(returns, *span) || of_if(*span)) => Some(spanned(value, *span)),
        _ => None,
    };
    let StmtKind::If(test, then, None) = &if_stmt.kind else {
        return;
    };
    if let (Some(yes), Some(no)) = (returned(then), returned(last))
        && folds_into_return(&yes, &no)
        && elses.iter().any(|(cond, ..)| within(cond, if_stmt.span))
    {
        // Its branches as written: the test negated where MIR turned them,
        // `if a == 0 || a > b { a } else { b }`'s `a !== 0 && !(a > b)` of
        // `return b` (ADR 0879, amended).
        let folded = match written_order(if_stmt.span, then, last, elses) {
            Some(true) => conditional(test.clone().negated_test(), no, yes, booleans, true),
            order => conditional(test.clone(), yes, no, booleans, order.is_some()),
        };
        let span = if_stmt.span;
        stmts.truncate(at);
        stmts.push(StmtKind::Return(Some(folded)).at(span));
    }
}

/// `if (!t) { exit } rest`, of an `if t { rest }` the program wrote without
/// an `else` (`Written::thens`), `rest` written in its `then`, never going
/// on, as `exit` doesn't: `if (t) { rest } exit`, as written, which runs
/// the same either way round. MIR's structure
/// makes a guard of it where `exit` is short, `if __DEV__ { .. } return
/// None;`'s `if (!__DEV__) { return null; } ..` (ADR 0879, amended).
pub(in crate::lower::mir::tidy) fn written_thens(stmts: &mut Vec<Stmt>, thens: &[(js::Span, js::Span)]) {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    for at in 0..stmts.len() {
        let (if_stmt, rest) = (&stmts[at], &stmts[at + 1..]);
        let StmtKind::If(_, exit, None) = &if_stmt.kind else {
            continue;
        };
        let Some((_, then)) = thens.iter().find(|(cond, _)| within(cond, if_stmt.span)) else {
            continue;
        };
        let written = |s: &Stmt| within(then, s.span);
        if !exit.last().is_some_and(ends)
            || !rest.iter().any(written)
            || !rest.iter().all(|s| s.span.is_none() || written(s))
            || !rest.last().is_some_and(ends)
        {
            continue;
        }
        let rest = stmts.split_off(at + 1);
        let StmtKind::If(test, exit, None) = std::mem::replace(&mut stmts[at].kind, StmtKind::Break(None)) else {
            unreachable!("matched")
        };
        stmts[at].kind = StmtKind::If(test.negated_test(), rest, None);
        stmts.extend(exit);
        break;
    }
}

/// `if (t) { rest } return;`, a unit function's end, of an `if !t { return;
/// }` the program wrote (`Written::thens`, its `return` written there),
/// `rest` written after it: `if (!t) { return; } rest`, as written, which
/// runs the same either way round, and so each such `if` the rest ends with
/// in turn. React's recordLegacyContextWarning's `if (didWarn.has(..)) {
/// return; }`, which MIR joins to the function's end (ADR 0879, amended).
pub(in crate::lower::mir::tidy) fn written_exits(
    stmts: &mut Vec<Stmt>,
    thens: &[(js::Span, js::Span)],
    returns: &[js::Span],
) {
    let within = |c: &js::Span, s: js::Span| !s.is_none() && c.lo <= s.lo && s.hi <= c.hi;
    loop {
        let [.., if_stmt, exit] = stmts.as_slice() else {
            return;
        };
        let (StmtKind::If(_, rest, None), StmtKind::Return(None)) = (&if_stmt.kind, &exit.kind) else {
            return;
        };
        let Some((_, then)) = thens.iter().find(|(cond, _)| within(cond, if_stmt.span)) else {
            return;
        };
        if !returns.iter().any(|r| within(then, *r)) || !first_written(rest).is_some_and(|at| at >= then.hi) {
            return;
        }
        let exit = stmts.pop().expect("matched");
        let if_stmt = stmts.pop().expect("matched");
        let StmtKind::If(test, rest, None) = if_stmt.kind else {
            unreachable!("matched")
        };
        let goes_on = !rest.last().is_some_and(ends);
        stmts.push(StmtKind::If(test.negated_test(), vec![exit.clone()], None).at(if_stmt.span));
        stmts.extend(rest);
        if !goes_on {
            return;
        }
        stmts.push(exit);
    }
}

/// `if (a) { x } else if (b) { y } else { z; continue; } break;`, a chain of
/// `if`s the program wrote without an `else` (`Written::thens`) whose jumps
/// MIR joined after it, the last `else` the rest, which never goes on: each
/// `if` ends with the jump it wrote, `if (a) { x; break; } if (b) { y;
/// break; } z; continue;`, as React's findCurrentFiberUsingSlowPath writes
/// it (ADR 0879, amended). The same statements run on each path: the jump
/// after the chain was reached only from its `then`s.
pub(in crate::lower::mir::tidy) fn written_jumps_of_ifs(stmts: &mut Vec<Stmt>, thens: &[(js::Span, js::Span)]) {
    let without_else = |s: &Stmt| {
        thens
            .iter()
            .any(|(cond, _)| !s.span.is_none() && cond.lo <= s.span.lo && s.span.hi <= cond.hi)
    };
    // Of `chain`, an `if` followed by `jump`: each `then` ending with it,
    // and the last `else`, which never goes on, after them.
    fn unchained(chain: &Stmt, jump: &Stmt, without_else: &dyn Fn(&Stmt) -> bool) -> Option<Vec<Stmt>> {
        let StmtKind::If(test, then, Some(otherwise)) = &chain.kind else {
            return None;
        };
        if !without_else(chain) {
            return None;
        }
        let rest = match otherwise.as_slice() {
            [.., last] if ends(last) => otherwise.clone(),
            [inner] => unchained(inner, jump, without_else)?,
            _ => return None,
        };
        let mut then = then.clone();
        if !then.last().is_some_and(ends) {
            then.push(jump.clone());
        }
        Some(
            std::iter::once(StmtKind::If(test.clone(), then, None).at(chain.span))
                .chain(rest)
                .collect(),
        )
    }
    if let [.., chain, jump] = stmts.as_slice()
        && matches!(
            jump.kind,
            StmtKind::Break(_) | StmtKind::Continue(_) | StmtKind::Return(_)
        )
        && let Some(unchained) = unchained(chain, jump, &without_else)
    {
        stmts.truncate(stmts.len() - 2);
        stmts.extend(unchained);
    }
}

/// What a run of statements a function ends with returns, as a person
/// writes it: `if (a) { if (b) { return t; } } return f;` is `return a &&
/// b ? t : f;` of a `bool`, and a `let … else`'s guard first, `if (!(a &&
/// b)) { return f; } t` (`branch_values`'s last).
pub(in crate::lower::mir::tidy) fn returned_branches(
    stmts: &mut Vec<Stmt>,
    returns: &[js::Span],
    booleans: &HashSet<String>,
    program: &crate::lower::mir::hints::Written,
) {
    let (continues, elses) = (&program.continues, &program.elses);
    // `if (a) { if (b) { return t; } } return f;`, last: `return a && b ? t : f;`.
    if let [.., last_if, last] = stmts.as_slice()
        && let StmtKind::Return(Some(no)) = &last.kind
        && let StmtKind::If(_, then, _) = &last_if.kind
        && let Some((tests, yes)) = returned_chain(std::slice::from_ref(last_if))
        && small(&yes)
        && small(no)
        && boolean_branch(&yes, no)
        // Not where the program wrote `return`, an early one.
        && !written(returns, last.span)
        && !writes_return(std::slice::from_ref(last_if), returns)
    {
        let test = (tests.into_iter())
            .rev()
            .reduce(|b, a| Expr::bin(Op::And, a, b))
            .expect("one");
        // Of an `if` the program wrote with an `else`, as it wrote it.
        let written_if = written_branches(last_if.span, then, std::slice::from_ref(last), elses)
            || written_values(last_if.span, (&yes, no), elses);
        let value = conditional(test, yes, no.clone(), booleans, written_if);
        let span = last_if.span;
        stmts.truncate(stmts.len() - 2);
        stmts.push(StmtKind::Return(Some(value)).at(span));
    } else if let [before @ .., last_if, last] = stmts.as_slice()
        && let StmtKind::Return(Some(_)) = &last.kind
        && written(returns, last.span)
        && let Some((tests, body)) = returning_chain(std::slice::from_ref(last_if))
        // Not `if c { return a; } else { return b; }`'s: a `let … else`'s
        // rest ends where it goes on, not at a `return` the program wrote;
        // nor `if c { continue; } return b;`'s, an `if` the program wrote
        // of a `continue` (ADR 0684).
        && !body.last().is_some_and(|s| matches!(s.kind, StmtKind::Return(_)) && written(returns, s.span))
        && !tests_continue(continues, last_if.span)
        && !declared(&body).any(|name| declared(before).any(|n| n == name))
    {
        // `f` the program wrote, a `let … else`'s else: `if (!(a && b)) {
        // return f; } t`.
        let test = (tests.into_iter())
            .rev()
            .reduce(|b, a| Expr::bin(Op::And, a, b))
            .expect("one");
        let (span, guard) = (last_if.span, stmts.pop().expect("the return"));
        stmts.pop();
        stmts.push(StmtKind::If(test.negated_test(), vec![guard], None).at(span));
        stmts.extend(body);
    }
}

/// Of `if (a) { const x = e; if (b) { s } }`, `s` ending where it leaves:
/// `[a, b]` and `const x = e; s`, what a level binds before its test, a
/// read that does nothing, written into later tests, as `broken_chain`'s:
/// each kept, so not a property, which a test would read again
/// (`bound_into`).
pub(in crate::lower::mir::tidy) fn returning_chain(stmts: &[Stmt]) -> Option<(Vec<Expr>, Vec<Stmt>)> {
    let [
        bound @ ..,
        Stmt {
            kind: StmtKind::If(test, inner, None),
            ..
        },
    ] = stmts
    else {
        return None;
    };
    let (mut tests, body) = match returning_chain(inner) {
        Some(chain) => chain,
        None if inner.last().is_some_and(ends) => (Vec::new(), inner.clone()),
        None => return None,
    };
    tests.insert(0, test.clone());
    for test in &mut tests {
        *test = bound_into(test, bound, &|_, _| 1)?;
    }
    let mut all = bound.to_vec();
    all.extend(body);
    unread_bound(&mut all, bound.len());
    Some((tests, all))
}

/// Whether of `c ? yes : no` one is `true` or `false`: `c || no` or `c &&
/// yes`, not `?:`.
pub(in crate::lower::mir::tidy) fn boolean_branch(yes: &Expr, no: &Expr) -> bool {
    matches!(yes.kind, ExprKind::Bool(_)) || matches!(no.kind, ExprKind::Bool(_))
}

/// Of `if (a) { if (b) { return t; } }`: its tests, `[a, b]`, and `t`.
pub(in crate::lower::mir::tidy) fn returned_chain(stmts: &[Stmt]) -> Option<(Vec<Expr>, Expr)> {
    match stmts {
        [
            Stmt {
                kind: StmtKind::Return(Some(yes)),
                ..
            },
        ] => Some((Vec::new(), yes.clone())),
        [
            Stmt {
                kind: StmtKind::If(test, inner, None),
                ..
            },
        ] => {
            let (mut tests, yes) = returned_chain(inner)?;
            tests.insert(0, test.clone());
            Some((tests, yes))
        }
        _ => None,
    }
}

/// `if (a) { if (b) { s } else { j } } else { j }`, `j` one jump, MIR's
/// `a && b` whose failing tests each jump the same way, `lanes !== 1 &&
/// lanes !== 2`'s past a guarded arm: `if (a && b) { s } else { j }`, as an
/// `&&` of calls reaches tidy (lb2).
pub(in crate::lower::mir::tidy) fn joined_ands(stmts: &mut [Stmt]) {
    let jump = |s: &[Stmt]| matches!(s, [j] if matches!(j.kind, StmtKind::Break(_) | StmtKind::Continue(_)));
    // A comparison of what's matched with a constant, `lanes !== 1`, as
    // MIR's `switchInt` makes the parts of one such guard: not a presence
    // test, `x != null`, nor parts of two subjects, which other passes
    // read as they are.
    fn compared(e: &Expr) -> Option<&Expr> {
        match &e.kind {
            ExprKind::Binary(Op::Eq | Op::Ne, x, c) => {
                matches!(c.kind, ExprKind::Num(..) | ExprKind::Str(..) | ExprKind::Var(..)).then_some(&**x)
            }
            _ => None,
        }
    }
    for stmt in stmts.iter_mut() {
        let StmtKind::If(test, then, Some(otherwise)) = &mut stmt.kind else {
            continue;
        };
        if let [
            Stmt {
                kind: StmtKind::If(b, inside, Some(inner)),
                ..
            },
        ] = &then[..]
            && jump(otherwise)
            && jump(inner)
            && crate::lower::mir::tidy::decisions::same_stmt(&inner[0], &otherwise[0])
            && let (Some(x), Some(y)) = (compared(test), compared(b))
            && js::same_path(x, y)
        {
            let (b, inside) = (b.clone(), inside.clone());
            *test = Expr::bin(Op::And, test.clone(), b);
            *then = inside;
        }
    }
}
