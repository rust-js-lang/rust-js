//! A `Drop` impl's body where its one drop is (ADR 0588).

use crate::js::{self, Expr, ExprKind, Item, Pattern, Prop, Stmt, StmtKind};
use std::collections::HashSet;

/// Of `items`, a module's, each function `drops` names, a `Drop` impl's
/// `drop`, which isn't exported, that no other module calls and that's
/// called once here, as a
/// statement: its body there, as a closure called where it's made is
/// (`js::applied_call`), and the function gone. The guard it's given, made
/// there of its fields and only read by them, is its fields:
///
/// ```js
/// const previousPriorityLevel = currentPriorityLevel;
/// try {
///   return workLoop(initialTime);
/// } finally {
///   currentPriorityLevel = previousPriorityLevel;
/// }
/// ```
pub(in crate::lower) fn inlined_drops(items: &mut Vec<Item>, drops: &HashSet<String>) {
    for name in drops {
        let Some(at) = items.iter().position(|item| {
            matches!(item, Item::Function(f) if &f.name == name && matches!(f.params.as_slice(), [Pattern::Name(_)]))
        }) else {
            continue;
        };
        let Item::Function(drop) = &items[at] else {
            unreachable!("found")
        };
        let (params, body) = (drop.params.clone(), drop.body.clone());
        let mentions: usize = (items.iter().enumerate())
            .map(|(i, item)| match i == at {
                true => js::mentions_in(&body, name),
                false => mentions_in_item(item, name),
            })
            .sum();
        if mentions != 1 {
            continue;
        }
        let mut inlined = false;
        for (i, item) in items.iter_mut().enumerate() {
            if i == at || inlined {
                continue;
            }
            each_function(item, &mut |function| {
                if inlined || js::mentions_in(&function.body, name) == 0 {
                    return;
                }
                let whole = function.body.clone();
                let mut given = None;
                js::statement_lists(&mut function.body, &mut |stmts| {
                    let Some(k) = stmts.iter().position(|s| calls(s, name)) else {
                        return;
                    };
                    if let Some(block) = js::applied_call(&stmts[k], &whole, &params, &body) {
                        given = argument(&stmts[k]);
                        stmts.splice(k..=k, block);
                        inlined = true;
                    }
                });
                if let Some(guard) = given {
                    fields_of(&mut function.body, &function.params, &guard);
                }
            });
        }
        if inlined {
            items.remove(at);
        }
    }
}

/// How often `item` names `name`.
fn mentions_in_item(item: &Item, name: &str) -> usize {
    let value = |e: &Expr| js::mentions_in(&[StmtKind::Expr(e.clone()).at(e.span)], name);
    match item {
        Item::Namespace(namespace) => {
            (namespace.constants.iter()).map(|c| value(&c.value)).sum::<usize>()
                + (namespace.methods.iter())
                    .map(|f| js::mentions_in(&f.body, name))
                    .sum::<usize>()
        }
        Item::Const(constant) => value(&constant.value),
        Item::Statements(stmts) => js::mentions_in(stmts, name),
        Item::Function(function) => js::mentions_in(&function.body, name),
    }
}

/// Each function `item` is or holds: a type's methods.
fn each_function(item: &mut Item, f: &mut dyn FnMut(&mut js::Function)) {
    match item {
        Item::Function(function) => f(function),
        Item::Namespace(namespace) => namespace.methods.iter_mut().for_each(f),
        Item::Const(_) | Item::Statements(_) => {}
    }
}

/// Whether `stmt` is `name(arg);`.
fn calls(stmt: &Stmt, name: &str) -> bool {
    matches!(&stmt.kind, StmtKind::Expr(e)
        if matches!(&e.kind, ExprKind::Call(callee, args)
            if args.len() == 1 && matches!(&callee.kind, ExprKind::Var(v) if v == name)))
}

/// The variable `f(guard);` gives.
fn argument(stmt: &Stmt) -> Option<String> {
    let StmtKind::Expr(e) = &stmt.kind else { return None };
    let ExprKind::Call(_, args) = &e.kind else { return None };
    match &args.first()?.kind {
        ExprKind::Var(v) => Some(v.clone()),
        _ => None,
    }
}

/// `const guard = { a: x, b: y };` of `body`, each of whose reads is of a
/// field, `guard.a`, none written: `const a = x; const b = y;`, read as
/// `a`, where no other name of the function is a field's.
fn fields_of(body: &mut Vec<Stmt>, params: &[Pattern], guard: &str) {
    let mut made = None;
    js::statement_lists(body, &mut |stmts| {
        if made.is_none()
            && let Some(k) = stmts
                .iter()
                .position(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == guard))
            && let StmtKind::Const(_, value) = &stmts[k].kind
            && let ExprKind::Object(props) = &value.kind
        {
            made = Some(props.clone());
        }
    });
    let Some(props) = made else { return };
    let mut fields = Vec::new();
    for prop in &props {
        match prop {
            Prop::Field(key, value) => fields.push((key.clone(), value.clone())),
            _ => return,
        }
    }
    // Every read of it a field's, and nothing written through it.
    let mut reads = 0;
    let mut stmts = body.clone();
    js::each_expr_mut(&mut stmts, &mut |e| {
        if let ExprKind::Member(of, key) = &e.kind
            && matches!(&of.kind, ExprKind::Var(v) if v == guard)
            && fields.iter().any(|(k, _)| k == key)
        {
            reads += 1;
        }
    });
    let mut written = false;
    js::each_block_mut(&mut stmts, &mut |list| {
        written |= list.iter().any(|s| {
            matches!(&s.kind, StmtKind::Assign(t, _) | StmtKind::Compound(_, t, _)
                if matches!(&t.kind, ExprKind::Member(of, _) if matches!(&of.kind, ExprKind::Var(v) if v == guard)))
        });
    });
    if written || reads != js::mentions_in(body, guard) {
        return;
    }
    let mut names = HashSet::new();
    js::names_in(body, &mut names);
    names.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
    if fields
        .iter()
        .any(|(key, _)| names.contains(key) || !crate::names::is_identifier(key))
    {
        return;
    }
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Member(of, key) = &e.kind
            && matches!(&of.kind, ExprKind::Var(v) if v == guard)
        {
            *e = Expr::var(key).or_at(e.span);
        }
    });
    js::statement_lists(body, &mut |stmts| {
        if let Some(k) = stmts
            .iter()
            .position(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == guard))
        {
            let span = stmts[k].span;
            let consts = (fields.iter()).map(|(key, value)| StmtKind::Const(key.clone(), value.clone()).at(span));
            stmts.splice(k..=k, consts.collect::<Vec<_>>());
        }
    });
}
