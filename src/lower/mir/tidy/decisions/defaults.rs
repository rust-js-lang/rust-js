//! A block MIR made of a value some of its ways out give, `x = true; break
//! bb7;`, and its end another: that one given first, as a person gives a
//! default (ADR 0364, amended).

use crate::js::{self, Expr, ExprKind, Stmt, StmtKind};

/// `bb7: { .. x = true; break bb7; .. x = false; }`, a block MIR made of a
/// value, its last statement a constant it gives where nothing else does,
/// each other written just before a `break` of it: `x = false;` first, as
/// a person gives a default, the block then left where it ends. Only
/// where nothing tells it was given first (ADR 0364, amended): each other
/// way out of the block gives its own; and what the block does before
/// its end can't see it, the variable declared in the block's list and
/// named by nothing since, or neither runs code nor throws nor leaves
/// another way.
pub(in crate::lower::mir::tidy) fn default_values(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::Labeled(label, body) = &stmts[i].kind
            && crate::lower::mir::is_block_label(label)
            && let Some((last, rest)) = body.split_last()
            && let StmtKind::Assign(target, value) = &last.kind
            && let ExprKind::Var(x) = &target.kind
            && value.is_constant()
            && let Some(given) = (Given {
                x,
                label,
                local: declared_unread(&stmts[..i], x),
            })
            .before_breaks(rest, &Inside::default())
            && js::mentions_in(rest, x) == given
        {
            let StmtKind::Labeled(_, body) = &mut stmts[i].kind else {
                unreachable!("matched")
            };
            let last = body.pop().expect("split");
            stmts.insert(i, last);
            i += 1;
        }
        i += 1;
    }
}

/// Whether `x` is declared in `before`, `let x = ..;`, and named by nothing
/// since: what can see it is only what follows in the list, which a
/// way out of it never reaches.
fn declared_unread(before: &[Stmt], x: &str) -> bool {
    (before.iter())
        .rposition(|s| matches!(&s.kind, StmtKind::Let(name, _) if name == x))
        .is_some_and(|at| js::mentions_in(&before[at + 1..], x) == 0)
}

/// A value `x` a block labeled `label` gives last, as `default_values`
/// asks of what comes before it.
struct Given<'a> {
    x: &'a str,
    label: &'a str,
    /// Whether `x` is declared in the block's list, named by nothing
    /// since (`declared_unread`): nothing but the block sees it before it
    /// ends, nor once it's left another way.
    local: bool,
}

/// Where in the block a statement is: in which of its own labeled
/// statements, and in a loop or a `switch` of its own, which a `break` of
/// no label leaves, or a loop, which a `continue` does.
#[derive(Clone, Default)]
struct Inside<'s> {
    labels: Vec<&'s str>,
    in_loop: bool,
    in_switch: bool,
}

impl<'s> Inside<'s> {
    fn labeled(&self, label: Option<&'s str>, is_loop: bool, is_switch: bool) -> Inside<'s> {
        let mut inside = self.clone();
        inside.labels.extend(label);
        inside.in_loop |= is_loop;
        inside.in_switch |= is_switch;
        inside
    }

    /// Whether a jump to `label`, or of no label, stays in the block.
    fn holds(&self, label: Option<&str>, is_continue: bool) -> bool {
        match label {
            Some(l) => self.labels.contains(&l),
            None if is_continue => self.in_loop,
            None => self.in_loop || self.in_switch,
        }
    }
}

impl Given<'_> {
    /// How many times `stmts` assign `x` just before a `break` of the
    /// block, what it's given read nowhere else: `None` where another way
    /// out of the block gives none (a `break` of it after anything else,
    /// and, of a variable that isn't local, a `return`, a `throw` or a
    /// jump past it), or, of one that isn't, where what runs before may
    /// see it given: code of the program's that it calls, or what throws.
    fn before_breaks<'s>(&self, stmts: &'s [Stmt], inside: &Inside<'s>) -> Option<usize> {
        let quiet = |e: &Expr| self.local || !e.may_throw();
        let gives = |stmt: &Stmt| matches!(&stmt.kind, StmtKind::Assign(target, _) if matches!(&target.kind, ExprKind::Var(v) if v == self.x));
        let mut given = 0;
        for (k, stmt) in stmts.iter().enumerate() {
            given += match &stmt.kind {
                StmtKind::Assign(_, value) if gives(stmt) => {
                    let breaks =
                        matches!(stmts.get(k + 1).map(|s| &s.kind), Some(StmtKind::Break(Some(l))) if l == self.label);
                    if !breaks || js::mentions_in(&[StmtKind::Expr(value.clone()).at(stmt.span)], self.x) > 0 {
                        return None;
                    }
                    if !quiet(value) {
                        return None;
                    }
                    1
                }
                // A way out of the block that gives no value of its own.
                StmtKind::Break(Some(l)) if l == self.label => {
                    if !k.checked_sub(1).is_some_and(|p| gives(&stmts[p])) {
                        return None;
                    }
                    0
                }
                StmtKind::Break(label) | StmtKind::Continue(label) => {
                    let is_continue = matches!(stmt.kind, StmtKind::Continue(_));
                    if !self.local && !inside.holds(label.as_deref(), is_continue) {
                        return None;
                    }
                    0
                }
                StmtKind::Return(_) | StmtKind::Throw(_) if !self.local => return None,
                StmtKind::If(test, a, b) => {
                    if !quiet(test) {
                        return None;
                    }
                    self.before_breaks(a, inside)? + b.as_deref().map_or(Some(0), |b| self.before_breaks(b, inside))?
                }
                StmtKind::Labeled(label, body) => {
                    self.before_breaks(body, &inside.labeled(Some(label), false, false))?
                }
                StmtKind::While { label, cond, body } => {
                    if !quiet(cond) {
                        return None;
                    }
                    self.before_breaks(body, &inside.labeled(label.as_deref(), true, false))?
                }
                StmtKind::For {
                    label,
                    start,
                    test,
                    step,
                    body,
                    ..
                } => {
                    if !quiet(start) || !quiet(test) || step.value().is_some_and(|v| !quiet(v)) {
                        return None;
                    }
                    self.before_breaks(body, &inside.labeled(label.as_deref(), true, false))?
                }
                // Its iterator's `next` may be the program's.
                StmtKind::ForOf { label, body, .. } if self.local => {
                    self.before_breaks(body, &inside.labeled(label.as_deref(), true, false))?
                }
                StmtKind::Switch {
                    discriminant,
                    cases,
                    default,
                } => {
                    if !quiet(discriminant) || !cases.iter().flat_map(|case| &case.values).all(&quiet) {
                        return None;
                    }
                    let inside = inside.labeled(None, false, true);
                    let mut given = 0;
                    for body in js::case_lists(cases, default) {
                        given += self.before_breaks(body, &inside)?;
                    }
                    given
                }
                // Of a variable from before, a `catch` or a `finally` may read it.
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) if self.local => {
                    self.before_breaks(a, inside)? + self.before_breaks(b, inside)?
                }
                StmtKind::Function(_) | StmtKind::Directive(_) | StmtKind::Comments => 0,
                _ if self.local || stmt.only_makes_values() => 0,
                _ => return None,
            };
        }
        Some(given)
    }
}
