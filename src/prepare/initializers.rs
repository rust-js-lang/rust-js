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
/// `export const jsxDEV = __DEV__ ? _jsxDev : undefined;`, as `then_some`'s
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
        // An exported one's value of a test, which no `let` the test
        // assigns can be (`assigned_in_place`), a conditional.
        let Some(([], value)) = valued(body, constant.export) else {
            continue;
        };
        constant.value = Expr {
            span: constant.value.span,
            ..value
        };
    }
}

/// What a closure's `body` gives, of `const`s and then one value: the
/// value it returns at its end; or, `guarded`, of one test, `if (t) {
/// return a; }`, and the value after it, `return b;`, or `undefined` where
/// it ends with nothing, `t ? a : b`, which runs `t` and then the one
/// branch, as the `if` did (ADR 0870, amended): `if __DEV__ { Some(f) }
/// else { None }`'s, as `__DEV__.then_some(f)` is. With the `const`s
/// before it.
fn valued(body: &[Stmt], guarded: bool) -> Option<(&[Stmt], Expr)> {
    let returned = |stmts: &[Stmt]| match stmts {
        [
            Stmt {
                kind: StmtKind::Return(Some(value)),
                ..
            },
        ] => Some(value.clone()),
        _ => None,
    };
    let guard = |stmt: &Stmt| match &stmt.kind {
        StmtKind::If(test, then, None) if guarded => Some((test.clone(), returned(then)?)),
        _ => None,
    };
    match body {
        [before @ .., last] if let Some(no) = returned(std::slice::from_ref(last)) => match before {
            [consts @ .., test] if let Some((test, yes)) = guard(test) => Some((consts, Expr::cond(test, yes, no))),
            _ => Some((before, no)),
        },
        [consts @ .., last] if let Some((test, yes)) = guard(last) => {
            Some((consts, Expr::cond(test, yes, Expr::undefined())))
        }
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
