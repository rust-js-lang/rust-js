//! Tidy's shadows: a `let` that shadows a binding the program names, read
//! no more after it, is that binding (ADRs 0609, 0650).

use super::bindings::{assigns, captured};
use super::*;

/// `const stack$1 = e;`, a `let` that shadows a binding of its own block,
/// or a parameter of the body or of the `for` whose block it is, each one
/// the program names, `user`, where that binding is read no more after it:
/// `stack = e;`, the binding assigned, a `let` now, read by its name after,
/// as React's `formatOwnerStack` sets its `stack` (ADR 0650, which extends
/// 0609's parameters to every binding). Of the binding itself, `let mut
/// stack = stack.as_str();`, nothing. Not where a closure, a function or a
/// `&mut`'s handle reads or sets the old one, which may run after; and of
/// `let x$1;`, `undefined` till it's set, only where nothing names the old
/// one once it may be set (`set_after_last_read`). A loop's block is a
/// block of its own, so a `let` there never shadows what's outside it,
/// which each turn reads anew. Of a closure's body too, which is a body.
pub(super) fn shadowed(params: &[js::Pattern], body: &mut Vec<Stmt>, user: &HashSet<String>) {
    let heads: Vec<String> = params.iter().flat_map(|p| p.names()).map(str::to_string).collect();
    let mut captured = captured(body);
    shadowed_block(body, &heads, user, &mut captured);
}

/// `shadowed` of the block `stmts`, whose bindings are `heads` and what it
/// declares, then of each block in it, a loop's its own. Whether one of
/// `heads` is set now, which a `for`'s pattern then declares with `let`.
fn shadowed_block(
    stmts: &mut Vec<Stmt>,
    heads: &[String],
    user: &HashSet<String>,
    captured: &mut HashSet<String>,
) -> bool {
    let mut heads_set = false;
    let mut i = 0;
    while i < stmts.len() {
        let Some(shadowed) = shadow_of(stmts, i, heads, user, captured) else {
            i += 1;
            continue;
        };
        if shadowed.set {
            match shadowed.at {
                Some(at) => set_binding(&mut stmts[at]),
                None => heads_set = true,
            }
        }
        match shadowed.assigned {
            true => i += 1,
            false => _ = stmts.remove(i),
        }
    }
    for stmt in stmts.iter_mut() {
        match &mut stmt.kind {
            StmtKind::ForOf {
                pattern, mutable, body, ..
            } => {
                let names: Vec<String> = pattern.names().into_iter().map(str::to_string).collect();
                if shadowed_block(body, &names, user, captured) {
                    *mutable = true;
                }
            }
            StmtKind::If(_, a, b) => {
                shadowed_block(a, &[], user, captured);
                if let Some(b) = b {
                    shadowed_block(b, &[], user, captured);
                }
            }
            StmtKind::While { body, .. } | StmtKind::For { body, .. } | StmtKind::Labeled(_, body) => {
                shadowed_block(body, &[], user, captured);
            }
            StmtKind::Switch { cases, default, .. } => {
                for body in js::case_lists_mut(cases, default) {
                    shadowed_block(body, &[], user, captured);
                }
            }
            StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                shadowed_block(a, &[], user, captured);
                shadowed_block(b, &[], user, captured);
            }
            // A function's, as a closure's, are its own body's.
            StmtKind::Function(_)
            | StmtKind::Const(..)
            | StmtKind::Let(..)
            | StmtKind::Destructure { .. }
            | StmtKind::Assign(..)
            | StmtKind::Compound(..)
            | StmtKind::Expr(_)
            | StmtKind::Break(_)
            | StmtKind::Continue(_)
            | StmtKind::Directive(_)
            | StmtKind::Comments
            | StmtKind::Return(_)
            | StmtKind::Throw(_) => {}
        }
    }
    heads_set
}

/// A shadow `shadow_of` made the binding it shadows: the statement of its
/// block that declares the binding, `None` of a head; whether the binding
/// is set now; and whether the shadow's statement is its assignment, or
/// is to go.
struct Shadowed {
    at: Option<usize>,
    set: bool,
    assigned: bool,
}

