//! A labeled block's branches that end where it does: `if (c) { s; break
//! l; } t` as `if (c) { s } else { t }`, and a block MIR made that ends
//! it, one with it (`branch_ends`).

use super::*;

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
    // it's written, by the name it gave it (ADR 0742, 0745). Past the
    // comments after it, a last `_ => {}`'s, kept before the jump its
    // statements end with, where `default_marks` reads them (rd702); not
    // past comments written before what it runs, `if c { // why } else {
    // s }`'s, whose mark ends that `then` (rd714). And past this block's own
    // `break` after them, which goes on where it ends anyway, `bb1: { .. }
    // // Fallthrough break bb24;`, left where a `return` arm's `return
    // result;` after it was gone only then (rd702): after a block it merges
    // that never goes on, its `return`, not one whose `if` does, `if (c) {
    // s; break l; } else { .. }` then `// Neither .. break l;`, whose
    // `break` is the jump past its `else` of only comments (rd722).
    let commented = |s: &[Stmt]| {
        let code = s.iter().rposition(|s| !matches!(s.kind, StmtKind::Comments));
        code.is_some_and(|at| {
            at + 1 < s.len() && matches!(&s[at].kind, StmtKind::Labeled(_, body) if body.last().is_some_and(exits))
        })
    };
    if let [
        rest @ ..,
        Stmt {
            kind: StmtKind::Break(Some(l)),
            ..
        },
    ] = stmts.as_slice()
        && l == label
        && commented(rest)
    {
        stmts.pop();
    }
    let last = (stmts.iter()).rposition(|s| !matches!(s.kind, StmtKind::Comments));
    let marked = last.and_then(|at| stmts.get(at + 1)).map(|s| s.span.lo);
    if let Some(at) = last
        && let Stmt {
            kind: StmtKind::Labeled(inner, body),
            span,
        } = &mut stmts[at]
        && span.is_none()
        && marked.is_none_or(|marked| super::continues::latest(body) < marked)
    {
        let (inner, mut body) = (inner.clone(), std::mem::take(body));
        js::statement_lists(&mut body, &mut |stmts| {
            for stmt in stmts {
                if matches!(&stmt.kind, StmtKind::Break(Some(l)) if *l == inner) {
                    stmt.kind = StmtKind::Break(Some(label.to_string()));
                }
            }
        });
        let comments: Vec<Stmt> = stmts.drain(at + 1..).collect();
        let jump = (body.last().is_some_and(ends) && !comments.is_empty()).then(|| body.pop());
        stmts.splice(at..=at, body);
        stmts.extend(comments);
        stmts.extend(jump.flatten());
    }
    // Not a `let … else`'s, `if (c) { s; break l; } throw e;`: a guard.
    // Nor one before a loop that never goes on, as a `return` doesn't,
    // `for (const k in props) { .. } return;` as MIR makes it (ADR 0805);
    // nor before an `if` each of whose branches returns so, the returns
    // MIR joined past a block, each where it's written again (rd98).
    fn exits(last: &Stmt) -> bool {
        match &last.kind {
            StmtKind::Return(_) | StmtKind::Throw(_) => true,
            StmtKind::While { .. } => ends(last),
            StmtKind::If(_, then, Some(otherwise)) => {
                then.last().is_some_and(exits) && otherwise.last().is_some_and(exits)
            }
            _ => false,
        }
    }
    let exits = stmts.last().is_some_and(exits);
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

/// An `if` of no `else` whose `then` ends with the jump the statement after
/// it makes, MIR's, `if (x != null) { log.push(x); break; } break;` of a
/// guarded arm of only an `if let`: that `then` goes on to it, `if (x !=
/// null) { log.push(x); } break;`, as a person writes it. Not a jump the
/// program wrote, which is where it's written; nor a `then` of only the
/// jump, whose `if` would then do nothing.
pub(in crate::lower::mir::tidy) fn jumps_made_next(stmts: &mut [Stmt]) {
    for at in 1..stmts.len() {
        let (before, after) = stmts.split_at_mut(at);
        let next = &after[0];
        if !matches!(next.kind, StmtKind::Break(_) | StmtKind::Continue(_)) {
            continue;
        }
        if let StmtKind::If(_, then, None) = &mut before[at - 1].kind
            && then.len() > 1
            && then
                .last()
                .is_some_and(|last| last.kind == next.kind && last.span.is_none())
        {
            then.pop();
        }
    }
}
