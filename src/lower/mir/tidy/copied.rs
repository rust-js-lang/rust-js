//! A binding the program writes of a variable, `if let Some(error) = raw`, which MIR copies: one variable, by the name that reads best (ADR 0598, amended).

use super::*;

/// `if (raw) { const error = raw; .. }`, a pattern's binding of a variable,
/// `if let Some(error) = raw`, which MIR copies where `raw` is borrowed:
/// `raw` where `error` is read, where the block sets `error` nowhere and
/// `raw` only past each read of `error`, declares neither, nor reads
/// `error` in a closure, which may run after; an `else`'s where the test
/// says `raw` isn't there, `if (newLast == null) { .. } else { const last =
/// newLast; .. }` of `match &new_last { None => .., Some(last) => .. }`
/// (ADR 0434, amended); and a `let`-`else`'s too, after the test that
/// leaves.
///
/// Of a variable rust-js named, `old` of `take()`'s value, and a binding the
/// program named, `if let Some(updates_for_lane) = hidden[i].take()`: the
/// program's name for both, `const updatesForLane = hidden[i]; ..
/// if (updatesForLane)`, where the variable is declared in `stmts` and the
/// binding's name is nowhere else in them (`user`, the program's names).
pub(super) fn copied_bindings(stmts: &mut Vec<Stmt>, user: &HashSet<String>) {
    named_for_bindings(stmts, user);
    for stmt in stmts.iter_mut() {
        let StmtKind::If(test, then, otherwise) = &mut stmt.kind else {
            continue;
        };
        // Its block where the test says the variable is there, `raw` of
        // `raw && ..`, `!!raw` or `raw != null`; or its `else` where it says
        // it isn't, `match &new_last { None => .., Some(last) => .. }`'s.
        let branches = [(Some(then), true), (otherwise.as_mut(), false)];
        for (block, there) in branches {
            let Some(block) = block else { continue };
            let Some(Stmt {
                kind: StmtKind::Const(b, value),
                ..
            }) = block.first()
            else {
                continue;
            };
            let ExprKind::Var(a) = &value.kind else {
                continue;
            };
            let tested = match there {
                true => tests_present(test, a),
                false => absent(test, a),
            };
            let (a, b) = (a.clone(), b.clone());
            if !tested || is_temporary(&a) || is_temporary(&b) || a == b {
                continue;
            }
            // The variable set again only past each read of the binding,
            // `last.next = clone; newLast = clone;`.
            let rest = &block[1..];
            if set_before_read(rest, &a, &b) || rebinds(rest, &b) || in_closure(rest, &b) {
                continue;
            }
            block.remove(0);
            js::each_expr_mut(block, &mut |e| {
                if let ExprKind::Var(v) = &mut e.kind
                    && *v == b
                {
                    *v = a.clone();
                }
            });
        }
    }
    let mut i = 0;
    while i + 1 < stmts.len() {
        if let StmtKind::If(test, then, None) = &stmts[i].kind
            && then.last().is_some_and(ends)
            && let StmtKind::Const(b, value) = &stmts[i + 1].kind
            && let ExprKind::Var(a) = &value.kind
            && absent(test, a)
            && !is_temporary(a)
            && !is_temporary(b)
            && a != b
        {
            let (a, b) = (a.clone(), b.clone());
            let rest = &stmts[i + 2..];
            if !set_before_read(rest, &a, &b) && !rebinds(rest, &b) && !in_closure(rest, &b) {
                // Of a variable rust-js named, `old` of `take()`'s value,
                // declared here, and a binding the program named: the
                // binding's name for both, as an `if let`'s is
                // (`named_for_bindings`, ADR 0598, amended).
                let own = (stmts[..i].iter())
                    .any(|s| matches!(&s.kind, StmtKind::Const(n, _) | StmtKind::Let(n, _) if *n == a));
                let named = !user.contains(a.as_str()) && user.contains(b.as_str()) && own;
                let renamed = named && js::mentions_in(stmts, &b) == js::mentions_in(&stmts[i + 1..], &b);
                stmts.remove(i + 1);
                // The variable named so where it's declared and read, or
                // the binding read as the variable after it.
                let (from, to, at) = match renamed {
                    true => (a, b, 0),
                    false => (b, a, i + 1),
                };
                for stmt in stmts[at..].iter_mut() {
                    if let StmtKind::Const(n, _) | StmtKind::Let(n, _) = &mut stmt.kind
                        && *n == from
                    {
                        *n = to.clone();
                    }
                }
                js::each_expr_mut(&mut stmts[at..], &mut |e| {
                    if let ExprKind::Var(v) = &mut e.kind
                        && *v == from
                    {
                        *v = to.clone();
                    }
                });
            }
        }
        i += 1;
    }
}

/// Whether `stmts` declare `a` again, or set it before a read of `b`, its
/// copy, on a way to it (ADR 0434, amended): `b` is then no `a`.
fn set_before_read(stmts: &[Stmt], a: &str, b: &str) -> bool {
    rebinds(stmts, a) && (declares(stmts, a) || !read_before_set(stmts, b, a, &mut false))
}

/// `const old = x; .. if (old) { const updatesForLane = old; .. }` of a
/// variable rust-js named and a binding the program did: `updatesForLane`
/// for both (`copied_bindings`).
fn named_for_bindings(stmts: &mut [Stmt], user: &HashSet<String>) {
    for i in 0..stmts.len() {
        let StmtKind::If(test, then, _) = &stmts[i].kind else {
            continue;
        };
        let Some(Stmt {
            kind: StmtKind::Const(b, value),
            ..
        }) = then.first()
        else {
            continue;
        };
        let ExprKind::Var(a) = &value.kind else {
            continue;
        };
        let declared = |name: &str, stmts: &[Stmt]| {
            let mut n = 0;
            js::each_declared(stmts, &mut |d| n += usize::from(d == name));
            n
        };
        let own = (stmts[..i].iter()).any(|s| matches!(&s.kind, StmtKind::Const(n, _) | StmtKind::Let(n, _) if n == a));
        if user.contains(a.as_str())
            || !user.contains(b.as_str())
            || is_temporary(a)
            || is_temporary(b)
            || !own
            || !tests_present(test, a)
            || declared(b, stmts) != 1
            || js::mentions_in(stmts, b) != js::mentions_in(then, b)
            || in_closure(&then[1..], b)
        {
            continue;
        }
        let (a, b) = (a.clone(), b.clone());
        if let StmtKind::If(_, then, _) = &mut stmts[i].kind {
            then.remove(0);
        }
        for stmt in stmts.iter_mut() {
            if let StmtKind::Const(n, _) | StmtKind::Let(n, _) = &mut stmt.kind
                && *n == a
            {
                *n = b.clone();
            }
        }
        js::each_expr_mut(stmts, &mut |e| {
            if let ExprKind::Var(v) = &mut e.kind
                && *v == a
            {
                *v = b.clone();
            }
        });
    }
}

/// Whether `test` says `a` is there: `a` of `a && ..`, `!!a` or `a != null`.
fn tests_present(test: &Expr, a: &str) -> bool {
    let mut first = test;
    while let ExprKind::Binary(Op::And, x, _) = &first.kind {
        first = x;
    }
    if let ExprKind::Unary(UnaryOp::Not, x) = &first.kind
        && let ExprKind::Unary(UnaryOp::Not, x) = &x.kind
    {
        first = x;
    }
    match &first.kind {
        ExprKind::Var(v) => v == a,
        _ => js::null_test(first, true).is_some_and(|v| matches!(&v.kind, ExprKind::Var(v) if v == a)),
    }
}