/// `stmts[i]`, a `const` or `let` of a name given apart, made the binding
/// of its block it shadows, where that's read no more after it: assigned
/// the shadow's value, and each read of the shadow the binding's.
fn shadow_of(
    stmts: &mut Vec<Stmt>,
    i: usize,
    heads: &[String],
    user: &HashSet<String>,
    captured: &mut HashSet<String>,
) -> Option<Shadowed> {
    let (shadow, value, is_let) = match &stmts[i].kind {
        StmtKind::Const(shadow, value) => (shadow.clone(), Some(value), false),
        StmtKind::Let(shadow, value) => (shadow.clone(), value.as_ref(), true),
        _ => return None,
    };
    let (base, _) = crate::names::apart(&shadow)?;
    // The nearest binding before it of its name, or of one given apart
    // from it, `stack$1` of `stack$2`.
    let kin = |name: &str| name == base || crate::names::apart(name).is_some_and(|(b, _)| b == base);
    let declaring = |s: &Stmt| -> Vec<String> {
        match &s.kind {
            StmtKind::Const(name, _) | StmtKind::Let(name, _) => vec![name.clone()],
            StmtKind::Destructure { pattern, .. } => pattern.names().into_iter().map(str::to_string).collect(),
            _ => Vec::new(),
        }
    };
    let (at, binding) = match stmts[..i].iter().rposition(|s| declaring(s).iter().any(|n| kin(n))) {
        Some(at) => (Some(at), declaring(&stmts[at]).into_iter().rev().find(|n| kin(n))?),
        None => (None, heads.iter().rev().find(|n| kin(n))?.clone()),
    };
    if !user.contains(&shadow) || !user.contains(&binding) || captured.contains(&binding) {
        return None;
    }
    // `let x$1;`, `undefined` till it's set, where `x` isn't: set where a
    // value of branches is, `let level = match level { .. }`'s cases (ADR
    // 0629), it's the binding where nothing names that once it may be set,
    // its own value nothing (ADR 0650, amended).
    let Some(value) = value else {
        // Not where a branch of an `if` is the binding's own value, which
        // would leave it empty, `if (a) {} else ..`: a case's `break;` is
        // upstream's. Each name is the body's own till it's reclaimed (ADR
        // 0352), so nothing after declares the binding's.
        let own = |s: &Stmt| {
            matches!(&s.kind, StmtKind::Assign(target, value)
                if matches!(&target.kind, ExprKind::Var(v) if *v == shadow)
                    && matches!(&value.kind, ExprKind::Var(v) if *v == binding))
        };
        let (mut owns, mut in_cases) = (0, 0);
        js::each_list(&stmts[i + 1..], &mut |list| {
            owns += list.iter().filter(|s| own(s)).count();
            for s in list {
                if let StmtKind::Switch { cases, default, .. } = &s.kind {
                    in_cases += js::case_lists(cases, default).flatten().filter(|s| own(s)).count();
                }
            }
        });
        if owns != in_cases {
            return None;
        }
        set_after_last_read(&stmts[i + 1..], &shadow, &binding, false)?;
        let mut rest = stmts.split_off(i + 1);
        js::statement_lists(&mut rest, &mut |list| list.retain(|s| !own(s)));
        js::renamed(&mut rest, &shadow, &binding);
        stmts.append(&mut rest);
        if captured.contains(&shadow) {
            captured.insert(binding);
        }
        return Some(Shadowed {
            at,
            set: true,
            assigned: false,
        });
    };
    if js::mentions_in(&stmts[i + 1..], &binding) > 0 {
        return None;
    }
    // Set now: assigned the shadow's value, but its own; or by what set the
    // shadow, a `let` the program sets.
    let assigned = !matches!(&value.kind, ExprKind::Var(v) if *v == binding);
    let set = assigned || is_let;
    if assigned {
        stmts[i] = StmtKind::Assign(Expr::var(&binding), value.clone()).at(stmts[i].span);
    }
    js::each_expr_mut(&mut stmts[i + 1..], &mut |e| {
        if matches!(&e.kind, ExprKind::Var(v) if *v == shadow) {
            *e = Expr::var(&binding).or_at(e.span);
        }
    });
    if captured.contains(&shadow) {
        captured.insert(binding);
    }
    Some(Shadowed { at, set, assigned })
}

/// Whether each way through `stmts`, the shadow `shadow` set already where
/// `set` says, names `old` nowhere once `shadow` may be set: `old`'s last
/// read before the shadow's first write, as `switch (level) { case 1:
/// level$1 = level; break; default: level$1 = 3; }` reads. `None` where
/// `old` may be named after it, or a loop or a `try` that sets it is
/// followed by anything that names `old`; else whether the shadow may be
/// set where they go on past their end, and whether it may be anywhere,
/// where a `break` goes on from.
fn set_after_last_read(stmts: &[Stmt], shadow: &str, old: &str, mut set: bool) -> Option<(bool, bool)> {
    let mut any = set;
    for (k, stmt) in stmts.iter().enumerate() {
        if js::mentions_in(&stmts[k..], old) == 0 {
            let rest = set || assigns(&stmts[k..], shadow);
            return Some((rest, any || rest));
        }
        if set && js::mentions_in(std::slice::from_ref(stmt), old) > 0 {
            return None;
        }
        if !assigns(std::slice::from_ref(stmt), shadow) {
            continue;
        }
        // A branch that leaves goes on nowhere after it.
        let on = |stmts: &[Stmt], (on, _): (bool, bool)| on && !stmts.last().is_some_and(js::ends);
        set = match &stmt.kind {
            // Its value read first, then the shadow set.
            StmtKind::Assign(target, _) if matches!(&target.kind, ExprKind::Var(v) if v == shadow) => true,
            StmtKind::If(_, then, otherwise) => {
                let otherwise = otherwise.as_deref().unwrap_or_default();
                let (a, b) = (
                    set_after_last_read(then, shadow, old, set)?,
                    set_after_last_read(otherwise, shadow, old, set)?,
                );
                any |= a.1 || b.1;
                on(then, a) || on(otherwise, b)
            }
            // A case is entered from the test, or from the case before
            // where that falls through to it (ADR 0905); a `break` goes on
            // past the `switch`.
            StmtKind::Switch { cases, default, .. } => {
                let mut fallen = false;
                for case in cases {
                    let end = set_after_last_read(&case.body, shadow, old, set || fallen)?;
                    any |= end.1;
                    fallen = case.falls && on(&case.body, end);
                }
                if let Some(default) = default {
                    any |= set_after_last_read(default, shadow, old, set || fallen)?.1;
                }
                set || any
            }
            // Its `break`s go on past it, as its end does.
            StmtKind::Labeled(_, body) => {
                any |= set_after_last_read(body, shadow, old, set)?.1;
                set || any
            }
            _ => return None,
        };
        any |= set;
    }
    Some((set, any))
}

/// The declaration of a binding the program sets now: a `const`'s a `let`'s.
fn set_binding(stmt: &mut Stmt) {
    match &mut stmt.kind {
        StmtKind::Const(name, value) => {
            stmt.kind = StmtKind::Let(std::mem::take(name), Some(std::mem::replace(value, Expr::undefined())));
        }
        StmtKind::Destructure { mutable, .. } => *mutable = true,
        _ => {}
    }
}
