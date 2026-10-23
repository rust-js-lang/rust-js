//! Drop flags, `noisy$live` (`mir/drops.rs`): each says whether a value is
//! still there to drop, set as it's moved, read by the drop a panic or the
//! end of a scope makes. MIR's cleanup is shared by every call that may
//! panic, so its drop tests the flag even where the flag is known: just set
//! `false`, as the value was moved into the call. Where it's known, the
//! test goes, a `try` whose `catch` then only throws on is no `try`, and a
//! flag nothing reads goes with its writes. A temporary's flag is named for
//! the variable its drop is of (ADR 0372).

use std::collections::{HashMap, HashSet};

use crate::js::{self, Expr, ExprKind, Stmt, StmtKind};

/// What's known of each flag where a statement runs: `true` or `false`.
type Known = HashMap<String, bool>;

/// `body` with what's known of its `flags` settled.
pub(in crate::lower::mir) fn settled_flags(body: &mut Vec<Stmt>, flags: &HashSet<String>) {
    if flags.is_empty() {
        return;
    }
    settle(body, &mut Known::new(), flags);
    for flag in flags {
        if !read(body, flag) {
            js::statement_lists(body, &mut |list| {
                list.retain(|stmt| match &stmt.kind {
                    StmtKind::Let(name, _) => name != flag,
                    StmtKind::Assign(target, _) => flag_of(target, flags) != Some(flag.as_str()),
                    _ => true,
                })
            });
        }
    }
}

/// Each flag a temporary's, `_14$live`, named for the variable the drop it
/// guards is of, `value$live`: after the temporaries are named.
pub(super) fn named_flags(body: &mut Vec<Stmt>, flags: &HashSet<String>) {
    let mut taken = HashSet::new();
    js::names_in(body, &mut taken);
    // In the order they're declared, so each build names them alike.
    let mut declared = Vec::new();
    js::statement_lists(body, &mut |list| {
        for stmt in list.iter() {
            if let StmtKind::Let(name, _) = &stmt.kind
                && flags.contains(name)
            {
                declared.push(name.clone());
            }
        }
    });
    for flag in &declared {
        let Some((base, rest)) = flag.split_once('$') else {
            continue;
        };
        if !super::is_temporary(base) {
            continue;
        }
        let Some(dropped) = guarded(body, flag) else {
            continue;
        };
        let named = dropped.split('$').next().unwrap_or(&dropped);
        let to = crate::names::fresh_in(&mut taken, &format!("{named}${rest}"));
        js::statement_lists(body, &mut |list| {
            for stmt in list.iter_mut() {
                if let StmtKind::Let(name, _) = &mut stmt.kind
                    && name == flag
                {
                    name.clone_from(&to);
                }
            }
        });
        js::each_expr_mut(body, &mut |e| {
            if let ExprKind::Var(v) = &mut e.kind
                && v == flag
            {
                v.clone_from(&to);
            }
        });
    }
}

/// The variable the drop `flag` guards is of: `x` of `if (flag) {
/// drop(x); }`, and of a part of it, `drop(x.a)`: what the drop reads
/// last, but the runtime's helpers.
fn guarded(body: &mut [Stmt], flag: &str) -> Option<String> {
    let mut found = None;
    let mut copy = body.to_vec();
    js::statement_lists(&mut copy, &mut |list| {
        for stmt in list.iter() {
            if found.is_none()
                && let StmtKind::If(test, then, _) = &stmt.kind
                && tests_flag(test, flag)
            {
                js::visit_stmts(then, &mut |name| {
                    if !name.starts_with('$') {
                        found = Some(name.to_string());
                    }
                });
            }
        }
    });
    found
}

/// Whether `test` is `flag`, or a part of an `&&` of it, `d && _13$0$live`,
/// a pattern's test joined with it (ADR 0614).
fn tests_flag(test: &Expr, flag: &str) -> bool {
    match &test.kind {
        ExprKind::Var(v) => v == flag,
        ExprKind::Binary(js::Op::And, a, b) => tests_flag(a, flag) || tests_flag(b, flag),
        _ => false,
    }
}

