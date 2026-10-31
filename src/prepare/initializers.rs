//! A module's `const` a closure called where it's made makes, a
//! thread-local's initializer of statements: the module's own statements,
//! as JS writes a module's top (ADRs 0601 and 0870).

use crate::js::{self, Expr, ExprKind, Stmt, StmtKind};

/// A module's `const` a closure called where it's made makes, a
/// thread-local's an `if` gives, not exported: `let name;` and the
/// closure's body, each `return v` at its ends `name = v`, an early one's
/// `if` the rest's `else`, as JS writes it (ADR 0601); a `return
/// undefined` nothing, as `let name;` is that already (ADR 0704):
///
/// ```js
/// let schedule;
/// if (typeof setImmediate !== "undefined") {
///   schedule = () => { .. };
/// } else {
///   ..
/// }
/// ```
///
/// Where a `return` is anywhere else, a loop's, or the body declares a
/// name at its top, which would be the module's, it stays the call; but
/// one of `const`s and then its value is the module's (`declared_at_top`).
pub(super) fn assigned_in_place(items: &mut [js::Item]) {
    for item in items {
        let js::Item::Const(constant) = item else { continue };
        if constant.export {
            continue;
        }
        let ExprKind::Call(callee, args) = &constant.value.kind else {
            continue;
        };
        let ExprKind::Arrow(params, body) = &callee.kind else {
            continue;
        };
        if !params.is_empty() || !args.is_empty() {
            continue;
        }
        let Some(assigned) = assigned(body, &constant.name) else {
            continue;
        };
        let declares = |s: &Stmt| {
            matches!(
                s.kind,
                StmtKind::Const(..) | StmtKind::Let(..) | StmtKind::Destructure { .. } | StmtKind::Function(_)
            )
        };
        if assigned.iter().any(declares) {
            continue;
        }
        let span = constant.span;
        let mut stmts = vec![StmtKind::Let(constant.name.clone(), None).at(span)];
        stmts.extend(assigned);
        *item = js::Item::Statements(stmts);
    }
}

/// A module's `const` a closure called where it's made makes, whose body
/// is its value alone, of a test too, `if (t) { return a; }`: that value,
/// `const jsxDEV = __DEV__ ? _jsxDev : undefined;`, as `then_some`'s
/// is (ADR 0870, amended). One of `const`s before its value stays the
/// closure, its names its own, the item's comment above it (#407):
///
/// ```js
/// // Scaled where the module loads.
/// const scale = (() => {
///   const k = flag("scale", true) ? lookup("scale", 3) : undefined;
///   return k != null ? (x) => Math.imul(x, k) >>> 0 : (x) => x;
/// })();
/// ```
pub(super) fn declared_at_top(module: &mut js::Module) {
    for item in &mut module.items {
        let js::Item::Const(constant) = item else { continue };
        let ExprKind::Call(callee, args) = &constant.value.kind else {
            continue;
        };
        let ExprKind::Arrow(params, body) = &callee.kind else {
            continue;
        };
        if !params.is_empty() || !args.is_empty() {
            continue;
        }
        // One of an `if` chain, one nested conditional.
        let value = match valued(body) {
            Some(([], value)) => value,
            _ => match chained(body) {
                Some(value) => value,
                None => continue,
            },
        };
        constant.value = Expr {
            span: constant.value.span,
            ..value
        };
    }
}

/// What a closure's `body` gives, of `const`s and then one value: the
/// value it returns at its end, with the `const`s before it. One of a
/// test, `if (t) { return a; }`, is `chained`'s: `t ? a : undefined`, as
/// `__DEV__.then_some(f)` is (ADR 0870, amended).
fn valued(body: &[Stmt]) -> Option<(&[Stmt], Expr)> {
    match body {
        [
            before @ ..,
            Stmt {
                kind: StmtKind::Return(Some(value)),
                ..
            },
        ] => Some((before, value.clone())),
        _ => None,
    }
}

