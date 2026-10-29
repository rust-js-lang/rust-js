//! `continue`s: a loop's step continued where it's written, and the jumps
//! past blocks to a loop's start.

use super::*;

/// A loop's body that ends in a block, `l: { … break l; … }`, its `break l`
/// the loop's `continue`: the block's statements in its place. And one that
/// ends `if (!c) { a; b }`, as MIR has a `continue` the program wrote,
/// `if (c) { continue; } a; b`.
pub(in crate::lower::mir::tidy) fn continued(stmts: &mut [Stmt], guards: &[(js::Span, js::Span)]) {
    for stmt in stmts.iter_mut() {
        let (looped, body) = match &mut stmt.kind {
            StmtKind::While { label, body, .. }
            | StmtKind::ForOf { label, body, .. }
            | StmtKind::For { label, body, .. } => (label.clone(), body),
            _ => continue,
        };
        // What can't follow a jump, before the body's end is read.
        unreachable_after(body);
        loop {
            match body.last().map(|s| &s.kind) {
                Some(StmtKind::Labeled(..)) => {
                    let Some(Stmt {
                        kind: StmtKind::Labeled(block, mut inner),
                        span,
                    }) = body.pop()
                    else {
                        unreachable!("matched")
                    };
                    let clashes = declared(&inner).any(|name| declared(body).any(|n| n == name));
                    if clashes || continues(&mut inner, &block, looped.as_deref(), false).is_none() {
                        body.push(StmtKind::Labeled(block, inner).at(span));
                        break;
                    }
                    continues(&mut inner, &block, looped.as_deref(), true);
                    body.extend(inner);
                }
                // One the program ended with a `continue`, `if (t) { a;
                // continue; }`, of what MIR has as its `else`.
                Some(StmtKind::If(_, _, Some(otherwise)))
                    if tests_continue(guards, body.last().expect("matched").span)
                        && !declared(otherwise).any(|name| declared(&body[..body.len() - 1]).any(|n| n == name)) =>
                {
                    let Some(Stmt {
                        kind: StmtKind::If(test, mut then, Some(otherwise)),
                        span,
                    }) = body.pop()
                    else {
                        unreachable!("matched")
                    };
                    if !then.last().is_some_and(ends) {
                        then.push(StmtKind::Continue(None).at(js::Span::NONE));
                    }
                    body.push(StmtKind::If(test, then, None).at(span));
                    body.extend(otherwise);
                }
                // Of one statement, where the program wrote the `continue`,
                // whatever its test, `i % 2 !== 0` of `i % 2 == 0`.
                Some(StmtKind::If(_, _, None)) => {
                    if !turned_continue(body, guards) {
                        break;
                    }
                }
                _ => break,
            }
        }
    }
}

/// Whether `span` is of the test of an `if` of a `continue` the program
/// wrote, of `continues` (`Written::continues`).
/// `l: { .. if (c) { break l; } .. } continue outer;`, a block MIR made of
/// what each `continue` the program wrote there goes on to: each `if` of
/// one (`Written::continues`) that leaves the block, the `continue` itself,
/// `if (c) { continue outer; }`, where it wrote it, which then needs no
/// block. React's getHostSibling's `if isScope(node) { continue 'siblings;
/// }` (ADR 0702, amended). The `break` goes to the `continue` and nowhere
/// else, so the two run the same; not into a loop inside the block, whose
/// own `continue` an unlabeled one would be.
pub(in crate::lower::mir::tidy) fn continues_written(stmts: &mut [Stmt], guards: &[(js::Span, js::Span)]) {
    fn replaced(stmts: &mut [Stmt], block: &str, jump: &Stmt, guards: &[(js::Span, js::Span)]) {
        for stmt in stmts.iter_mut() {
            let span = stmt.span;
            match &mut stmt.kind {
                StmtKind::If(_, then, otherwise)
                    if matches!(then.as_slice(), [Stmt { kind: StmtKind::Break(Some(l)), .. }] if l == block)
                        && tests_continue(guards, span) =>
                {
                    then[0] = Stmt {
                        kind: jump.kind.clone(),
                        span: then[0].span,
                    };
                    if let Some(otherwise) = otherwise {
                        replaced(otherwise, block, jump, guards);
                    }
                }
                StmtKind::If(_, then, otherwise) => {
                    replaced(then, block, jump, guards);
                    if let Some(otherwise) = otherwise {
                        replaced(otherwise, block, jump, guards);
                    }
                }
                StmtKind::Labeled(_, body) => replaced(body, block, jump, guards),
                _ => {}
            }
        }
    }
    for at in 1..stmts.len() {
        let (before, after) = stmts.split_at_mut(at);
        if let (StmtKind::Labeled(block, body), StmtKind::Continue(_)) = (&mut before[at - 1].kind, &after[0].kind) {
            replaced(body, block, &after[0], guards);
        }
    }
}

