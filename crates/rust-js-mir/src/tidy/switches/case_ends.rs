//! How a `switch`'s case ends (ADR 0731, ADR 0790): the `break`s that
//! leave it, as a person writes a case's end, a block around it whose
//! label nothing names then gone, and the `None` test before a `switch` of
//! its value left out where its `default` does nothing.

use super::*;

/// A block's `break` in a `switch` its statements end with, `l: { if (c) {
/// .. } else { switch (k) { case 1: .. break l; } } }`, which goes where
/// the `switch`'s own `break` does, nothing between: that `break`, as a
/// person writes a case's end (ADR 0731); and the block's label, which
/// nothing names now, gone.
///
/// And an `Option`'s test of `None` before a `switch` of its value, `if (x
/// == null) { break l; } switch (x) { .. }` where they end the block, or
/// `if (x != null) { switch (x) { .. } }`, where no case is `null`'s and
/// the `default` does nothing: the `switch` alone, as JS's `switch` takes
/// what no case is, `switch (thenable.status) { case "fulfilled": .. }`, of
/// `match status.as_deref() { Some("fulfilled") => .., _ => {} }` (ADR
/// 0629, amended); a `default` of a `break` alone, nothing.
pub(in crate::tidy) fn breaks_past_switches(stmts: &mut Vec<Stmt>, switches: &[Switch]) {
    for stmt in stmts.iter_mut() {
        if let StmtKind::Labeled(label, body) = &mut stmt.kind {
            let label = label.clone();
            some_switch_defaults(body, switches);
            ending_switches(body, &mut |case| case_breaks(case, &label));
            // `None`, which no case takes, goes past the `switch` to the
            // block's end, as the `break` does.
            if let [.., guard, switch] = &body[..]
                && let StmtKind::If(test, then, None) = &guard.kind
                && let [
                    Stmt {
                        kind: StmtKind::Break(Some(to)),
                        ..
                    },
                ] = &then[..]
                && *to == label
                && js::null_test(test, false).is_some_and(|x| switch_of(switch, x, switches))
            {
                body.remove(body.len() - 2);
            }
        }
    }
    unlabeled(stmts);
    let mut i = 0;
    while i < stmts.len() {
        nones_defaulted(&mut stmts[i], switches);
        if let StmtKind::If(test, then, None) = &stmts[i].kind
            && let [switch] = &then[..]
            && js::null_test(test, true).is_some_and(|x| switch_of(switch, x, switches))
        {
            let switch = switch.clone();
            stmts[i] = switch;
        }
        crate::tidy::case_rests::copied_returns_broken(stmts, i, switches);
        if let StmtKind::Switch { cases, default, .. } = &mut stmts[i].kind {
            if default.as_deref().is_some_and(does_nothing) {
                *default = None;
            }
            // Each clause that goes on past the `switch` (#488).
            let past = (cases.iter_mut().filter(|case| !case.falls)).map(|case| &mut case.body);
            for body in past.chain(default.as_mut()) {
                guard_held(body, switches);
                // Its own `break`, which its end does (ADR 0629, amended),
                // as the printer ends a case that goes on with one.
                if matches!(body.last().map(|s| &s.kind), Some(StmtKind::Break(None))) {
                    body.pop();
                }
                ending_breaks(body);
            }
            // A last case then of nothing, `case 3:` of `3 if n > 1 =>
            // continue` before `_ => {}`, which goes past the `switch` either
            // way: none, as what it tests is only constants (#488).
            while default.is_none() && cases.last().is_some_and(|case| case.body.is_empty()) {
                cases.pop();
            }
            if discriminant_in_place(stmts, i) {
                i -= 1;
            }
        }
        i += 1;
    }
}

/// Of `stmts`, a case's that goes on past the `switch`, each `break;` MIR
/// made that ends a branch of the `if` they end with, however deep, gone,
/// which goes where that branch's end does; a branch then empty, as
/// `emptied_branch` writes it: `case 1: if (n > 3) { break; } break;` of a
/// guarded arm of `continue` in a loop's last `match` is `case 1: break;`
/// (#488). Not one the program wrote, `break 'get_tag;`, whose `if` has no
/// `else` (ADR 0944).
pub(in crate::tidy) fn ending_breaks(stmts: &mut Vec<Stmt>) {
    let Some(StmtKind::If(_, then, otherwise)) = stmts.last_mut().map(|s| &mut s.kind) else {
        return;
    };
    for branch in std::iter::once(then).chain(otherwise) {
        match branch.last() {
            Some(end) if matches!(end.kind, StmtKind::Break(None)) && end.span.is_none() => {
                branch.pop();
            }
            _ => ending_breaks(branch),
        }
    }
    let last = stmts.pop().expect("matched");
    if let Some(kind) = crate::tidy::branches::emptied_branch(last.kind) {
        stmts.push(Stmt { kind, ..last });
    }
}

