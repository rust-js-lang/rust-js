//! A module's `const` a closure called where it's made makes, a
//! thread-local's initializer of statements: the module's own statements,
//! as JS writes a module's top (ADRs 0601 and 0870).

use super::reclaim;
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

/// A module's `const` a closure called where it's made makes, not
/// exported, whose body is `const`s and then the value it returns, a
/// thread-local's of a let chain's value (ADR 0870): those `const`s the
/// module's, where the closure was, and the value the `const`'s, as a
/// person writes a module's top:
///
/// ```js
/// const createTask$1 = __DEV__ ? console.createTask : undefined;
/// const createTask = createTask$1 ? (name) => createTask$1(name) : noTask;
/// ```
///
/// The same code, run once, in its order, where the module loads. Each is
/// named apart from what the module names elsewhere, `createTask$1` of the
/// `const`'s own `createTask`, as JS can't declare a name twice, and as a
/// module's top-level `const` would otherwise be what another item reads
/// of a global; and from what the closure names, so each of its names,
/// deeper ones too, given the new name, reads what it read.
pub(super) fn declared_at_top(module: &mut js::Module) {
    let mut i = 0;
    while i < module.items.len() {
        let js::Item::Const(constant) = &module.items[i] else {
            i += 1;
            continue;
        };
        let body = match &constant.value.kind {
            ExprKind::Call(callee, args) if args.is_empty() && !constant.export => match &callee.kind {
                ExprKind::Arrow(params, body) if params.is_empty() => body,
                _ => {
                    i += 1;
                    continue;
                }
            },
            _ => {
                i += 1;
                continue;
            }
        };
        let [consts @ .., last] = body.as_slice() else {
            i += 1;
            continue;
        };
        let (StmtKind::Return(Some(value)), false) = (&last.kind, consts.is_empty()) else {
            i += 1;
            continue;
        };
        let Some(names) = (consts.iter())
            .map(|s| match &s.kind {
                StmtKind::Const(name, _) => Some(name.clone()),
                _ => None,
            })
            .collect::<Option<Vec<String>>>()
        else {
            i += 1;
            continue;
        };
        let mut item = js::Item::Statements(body.clone());
        let mut named = Vec::new();
        reclaim::names_mut(&mut item, true, &mut |name| named.push(name.clone()));
        let (name, span, mutable, value) = (constant.name.clone(), constant.span, constant.mutable, value.clone());
        let consts = consts.to_vec();
        // What the module names, but in this closure.
        let own = std::mem::replace(&mut module.items[i], js::Item::Statements(Vec::new()));
        let mut taken = reclaim::mentions(module);
        module.items[i] = own;
        taken.insert(name.clone());
        // And what it names itself, but what it declares at its top: a
        // name given apart, `x$1`, which the new names are told apart from.
        taken.extend(named.into_iter().filter(|n| !names.contains(n)));
        let renames: Vec<(String, String)> = (names.iter())
            .map(|n| (n.clone(), crate::names::fresh_apart(&mut taken, n)))
            .collect();
        let renamed = |name: &mut String| {
            if let Some((_, to)) = renames.iter().find(|(from, _)| from == name) {
                name.clone_from(to);
            }
        };
        let mut made: Vec<js::Item> = (consts.into_iter())
            .map(|stmt| {
                let StmtKind::Const(mut name, value) = stmt.kind else {
                    unreachable!("matched")
                };
                renamed(&mut name);
                js::Item::Const(js::Const {
                    name,
                    value,
                    export: false,
                    mutable: false,
                    span: stmt.span,
                })
            })
            .collect();
        // The module's `const`, by its own name, of what it read.
        made.push(js::Item::Const(js::Const {
            name,
            value,
            export: false,
            mutable,
            span,
        }));
        for item in &mut made {
            reclaim::names_mut(item, true, &mut |name| renamed(name));
        }
        let n = made.len();
        module.items.splice(i..=i, made);
        i += n;
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