/// Each `continue` and loop `break` the program wrote last in a branch of
/// an `if`, `if c { ..; continue; } else { break; }`, kept where it wrote
/// it, as its `else`s and labels are (the user's decision, 2026-10-29; ADR
/// 0684, amended): where the loop's turn ends anyway after the branch, the
/// `continue` tidy took, given back; and in the loop a function ends with,
/// the `return;` MIR made of a `break`, `break;`, as React DOM's
/// replayUnblockedFormActions writes them; not where an `if` the program
/// wrote of a `return` ends so. Not in a loop or a `switch` inside the
/// loop, whose own a jump of no label would be. What MIR then put in the `else` of an `if`
/// the program wrote without one, which now never goes on, preparation's
/// `unelsed` puts after it again (ADR 0237).
pub(in crate::lower::mir::tidy) fn kept_jumps(body: &mut Vec<Stmt>, written: &crate::lower::mir::hints::Written) {
    // The loop before the `return;` the function ends with, MIR's, too (ag6).
    let end = body.len() - usize::from(body.last().is_some_and(|s| matches!(s.kind, StmtKind::Return(None))));
    if let Some(Stmt {
        kind: StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. },
        ..
    }) = body[..end].last_mut()
    {
        kept_in_turn(body, written, Kept::Breaks, true);
    }
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
type BranchJumps<'w> = (&'w [(js::Span, js::Span)], &'w [(js::Span, js::Span)]);

/// Which of the jumps the program wrote `kept_in_turn` gives back.
#[derive(Clone, Copy, PartialEq)]
enum Kept {
    /// A `continue` where the turn ends anyway.
    Continues,
    /// A `break` of the loop a function ends with, MIR's `return;`.
    Breaks,
}

