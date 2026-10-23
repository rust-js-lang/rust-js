//! What holds of a body between tidy's stages (ADR 0659), checked in a
//! debug build after each: of every body, its jumps land and its names are
//! declared once in a list; and, once a stage ensures it, nothing follows a
//! jump, and each label is one a jump names. A check that fails names the
//! first stage that broke it.

use crate::js::{Stmt, StmtKind};

/// A property of a body, and why a body doesn't have it.
pub(super) struct Contract {
    pub(super) name: &'static str,
    pub(super) broken: fn(&[Stmt]) -> Option<String>,
}

/// What holds of every body, after every stage.
pub(super) const ALWAYS: &[Contract] = &[JUMPS_LAND, DECLARED_ONCE];

/// Each `break` is in a loop, a `switch` or the block it names, and each
/// `continue` in a loop, the one it names: JS that parses.
pub(super) const JUMPS_LAND: Contract = Contract {
    name: "each jump lands",
    broken: |body| unlanded(body, &mut Vec::new()),
};

/// No list declares a name twice, which JS refuses.
pub(super) const DECLARED_ONCE: Contract = Contract {
    name: "each name declared once in a list",
    broken: |body| twice_declared(body),
};

/// No statement follows a `return`, a `throw`, a `break` or a `continue` in
/// its list: none that never runs.
pub(super) const NOTHING_AFTER_A_JUMP: Contract = Contract {
    name: "nothing after a jump",
    broken: |body| {
        let mut found = None;
        crate::js::each_list(body, &mut |stmts| {
            if found.is_none()
                && let Some(i) = stmts.iter().position(jumps)
                && i + 1 < stmts.len()
            {
                found = Some(format!("{} after a {}", described(&stmts[i + 1]), jump_name(&stmts[i])));
            }
        });
        found
    },
};

/// Each label is one a jump inside it names.
pub(super) const LABELS_NAMED: Contract = Contract {
    name: "each label named",
    broken: |body| {
        let mut found = None;
        crate::js::each_list(body, &mut |stmts| {
            for stmt in stmts.iter() {
                if found.is_none()
                    && let StmtKind::Labeled(label, inner)
                    | StmtKind::While {
                        label: Some(label),
                        body: inner,
                        ..
                    }
                    | StmtKind::ForOf {
                        label: Some(label),
                        body: inner,
                        ..
                    }
                    | StmtKind::For {
                        label: Some(label),
                        body: inner,
                        ..
                    } = &stmt.kind
                    && !crate::js::names_jump(inner, label)
                {
                    found = Some(format!("`{label}` named by no jump"));
                }
            }
        });
        found
    },
};

/// A statement, as a message names it: `const x`, an `if`.
fn described(stmt: &Stmt) -> String {
    match &stmt.kind {
        StmtKind::Const(name, _) => format!("`const {name}`"),
        StmtKind::Let(name, _) => format!("`let {name}`"),
        StmtKind::Destructure { .. } => "a destructuring".into(),
        StmtKind::Assign(..) | StmtKind::Compound(..) => "an assignment".into(),
        StmtKind::Expr(_) => "an expression".into(),
        StmtKind::If(..) => "an `if`".into(),
        StmtKind::While { .. } | StmtKind::ForOf { .. } | StmtKind::For { .. } => "a loop".into(),
        StmtKind::Labeled(label, _) => format!("block `{label}`"),
        StmtKind::Switch { .. } => "a `switch`".into(),
        StmtKind::Try(..) | StmtKind::TryCatch(..) => "a `try`".into(),
        _ => jump_name(stmt).into(),
    }
}

fn jump_name(stmt: &Stmt) -> &'static str {
    match stmt.kind {
        StmtKind::Return(_) => "`return`",
        StmtKind::Throw(_) => "`throw`",
        StmtKind::Break(_) => "`break`",
        _ => "`continue`",
    }
}

fn jumps(stmt: &Stmt) -> bool {
    matches!(
        stmt.kind,
        StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(_) | StmtKind::Continue(_)
    )
}

