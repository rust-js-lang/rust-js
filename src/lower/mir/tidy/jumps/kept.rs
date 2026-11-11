//! The jumps the program wrote, kept where it wrote them (ADR 0684,
//! amended): each `continue` and loop `break` it wrote last in a branch
//! of an `if`, its comment before it, and a loop's `continue` where its
//! body ends anyway, none, but one written before more, as written.

use super::*;

/// Each `continue` and loop `break` the program wrote last in a branch of
/// an `if`, `if c { ..; continue; } else { break; }`, kept where it wrote
/// it, as its `else`s and labels are (the user's decision, 2026-10-29; ADR
/// 0684, amended): where the loop's turn ends anyway after the branch, the
/// `continue` tidy took, given back; and in the loop a function ends with,
/// or a branch of an `if` it ends with does, the `return;` MIR made of a
/// `break`, `break;`, as React DOM's
/// replayUnblockedFormActions writes them; not where an `if` the program
/// wrote of a `return` ends so. Not in a loop or a `switch` inside the
/// loop, whose own a jump of no label would be. What MIR then put in the `else` of an `if`
/// the program wrote without one, which now never goes on, preparation's
/// `unelsed` puts after it again (ADR 0237).
pub(in crate::lower::mir::tidy) fn kept_jumps(
    body: &mut Vec<Stmt>,
    written: &crate::lower::mir::hints::Written,
    returns: &[js::Span],
) {
    // The loop the function ends with, or a branch of an `if` it ends with
    // does, React DOM's warnForBlockInsideInline's in its `if (__DEV__)`:
    // no `return;` of MIR's follows it here, as `tail returns` took it
    // (#410).
    fn ending_loops(stmts: &mut [Stmt], written: &crate::lower::mir::hints::Written, returns: &[js::Span]) {
        match stmts.last_mut().map(|s| &mut s.kind) {
            Some(StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. }) => {
                kept_in_turn(body, written, Kept::Breaks(returns, &written.arm_breaks), true);
            }
            Some(StmtKind::If(_, then, otherwise)) => {
                ending_loops(then, written, returns);
                if let Some(otherwise) = otherwise {
                    ending_loops(otherwise, written, returns);
                }
            }
            _ => {}
        }
    }
    ending_loops(body, written, returns);
    js::statement_lists(body, &mut |stmts| {
        for stmt in stmts.iter_mut() {
            if let StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::For { body, .. } = &mut stmt.kind
            {
                kept_in_turn(body, written, Kept::Continues, true);
            }
        }
    });
}

/// The `if`s the program wrote of a `continue` and of a loop's `break` a
/// branch of theirs ends with (`Written::continues` and `breaks`, or
/// `else_continues` and `else_breaks`): where each tests, and its jump.
pub(super) type BranchJumps<'w> = (&'w [(js::Span, js::Span)], &'w [(js::Span, js::Span)]);

/// Which of the jumps the program wrote `kept_in_turn` gives back.
#[derive(Clone, Copy, PartialEq)]
enum Kept<'r> {
    /// A `continue` where the turn ends anyway.
    Continues,
    /// A `break` of the loop a function ends with, MIR's `return;`, an
    /// `if`'s or a `match`'s arm's (`Written::arm_breaks`, the second);
    /// not a `return` the program wrote, of the first, which another arm
    /// of a `match` of a `break` may be (rd165b).
    Breaks(&'r [js::Span], &'r [(js::Span, js::Span)]),
}

/// Of `stmts`, a loop's body or what's in it but another loop's or a
/// `switch`'s, `at_end` where the turn ends after it: the jumps of `kept`
/// each `if`'s branches end with as written (`kept_jumps`).
fn kept_in_turn(stmts: &mut Vec<Stmt>, written: &crate::lower::mir::hints::Written, kept: Kept<'_>, at_end: bool) {
    if at_end && kept == Kept::Continues {
        turned_continue(stmts, &written.continues);
    }
    let last = stmts.len().saturating_sub(1);
    for (k, stmt) in stmts.iter_mut().enumerate() {
        let end = at_end && k == last;
        let span = stmt.span;
        if let StmtKind::If(_, then, otherwise) = &mut stmt.kind {
            kept_in_turn(then, written, kept, end);
            let (continues, breaks) = (written.continues.as_slice(), written.breaks.as_slice());
            kept_in_branch(then, span, (continues, breaks), kept, end);
            if let Some(otherwise) = otherwise {
                kept_in_turn(otherwise, written, kept, end);
                let elses = (written.else_continues.as_slice(), written.else_breaks.as_slice());
                kept_in_branch(otherwise, span, elses, kept, end);
            }
        }
    }
}