/// What a closure's `body` of one `if` chain gives, each arm a `return` of
/// its value, whatever that is, an arrow, a call or a chain of its own: one
/// nested conditional, `t ? a : b ? c : d`, which runs each test in turn
/// and then the one arm, as the chain did, as React DOM's scheduleMicrotask
/// is written (rd234, the user's decision; ADR 0969, amended), a guard of a
/// test MIR turned, `!t`, its arms the other way round. An arm of `None`,
/// or a chain that ends with nothing, is `undefined` there, exported or
/// not, as React DOM's `const localPromise = typeof Promise === "function"
/// ? Promise : undefined;` (rd247). Not where an arm does more than give
/// its value, which `assigned_in_place` runs as the module loads.
fn chained(body: &[Stmt]) -> Option<Expr> {
    // The value of `list`, whose end, where `end`, is the body's, where
    // its function gives `undefined`; and otherwise falls through to what
    // follows, which no arm may.
    fn value(list: &[Stmt], end: bool) -> Option<Expr> {
        let cond = |t: &Expr, yes: Expr, no: Expr, at: js::Span| Expr::cond(t.clone(), yes, no).or_at(at);
        match list {
            [] => end.then(Expr::undefined),
            [
                Stmt {
                    kind: StmtKind::Return(Some(v)),
                    ..
                },
            ] => Some(v.clone()),
            [
                Stmt {
                    kind: StmtKind::If(t, then, Some(otherwise)),
                    span,
                },
            ] => Some(cond(t, value(then, end)?, value(otherwise, end)?, *span)),
            [
                Stmt {
                    kind: StmtKind::If(t, then, None),
                    span,
                },
                rest @ ..,
            ] => {
                let (yes, no) = (value(then, false)?, value(rest, end)?);
                // A guard MIR turned, `if (!t) { return a; } return b;`,
                // as the program wrote it, `t ? b : a`.
                Some(match &t.kind {
                    ExprKind::Unary(js::UnaryOp::Not, inner) => cond(inner, no, yes, *span),
                    _ => cond(t, yes, no, *span),
                })
            }
            _ => None,
        }
    }
    match body.first()?.kind {
        StmtKind::If(..) => value(body, true),
        _ => None,
    }
}

/// `stmts`, which end in a `return`, each `return v` at their ends
/// `name = v`; `None` where one is elsewhere.
fn assigned(stmts: &[Stmt], name: &str) -> Option<Vec<Stmt>> {
    let mut out = Vec::new();
    for (i, stmt) in stmts.iter().enumerate() {
        let last = i + 1 == stmts.len();
        match &stmt.kind {
            StmtKind::Return(Some(value)) if last => {
                if !matches!(value.kind, ExprKind::Undefined) {
                    out.push(StmtKind::Assign(Expr::var(name), value.clone()).at(stmt.span));
                }
            }
            StmtKind::If(test, then, otherwise) if js::returns(std::slice::from_ref(stmt)).0 => {
                let then = assigned(then, name)?;
                let otherwise = match otherwise {
                    Some(otherwise) if last => assigned(otherwise, name)?,
                    // An early `return`'s: what follows is its `else`.
                    None if !last => assigned(&stmts[i + 1..], name)?,
                    // The last, of no `else`: the function falls off its end
                    // otherwise, `undefined`, what the `let` starts as (ADR
                    // 0237, amended).
                    None => Vec::new(),
                    _ => return None,
                };
                // Of a branch left with nothing to do, the other's test.
                out.push(
                    match (then.is_empty(), otherwise.is_empty()) {
                        (_, true) => StmtKind::If(test.clone(), then, None),
                        (true, false) => StmtKind::If(test.clone().negated_test(), otherwise, None),
                        (false, false) => StmtKind::If(test.clone(), then, Some(otherwise)),
                    }
                    .at(stmt.span),
                );
                return Some(out);
            }
            // A `return` a loop's or a `try`'s, or no `return` at the end.
            _ if last || js::returns(std::slice::from_ref(stmt)).0 => return None,
            _ => out.push(stmt.clone()),
        }
    }
    Some(out)
}