/// Of `stmts`, a loop's body or what's in it but another loop's or a
/// `switch`'s, `at_end` where the turn ends after it: the jumps of `kept`
/// each `if`'s branches end with as written (`kept_jumps`).
fn kept_in_turn(stmts: &mut Vec<Stmt>, written: &crate::lower::mir::hints::Written, kept: Kept, at_end: bool) {
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
fn turned_continue(stmts: &mut Vec<Stmt>, continues: &[(js::Span, js::Span)]) -> bool {
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
fn kept_in_branch(branch: &mut Vec<Stmt>, span: js::Span, (continues, breaks): BranchJumps, kept: Kept, end: bool) {
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
        Kept::Breaks => {
            if let Some(last) = branch.last()
                && matches!(last.kind, StmtKind::Return(None))
                && let Some(at) = tested(breaks)
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
/// to goes with them.
pub(in crate::lower::mir::tidy) fn tail_continues(stmts: &mut [Stmt], written: &crate::lower::mir::hints::Written) {
    for stmt in stmts.iter_mut() {
        if let StmtKind::While { label, body, .. }
        | StmtKind::ForOf { label, body, .. }
        | StmtKind::For { label, body, .. } = &mut stmt.kind
        {
            let named = label.as_deref().is_some_and(|l| js::names_jump(body, l));
            ends_going_on(body, label, written);
            if named && label.as_deref().is_some_and(|l| !js::names_jump(body, l)) {
                *label = None;
            }
        }
    }
}

/// A `continue` where the body of loop `looped` ends anyway, in the
/// branches that end it: none. Of an `else` that's only one, `if (c) {
/// ..; return; } else { continue; }`, no `else` (ADR 0684).
fn ends_going_on(stmts: &mut Vec<Stmt>, looped: &Option<String>, written: &crate::lower::mir::hints::Written) {
    let continues = &written.continues;
    // The `if` of a `continue` the program wrote, what goes on after it its
    // `else`: that after it, as written, its `continue` kept.
    let going_on = |l: &Option<String>| l.is_none() || l == looped;
    if let Some(Stmt {
        kind: StmtKind::If(test, then, otherwise @ Some(_)),
        span,
    }) = stmts.last_mut()
        && tests_continue(continues, *span)
        && !test.facts.has(js::Known::Else)
        && matches!(then.last(), Some(Stmt { kind: StmtKind::Continue(l), .. }) if going_on(l))
    {
        let rest = otherwise.take().expect("matched");
        stmts.extend(rest);
        return ends_going_on(stmts, looped, written);
    }
    let more = stmts.len() > 1;
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(StmtKind::Continue(l)) if (l.is_none() || l == looped) && more => {
            stmts.pop();
        }
        Some(StmtKind::If(_, then, otherwise)) => {
            ends_going_on(then, looped, written);
            if let Some(branch) = otherwise {
                if matches!(branch.as_slice(), [Stmt { kind: StmtKind::Continue(l), .. }] if going_on(l)) {
                    *otherwise = None;
                } else {
                    ends_going_on(branch, looped, written);
                }
            }
        }
        _ => {}
    }
}

/// A labeled block MIR made followed by a jump, `l: { .. break l; .. }
/// continue;`: each `break l` that jump, which goes on from there anyway;
/// `unlabeled` then leaves the block, which nothing names. A jump of no
/// label, `continue;` once `innermost jumps` has it so, only where it goes
/// to the same loop: not in a loop inside the block, or a `switch` for a
/// `break` (ADR 0745). Or a `return` of one value, `l: { .. break l; .. }
/// return;`, the `return` past a drop MIR shares, as it's written (ADR
/// 0755).
pub(in crate::lower::mir::tidy) fn jumps_past_blocks(stmts: &mut [Stmt]) {
    fn rejump(stmts: &mut [Stmt], label: &str, jump: &StmtKind) {
        // Nor a `return` in a loop, whose `break` of the block the loop's
        // own way out may be (`blocks loops end`).
        let unnamed = matches!(
            jump,
            StmtKind::Continue(None) | StmtKind::Break(None) | StmtKind::Return(_)
        );
        for stmt in stmts {
            match &mut stmt.kind {
                StmtKind::Break(Some(l)) if l == label => stmt.kind = jump.clone(),
                StmtKind::If(_, a, b) => {
                    rejump(a, label, jump);
                    if let Some(b) = b {
                        rejump(b, label, jump);
                    }
                }
                StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. }
                    if !unnamed =>
                {
                    rejump(body, label, jump)
                }
                StmtKind::Labeled(_, body) => rejump(body, label, jump),
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    rejump(a, label, jump);
                    rejump(b, label, jump);
                }
                _ => {}
            }
        }
    }
    for i in 0..stmts.len().saturating_sub(1) {
        let jump = match &stmts[i + 1].kind {
            jump @ (StmtKind::Continue(_) | StmtKind::Break(_)) => jump.clone(),
            // A `return` past a drop MIR shares, the block's end then its
            // way out (ADR 0755): of one value, as each reads as written.
            jump @ StmtKind::Return(_) if returns_one_value(&stmts[i + 1]) => jump.clone(),
            _ => continue,
        };
        // Not one the program labeled, `'sum: { .. }`, written by its name
        // (ADR 0742): where it's written is its span.
        let written = !stmts[i].span.is_none();
        if let StmtKind::Labeled(label, body) = &mut stmts[i].kind
            && crate::lower::mir::is_block_label(label)
            && !written
        {
            let label = label.clone();
            // A `return` where the block ends anyway is written once, after
            // it: the `break`s that end it, gone first.
            if matches!(jump, StmtKind::Return(_)) {
                drop_tail(body, &StmtKind::Break(Some(label.clone())));
            }
            rejump(body, &label, &jump);
        }
    }
}

/// An `if` one branch of which never goes on, followed by a jump, `if (c)
/// { a } else { b; break l; } continue;`, as `jumps past blocks` leaves a
/// `continue` the program wrote in `a`: the jump where the other branch
/// ends, `if (c) { a; continue; } else { b; break l; }`, where it goes on
/// from (ADR 0745). Of an `else if` chain, each of its arms that goes on,
/// where one leaves: `if (a) { x } else if (b) { y } else { break l; }
/// continue;` is `if (a) { x; continue; } else if (b) { y; continue; } else
/// { break l; }`, as React's commitBeforeMutationEffects_begin writes each
/// arm's `continue` (ADR 0745, amended). Not a chain of a `match` whose
/// guarded arms fall to the next, `1 if on(n) => break, 1 | 2 => { .. }`,
/// whose arms `switches` reads by the blocks MIR leaves them by, then the
/// `if` chain they are (#389c).
pub(in crate::lower::mir::tidy) fn jumps_into_branches(stmts: &mut Vec<Stmt>, switches: &[super::Switch]) {
    // An `else` that is one `if` with an `else` of its own: the chain's
    // next link.
    fn linked(otherwise: &[Stmt]) -> bool {
        matches!(
            otherwise,
            [Stmt {
                kind: StmtKind::If(_, _, Some(_)),
                ..
            }]
        )
    }
    // Whether each arm of the chain `stmt`, an `if` with an `else`, begins
    // leaves, in order.
    fn arms(stmt: &Stmt, leaves: &mut Vec<bool>) {
        let StmtKind::If(_, then, Some(otherwise)) = &stmt.kind else {
            unreachable!("an `if` with an `else`")
        };
        leaves.push(then.last().is_some_and(ends));
        if linked(otherwise) {
            arms(&otherwise[0], leaves);
        } else {
            leaves.push(otherwise.last().is_some_and(ends));
        }
    }
    // An `if` one branch of which leaves, its `else` whole; or a chain
    // only whose last `else` does, what MIR makes of its fall-through: not
    // a chain of arms that leave each their own way, a `match`'s, which
    // `switches` reads as they are.
    let into_arms = |stmt: &Stmt| {
        let StmtKind::If(test, then, Some(otherwise)) = &stmt.kind else {
            return false;
        };
        let falling = switches.iter().filter(|s| s.arms.iter().any(|arm| arm.falls));
        if switched(test, stmt.span, falling) {
            return false;
        }
        if then.last().is_some_and(ends) != otherwise.last().is_some_and(ends) {
            return true;
        }
        let mut leaves = Vec::new();
        arms(stmt, &mut leaves);
        matches!(&leaves[..], [going_on @ .., true] if going_on.iter().all(|leaves| !leaves))
    };
    let mut i = 0;
    while i + 1 < stmts.len() {
        if matches!(stmts[i + 1].kind, StmtKind::Continue(_) | StmtKind::Break(_)) && into_arms(&stmts[i]) {
            let jump = stmts.remove(i + 1);
            let StmtKind::If(_, then, Some(otherwise)) = &mut stmts[i].kind else {
                unreachable!("matched")
            };
            // An `else if` given it has it in each of its own arms that goes
            // on as its list's turn comes, after this one's.
            for arm in [then, otherwise] {
                if !arm.last().is_some_and(ends) {
                    arm.push(jump.clone());
                }
            }
        }
        i += 1;
    }
}

/// A loop's body that ends with a block MIR made, then the loop's
/// `break;`, `l: { if (c === 97) { break l; } ..; continue; } break;`, a
/// `match` whose arm leaves the loop, `b'a' => break`, MIR's way out
/// through the block: each `break l` the loop's `break;`, which goes where
/// the block's end does, the block's statements in its place, so the arms
/// break the loop as written and `switches` reads them so (#389c). Not one
/// a `break l` of which is in a loop or a `switch` of its own, where a
/// `break;` would leave that, but of a loop the program labeled, whose
/// such `break` is of its label, `break 'rows` in an inner `while let`'s
/// `break rows;` (uz3); a `switch` it ends, `case_rests::loops_left_by_cases`
/// reads once `switches` has made it. A name
/// the block declares that the turn's before it does too is the names
/// stage's to tell apart, later.
pub(in crate::lower::mir::tidy) fn blocks_breaking_loops(
    stmts: &mut [Stmt],
    loops: &[(js::Span, bool, Option<String>)],
) {
    for stmt in stmts {
        let named = written_loop(loops, stmt.span).and_then(|(.., label)| label.clone());
        let (StmtKind::While {
            body, label: looped, ..
        }
        | StmtKind::ForOf {
            body, label: looped, ..
        }
        | StmtKind::For {
            body, label: looped, ..
        }) = &mut stmt.kind
        else {
            continue;
        };
        let [.., block, after] = body.as_slice() else {
            continue;
        };
        let (StmtKind::Labeled(label, inner), StmtKind::Break(None)) = (&block.kind, &after.kind) else {
            continue;
        };
        if !block.span.is_none() || !crate::lower::mir::is_block_label(label) {
            continue;
        }
        let (block, mut broken) = (label.clone(), inner.clone());
        loop_breaks(&mut broken, &block);
        let at = body.len() - 2;
        // One in a loop of its own, of a loop the program labeled, `break
        // 'rows` in an inner `while let`: of its label, as written (uz3).
        // Else a loop the program labeled is `loops_left_by_cases`', whose
        // cases it ends with leave by its label once `switches` has made
        // them.
        if !js::names_break(&broken, &block) && named.is_some() {
            continue;
        }
        if js::names_break(&broken, &block) {
            let Some(named) = named
                .as_ref()
                .filter(|named| looped.as_ref().is_none_or(|l| l == *named))
            else {
                continue;
            };
            super::tails::relabeled(&mut broken, &block, named);
            *looped = Some(named.clone());
        }
        body.splice(at..=at, broken);
        if body[..body.len() - 1].last().is_some_and(ends) {
            body.pop();
        }
    }
}

/// Each `break label` of `stmts`, not in a loop or a `switch` of their
/// own, a `break;` of the loop they're in.
fn loop_breaks(stmts: &mut [Stmt], label: &str) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Break(broken) if broken.as_deref() == Some(label) => *broken = None,
            StmtKind::If(_, a, b) => {
                loop_breaks(a, label);
                if let Some(b) = b {
                    loop_breaks(b, label);
                }
            }
            StmtKind::Labeled(_, body) => loop_breaks(body, label),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                loop_breaks(a, label);
                loop_breaks(b, label);
            }
            _ => {}
        }
    }
}