/// A block a case ends with, through the branches of an `if` it ends with,
/// `case "object": if (..) { .. } else { desc: { .. break desc; .. } }`:
/// where it ends, the case does, so each of its `break`s, outside a loop or
/// `switch` of its own, is the case's `break;`, as a person leaves a case
/// early, and its label, which nothing names now, gone (ADR 0790). A block
/// the program labeled, `'desc: { .. }`, Rust's way to leave a `match`'s
/// arm early, reads as JS leaves a case.
///
/// An `if` the program wrote with no `else`, `if (c) { ..; return; } t`,
/// to whose `then` MIR gave one, `t`, as both went on to the block's end,
/// has none once the `then` leaves where the case does: `t` after it, as
/// written (ADR 0805), not the `else` of an `else if` it reads as, of the
/// arm before it, where the block was all that arm's `else` held.
pub(in crate::tidy) fn blocks_ending_cases(stmts: &mut [Stmt], elses: &[(js::Span, js::Span, js::Span)]) {
    fn ending(stmts: &mut Vec<Stmt>, elses: &[(js::Span, js::Span, js::Span)]) {
        match stmts.last_mut().map(|s| &mut s.kind) {
            Some(StmtKind::If(_, a, b)) => {
                ending(a, elses);
                if let Some(b) = b {
                    ending(b, elses);
                }
            }
            Some(StmtKind::Labeled(label, body)) => {
                let label = label.clone();
                case_breaks(body, &label);
                ending(body, elses);
            }
            _ => {}
        }
        unlabeled(stmts);
        unwritten_else(stmts, elses);
    }
    for stmt in stmts.iter_mut() {
        if let StmtKind::Switch { cases, default, .. } = &mut stmt.kind {
            for case in js::case_lists_mut(cases, default) {
                ending(case, elses);
            }
        }
    }
}

/// `stmts`' last `if`, `if (c) { a } else { b }`, as `blocks_ending_cases`
/// has it: of an `if` written with no `else` (`Written::elses`) whose `a`
/// leaves, `if (c) { a } b`. What `b` declares is a body's name of its
/// own, given apart from the rest, `name$1`, which preparation names
/// `name` again only where JS's scopes let it (ADR 0352).
fn unwritten_else(stmts: &mut Vec<Stmt>, elses: &[(js::Span, js::Span, js::Span)]) {
    let Some(last) = stmts.last() else {
        return;
    };
    let StmtKind::If(_, a, Some(_)) = &last.kind else {
        return;
    };
    let written =
        |span: js::Span| span.is_none() || elses.iter().any(|(cond, ..)| cond.lo <= span.lo && span.hi <= cond.hi);
    if written(last.span) || !a.last().is_some_and(js::ends) {
        return;
    }
    let Some(Stmt {
        kind: StmtKind::If(_, _, b),
        ..
    }) = stmts.last_mut()
    else {
        unreachable!("matched")
    };
    let b = b.take().expect("matched");
    stmts.extend(b);
}

/// Each `switch` `stmts` end with, through the branches of an `if` they
/// end with and a block's statements, each of its cases given to `f`.
fn ending_switches(stmts: &mut [Stmt], f: &mut dyn FnMut(&mut Vec<Stmt>)) {
    match stmts.last_mut().map(|s| &mut s.kind) {
        Some(StmtKind::If(_, a, b)) => {
            ending_switches(a, f);
            if let Some(b) = b {
                ending_switches(b, f);
            }
        }
        Some(StmtKind::Labeled(_, body)) => ending_switches(body, f),
        Some(StmtKind::Switch { cases, default, .. }) => {
            for case in js::case_lists_mut(cases, default) {
                f(case);
            }
        }
        _ => {}
    }
}

/// Each `break label` of a case's `stmts`, outside a loop or `switch` of
/// its own, which a bare `break` would leave instead: a bare `break`.
pub(in crate::tidy) fn case_breaks(stmts: &mut [Stmt], label: &js::Name) {
    for stmt in stmts.iter_mut() {
        match &mut stmt.kind {
            StmtKind::Break(Some(l)) if l == label => stmt.kind = StmtKind::Break(None),
            StmtKind::If(_, a, b) => {
                case_breaks(a, label);
                if let Some(b) = b {
                    case_breaks(b, label);
                }
            }
            StmtKind::Labeled(_, body) => case_breaks(body, label),
            _ => {}
        }
    }
}
