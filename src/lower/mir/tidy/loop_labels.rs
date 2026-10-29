//! The loop a `switch`'s cases break: named by the label the program
//! wrote, `break rows;`; else, where the loop's turn ends at the `switch`,
//! left by a `break;` after it, each case going on `continue;` (ADR 0629,
//! amended, the user's decision of #389: no label the program didn't
//! write); and of guarded arms that fall through, which only the label
//! leaves from a `switch`, the `if` chain they are, its tests in the order
//! Rust tries them (#389b).

use std::collections::HashSet;

use crate::js::{self, Expr, Op, Stmt, StmtKind};

/// The label a loop's given where the program named it none and a `switch`
/// of guarded arms that fall through, which MIR's tests give no `if` chain
/// of, breaks it: `loop`, as ADR 0015 names one, never one the program
/// wrote, which Rust can't spell so; gone once the `switch` is its `if`
/// chain (`guarded_loops`).
pub(super) const LOOP_LABEL: &str = "loop";

/// Each `break` of no label in `stmts`, not in a loop of their own, of
/// `label`: the loop's around the `switch` they're a case of.
pub(super) fn labeled_breaks(stmts: &mut [Stmt], label: &str) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Break(l @ None) => *l = Some(label.to_string()),
            StmtKind::If(_, a, b) => {
                labeled_breaks(a, label);
                if let Some(b) = b {
                    labeled_breaks(b, label);
                }
            }
            StmtKind::Labeled(_, body) => labeled_breaks(body, label),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                labeled_breaks(a, label);
                labeled_breaks(b, label);
            }
            _ => {}
        }
    }
}

/// Each label in `stmts`, a loop's or a block's.
pub(super) fn labels_in(stmts: &[Stmt], labels: &mut HashSet<String>) {
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::While { label, body, .. }
            | StmtKind::ForOf { label, body, .. }
            | StmtKind::For { label, body, .. } => {
                labels.extend(label.clone());
                labels_in(body, labels);
            }
            StmtKind::Labeled(label, body) => {
                labels.insert(label.clone());
                labels_in(body, labels);
            }
            StmtKind::If(_, a, b) => {
                labels_in(a, labels);
                labels_in(b.as_deref().unwrap_or_default(), labels);
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                labels_in(a, labels);
                labels_in(b, labels);
            }
            StmtKind::Switch { cases, default, .. } => {
                js::case_lists(cases, default).for_each(|body| labels_in(body, labels));
            }
            _ => {}
        }
    }
}

/// Of a `switch` the loop's turn ends at, which a `break;` after it leaves
/// the loop by: each case that goes on past it to the loop's next turn,
/// `continue;`, and a value of none of them, `default: continue;`.
pub(super) fn continued(switch: &mut StmtKind) {
    let StmtKind::Switch { cases, default, .. } = switch else {
        return;
    };
    let goes_on = |body: &[Stmt]| !body.last().is_some_and(js::ends);
    for case in cases.iter_mut().filter(|case| !case.falls && goes_on(&case.body)) {
        case.body.push(StmtKind::Continue(None).at(js::Span::NONE));
    }
    match default {
        Some(body) if goes_on(body) => body.push(StmtKind::Continue(None).at(js::Span::NONE)),
        Some(_) => {}
        None => *default = Some(vec![StmtKind::Continue(None).at(js::Span::NONE)]),
    }
}

/// Each loop of `stmts` labeled `loop` (`LOOP_LABEL`, tidy's, never the
/// program's) that a `switch` of guarded arms that fall through breaks:
/// each such `switch`, its subject read again as it was, the `if` chain
/// its arms are (`guarded_chain`), and the label gone where nothing names
/// it then. One whose case leaves the `switch` from inside keeps its
/// label (#389b, the user's decision: no label the program didn't
/// write).
pub(super) fn guarded_loops(stmts: &mut Vec<Stmt>) {
    js::statement_lists(stmts, &mut |list| {
        for stmt in list.iter_mut() {
            let (StmtKind::While { label, body, .. }
            | StmtKind::DoWhile { label, body, .. }
            | StmtKind::ForOf { label, body, .. }
            | StmtKind::For { label, body, .. }) = &mut stmt.kind
            else {
                continue;
            };
            let Some(name) = label.clone().filter(|l| tidy_made(l)) else {
                continue;
            };
            chained_in(body, &name);
            if !js::names_jump(body, &name) {
                *label = None;
            }
        }
    });
}