/// A labeled block MIR made that a loop ends, `l: { .. if (t) { .. while
/// (true) { ..; break l; } } }`, as the last of loops each in the one
/// before's way out is once `exits after` puts each after it: the `break
/// l` the loop's body ends with, the loop's `break`, which goes where the
/// block ends too, and which `while_tests` reads the loop's test by;
/// `unlabeled` then leaves the block, where nothing else names it.
///
/// With `every`, once what's after each loop is placed, each `break l` of
/// the loop's own wherever it is in the body, not a loop's or a `switch`'s
/// inside it, as a `loop` that ends an `if`'s branch, an `else` after it,
/// leaves: `l: { if (t) { while (true) { if (d) { break l; } .. } } else {
/// .. } }` is `if (t) { while (true) { if (d) { break; } .. } } else { ..
/// }`, as the program wrote it, with no label (ADR 0859). And one of a loop
/// or a `switch` inside it, `break 'rows` in a `for` of the `loop` it
/// names, the loop's, by its label, which `named labels` then names as the
/// program did.
pub(in crate::lower::mir::tidy) fn blocks_loops_end(stmts: &mut [Stmt], every: bool) {
    // Each `break label` of the loop whose body is `stmts`: where a `break`
    // of no label leaves it.
    fn own_breaks(stmts: &mut [Stmt], label: &str) {
        for stmt in stmts {
            match &mut stmt.kind {
                StmtKind::Break(broken) if broken.as_deref() == Some(label) => *broken = None,
                StmtKind::If(_, a, b) => {
                    own_breaks(a, label);
                    if let Some(b) = b {
                        own_breaks(b, label);
                    }
                }
                StmtKind::Labeled(_, body) => own_breaks(body, label),
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    own_breaks(a, label);
                    own_breaks(b, label);
                }
                _ => {}
            }
        }
    }
    // Each loop `stmts` end with, through the `if`s and blocks they end
    // with: where it ends, they do. One whose body still names `label`
    // once its own `break`s are, by a label of its own: that label.
    fn ending(stmts: &mut [Stmt], label: &str, every: bool) {
        let Some(last) = stmts.last_mut() else {
            return;
        };
        match &mut last.kind {
            StmtKind::While {
                label: looped, body, ..
            }
            | StmtKind::ForOf {
                label: looped, body, ..
            }
            | StmtKind::For {
                label: looped, body, ..
            } => {
                if !every {
                    if let Some(Stmt {
                        kind: StmtKind::Break(broken),
                        ..
                    }) = body.last_mut()
                        && broken.as_deref() == Some(label)
                    {
                        *broken = None;
                    }
                    return;
                }
                own_breaks(body, label);
                if let Some(looped) = looped
                    && js::names_break(body, label)
                {
                    super::tails::relabeled(body, label, looped);
                }
            }
            StmtKind::If(_, a, b) => {
                ending(a, label, every);
                if let Some(b) = b {
                    ending(b, label, every);
                }
            }
            StmtKind::Labeled(_, body) => ending(body, label, every),
            _ => {}
        }
    }
    for stmt in stmts.iter_mut() {
        if let StmtKind::Labeled(label, body) = &mut stmt.kind
            && crate::lower::mir::is_block_label(label)
        {
            ending(body, label, every);
        }
    }
}

