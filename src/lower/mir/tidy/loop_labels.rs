//! The loop a `switch`'s cases break: named by the label the program
//! wrote, `break rows;`; else, where the loop's turn ends at the `switch`,
//! left by a `break;` after it, each case going on `continue;` (ADR 0629,
//! amended, the user's decision of #389: no label the program didn't
//! write).

use std::collections::HashSet;

use crate::js::{self, Stmt, StmtKind};

/// The label a loop's given where the program named it none and a `switch`
/// of guarded arms that fall through, which MIR's tests give no `if` chain
/// of, breaks it: `loop`, as ADR 0015 names one (#389's rest).
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
