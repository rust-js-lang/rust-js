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
                        then.push(StmtKind::Continue(None).at(span));
                    }
                    body.push(StmtKind::If(test, then, None).at(span));
                    body.extend(otherwise);
                }
                // Of one statement, where the program wrote the `continue`,
                // whatever its test, `i % 2 !== 0` of `i % 2 == 0`.
                Some(StmtKind::If(_, _, None)) if tests_continue(guards, body.last().expect("matched").span) => {
                    let Some(Stmt {
                        kind: StmtKind::If(test, then, None),
                        span,
                    }) = body.pop()
                    else {
                        unreachable!("matched")
                    };
                    body.push(
                        StmtKind::If(test.negated_test(), vec![StmtKind::Continue(None).at(span)], None).at(span),
                    );
                    body.extend(then);
                }
                _ => break,
            }
        }
    }
}

/// Whether `span` is of the test of an `if` of a `continue` the program
/// wrote, of `continues` (`Written::continues`).
pub(in crate::lower::mir::tidy) fn tests_continue(continues: &[(js::Span, js::Span)], span: js::Span) -> bool {
    continues
        .iter()
        .any(|(test, _)| written(std::slice::from_ref(test), span))
}

/// The `continue` an `if` of `continues` ends its `then` with, where the
/// program wrote it: its comment is before it (ADR 0684).
pub(in crate::lower::mir::tidy) fn written_continues(stmts: &mut [Stmt], continues: &[(js::Span, js::Span)]) {
    for stmt in stmts.iter_mut() {
        let span = stmt.span;
        if let StmtKind::If(_, then, _) = &mut stmt.kind
            && let Some(last) = then.last_mut()
            && matches!(last.kind, StmtKind::Continue(_))
            && last.span.is_none()
            && let Some((_, at)) = continues
                .iter()
                .find(|(test, _)| written(std::slice::from_ref(test), span))
        {
            last.span = *at;
        }
    }
}

/// Each loop's `continue` where its body ends anyway, in the branches that
/// end it: none, as a block returned in place leaves them (ADR 0432). One
/// the program wrote there, `if (c) { a; continue; }` last, does nothing
/// either; one it wrote before more, `if (c) { continue; } return b;`, is
/// kept, as written (ADR 0684): of `if (c) { a; continue; } else { b }`
/// last, where `jumps into branches` made the rest its `else`, `if (c) {
/// a; continue; } b` (ADR 0745).
pub(in crate::lower::mir::tidy) fn tail_continues(stmts: &mut [Stmt], continues: &[(js::Span, js::Span)]) {
    for stmt in stmts.iter_mut() {
        if let StmtKind::While { label, body, .. }
        | StmtKind::ForOf { label, body, .. }
        | StmtKind::For { label, body, .. } = &mut stmt.kind
        {
            ends_going_on(body, label, continues);
        }
    }
}