/// A loop that ends the body of loop `l` around it, one the program names
/// no label, as `loops` say, not `'rows`, whose `continue 'rows` it wrote:
/// each `continue l` of its own, not a loop's inside it, its `break`, which
/// ends `l`'s turn as well, as Rust's `break` of a loop that ends
/// another's does. So does one that ends a branch of an `if` that ends it,
/// React's `if (..) { return true; } else { while (..) { .. } }` (ADR 0961).
pub(in crate::lower::mir::tidy) fn outer_continues(stmts: &mut [Stmt], loops: &[(js::Span, bool, Option<String>)]) {
    for stmt in stmts {
        let named = matches!(written_loop(loops, stmt.span), Some((_, _, Some(_))));
        if let StmtKind::While {
            label: Some(label),
            body,
            ..
        }
        | StmtKind::ForOf {
            label: Some(label),
            body,
            ..
        }
        | StmtKind::For {
            label: Some(label),
            body,
            ..
        } = &mut stmt.kind
            && !named
        {
            ending(body, label);
        }
    }
}

/// Each loop that ends `stmts`, or a branch of an `if` that ends them: its
/// own `continue looped`, its `break` (`outer_continues`).
fn ending(stmts: &mut [Stmt], looped: &str) {
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(
            StmtKind::While { body: inner, .. }
            | StmtKind::ForOf { body: inner, .. }
            | StmtKind::For { body: inner, .. },
        ) => outer_broken(inner, looped),
        Some(StmtKind::If(_, a, b)) => {
            ending(a, looped);
            if let Some(b) = b {
                ending(b, looped);
            }
        }
        _ => {}
    }
}