fn flag_of<'e>(e: &'e Expr, flags: &HashSet<String>) -> Option<&'e str> {
    match &e.kind {
        ExprKind::Var(v) if flags.contains(v) => Some(v),
        _ => None,
    }
}

/// Whether `flag` is read anywhere in `body`, not only written.
fn read(body: &[Stmt], flag: &str) -> bool {
    let mut mentions = 0;
    js::visit_stmts(body, &mut |name| mentions += usize::from(name == flag));
    let mut written = 0;
    js::each_list(body, &mut |list| {
        for stmt in list.iter() {
            if let StmtKind::Assign(target, _) = &stmt.kind
                && matches!(&target.kind, ExprKind::Var(v) if v == flag)
            {
                written += 1;
            }
        }
    });
    mentions > written
}

/// Each flag `stmts` assign, anywhere in them, and what to: `None` of one
/// assigned both, or what isn't `true` or `false`.
fn assigned(stmts: &[Stmt], flags: &HashSet<String>) -> HashMap<String, Option<bool>> {
    let mut found: HashMap<String, Option<bool>> = HashMap::new();
    js::each_list(stmts, &mut |list| {
        for stmt in list.iter() {
            if let StmtKind::Assign(target, value) = &stmt.kind
                && let Some(flag) = flag_of(target, flags)
            {
                let value = match value.kind {
                    ExprKind::Bool(b) => Some(b),
                    _ => None,
                };
                found
                    .entry(flag.to_string())
                    .and_modify(|was| {
                        if *was != value {
                            *was = None;
                        }
                    })
                    .or_insert(value);
            }
        }
    });
    found
}

/// `known` with each flag `stmts` assign forgotten: what's known after them,
/// where they may or may not have run.
fn forget(known: &mut Known, stmts: &[Stmt], flags: &HashSet<String>) {
    for flag in assigned(stmts, flags).keys() {
        known.remove(flag);
    }
}

/// Whether a panic, `new Error(..)`, is made of strings and variables
/// alone: what a drop, run before it, can't change.
fn made_of_values(panic: &Expr) -> bool {
    let ExprKind::New(class, args) = &panic.kind else {
        return false;
    };
    matches!(&class.kind, ExprKind::Var(name) if name == "Error")
        && args.iter().all(|arg| match &arg.kind {
            ExprKind::Str(_) => true,
            ExprKind::Template(_, parts, _) => parts.iter().all(|part| matches!(part.kind, ExprKind::Var(_))),
            _ => false,
        })
}

/// `true` or `false`, of a literal.
fn bool_of(value: &Expr) -> Option<bool> {
    match value.kind {
        ExprKind::Bool(b) => Some(b),
        _ => None,
    }
}