/// What a jump may land on, innermost last: a loop, by its label if it has
/// one, a `switch`, or a labeled block.
enum Target<'a> {
    Loop(Option<&'a str>),
    Switch,
    Block(&'a str),
}

fn unlanded<'a>(stmts: &'a [Stmt], targets: &mut Vec<Target<'a>>) -> Option<String> {
    for stmt in stmts {
        let found = match &stmt.kind {
            StmtKind::Break(None) => (!targets.iter().any(|t| matches!(t, Target::Loop(_) | Target::Switch)))
                .then(|| "a `break` outside a loop or a `switch`".to_string()),
            StmtKind::Continue(None) => (!targets.iter().any(|t| matches!(t, Target::Loop(_))))
                .then(|| "a `continue` outside a loop".to_string()),
            StmtKind::Break(Some(label)) => (!targets
                .iter()
                .any(|t| matches!(t, Target::Loop(Some(l)) | Target::Block(l) if l == label)))
            .then(|| format!("a `break {label}` outside `{label}`")),
            StmtKind::Continue(Some(label)) => {
                (!targets.iter().any(|t| matches!(t, Target::Loop(Some(l)) if l == label)))
                    .then(|| format!("a `continue {label}` outside its loop"))
            }
            _ => None,
        };
        if found.is_some() {
            return found;
        }
        let mut inside = |target: Option<Target<'a>>, body: &'a [Stmt]| {
            let pushed = target.is_some();
            targets.extend(target);
            let found = unlanded(body, targets);
            if pushed {
                targets.pop();
            }
            found
        };
        let found = match &stmt.kind {
            StmtKind::While { label, body, .. }
            | StmtKind::ForOf { label, body, .. }
            | StmtKind::For { label, body, .. } => inside(Some(Target::Loop(label.as_deref())), body),
            StmtKind::Labeled(label, body) => inside(Some(Target::Block(label)), body),
            StmtKind::Switch { cases, default, .. } => {
                crate::js::case_lists(cases, default).find_map(|body| inside(Some(Target::Switch), body))
            }
            StmtKind::If(_, a, b) => inside(None, a).or_else(|| b.as_deref().and_then(|b| inside(None, b))),
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => inside(None, a).or_else(|| inside(None, b)),
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

fn twice_declared(body: &[Stmt]) -> Option<String> {
    let mut found = None;
    crate::js::each_list(body, &mut |stmts| {
        let mut seen = std::collections::HashSet::new();
        for stmt in stmts {
            if let StmtKind::Let(name, _) | StmtKind::Const(name, _) = &stmt.kind
                && !seen.insert(name.as_str())
                && found.is_none()
            {
                found = Some(format!("`{name}` declared twice"));
            }
        }
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::{Expr, Span};

    fn stmt(kind: StmtKind) -> Stmt {
        kind.at(Span::NONE)
    }

    fn holds(contract: &Contract, body: &[Stmt]) -> bool {
        (contract.broken)(body).is_none()
    }

    #[test]
    fn a_jump_lands_where_it_may() {
        let loop_ = |label: Option<&str>, body| {
            stmt(StmtKind::While {
                label: label.map(String::from),
                cond: Expr::bool(true),
                body,
            })
        };
        let break_ = |label: Option<&str>| stmt(StmtKind::Break(label.map(String::from)));
        let continue_ = |label: Option<&str>| stmt(StmtKind::Continue(label.map(String::from)));
        assert!(holds(&JUMPS_LAND, &[loop_(None, vec![break_(None), continue_(None)])]));
        assert!(holds(
            &JUMPS_LAND,
            &[stmt(StmtKind::Labeled("l".into(), vec![break_(Some("l"))]))]
        ));
        assert!(holds(
            &JUMPS_LAND,
            &[loop_(Some("o"), vec![loop_(None, vec![continue_(Some("o"))])])]
        ));
        assert!(!holds(&JUMPS_LAND, &[break_(None)]));
        assert!(!holds(
            &JUMPS_LAND,
            &[stmt(StmtKind::Labeled("l".into(), vec![continue_(None)]))]
        ));
        assert!(!holds(
            &JUMPS_LAND,
            &[stmt(StmtKind::Labeled("l".into(), vec![continue_(Some("l"))]))]
        ));
        assert!(!holds(&JUMPS_LAND, &[loop_(Some("o"), vec![]), break_(Some("o"))]));
    }

    #[test]
    fn a_name_is_declared_once_in_a_list() {
        let const_ = |name: &str| stmt(StmtKind::Const(name.into(), Expr::bool(true)));
        assert!(holds(
            &DECLARED_ONCE,
            &[
                const_("a"),
                stmt(StmtKind::If(Expr::bool(true), vec![const_("a")], None))
            ]
        ));
        assert!(!holds(
            &DECLARED_ONCE,
            &[const_("a"), stmt(StmtKind::Let("a".into(), None))]
        ));
    }

    #[test]
    fn nothing_follows_a_jump() {
        let ret = stmt(StmtKind::Return(None));
        let after = stmt(StmtKind::Expr(Expr::bool(true)));
        assert!(holds(&NOTHING_AFTER_A_JUMP, &[after.clone(), ret.clone()]));
        assert!(!holds(&NOTHING_AFTER_A_JUMP, &[ret.clone(), after.clone()]));
        assert!(!holds(
            &NOTHING_AFTER_A_JUMP,
            &[stmt(StmtKind::If(Expr::bool(true), vec![ret, after], None))]
        ));
    }

    #[test]
    fn a_label_is_named() {
        let block = |body| stmt(StmtKind::Labeled("l".into(), body));
        assert!(holds(
            &LABELS_NAMED,
            &[block(vec![stmt(StmtKind::Break(Some("l".into())))])]
        ));
        assert!(!holds(&LABELS_NAMED, &[block(vec![stmt(StmtKind::Break(None))])]));
        let loop_ = stmt(StmtKind::While {
            label: Some("o".into()),
            cond: Expr::bool(true),
            body: vec![stmt(StmtKind::Continue(Some("o".into())))],
        });
        assert!(holds(&LABELS_NAMED, &[loop_]));
    }
}