/// Each `continue looped` of `stmts`, not a loop's inside them, `break`.
fn outer_broken(stmts: &mut [Stmt], looped: &str) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Continue(Some(l)) if l == looped => stmt.kind = StmtKind::Break(None),
            StmtKind::If(_, a, b) => {
                outer_broken(a, looped);
                if let Some(b) = b {
                    outer_broken(b, looped);
                }
            }
            StmtKind::Labeled(_, body) => outer_broken(body, looped),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                outer_broken(a, looped);
                outer_broken(b, looped);
            }
            _ => {}
        }
    }
}

/// A `continue l` or a `break l` of loop `l` where it's the innermost
/// loop: `continue` or `break`, as JS's unlabeled ones go there.
pub(in crate::lower::mir::tidy) fn innermost_jumps(stmts: &mut [Stmt]) {
    fn unlabel(stmts: &mut [Stmt], looped: &str) {
        for stmt in stmts {
            match &mut stmt.kind {
                StmtKind::Continue(label) | StmtKind::Break(label) if label.as_deref() == Some(looped) => *label = None,
                StmtKind::If(_, a, b) => {
                    unlabel(a, looped);
                    if let Some(b) = b {
                        unlabel(b, looped);
                    }
                }
                StmtKind::Labeled(_, body) => unlabel(body, looped),
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    unlabel(a, looped);
                    unlabel(b, looped);
                }
                _ => {}
            }
        }
    }
    for stmt in stmts {
        if let StmtKind::While {
            label: Some(label),
            body,
            ..
        }
        | StmtKind::ForOf {
            label: Some(label),
            body,
            ..
        }
        | StmtKind::For {
            label: Some(label),
            body,
            ..
        } = &mut stmt.kind
        {
            unlabel(body, label);
        }
    }
}