/// `stmts`, run where `known` is known, settled: `known` is then what's
/// known after them.
fn settle(stmts: &mut Vec<Stmt>, known: &mut Known, flags: &HashSet<String>) {
    let mut i = 0;
    while i < stmts.len() {
        // What a flag is set to: one set again to what it is goes.
        let set = match &stmts[i].kind {
            StmtKind::Let(name, value) if flags.contains(name) => {
                Some((name.clone(), value.as_ref().and_then(bool_of), false))
            }
            StmtKind::Assign(target, value) if let Some(flag) = flag_of(target, flags) => {
                Some((flag.to_string(), bool_of(value), true))
            }
            _ => None,
        };
        if let Some((flag, value, again)) = set {
            match value {
                Some(b) if again && known.get(&flag) == Some(&b) => {
                    stmts.remove(i);
                    continue;
                }
                Some(b) => {
                    known.insert(flag, b);
                }
                None => {
                    known.remove(&flag);
                }
            }
            i += 1;
            continue;
        }
        // A test of a known flag: the branch it takes, in its place.
        let tested = match &stmts[i].kind {
            StmtKind::If(test, ..) => flag_of(test, flags).and_then(|flag| known.get(flag).copied()),
            _ => None,
        };
        if let Some(holds) = tested
            && let StmtKind::If(_, then, els) = &mut stmts[i].kind
        {
            let taken = match holds {
                true => std::mem::take(then),
                false => els.take().unwrap_or_default(),
            };
            stmts.splice(i..=i, taken);
            continue;
        }
        match &mut stmts[i].kind {
            StmtKind::TryCatch(inner, error, handler) => {
                let before = known.clone();
                settle(inner, known, flags);
                // Where it's caught, what was known before, of each flag
                // what it runs sets to nothing else.
                let written = assigned(inner, flags);
                let mut caught: Known = before
                    .into_iter()
                    .filter(|(flag, b)| written.get(flag).is_none_or(|to| *to == Some(*b)))
                    .collect();
                settle(handler, &mut caught, flags);
                let rethrows = matches!(
                    (error.as_deref(), handler.as_slice()),
                    (Some(error), [Stmt { kind: StmtKind::Throw(Expr { kind: ExprKind::Var(thrown), .. }), .. }])
                        if thrown == error
                );
                if rethrows {
                    let inner = std::mem::take(inner);
                    let count = inner.len();
                    stmts.splice(i..=i, inner);
                    i += count;
                    continue;
                }
                // A panic's `try` of its own `throw`: what the `catch` drops,
                // then the panic, its message made first, as Rust makes it,
                // of what a drop can't change.
                if let [
                    Stmt {
                        kind: StmtKind::Throw(panic),
                        span,
                    },
                ] = inner.as_slice()
                    && let Some(error) = error.as_deref()
                    && let Some((last, dropping)) = handler.split_last()
                    && matches!(&last.kind, StmtKind::Throw(Expr { kind: ExprKind::Var(thrown), .. }) if thrown == error)
                    && js::mentions_in(dropping, error) == 0
                    && made_of_values(panic)
                {
                    let mut unwound = dropping.to_vec();
                    unwound.push(StmtKind::Throw(panic.clone()).at(*span));
                    let count = unwound.len();
                    stmts.splice(i..=i, unwound);
                    i += count;
                    continue;
                }
                // A `catch` that goes on, a writer's: what either ends with.
                if !matches!(handler.last().map(|s| &s.kind), Some(StmtKind::Throw(_))) {
                    known.retain(|flag, b| caught.get(flag) == Some(b));
                }
            }
            StmtKind::If(_, then, els) => {
                settle(then, &mut known.clone(), flags);
                if let Some(els) = els {
                    settle(els, &mut known.clone(), flags);
                }
                // A variant's test whose drops all settled away, of a field
                // moved out, is emptied: `branches::emptied_branch` writes
                // what's left, as the branch passes ask it.
                forget(known, &stmts[i..=i], flags);
            }
            StmtKind::Labeled(_, body) => {
                settle(body, &mut known.clone(), flags);
                forget(known, &stmts[i..=i], flags);
            }
            StmtKind::Switch { cases, default, .. } => {
                for body in js::case_lists_mut(cases, default) {
                    settle(body, &mut known.clone(), flags);
                }
                forget(known, &stmts[i..=i], flags);
            }
            StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => {
                // Each time round, what an earlier time set may be there.
                let mut entry = known.clone();
                forget(&mut entry, body, flags);
                settle(body, &mut entry, flags);
                forget(known, &stmts[i..=i], flags);
            }
            StmtKind::Try(body, finally) => {
                settle(body, &mut known.clone(), flags);
                let mut entry = known.clone();
                forget(&mut entry, body, flags);
                settle(finally, &mut entry, flags);
                forget(known, &stmts[i..=i], flags);
            }
            _ => {}
        }
        i += 1;
    }
}