/// Whether `stmt` is a `switch` of guarded arms that fall through, which
/// leaves a loop by tidy's label, whose subject its `if` chain will read
/// for each test (`guarded_chain`): held in the temporary MIR made, `const
/// nextCode = c.nextCode();`, read once, named as what it holds (#389c,
/// the user's decision).
pub(super) fn held_subject(stmt: &Stmt) -> bool {
    let StmtKind::Switch { cases, default, .. } = &stmt.kind else {
        return false;
    };
    let mut labels = HashSet::new();
    js::case_lists(cases, default).for_each(|body| jumped_labels(body, &mut labels));
    cases.iter().any(|case| case.falls) && labels.iter().any(|l| tidy_made(l))
}

/// Each label a `break` or `continue` of `stmts` names.
fn jumped_labels(stmts: &[Stmt], labels: &mut HashSet<String>) {
    js::each_list(stmts, &mut |list| {
        for stmt in list {
            if let StmtKind::Break(Some(l)) | StmtKind::Continue(Some(l)) = &stmt.kind {
                labels.insert(l.clone());
            }
        }
    });
}

/// Whether `label` is tidy's `loop` or `loop$1`.
fn tidy_made(label: &str) -> bool {
    label.strip_prefix(LOOP_LABEL).is_some_and(|n| {
        n.is_empty()
            || n.strip_prefix('$')
                .is_some_and(|n| n.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// Each `switch` of `stmts`, not in a loop or a `switch` of its own, whose
/// cases leave the loop `label` names: its `if` chain, where it is one,
/// which reads its subject as many times as it tests it, as MIR's tests
/// did: a place, `codes[i]`, which no guard can change, Rust's borrow of
/// what's matched forbids it, or a temporary that holds what does
/// something, `const nextCode = c.nextCode();`; and a `switch` that stays,
/// held still where the `switch` stays one.
fn chained_in(stmts: &mut [Stmt], label: &str) {
    for stmt in stmts.iter_mut() {
        let span = stmt.span;
        match &mut stmt.kind {
            StmtKind::Switch {
                discriminant,
                cases,
                default,
            } if js::case_lists(cases, default).any(|body| js::names_jump(body, label))
                && discriminant.reads_same() =>
            {
                if let Some(chain) = guarded_chain(discriminant, cases, default.as_deref(), label, span) {
                    *stmt = chain;
                }
            }
            StmtKind::If(_, a, b) => {
                chained_in(a, label);
                chained_in(b.as_deref_mut().unwrap_or_default(), label);
            }
            StmtKind::Labeled(_, body) => chained_in(body, label),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                chained_in(a, label);
                chained_in(b, label);
            }
            _ => {}
        }
    }
}

/// The `if` chain the arms of a `switch` of `subject` are, as Rust tries
/// them, of guarded arms that fall through (ADR 0905): each case's test,
/// of its values and those of each case that falls to it, `subject === 2
/// || subject === 3`, and of its guard where it falls, `&& on(three)`, its
/// statements its own, each jump of the loop `label` names one of no label.
/// `None` where a case isn't so: one that falls isn't its guard's `if`
/// alone, or one leaves the `switch` but at its end.
fn guarded_chain(
    subject: &Expr,
    cases: &[js::Case],
    default: Option<&[Stmt]>,
    label: &str,
    span: js::Span,
) -> Option<Stmt> {
    let mut links: Vec<(Option<Expr>, Vec<Stmt>)> = Vec::new();
    let mut fallen: Vec<Expr> = Vec::new();
    let arms = (cases
        .iter()
        .map(|case| (Some(&case.values), &case.body[..], case.falls)))
    .chain(default.map(|body| (None, body, false)));
    for (values, body, falls) in arms {
        let mut tested = std::mem::take(&mut fallen);
        tested.extend(values.into_iter().flatten().cloned());
        let (guard, mut body) = match (falls, body) {
            (false, _) => (None, body.to_vec()),
            (
                true,
                [
                    Stmt {
                        kind: StmtKind::If(guard, then, None),
                        ..
                    },
                ],
            ) if values.is_some() => (Some(guard.clone()), then.clone()),
            (true, _) => return None,
        };
        if falls {
            fallen = tested.clone();
        }
        // The `break;` that left the `switch`, at its end, and each that
        // ends a branch of the `if` it ends with, which goes there anyway.
        if matches!(
            body.last(),
            Some(Stmt {
                kind: StmtKind::Break(None),
                ..
            })
        ) {
            body.pop();
        }
        elses_of_breaks(&mut body);
        super::switches::ending_breaks(&mut body);
        own_jumps(&mut body, label)?;
        let test = match values {
            None => None,
            Some(_) => {
                let any = (tested.into_iter())
                    .map(|value| Expr::bin(Op::Eq, subject.clone(), value))
                    .reduce(|a, b| Expr::bin(Op::Or, a, b))?;
                Some(guard.into_iter().fold(any, |any, guard| Expr::bin(Op::And, any, guard)))
            }
        };
        links.push((test, body));
    }
    // A link of no statements, `b' ' => {}`'s: the rest's test, `else if
    // (code !== 32) { .. }`; the last, where its test does nothing, none.
    let mut rest: Option<Vec<Stmt>> = None;
    for (test, body) in links.into_iter().rev() {
        let after = rest.take().filter(|r| !r.is_empty());
        rest = Some(match (test, after) {
            (None, _) => body,
            (Some(test), Some(after)) if body.is_empty() => {
                vec![StmtKind::If(test.negated_test(), after, None).at(span)]
            }
            (Some(test), None) if body.is_empty() => match test.has_effects() {
                true => return None,
                false => Vec::new(),
            },
            (Some(test), after) => vec![StmtKind::If(test, body, after).at(span)],
        });
    }
    match &rest?[..] {
        [
            chain @ Stmt {
                kind: StmtKind::If(..), ..
            },
        ] => Some(chain.clone()),
        _ => None,
    }
}

/// Of a case's statements, an `if` whose branch leaves the `switch`, `if
/// (c) { a; break; } b`, which an `if` chain's link can't: its `else` what
/// follows it, `if (c) { a; break; } else { b }`, so that `break;` ends a
/// branch of the `if` the statements end with, which `ending_breaks`
/// takes away; and so in each branch.
fn elses_of_breaks(stmts: &mut Vec<Stmt>) {
    let leaves = |branch: &[Stmt]| matches!(branch.last().map(|s| &s.kind), Some(StmtKind::Break(None)));
    if let Some(k) = (stmts.iter()).position(|s| matches!(&s.kind, StmtKind::If(_, then, None) if leaves(then)))
        && k + 1 < stmts.len()
    {
        let rest = stmts.split_off(k + 1);
        if let StmtKind::If(_, _, otherwise) = &mut stmts[k].kind {
            *otherwise = Some(rest);
        }
    }
    for stmt in stmts.iter_mut() {
        if let StmtKind::If(_, then, otherwise) = &mut stmt.kind {
            elses_of_breaks(then);
            if let Some(otherwise) = otherwise {
                elses_of_breaks(otherwise);
            }
        }
    }
}

/// Of a case's statements, each `break label;` and `continue label;` one
/// of no label, not in a loop or a `switch` of its own, whose jumps stay
/// as they are, the loop keeping its label; `None` where one leaves the
/// `switch`, `break;`.
fn own_jumps(stmts: &mut [Stmt], label: &str) -> Option<()> {
    for stmt in stmts.iter_mut() {
        match &mut stmt.kind {
            StmtKind::Break(None) => return None,
            StmtKind::Break(l) | StmtKind::Continue(l) if l.as_deref() == Some(label) => *l = None,
            StmtKind::If(_, a, b) => {
                own_jumps(a, label)?;
                own_jumps(b.as_deref_mut().unwrap_or_default(), label)?;
            }
            StmtKind::Labeled(_, body) => own_jumps(body, label)?,
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                own_jumps(a, label)?;
                own_jumps(b, label)?;
            }
            _ => {}
        }
    }
    Some(())
}