/// The last of `stmts`, where the turn ends after them, `if (!c) { rest }`
/// of an `if` the program wrote of a `continue`, `if c { continue; } rest`,
/// whose `then` is what it wrote after it, as MIR turns it: as written,
/// `if (c) { continue; } rest`, as React's findAllNodes skips a hidden
/// subtree; of a loop body's last too, first (`continued`). What the rest
/// declares is named apart from what's before it already, `doubled$1`
/// of a second `let doubled` (`written_jumps_kept.rs`'s `shadowed`).
pub(super) fn turned_continue(stmts: &mut Vec<Stmt>, continues: &[(js::Span, js::Span)]) -> bool {
    let Some(Stmt {
        kind: StmtKind::If(_, then, None),
        span,
    }) = stmts.last()
    else {
        return false;
    };
    let Some(at) = jump_tested(continues, *span).map(|k| continues[k].1) else {
        return false;
    };
    if !then.iter().all(|s| s.span.is_none() || s.span.hi > at.lo) {
        return false;
    }
    let Some(Stmt {
        kind: StmtKind::If(test, then, None),
        span,
    }) = stmts.pop()
    else {
        unreachable!("matched")
    };
    let jump = vec![StmtKind::Continue(None).at(at)];
    stmts.push(StmtKind::If(test.negated_test(), jump, None).at(span));
    stmts.extend(then);
    true
}

/// A branch of the `if` at `span`, given the jump the program
/// wrote it ends with, of `jumps`, its `continue`s' and its `break`s' (the
/// `then`'s or the `else`'s, `Written::continues` or `else_continues`),
/// where it's of `kept`; at that jump's place, which its comment is before.
/// Only of a branch of nothing written after the jump, so not the other of
/// an `if` tidy turned, `std_enumerate.rs`'s `return;`, React's
/// findAllNodes' `if (!isHiddenSubtree(node)) { .. }`.
fn kept_in_branch(branch: &mut Vec<Stmt>, span: js::Span, (continues, breaks): BranchJumps, kept: Kept<'_>, end: bool) {
    let tested = |written: &[(js::Span, js::Span)]| jump_tested(written, span).map(|k| written[k].1);
    let after = |at: js::Span| {
        let mut after = false;
        js::each_list(branch, &mut |list| {
            after |= list.iter().any(|s| !s.span.is_none() && s.span.lo >= at.hi);
        });
        after
    };
    match kept {
        Kept::Continues if end && !branch.last().is_some_and(ends) => {
            if let Some(at) = tested(continues)
                && !after(at)
            {
                branch.push(StmtKind::Continue(None).at(at));
            }
        }
        Kept::Breaks(returns, arms) => {
            if let Some(last) = branch.last()
                && matches!(last.kind, StmtKind::Return(None))
                && !super::written(returns, last.span)
                && let Some(at) = tested(breaks).or_else(|| tested(arms))
                && !after(at)
            {
                *branch.last_mut().expect("matched") = StmtKind::Break(None).at(at);
            }
        }
        Kept::Continues => {}
    }
}

pub(in crate::lower::mir::tidy) fn tests_continue(continues: &[(js::Span, js::Span)], span: js::Span) -> bool {
    continues
        .iter()
        .any(|(test, _)| written(std::slice::from_ref(test), span))
}

/// The `continue` an `if` of `continues` ends its `then` with, and the
/// `break` one of `breaks` does, where the program wrote it: its comment is
/// before it (ADR 0684, amended).
pub(in crate::lower::mir::tidy) fn written_jumps(
    stmts: &mut [Stmt],
    continues: &[(js::Span, js::Span)],
    breaks: &[(js::Span, js::Span)],
) {
    for stmt in stmts.iter_mut() {
        let span = stmt.span;
        let StmtKind::If(_, then, _) = &mut stmt.kind else {
            continue;
        };
        let written = match then.last().map(|last| &last.kind) {
            Some(StmtKind::Continue(_)) if then.last().is_some_and(|last| last.span.is_none()) => continues,
            Some(StmtKind::Break(None)) => breaks,
            _ => continue,
        };
        if let Some((_, at)) = jump_tested(written, span).map(|k| &written[k])
            && let Some(last) = then.last_mut()
        {
            last.span = *at;
        }
    }
}

