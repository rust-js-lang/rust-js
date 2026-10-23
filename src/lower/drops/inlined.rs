//! A `Drop` impl's body where it's dropped (ADRs 0589, 0608).

use crate::js::{self, Expr, ExprKind, Item, Pattern, Prop, Stmt, StmtKind};
use std::collections::HashSet;

/// Of `items`, a module's, each function `drops` names, a `Drop` impl's
/// `drop`, which isn't exported and that no other module calls: where each
/// of its calls is a statement whose place its body fits, a closure's too,
/// its body there, as a closure called where it's made is
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
    // Each in turn, one no other drop left calls first, so a `Drop` impl's
    // body is where its value's dropped before the drops it calls are, each
    // guard taken apart where it's made; among those, in the module's order,
    // never a set's, which would change the JS from one build to the next.
    let mut left: Vec<String> = (items.iter())
        .filter_map(|item| match item {
            Item::Function(f) if drops.contains(&f.name) => Some(f.name.clone()),
            _ => None,
        })
        .collect();
    while !left.is_empty() {
        let called = |name: &String| {
            (left.iter()).any(|other| {
                other != name
                    && (items.iter()).any(
                        |item| matches!(item, Item::Function(f) if f.name == *other && js::mentions_in(&f.body, name) > 0),
                    )
            })
        };
        let next = left.iter().position(|name| !called(name)).unwrap_or(0);
        let name = left.remove(next);
        inline_drop(items, &name);
    }
}

/// `inlined_drops`' of the drop `name`.
fn inline_drop(items: &mut Vec<Item>, name: &str) {
    let Some(at) = items.iter().position(
        |item| matches!(item, Item::Function(f) if f.name == name && matches!(f.params.as_slice(), [Pattern::Name(_)])),
    ) else {
        return;
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
    // Each drop its body, where each is a call as a statement (ADR
    // 0589): each function's body made so, then, where every mention is
    // one, set.
    let mut bodies = Vec::new();
    let mut inlined = 0;
    let mut each = true;
    for (i, item) in items.iter_mut().enumerate() {
        if i == at {
            continue;
        }
        each_function(item, &mut |function| {
            if js::mentions_in(&function.body, name) == 0 {
                return;
            }
            let mut made = function.body.clone();
            let mut given = Vec::new();
            loop {
                let whole = made.clone();
                let mut done = false;
                js::each_block_mut(&mut made, &mut |stmts| {
                    if done {
                        return;
                    }
                    let Some(k) = stmts.iter().position(|s| calls(s, name)) else {
                        return;
                    };
                    match js::applied_call(&stmts[k], &whole, &params, &body) {
                        Some(block) => {
                            given.extend(argument(&stmts[k]));
                            stmts.splice(k..=k, block);
                            inlined += 1;
                        }
                        None => each = false,
                    }
                    done = true;
                });
                if !done || !each {
                    break;
                }
            }
            for guard in given {
                fields_of(&mut made, &function.params, &guard);
                one_field_of(&mut made, &guard);
            }
            bodies.push(made);
        });
    }
    if !each || inlined == 0 || inlined != mentions {
        return;
    }
    let mut bodies = bodies.into_iter();
    for (i, item) in items.iter_mut().enumerate() {
        if i == at {
            continue;
        }
        each_function(item, &mut |function| {
            if js::mentions_in(&function.body, name) > 0 {
                function.body = bodies.next().expect("one made of each");
            }
        });
    }
    items.remove(at);
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
/// field, `guard.a`: `const a = x; const b = y;`, read as `a`, a `let` of
/// one written, `let hasMoreWork = true;`, where no other name of the
/// function is a field's.
fn fields_of(body: &mut Vec<Stmt>, params: &[Pattern], guard: &str) {
    let mut made = None;
    js::each_block_mut(body, &mut |stmts| {
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
    // Each field written through it, a `let` of its own.
    let mut written = HashSet::new();
    js::each_block_mut(&mut stmts, &mut |list| {
        for s in list.iter() {
            if let StmtKind::Assign(t, _) | StmtKind::Compound(_, t, _) = &s.kind
                && let ExprKind::Member(of, key) = &t.kind
                && matches!(&of.kind, ExprKind::Var(v) if v == guard)
            {
                written.insert(key.clone());
            }
        }
    });
    if reads != js::mentions_in(body, guard) {
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
    js::each_block_mut(body, &mut |stmts| {
        if let Some(k) = stmts
            .iter()
            .position(|s| matches!(&s.kind, StmtKind::Const(n, _) if n == guard))
        {
            let span = stmts[k].span;
            let consts = (fields.iter()).map(|(key, value)| match written.contains(key) {
                true => StmtKind::Let(key.clone(), Some(value.clone())).at(span),
                false => StmtKind::Const(key.clone(), value.clone()).at(span),
            });
            stmts.splice(k..=k, consts.collect::<Vec<_>>());
        }
    });
}

/// `const guard = [x];`, a one-field tuple struct's guard, each of whose
/// reads is `guard[0]`, none written: `const guard = x;`, read as `guard`,
/// the field it's of (ADR 0608).
fn one_field_of(body: &mut Vec<Stmt>, guard: &str) {
    let mut made = false;
    js::each_block_mut(body, &mut |stmts| {
        made |= stmts.iter().any(|s| {
            matches!(&s.kind, StmtKind::Const(n, value)
                if n == guard && matches!(&value.kind, ExprKind::Array(items) if items.len() == 1))
        });
    });
    if !made {
        return;
    }
    let field = |e: &Expr| {
        matches!(&e.kind, ExprKind::Index(of, at)
            if matches!(&of.kind, ExprKind::Var(v) if v == guard) && at.as_int() == Some(0))
    };
    let mut reads = 0;
    let mut written = false;
    let mut stmts = body.clone();
    js::each_expr_mut(&mut stmts, &mut |e| reads += usize::from(field(e)));
    js::each_block_mut(&mut stmts, &mut |list| {
        written |= list
            .iter()
            .any(|s| matches!(&s.kind, StmtKind::Assign(t, _) | StmtKind::Compound(_, t, _) if field(t)));
    });
    if written || reads != js::mentions_in(body, guard) {
        return;
    }
    // Read now, so not by Rust's name for what's only dropped, `_restore`,
    // where `restore` is free.
    let mut names = HashSet::new();
    js::names_in(body, &mut names);
    // Not a word JS keeps, `_in`'s `in`.
    let read = match guard.trim_start_matches('_') {
        free if !free.is_empty() && !names.contains(free) && !crate::names::kept_word(free) => free.to_string(),
        _ => guard.to_string(),
    };
    js::each_expr_mut(body, &mut |e| {
        if field(e) {
            *e = Expr::var(&read).or_at(e.span);
        }
    });
    js::each_block_mut(body, &mut |stmts| {
        for s in stmts.iter_mut() {
            if let StmtKind::Const(n, value) = &mut s.kind
                && n == guard
                && let ExprKind::Array(items) = &mut value.kind
                && items.len() == 1
            {
                *n = read.clone();
                *value = items.remove(0);
            }
        }
    });
}
