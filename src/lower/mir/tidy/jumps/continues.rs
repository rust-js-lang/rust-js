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
                    // Its branches first, `if (c) { s; break l; } t` an `else`,
                    // as `branch_ends` has them: a `continue` the program
                    // wrote, which ends one, comes back (below).
                    ending(&mut inner, &block);
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
                Some(StmtKind::If(test, then, None))
                    if tests_continue(guards, body.last().expect("matched").span)
                        || (negated(test) && then.len() > 1) =>
                {
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
/// kept, as written (ADR 0684).
pub(in crate::lower::mir::tidy) fn tail_continues(stmts: &mut [Stmt]) {
    for stmt in stmts.iter_mut() {
        if let StmtKind::While { label, body, .. }
        | StmtKind::ForOf { label, body, .. }
        | StmtKind::For { label, body, .. } = &mut stmt.kind
        {
            ends_going_on(body, label);
        }
    }
}

/// A `continue` where the body of loop `looped` ends anyway, in the
/// branches that end it: none.
fn ends_going_on(stmts: &mut Vec<Stmt>, looped: &Option<String>) {
    let more = stmts.len() > 1;
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(StmtKind::Continue(l)) if (l.is_none() || l == looped) && more => {
            stmts.pop();
        }
        Some(StmtKind::If(_, then, otherwise)) => {
            ends_going_on(then, looped);
            if let Some(otherwise) = otherwise {
                ends_going_on(otherwise, looped);
            }
        }
        _ => {}
    }
}

/// A labeled block MIR made followed by a labeled jump, `l: { .. break l;
/// .. } continue m;`: each `break l` that jump, which goes on from there
/// anyway; `unlabeled` then leaves the block, which nothing names.
pub(in crate::lower::mir::tidy) fn jumps_past_blocks(stmts: &mut [Stmt]) {
    fn rejump(stmts: &mut [Stmt], label: &str, jump: &StmtKind) {
        for stmt in stmts {
            match &mut stmt.kind {
                StmtKind::Break(Some(l)) if l == label => stmt.kind = jump.clone(),
                StmtKind::If(_, a, b) => {
                    rejump(a, label, jump);
                    if let Some(b) = b {
                        rejump(b, label, jump);
                    }
                }
                StmtKind::While { body, .. }
                | StmtKind::ForOf { body, .. }
                | StmtKind::For { body, .. }
                | StmtKind::Labeled(_, body) => rejump(body, label, jump),
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
            jump @ (StmtKind::Continue(Some(_)) | StmtKind::Break(Some(_))) => jump.clone(),
            _ => continue,
        };
        if let StmtKind::Labeled(label, body) = &mut stmts[i].kind
            && crate::lower::mir::is_block_label(label)
        {
            let label = label.clone();
            rejump(body, &label, &jump);
        }
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