/// Which of `written`, `if`s the program wrote of a jump (`Written::continues`,
/// `Written::breaks`), a test at `at` is of: at its test, or at the `if`
/// it's written in, which ends where its test does.
pub(in crate::lower::mir::tidy) fn jump_tested(written: &[(js::Span, js::Span)], at: js::Span) -> Option<usize> {
    (written.iter()).position(|(test, _)| {
        super::written(std::slice::from_ref(test), at) || (!at.is_none() && at.lo <= test.lo && at.hi == test.hi)
    })
}

/// Where `test` is written: of one of no place, a test `&&` or `||` joins
/// where the program didn't, a `!` or a read tidy made of a method's call,
/// `!words[i]` of `!words[i].is_empty()`, the first place of what it's
/// made of.
pub(in crate::lower::mir::tidy) fn written_at(test: &Expr) -> js::Span {
    let mut at = js::Span::NONE;
    let mut stmts = vec![StmtKind::Expr(test.clone()).at(js::Span::NONE)];
    js::each_expr_mut(&mut stmts, &mut |e| {
        if at.is_none() {
            at = e.span;
        }
    });
    at
}

/// Each loop's `continue` where its body ends anyway, in the branches that
/// end it: none, as a block returned in place leaves them (ADR 0432). One
/// the program wrote there, `if (c) { a; continue; }` last, does nothing
/// either; one it wrote before more, `if (c) { continue; } return b;`, is
/// kept, as written (ADR 0684): of `if (c) { a; continue; } else { b }`
/// last, where `jumps into branches` made the rest its `else`, `if (c) {
/// a; continue; } b` (ADR 0745). A loop's label those were the last jumps
/// to is taken off as any nothing names is: MIR's by tidy's `unlabeled`,
/// the program's by preparation (#799).
pub(in crate::lower::mir::tidy) fn tail_continues(
    stmts: &mut [Stmt],
    written: &crate::lower::mir::hints::Written,
    commented: &[js::Span],
) {
    for stmt in stmts.iter_mut() {
        if let StmtKind::While { label, body, .. }
        | StmtKind::ForOf { label, body, .. }
        | StmtKind::For { label, body, .. } = &mut stmt.kind
        {
            ends_going_on(body, label, written, commented);
        }
    }
}

/// A `continue` where the body of loop `looped` ends anyway, in the
/// branches that end it: none. Of an `else` that's only one, `if (c) {
/// ..; return; } else { continue; }`, no `else` (ADR 0684). But an
/// `else` that's the `continue` of an arm with comments above it,
/// `commented`'s, `_ => continue,`, keeps it, as written, the comments
/// before it (#610, the user's decision).
fn ends_going_on(
    stmts: &mut Vec<Stmt>,
    looped: &Option<String>,
    written: &crate::lower::mir::hints::Written,
    commented: &[js::Span],
) {
    let continues = &written.continues;
    // The `if` of a `continue` the program wrote, what goes on after it its
    // `else`: that after it, as written, its `continue` kept.
    let going_on = |l: &Option<String>| l.is_none() || l == looped;
    if let Some(Stmt {
        kind: StmtKind::If(test, then, otherwise @ Some(_)),
        span,
    }) = stmts.last_mut()
        && let Some(Stmt {
            kind: StmtKind::Continue(l),
            span: at,
        }) = then.last()
        && (tests_continue(continues, *span) || !at.is_none())
        && !test.facts.has(js::Known::Else)
        && going_on(l)
    {
        let rest = otherwise.take().expect("matched");
        stmts.extend(rest);
        return ends_going_on(stmts, looped, written, commented);
    }
    let more = stmts.len() > 1;
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(StmtKind::Continue(l)) if (l.is_none() || l == looped) && more => {
            stmts.pop();
        }
        Some(StmtKind::If(_, then, otherwise)) => {
            ends_going_on(then, looped, written, commented);
            if let Some(branch) = otherwise {
                if matches!(branch.as_slice(), [Stmt { kind: StmtKind::Continue(l), span }] if going_on(l) && !commented.contains(span))
                {
                    *otherwise = None;
                } else {
                    ends_going_on(branch, looped, written, commented);
                }
            }
        }
        _ => {}
    }
}