/// A `continue` where the body of loop `looped` ends anyway, in the
/// branches that end it: none. Of an `else` that's only one, `if (c) {
/// ..; return; } else { continue; }`, no `else` (ADR 0684).
fn ends_going_on(stmts: &mut Vec<Stmt>, looped: &Option<String>, continues: &[(js::Span, js::Span)]) {
    // The `if` of a `continue` the program wrote, what goes on after it its
    // `else`: that after it, as written, its `continue` kept.
    let going_on = |l: &Option<String>| l.is_none() || l == looped;
    if let Some(Stmt {
        kind: StmtKind::If(_, then, otherwise @ Some(_)),
        span,
    }) = stmts.last_mut()
        && tests_continue(continues, *span)
        && matches!(then.last(), Some(Stmt { kind: StmtKind::Continue(l), .. }) if going_on(l))
    {
        let rest = otherwise.take().expect("matched");
        stmts.extend(rest);
        return ends_going_on(stmts, looped, continues);
    }
    let more = stmts.len() > 1;
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(StmtKind::Continue(l)) if (l.is_none() || l == looped) && more => {
            stmts.pop();
        }
        Some(StmtKind::If(_, then, otherwise)) => {
            ends_going_on(then, looped, continues);
            if let Some(branch) = otherwise {
                if matches!(branch.as_slice(), [Stmt { kind: StmtKind::Continue(l), .. }] if going_on(l)) {
                    *otherwise = None;
                } else {
                    ends_going_on(branch, looped, continues);
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
/// from (ADR 0745).
pub(in crate::lower::mir::tidy) fn jumps_into_branches(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 1 < stmts.len() {
        let going_on = match &stmts[i].kind {
            StmtKind::If(_, then, Some(otherwise))
                if matches!(stmts[i + 1].kind, StmtKind::Continue(_) | StmtKind::Break(_)) =>
            {
                let (a, b) = (then.last().is_some_and(ends), otherwise.last().is_some_and(ends));
                (a != b).then_some(b)
            }
            _ => None,
        };
        if let Some(in_then) = going_on {
            let jump = stmts.remove(i + 1);
            let StmtKind::If(_, then, Some(otherwise)) = &mut stmts[i].kind else {
                unreachable!("matched")
            };
            if in_then { then } else { otherwise }.push(jump);
        }
        i += 1;
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
/// names, the loop's, by its label, or the block's label where it has none,
/// which `named labels` then names as the program did.
pub(in crate::lower::mir::tidy) fn blocks_loops_end(stmts: &mut Vec<Stmt>, every: bool) {
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
    // with: where it ends, they do.
    // A loop `stmts` end with, through the `if`s and blocks they end with,
    // whose body still names `label` once its own `break`s are: given the
    // label `to`, as each of those `break`s is, where `to` is given.
    fn ending(stmts: &mut [Stmt], label: &str, every: bool, to: &mut Option<String>) {
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
                match looped {
                    _ if !js::names_break(body, label) => {}
                    Some(looped) => super::tails::relabeled(body, label, looped),
                    None => {
                        if let Some(to) = to.take() {
                            super::tails::relabeled(body, label, &to);
                            *looped = Some(to);
                        }
                    }
                }
            }
            StmtKind::If(_, a, b) => {
                ending(a, label, every, to);
                if let Some(b) = b {
                    ending(b, label, every, to);
                }
            }
            StmtKind::Labeled(_, body) => ending(body, label, every, to),
            _ => {}
        }
    }
    // The loop labeled `GIVEN` in `stmts`, however deep: `label` its label
    // and its `break`s' where `keep`, or none and the block's again.
    fn relabel_loop(stmts: &mut Vec<Stmt>, label: &str, keep: bool) {
        js::statement_lists(stmts, &mut |list| {
            for stmt in list.iter_mut() {
                if let StmtKind::While {
                    label: looped, body, ..
                }
                | StmtKind::ForOf {
                    label: looped, body, ..
                }
                | StmtKind::For {
                    label: looped, body, ..
                } = &mut stmt.kind
                    && looped.as_deref() == Some(GIVEN)
                {
                    super::tails::relabeled(body, GIVEN, label);
                    *looped = keep.then(|| label.to_string());
                }
            }
        });
    }
    // A name no label has, a loop's while it's given the block's.
    const GIVEN: &str = "$given";
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::Labeled(label, body) = &mut stmts[i].kind else {
            i += 1;
            continue;
        };
        if !crate::lower::mir::is_block_label(label) {
            i += 1;
            continue;
        }
        let label = label.clone();
        let mut to = Some(GIVEN.to_string());
        ending(body, &label, every, &mut to);
        // A loop given the label, which nothing else names: the loop's,
        // and the block's statements in its place, where they fit.
        if to.is_none() {
            let StmtKind::Labeled(_, body) = &stmts[i].kind else {
                unreachable!("matched")
            };
            let leaves = !js::names_jump(body, &label) && super::labels::fits_in_place(stmts, i, body);
            let StmtKind::Labeled(_, body) = &mut stmts[i].kind else {
                unreachable!("matched")
            };
            relabel_loop(body, &label, leaves);
            if leaves {
                let StmtKind::Labeled(_, body) = stmts.remove(i).kind else {
                    unreachable!("matched")
                };
                let n = body.len();
                stmts.splice(i..i, body);
                i += n;
                continue;
            }
        }
        i += 1;
    }
}

/// A loop that ends the body of loop `l` around it, one the program names
/// no label, as `loops` say, not `'rows`, whose `continue 'rows` it wrote:
/// each `continue l` of its own, not a loop's inside it, its `break`, which
/// ends `l`'s turn as well, as Rust's `break` of a loop that ends
/// another's does.
pub(in crate::lower::mir::tidy) fn outer_continues(stmts: &mut [Stmt], loops: &[(js::Span, bool, Option<String>)]) {
    fn broken(stmts: &mut [Stmt], looped: &str) {
        for stmt in stmts {
            match &mut stmt.kind {
                StmtKind::Continue(Some(l)) if l == looped => stmt.kind = StmtKind::Break(None),
                StmtKind::If(_, a, b) => {
                    broken(a, looped);
                    if let Some(b) = b {
                        broken(b, looped);
                    }
                }
                StmtKind::Labeled(_, body) => broken(body, looped),
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    broken(a, looped);
                    broken(b, looped);
                }
                _ => {}
            }
        }
    }
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
            && let Some(
                StmtKind::While { body: inner, .. }
                | StmtKind::ForOf { body: inner, .. }
                | StmtKind::For { body: inner, .. },
            ) = body.last_mut().map(|s| &mut s.kind)
        {
            broken(inner, label);
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
