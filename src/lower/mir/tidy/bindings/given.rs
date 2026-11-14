//! A binding of a variable's own name, given apart, that the variable is
//! given after it: the variable given it where it's made (ADR 0853).

use super::*;

/// `const node$1 = createNode(); cache.set(key, node$1); node = node$1;`, a
/// binding of a variable's own name, given apart, that the variable is
/// given after it, where nothing between names the variable: the variable
/// given it where it's made, `node = createNode(); cache.set(key, node);`,
/// as `node = match .. { None => { let node = make(); ..; node } .. }`
/// reads (ADR 0853). Only of a variable the body declares, or a parameter
/// of it: one a closure captures may be read after what's between throws,
/// as it was, by what called the closure. Not where a `break` or a
/// `continue` between may leave before the variable's given it, nor in a
/// `try`, whose `catch` or `finally` may read the variable as it was, nor
/// of what a closure or a `&mut`'s handle reads that's made before it's
/// given, which may run between: one made after can't, as it isn't yet,
/// `return () => listeners.delete(l)` after `listeners = new Set()`
/// (rd105), unless a loop around both makes it again first.
pub(in crate::lower::mir::tidy) fn given_last(params: &[js::Pattern], body: &mut Vec<Stmt>) {
    let mut declared: IndexSet<String> = params.iter().flat_map(|p| p.names()).map(str::to_string).collect();
    declared_in(body, &mut declared);
    given_last_in(body, false, (&IndexSet::default(), &declared));
}

/// What `stmts` declare, in their blocks too: not their closures', which
/// are their own bodies.
fn declared_in(stmts: &[Stmt], declared: &mut IndexSet<String>) {
    for stmt in stmts {
        match &stmt.kind {
            StmtKind::Const(name, _) | StmtKind::Let(name, _) | StmtKind::For { name, .. } => {
                declared.insert(name.clone());
            }
            StmtKind::Destructure { pattern, .. } => declared.extend(pattern.names().into_iter().map(str::to_string)),
            _ => {}
        }
        match &stmt.kind {
            StmtKind::If(_, a, b) => {
                declared_in(a, declared);
                if let Some(b) = b {
                    declared_in(b, declared);
                }
            }
            StmtKind::ForOf { pattern, body, .. } => {
                declared.extend(pattern.names().into_iter().map(str::to_string));
                declared_in(body, declared);
            }
            StmtKind::While { body, .. } | StmtKind::For { body, .. } | StmtKind::Labeled(_, body) => {
                declared_in(body, declared);
            }
            StmtKind::Switch { cases, default, .. } => {
                js::case_lists(cases, default).for_each(|body| declared_in(body, declared));
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                declared_in(a, declared);
                declared_in(b, declared);
            }
            _ => {}
        }
    }
}

/// `given_last` of `stmts`, and of each block in them: in a `try`'s, none.
/// `made` is what's captured by what's made before `stmts` run, and of the
/// statements around them, a loop's again before them.
fn given_last_in(stmts: &mut Vec<Stmt>, in_try: bool, (made, declared): (&IndexSet<String>, &IndexSet<String>)) {
    if !in_try {
        given_first(stmts, (made, declared));
    }
    let mut made = made.clone();
    for stmt in stmts.iter_mut() {
        made.extend(captured(std::slice::from_ref(&*stmt)));
        let names = (&made, declared);
        match &mut stmt.kind {
            StmtKind::If(_, a, b) => {
                given_last_in(a, in_try, names);
                if let Some(b) = b {
                    given_last_in(b, in_try, names);
                }
            }
            StmtKind::While { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::Labeled(_, body) => given_last_in(body, in_try, names),
            StmtKind::Switch { cases, default, .. } => {
                for body in js::case_lists_mut(cases, default) {
                    given_last_in(body, in_try, names);
                }
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                given_last_in(a, true, names);
                given_last_in(b, true, names);
            }
            _ => {}
        }
    }
}

/// `given_last` of the block `stmts` itself: each variable given a binding
/// made before it in the block, the binding named nowhere after.
fn given_first(stmts: &mut Vec<Stmt>, (made, declared): (&IndexSet<String>, &IndexSet<String>)) {
    let mut given = 0;
    while given < stmts.len() {
        let Some((at, x, y)) = given_from(stmts, given, (made, declared)) else {
            given += 1;
            continue;
        };
        let StmtKind::Const(_, made) = &mut stmts[at].kind else {
            unreachable!("`given_from`'s `const`")
        };
        let made = std::mem::replace(made, Expr::taken());
        stmts[at].kind = StmtKind::Assign(Expr::var(&x), made);
        stmts.remove(given);
        js::each_expr_mut(&mut stmts[at + 1..given], &mut |e| {
            if matches!(&e.kind, ExprKind::Var(v) if *v == y) {
                *e = Expr::var(&x).or_at(e.span);
            }
        });
    }
}

/// Of `stmts[given]`, `x = y;`, `given_first`'s: where `y` is made, `x`
/// and `y`. What's captured is `made`'s, and what the block makes before.
fn given_from(
    stmts: &[Stmt],
    given: usize,
    (made, declared): (&IndexSet<String>, &IndexSet<String>),
) -> Option<(usize, String, String)> {
    let StmtKind::Assign(target, value) = &stmts[given].kind else {
        return None;
    };
    let (ExprKind::Var(x), ExprKind::Var(y)) = (&target.kind, &value.kind) else {
        return None;
    };
    if !kin(x, y) || !declared.contains(x) {
        return None;
    }
    let captured = captured(&stmts[..given]);
    let early = |name: &str| made.contains(name) || captured.contains(name);
    if early(x) || early(y) {
        return None;
    }
    let at = (stmts[..given].iter()).position(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == y))?;
    let StmtKind::Const(_, made) = &stmts[at].kind else {
        return None;
    };
    let between = &stmts[at + 1..given];
    let kept = made.mentions_var(x)
        || js::mentions_in(between, x) > 0
        || jumps(between)
        || js::mentions_in(&stmts[given + 1..], y) > 0;
    (!kept).then(|| (at, x.clone(), y.clone()))
}

/// Whether a `break` or a `continue` is anywhere in `stmts`.
fn jumps(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::Break(_) | StmtKind::Continue(_) => true,
        StmtKind::If(_, a, b) => jumps(a) || b.as_deref().is_some_and(jumps),
        StmtKind::While { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::Labeled(_, body) => jumps(body),
        StmtKind::Switch { cases, default, .. } => js::case_lists(cases, default).any(|body| jumps(body)),
        StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => jumps(a) || jumps(b),
        _ => false,
    })
}
