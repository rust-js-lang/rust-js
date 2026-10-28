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

/// Whether `label` is tidy's `loop` or `loop$1`.
fn tidy_made(label: &str) -> bool {
    label.strip_prefix(LOOP_LABEL).is_some_and(|n| {
        n.is_empty()
            || n.strip_prefix('$')
                .is_some_and(|n| n.bytes().all(|b| b.is_ascii_digit()))
    })
}

/// Each `switch` of `stmts`, not in a loop or a `switch` of its own, whose
/// cases leave the loop `label` names: its `if` chain, where it is one.
fn chained_in(stmts: &mut [Stmt], label: &str) {
    for stmt in stmts.iter_mut() {
        let span = stmt.span;
        match &mut stmt.kind {
            StmtKind::Switch {
                discriminant,
                cases,
                default,
            } if js::case_lists(cases, default).any(|body| js::names_jump(body, label))
                && discriminant.reads_only_vars() =>
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
        // The `break;` that left the `switch`, at its end alone.
        if matches!(
            body.last(),
            Some(Stmt {
                kind: StmtKind::Break(None),
                ..
            })
        ) {
            body.pop();
        }
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
    let mut rest: Option<Vec<Stmt>> = None;
    for (test, body) in links.into_iter().rev() {
        rest = Some(match test {
            None => body,
            Some(test) => vec![StmtKind::If(test, body, rest.filter(|r| !r.is_empty())).at(span)],
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
