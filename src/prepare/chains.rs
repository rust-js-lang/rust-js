//! Two places set the same value one after the other, `b = v; a = v;`, as
//! one statement, `a = b = v`, as JS writes them: React's hooks'
//! `hook.memoizedState = hook.baseState = initialState;` and
//! `workInProgressHook = workInProgressHook.next = hook;` (ADR 0968).

use crate::js::{self, Expr, ExprKind, Known, Stmt, StmtKind};

/// Each `b = v;` followed by `a = v;` in `stmts` made `a = b = v;`, where
/// they do the same: `v` a variable or a constant, read the same twice;
/// `a` and `b` each a variable or a plain field of a path, `x.y.f`, no
/// binding's setter, whose objects read no getter; `b` not `v` itself; and
/// `a`'s object, which JS reads before `b` is set, reading nothing `b`
/// sets: not `b`'s variable, nor a property of `b`'s name, which may be of
/// the same object. Two alone, as a person chains them: a run of three or
/// more, drop flags' `a$live = false;` each, stays a statement each.
pub(super) fn chained(stmts: &mut Vec<Stmt>) {
    let mut i = 0;
    while i < stmts.len() {
        let run = run(&stmts[i..]);
        if run == 2 {
            let next = stmts.remove(i + 1);
            let stmt = &mut stmts[i];
            if let (StmtKind::Assign(second, _), StmtKind::Assign(first, value)) = (next.kind, &mut stmt.kind) {
                let first = std::mem::replace(first, Expr::undefined());
                let value = std::mem::replace(value, Expr::undefined());
                stmt.kind = StmtKind::Assign(second, Expr::assigned(first, value));
            }
        }
        i += run.max(1);
    }
}

/// How many of `stmts`' first statements set places the same value, one
/// after another, each of which `chained` could set in one statement with
/// those before it.
fn run(stmts: &[Stmt]) -> usize {
    let Some(StmtKind::Assign(first, value)) = stmts.first().map(|stmt| &stmt.kind) else {
        return 0;
    };
    let mut places = vec![first];
    for stmt in &stmts[1..] {
        let StmtKind::Assign(second, again) = &stmt.kind else {
            break;
        };
        let joins = same_value(value, again)
            && plain(second)
            && places.iter().all(|place| {
                plain(place)
                    && !js::same_path(place, second)
                    && !matches!((&place.kind, &value.kind), (ExprKind::Var(x), ExprKind::Var(v)) if x == v)
                    && !reads_what_it_sets(second, place)
            });
        if !joins {
            break;
        }
        places.push(second);
    }
    places.len()
}

/// Whether `value` and `again` are the same variable or the same
/// constant, which reads the same however often it's read: not a value
/// made of more, `n + 1`, which Rust evaluates twice.
fn same_value(value: &Expr, again: &Expr) -> bool {
    match (&value.kind, &again.kind) {
        (ExprKind::Var(x), ExprKind::Var(y)) => x == y,
        (ExprKind::Num(_) | ExprKind::Str(_) | ExprKind::Bool(_) | ExprKind::Null | ExprKind::Undefined, _) => {
            value.kind == again.kind
        }
        _ => false,
    }
}

/// Whether `target` is a variable, or a plain field of a path, which no
/// setter writes and whose object no getter reads.
fn plain(target: &Expr) -> bool {
    match &target.kind {
        ExprKind::Var(_) => true,
        ExprKind::Member(object, _) => !target.facts.has(Known::Setter) && path(object),
        _ => false,
    }
}

/// Whether `e` is a variable or a chain of fields of one, `x.y`, none of
/// them a binding's getter.
fn path(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Var(_) => true,
        ExprKind::Member(object, _) => !e.facts.has(Known::Getter) && path(object),
        _ => false,
    }
}

/// Whether `second`'s object, read before `first` is set where the two are
/// one statement, reads what `first` sets: its variable, or a property of
/// its name.
fn reads_what_it_sets(second: &Expr, first: &Expr) -> bool {
    let ExprKind::Member(object, _) = &second.kind else {
        return false;
    };
    match &first.kind {
        ExprKind::Var(name) => object.mentions_var(name),
        ExprKind::Member(_, set) => {
            let mut reads = false;
            object.each(&mut |e| reads |= matches!(&e.kind, ExprKind::Member(_, field) if field == set));
            reads
        }
        _ => true,
    }
}
