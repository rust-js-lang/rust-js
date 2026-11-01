//! Where each of a module's statements and items starts and ends, in the
//! order they're printed: what a comment is attached to (ADR 0444), a block
//! of only comments ends (ADR 0753), a class's doc comment is before (ADR
//! 0761) and a function kept out of line says so (ADR 0649).

use super::*;

/// The span of each of `list`'s statements that has one: a temporary
/// rust-js made has the next's start when it's printed (`push_stmts`).
fn list_spans(list: &[js::Stmt]) -> impl Iterator<Item = (u32, u32)> + '_ {
    (list.iter())
        .filter(|stmt| !stmt.span.is_none())
        .map(|stmt| (stmt.span.lo, stmt.span.hi))
}

/// Where each of `module`'s functions marked `#[inline(never)]` starts: an
/// item's, a method's, and one written in a body, `function inner() {
/// .. }`, however deep (ADR 0649, amended).
pub(super) fn noinline_starts(module: &Module) -> std::collections::HashSet<u32> {
    let mut starts = std::collections::HashSet::new();
    let mut note = |f: &js::Function| {
        if f.noinline && !f.span.is_none() {
            starts.insert(f.span.lo);
        }
    };
    for item in module.items.iter() {
        match item {
            js::Item::Function(f) => note(f),
            js::Item::Namespace(n) => n.methods.iter().for_each(&mut note),
            js::Item::Statements(_) | js::Item::Const(_) => {}
        }
    }
    for mut body in bodies(module) {
        js::each_block_mut(&mut body, &mut |stmts| {
            for stmt in stmts.iter() {
                if let StmtKind::Function(f) = &stmt.kind {
                    note(f);
                }
            }
        });
    }
    starts
}

/// Each body of `module`'s statements: its functions', its methods', its
/// statements', and of a constant, its value's, as a statement.
/// Where each statement starts that a blank line is before, as it's set
/// apart: a function declared away from where it's written, and what's
/// after it but another such function (`Function::set_apart`, ADR 1043).
pub(super) fn set_apart_starts(module: &Module) -> std::collections::HashSet<u32> {
    let mut starts = std::collections::HashSet::new();
    for mut body in bodies(module) {
        js::each_block_mut(&mut body, &mut |stmts| {
            let apart = |stmt: &js::Stmt| matches!(&stmt.kind, StmtKind::Function(f) if f.set_apart);
            for (i, stmt) in stmts.iter().enumerate() {
                if apart(stmt) && !stmt.span.is_none() {
                    starts.insert(stmt.span.lo);
                }
                // What's after the last, where it's printed: a temporary
                // rust-js made is at the next statement's start.
                if apart(stmt)
                    && let Some(next) = stmts[i + 1..]
                        .iter()
                        .take_while(|s| !apart(s))
                        .find(|s| !s.span.is_none())
                {
                    starts.insert(next.span.lo);
                }
            }
        });
    }
    starts
}

pub(super) fn bodies(module: &Module) -> Vec<Vec<js::Stmt>> {
    (module.items.iter())
        .flat_map(|item| match item {
            js::Item::Function(f) => vec![f.body.clone()],
            js::Item::Namespace(n) => n.methods.iter().map(|method| method.body.clone()).collect(),
            js::Item::Statements(stmts) => vec![stmts.clone()],
            js::Item::Const(c) => vec![vec![StmtKind::Expr(c.value.clone()).at(js::Span::NONE)]],
        })
        .collect()
}

/// The span of each statement and item of `module`, closures' and nested
/// functions' too, each list's in the order it's printed: what a comment is
/// attached to (ADR 0444); and where each block of only comments ends,
/// `StmtKind::Comments`, whose comments oxc prints there (ADR 0753).
pub(super) fn statement_spans(module: &Module) -> (Vec<(u32, u32)>, std::collections::HashSet<u32>) {
    type Found = (Vec<(u32, u32)>, std::collections::HashSet<u32>);
    fn list(list: &[js::Stmt], (starts, ends): &mut Found) {
        starts.extend(list_spans(list));
        let comments = list
            .iter()
            .filter(|s| matches!(s.kind, StmtKind::Comments) && !s.span.is_none());
        ends.extend(comments.map(|s| s.span.lo));
    }
    fn of_stmts(stmts: &[js::Stmt], starts: &mut Found) {
        let mut copy = stmts.to_vec();
        let mut functions = Vec::new();
        // A function declared in it, its body's own, after: taken out, so
        // its closures' statements aren't before its own (ADR 0882,
        // amended).
        js::statement_lists(&mut copy, &mut |stmts| {
            list(stmts, starts);
            for stmt in stmts.iter_mut() {
                if let StmtKind::Function(f) = &mut stmt.kind {
                    functions.push(std::mem::take(&mut f.body));
                }
            }
        });
        // An object's methods, each printed with its statements, its value's
        // `return` too, `maySuspendCommit(type) { return ..; }` (rd296).
        let mut methods = Vec::new();
        js::each_expr_mut(&mut copy, &mut |e| match &mut e.kind {
            ExprKind::Object(props, _) => methods.extend(props.iter().filter_map(|prop| match prop {
                js::Prop::Method(_, value) => Some(value.span).filter(|s| !s.is_none()),
                _ => None,
            })),
            // One whose body is its value, `() => f()`, has no statement:
            // what's before the value is its arm's (`arm_starts`).
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body)
                if expressions::arrow_value(body).is_some() && !methods.contains(&e.span) => {}
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => {
                js::statement_lists(body, &mut |stmts| list(stmts, starts));
            }
            ExprKind::Function(f) => functions.push(f.body.clone()),
            _ => {}
        });
        for body in functions {
            of_stmts(&body, starts);
        }
    }
    let mut starts: Found = (Vec::new(), Default::default());
    // A `pub use`'s re-export, which its comments are before.
    for reexport in &module.reexports {
        if !reexport.span.is_none() {
            starts.0.push((reexport.span.lo, reexport.span.hi));
        }
    }
    let function = |f: &js::Function, starts: &mut Found| {
        if !f.span.is_none() {
            starts.0.push((f.span.lo, f.span.hi));
        }
        of_stmts(&f.body, starts);
    };
    for item in &module.items {
        match item {
            js::Item::Const(c) => {
                if !c.span.is_none() {
                    starts.0.push((c.span.lo, c.span.hi));
                }
                of_stmts(&[StmtKind::Expr(c.value.clone()).at(js::Span::NONE)], &mut starts);
            }
            js::Item::Function(f) => function(f, &mut starts),
            js::Item::Namespace(n) => {
                for method in &n.methods {
                    function(method, &mut starts);
                }
                // A class's struct, whose comments are its constructor's.
                if let Some((declared, _)) = declared_class(n) {
                    starts.0.push((declared.lo, declared.hi));
                }
            }
            js::Item::Statements(stmts) => of_stmts(stmts, &mut starts),
        }
    }
    starts
}

/// A class's struct, where it's declared, and its constructor function,
/// which the comments before the struct, its `///` doc, are before (ADR
/// 0761): `/** Base class helpers .. */ function Component(..)`.
pub(super) fn declared_class(n: &js::Namespace) -> Option<(js::Span, js::Span)> {
    let class = n.class.as_ref().filter(|class| !class.declared.is_none())?;
    // An ES class, which is where its struct is (rd331).
    if class.extends.is_some() {
        return Some((class.declared, class.declared));
    }
    let constructor = n.methods.iter().find(|f| f.name == class.constructor)?;
    Some((class.declared, constructor.span))
}