/// Each `break block` in `stmts` a `continue` of the loop around, labeled
/// `looped`, written if `write`: `None` where one inside a loop of its own
/// would need the loop's label, and it has none.
pub(in crate::lower::mir::tidy) fn continues(
    stmts: &mut [Stmt],
    block: &str,
    looped: Option<&str>,
    write: bool,
) -> Option<()> {
    fn each(stmts: &mut [Stmt], block: &str, looped: Option<&str>, inner: bool, write: bool) -> Option<()> {
        for stmt in stmts {
            match &mut stmt.kind {
                StmtKind::Break(Some(l)) if l == block => {
                    let label = match inner {
                        true => Some(looped?.to_string()),
                        false => None,
                    };
                    if write {
                        stmt.kind = StmtKind::Continue(label);
                    }
                }
                StmtKind::If(_, a, b) => {
                    each(a, block, looped, inner, write)?;
                    if let Some(b) = b {
                        each(b, block, looped, inner, write)?;
                    }
                }
                StmtKind::Labeled(_, body) => each(body, block, looped, inner, write)?,
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    each(a, block, looped, inner, write)?;
                    each(b, block, looped, inner, write)?;
                }
                StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
                    each(body, block, looped, true, write)?
                }
                _ => {}
            }
        }
        Some(())
    }
    // Not `looped` itself, a label the block's statements can't name twice.
    if looped == Some(block) {
        return None;
    }
    each(stmts, block, looped, false, write)
}

/// Whether `test` says `item`, what `$next` gave, is none: `item == null`,
/// or `!item` of an array's.
pub(in crate::lower::mir::tidy) fn absent(test: &Expr, item: &str) -> bool {
    let is_item = |e: &Expr| matches!(&e.kind, ExprKind::Var(v) if v == item);
    match &test.kind {
        ExprKind::Unary(UnaryOp::Not, a) => is_item(a),
        _ => js::null_test(test, false).is_some_and(is_item),
    }
}

/// Whether `test` says `item` is there: `item != null`, or `item`.
pub(in crate::lower::mir::tidy) fn present(test: &Expr, item: &str) -> bool {
    match &test.kind {
        ExprKind::Var(v) => v == item,
        _ => js::null_test(test, true).is_some_and(|e| matches!(&e.kind, ExprKind::Var(v) if v == item)),
    }
}
