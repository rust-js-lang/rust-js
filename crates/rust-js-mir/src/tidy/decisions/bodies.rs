//! What an arm's body is, as a chain reads it: how many statements it is
//! (`size`), and whether it only binds, so a longer arm is first (#774).

use super::*;

/// How many statements `stmt` is, with those inside it.
pub(super) fn size(stmt: &Stmt) -> usize {
    let list = |stmts: &[Stmt]| stmts.iter().map(size).sum::<usize>();
    1 + match &stmt.kind {
        StmtKind::If(_, a, b) => list(a) + b.as_deref().map_or(0, list),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => list(body),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => list(a) + list(b),
        _ => 0,
    }
}

/// Whether, of a chain of two arms, `first` only binds and `other` is
/// longer, so `other` is first, of the test negated, as a person writes
/// the case that does the work first: `if (!stacks$1) { stacks = []; .. }
/// else { stacks = stacks$1; }`, not `if (stacks$1) { stacks = stacks$1;
/// } else { .. }` (#774, the user's decision).
pub(super) fn longer_first(first: &[Stmt], other: &[Stmt]) -> bool {
    let length = |body: &[Stmt]| body.iter().map(size).sum::<usize>();
    binding_only(first) && length(other) > length(first)
}

/// Whether `body` only binds variables to what's read, the pattern's
/// binding and the `match`'s value, `const stacks$1 = _5; stacks =
/// stacks$1;` or `x = o._0`: no call, no value made, nothing set but a
/// variable.
fn binding_only(body: &[Stmt]) -> bool {
    let read = |value: &Expr| {
        matches!(
            value.kind,
            ExprKind::Var(_) | ExprKind::Member(..) | ExprKind::Index(..)
        ) && pure(value)
    };
    !body.is_empty()
        && body.iter().all(|s| match &s.kind {
            StmtKind::Const(_, value) | StmtKind::Let(_, Some(value)) => read(value),
            StmtKind::Assign(target, value) => matches!(target.kind, ExprKind::Var(_)) && read(value),
            _ => false,
        })
}
