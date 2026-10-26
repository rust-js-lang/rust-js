//! Readability preparation on the completed JS tree. No rustc types or APIs.
//! Nothing moves: what JSX holds stays where it's written, a list's callback
//! of several statements and a handler's too, as a person writes them, and
//! oxfmt lays them out where they are (ADR 0218). The printer only lays out
//! the result.

use std::collections::HashSet;

use crate::js::{self, Expr, ExprKind, JsxTag, Op, Prop, Stmt, StmtKind, UnaryOp};

mod catches;
mod initializers;
mod reclaim;
mod truths;
mod undefined_keys;

/// A closure's parameter given its name back, preparation's, which a body
/// lowered from its MIR asks before it's linked, as an import's alias is
/// chosen around it.
mod scopes;

pub fn module(module: &mut js::Module) {
    initializers::declared_at_top(module);
    initializers::assigned_in_place(&mut module.items);
    forwarding_functions(module);
    for function in module.items.iter_mut().flat_map(js::Item::functions_mut) {
        prepared(function);
    }
    for item in &mut module.items {
        match item {
            js::Item::Const(constant) => expr(&mut constant.value),
            js::Item::Statements(stmts) => block(stmts),
            js::Item::Namespace(_) | js::Item::Function(_) => {}
        }
    }
    js::each_item_slotted_block_mut(&mut module.items, &mut |stmts, slot| unelsed(stmts, slot));
    js::each_item_block_mut(&mut module.items, &mut tested_assignments);
    reclaim::module(module);
    compound(module);
    read_together(module);
}

/// `function getTime(n) { return GET_TIME(n); }`, a function that only
/// calls a module variable's function with its parameters, a thread-local's
/// of a closure: that variable, by the function's name, `let getTime; if
/// (..) { getTime = (n) => .. }`, exported where the function was (ADR
/// 0610). Not where the module's load calls the function, which a
/// declaration's hoisting lets run before the variable is set.
fn forwarding_functions(module: &mut js::Module) {
    let mut i = 0;
    while i < module.items.len() {
        // One kept out of line, `#[inline(never)]`, stays itself (ADR 0649).
        let js::Item::Function(function) = &module.items[i] else {
            i += 1;
            continue;
        };
        if function.noinline {
            i += 1;
            continue;
        }
        let names: Option<Vec<&str>> = (function.params.iter())
            .map(|p| match p {
                js::Pattern::Name(name) => Some(name.as_str()),
                _ => None,
            })
            .collect();
        // Its call's value, `return GET_TIME(n);`, or none, `QUEUE(callback);`
        // of a function of no value.
        let (call, returns) = match function.body.as_slice() {
            [
                Stmt {
                    kind: StmtKind::Return(Some(call)),
                    ..
                },
            ] => (Some(call), true),
            [
                Stmt {
                    kind: StmtKind::Expr(call),
                    ..
                },
            ] => (Some(call), false),
            _ => (None, false),
        };
        let arity = names.as_ref().map_or(0, Vec::len);
        let variable = match (names, call) {
            (Some(names), Some(call)) => match &call.kind {
                ExprKind::Call(callee, args)
                    if args.len() == names.len()
                        && (args.iter().zip(&names)).all(|(a, n)| matches!(&a.kind, ExprKind::Var(v) if v == n)) =>
                {
                    match &callee.kind {
                        ExprKind::Var(v) if *v != function.name && !names.contains(&v.as_str()) => Some(v.clone()),
                        _ => None,
                    }
                }
                _ => None,
            },
            _ => None,
        };
        let (name, export) = (function.name.clone(), function.export);
        // Its variable's declaration: a `const`, or a `let` set in place.
        let declared = variable.as_ref().and_then(|v| {
            module.items.iter().position(|item| match item {
                js::Item::Const(constant) => constant.name == *v && !constant.export,
                js::Item::Statements(stmts) => {
                    matches!(stmts.first().map(|s| &s.kind), Some(StmtKind::Let(n, None)) if n == v)
                }
                _ => false,
            })
        });
        // Called where the module loads, by a constant's value or its code.
        let mut loaded = Vec::new();
        for item in &module.items {
            match item {
                js::Item::Const(constant) => loaded.push(StmtKind::Expr(constant.value.clone()).at(constant.span)),
                js::Item::Statements(stmts) => loaded.extend(stmts.iter().cloned()),
                _ => {}
            }
        }
        let (Some(variable), Some(at)) = (variable, declared) else {
            i += 1;
            continue;
        };
        // Each value it's given an arrow of Rust's, which takes no more than
        // its parameters, as the function did: not a JS function, which a
        // caller would give what it's given past them.
        fn arrows(e: &Expr) -> bool {
            match &e.kind {
                ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) => true,
                ExprKind::Cond(_, a, b) => arrows(a) && arrows(b),
                _ => false,
            }
        }
        let mut given = Vec::new();
        match &module.items[at] {
            js::Item::Const(constant) => given.push(constant.value.clone()),
            js::Item::Statements(stmts) => {
                let mut stmts = stmts.clone();
                js::each_block_mut(&mut stmts, &mut |list| {
                    for s in list.iter() {
                        if let StmtKind::Assign(target, value) = &s.kind
                            && matches!(&target.kind, ExprKind::Var(v) if *v == variable)
                        {
                            given.push(value.clone());
                        }
                    }
                });
            }
            _ => {}
        }
        // Or only called, by the crate's own code, by its parameters: a
        // function of a value gives the variable's, and one of none gives
        // `undefined`, which no caller reads, a `()` being `undefined`
        // where it's read; and none gives it more than its parameters, a
        // JS function's too. React's ReactAct has `const
        // queueSeveralMicrotasks = .. ? (callback) => { .. } :
        // queueMacrotask;` (ADR 0610, amended).
        let called_only = !export
            && !module.aliases.iter().any(|(local, _)| *local == name)
            && only_called(&module.items, i, &name, arity);
        if given.is_empty() || !(returns && given.iter().all(arrows) || called_only) {
            i += 1;
            continue;
        }
        if js::mentions_in(&loaded, &name) > 0 || module.default_export.as_ref() == Some(&name) {
            i += 1;
            continue;
        }
        module.items.remove(i);
        let at = if at > i { at - 1 } else { at };
        let rename = |n: &mut String| {
            if *n == variable {
                n.clone_from(&name);
            }
        };
        js::each_item_expr_mut(&mut module.items, &mut |e| {
            if let ExprKind::Var(v) = &mut e.kind {
                rename(v);
            }
        });
        match &mut module.items[at] {
            js::Item::Const(constant) => {
                rename(&mut constant.name);
                constant.export = export;
            }
            js::Item::Statements(stmts) => {
                if let Some(StmtKind::Let(n, None)) = stmts.first_mut().map(|s| &mut s.kind) {
                    rename(n);
                }
                if export {
                    module.aliases.push((name.clone(), name.clone()));
                }
            }
            _ => unreachable!("found"),
        }
    }
}

/// Whether each read of `name` in `items` but the function at `at` is a
/// call of it by `arity` arguments.
fn only_called(items: &[js::Item], at: usize, name: &str, arity: usize) -> bool {
    let (mut reads, mut calls) = (0, 0);
    let mut each = |e: &Expr| {
        calls += usize::from(matches!(&e.kind, ExprKind::Call(callee, args)
            if args.len() == arity && matches!(&callee.kind, ExprKind::Var(v) if v == name)));
    };
    for (j, item) in items.iter().enumerate() {
        if j == at {
            continue;
        }
        item.visit_vars(&mut |v| reads += usize::from(v == name));
        match item {
            js::Item::Function(function) => js::each_expr(&function.body, &mut each),
            js::Item::Statements(stmts) => js::each_expr(stmts, &mut each),
            js::Item::Const(constant) => constant.value.each(&mut each),
            js::Item::Namespace(namespace) => {
                namespace.constants.iter().for_each(|c| c.value.each(&mut each));
                namespace.methods.iter().for_each(|m| js::each_expr(&m.body, &mut each));
            }
        }
    }
    reads == calls
}

/// An `if` whose branch never goes on, `js::ends`', by a `return`, a
/// `throw`, a `break` or a `continue`, has no `else`: what was in it
/// follows, as JS writes it, `if (c) { return a; } return b;` (ADR 0237). Before its
/// locals are named, which then see the block they're in. An `else if` of
/// an arm that goes on stays one, `if (a) { x = 1; } else if (b) { return;
/// } else { .. }`: what's in its `else` would be the outer `else`'s, the
/// chain broken, as ESLint's `no-else-return` allows it (ADR 0237,
/// amended). Which `else` that is the walk says, `slot`: an `else` whose
/// one statement is an `if`, a link of a chain.
fn unelsed(stmts: &mut Vec<Stmt>, slot: js::Slot) {
    let link = slot == js::Slot::Else
        && matches!(
            stmts.as_slice(),
            [Stmt {
                kind: StmtKind::If(..),
                ..
            }]
        );
    let mut i = 0;
    while i < stmts.len() {
        if let StmtKind::If(_, then, els @ Some(_)) = &mut stmts[i].kind
            && !link
            && then.last().is_some_and(js::ends)
        {
            let els = els.take().expect("matched");
            stmts.splice(i + 1..i + 1, els);
        }
        i += 1;
    }
}

/// `while (true) { const step = iterator.next(); if (step.done) { break; }
/// .. }`, a loop whose first statement makes what the test that leaves it
/// then reads, once and first: the loop of that test, which makes it, `let
/// step; while (!(step = iterator.next()).done) { .. }`, as JS writes a
/// walk of an iterator's steps, React's `mapIntoArray`, or of a regular
/// expression's matches (ADR 0860). Before its locals are named, so the
/// `let` is one name of the block it's in. Not where a closure reads it,
/// which a `const` gives a variable of each turn, nor where the test reads
/// it again, `last == null || last.TAG !== "Op"`, or reads anything first.
fn tested_assignments(stmts: &mut Vec<Stmt>) {
    // What a test reads first, of what it evaluates first.
    fn first_read(e: &Expr) -> &Expr {
        match &e.kind {
            ExprKind::Member(a, _)
            | ExprKind::OptionalMember(a, _)
            | ExprKind::Unary(_, a)
            | ExprKind::Stringed(a)
            | ExprKind::Index(a, _)
            | ExprKind::OptionalIndex(a, _)
            | ExprKind::Binary(_, a, _)
            | ExprKind::Cond(a, _, _)
            | ExprKind::Call(a, _)
            | ExprKind::OptionalCall(a, _) => first_read(a),
            _ => e,
        }
    }
    let mut i = 0;
    while i < stmts.len() {
        let StmtKind::While { label, cond, body } = &mut stmts[i].kind else {
            i += 1;
            continue;
        };
        let assigned = match body.as_slice() {
            [
                Stmt {
                    kind: StmtKind::Const(name, made),
                    ..
                },
                Stmt {
                    kind: StmtKind::If(test, then, None),
                    ..
                },
                rest @ ..,
            ] if matches!(cond.kind, ExprKind::Bool(true))
                && matches!(then.as_slice(), [Stmt { kind: StmtKind::Break(l), .. }] if l.is_none() || l == label)
                && matches!(&first_read(test).kind, ExprKind::Var(v) if v == name)
                && {
                    let mut reads = 0;
                    test.visit_vars(&mut |v| reads += usize::from(v == name));
                    reads == 1
                } =>
            {
                let caught = |e: &Expr| {
                    matches!(
                        e.kind,
                        ExprKind::Arrow(..) | ExprKind::AsyncArrow(..) | ExprKind::Function(_)
                    ) && e.mentions_var(name)
                };
                let mut captured = false;
                js::each_expr(rest, &mut |e| captured |= caught(e));
                let assigned = Expr::assigned(name, made.clone());
                (!captured)
                    .then(|| test.substitute(&|n: &str| (n == name).then(|| assigned.clone())))
                    .flatten()
                    .map(|test| (name.clone(), test))
            }
            _ => None,
        };
        let Some((name, test)) = assigned else {
            i += 1;
            continue;
        };
        *cond = test.negated_test();
        body.drain(..2);
        stmts.insert(i, StmtKind::Let(name, None).at(js::Span::NONE));
        i += 2;
    }
}

/// Constants side by side that each read a property of one variable,
/// `const P = MDXComponents.p;`, none exported nor set: its destructuring,
/// as JS writes it, `const { Intro, p: P } = MDXComponents;`.
fn read_together(module: &mut js::Module) {
    // What a constant reads, the variable and the property, if it's one.
    fn read(item: &js::Item) -> Option<(&Expr, &str)> {
        match item {
            js::Item::Const(constant) if !constant.export && !constant.mutable => match &constant.value.kind {
                ExprKind::Member(object, property)
                    if matches!(object.kind, ExprKind::Var(_)) && crate::names::is_identifier(property) =>
                {
                    Some((&**object, property.as_str()))
                }
                _ => None,
            },
            _ => None,
        }
    }
    let same_var = |a: &Expr, b: &Expr| matches!((&a.kind, &b.kind), (ExprKind::Var(a), ExprKind::Var(b)) if a == b);
    let mut items = std::mem::take(&mut module.items).into_iter().peekable();
    while let Some(item) = items.next() {
        let Some((object, property)) = read(&item).map(|(object, property)| (object.clone(), property.to_string()))
        else {
            module.items.push(item);
            continue;
        };
        let js::Item::Const(first) = item else {
            unreachable!("a constant reads")
        };
        let mut fields = vec![(property, js::Pattern::Name(first.name.clone()), None)];
        while let Some(next) = items.peek()
            && let Some((of, property)) = read(next)
            && same_var(&object, of)
        {
            let property = property.to_string();
            let Some(js::Item::Const(constant)) = items.next() else {
                unreachable!("a constant reads")
            };
            fields.push((property, js::Pattern::Name(constant.name), None));
        }
        module.items.push(match fields.len() {
            1 => js::Item::Const(first),
            _ => js::Item::Statements(vec![
                StmtKind::Destructure {
                    pattern: js::Pattern::Object(fields, None),
                    value: object,
                    mutable: false,
                }
                .at(first.span),
            ]),
        });
    }
    js::each_item_block_mut(&mut module.items, &mut read_together_in);
}

/// `read_together` of a body's `const`s side by side, each named as the
/// property it reads, `const tag = fiber.tag; const type = fiber.type;`, a
/// field's or a cell's that's its property (ADR 0863): `const { tag, type
/// } = fiber;`, as React writes it (ADR 0031, amended). Each is read once,
/// in its order, either way. Not one the program names otherwise, `const
/// returnFiber = node.return;`, which reads best as it is.
fn read_together_in(stmts: &mut Vec<Stmt>) {
    fn read(stmt: &Stmt) -> Option<(&str, &str, &str)> {
        let StmtKind::Const(name, value) = &stmt.kind else {
            return None;
        };
        let ExprKind::Member(object, property) = &value.kind else {
            return None;
        };
        let ExprKind::Var(object) = &object.kind else {
            return None;
        };
        (name == property).then_some((object.as_str(), property.as_str(), name.as_str()))
    }
    let mut i = 0;
    while i < stmts.len() {
        let Some((object, _, _)) = read(&stmts[i]) else {
            i += 1;
            continue;
        };
        let run = stmts[i..]
            .iter()
            .take_while(|s| read(s).is_some_and(|(of, ..)| of == object))
            .count();
        if run < 2 {
            i += 1;
            continue;
        }
        let object = Expr::var(object);
        let fields = (stmts[i..i + run].iter())
            .filter_map(read)
            .map(|(_, property, name)| (property.to_string(), js::Pattern::Name(name.to_string()), None))
            .collect();
        let span = stmts[i].span;
        let destructured = StmtKind::Destructure {
            pattern: js::Pattern::Object(fields, None),
            value: object,
            mutable: false,
        };
        stmts.splice(i..i + run, [destructured.at(span)]);
        i += 1;
    }
}

/// `target = target op value` as JS writes it, `target op= value`: `s += t`
/// of a string built up, `r *= 2`, `x ??= d`, `lanes &= ~lane`, `flags <<=
/// 1`. Of every statement, a closure's in a constant too. The target is a
/// path (`same_path`), so reading it once where it was read twice reads
/// the same value: JS evaluates its object and its key before the value,
/// in either form.
fn compound(module: &mut js::Module) {
    let mut each = |stmts: &mut Vec<Stmt>| {
        for stmt in stmts.iter_mut() {
            let StmtKind::Assign(target, value) = &mut stmt.kind else {
                continue;
            };
            let ExprKind::Binary(op, lhs, rhs) = &mut value.kind else {
                continue;
            };
            let compounds = matches!(op, Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Rem | Op::Coalesce)
                || matches!(op, Op::BitAnd | Op::BitOr | Op::BitXor | Op::Shl | Op::Shr | Op::UShr);
            if !compounds {
                continue;
            }
            // An item's, its index checked on one side, `v[i] = v[$at(v, i)]
            // | lane`: the checked place's, `v[$at(v, i)] |= lane`, which
            // checks it, reads it, evaluates `lane` and writes, in that order,
            // as the assignment did (ADR 0894, amended).
            let checked = match (checked_item(target, lhs), checked_item(lhs, target)) {
                (true, _) => Some(std::mem::replace(target, Expr::undefined())),
                (_, true) => Some(std::mem::replace(&mut **lhs, Expr::undefined())),
                _ => js::same_path(lhs, target).then(|| std::mem::replace(target, Expr::undefined())),
            };
            if let Some(target) = checked {
                let (op, rhs) = (*op, std::mem::replace(&mut **rhs, Expr::undefined()));
                stmt.kind = StmtKind::Compound(op, target, rhs);
            }
        }
    };
    js::each_item_block_mut(&mut module.items, &mut each);
}

/// Whether `checked` is `v[$at(v, i)]` of the item `other` is, `v[i]`, its
/// index checked: the same array's path, the same key.
fn checked_item(checked: &Expr, other: &Expr) -> bool {
    let (ExprKind::Index(items, at), ExprKind::Index(same, key)) = (&checked.kind, &other.kind) else {
        return false;
    };
    let ExprKind::Call(helper, args) = &at.kind else {
        return false;
    };
    let [of, index] = args.as_slice() else {
        return false;
    };
    helper.helper_name().is_some_and(crate::runtime::checks_an_index)
        && js::same_path(items, same)
        && js::same_path(of, items)
        && (js::same_path(index, key)
            || matches!((&index.kind, &key.kind), (ExprKind::Num(a), ExprKind::Num(b)) if a == b))
}

/// A function made readable.
fn prepared(function: &mut js::Function) {
    function_body(&mut function.params, &mut function.body);
    shadows(&mut function.body, None);
    // A closure's own parameter, shadowed in it, or a variable it declares,
    // `for (const arg of args)`'s: nothing outside it sets one (ADR 0853).
    js::each_expr_mut(&mut function.body, &mut |e| {
        if let ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) = &mut e.kind {
            let mut own: HashSet<String> = params.iter().flat_map(|p| p.names()).map(str::to_string).collect();
            js::each_block_mut(body, &mut |stmts| {
                for stmt in stmts.iter() {
                    match &stmt.kind {
                        StmtKind::Const(name, _) | StmtKind::Let(name, _) => _ = own.insert(name.clone()),
                        StmtKind::Destructure { pattern, .. } | StmtKind::ForOf { pattern, .. } => {
                            own.extend(pattern.names().into_iter().map(str::to_string));
                        }
                        _ => {}
                    }
                }
            });
            shadows(body, Some(&own));
        }
    });
    tested_constants(&mut function.body);
    unread(&mut function.body);
    js::each_block_mut(&mut function.body, &mut |stmts| imported(stmts));
}

/// `const run = await import(spec).then((m) => m.run)` is `const { run } =
/// await import(spec)`, and another name `{ run: next }` (ADR 0304).
fn imported(stmts: &mut [Stmt]) {
    for stmt in stmts {
        let StmtKind::Const(name, value) = &stmt.kind else {
            continue;
        };
        let ExprKind::Await(awaited) = &value.kind else {
            continue;
        };
        let ExprKind::Call(then, args) = &awaited.kind else {
            continue;
        };
        let (ExprKind::Member(import, method), [read]) = (&then.kind, args.as_slice()) else {
            continue;
        };
        let (ExprKind::Import(_), "then", ExprKind::Arrow(params, body)) = (&import.kind, method.as_str(), &read.kind)
        else {
            continue;
        };
        let (
            [js::Pattern::Name(m)],
            [
                Stmt {
                    kind: StmtKind::Return(Some(got)),
                    ..
                },
            ],
        ) = (params.as_slice(), body.as_slice())
        else {
            continue;
        };
        let ExprKind::Member(of, export) = &got.kind else {
            continue;
        };
        if !matches!(&of.kind, ExprKind::Var(v) if v == m) {
            continue;
        }
        stmt.kind = StmtKind::Destructure {
            pattern: js::Pattern::Object(vec![(export.clone(), js::Pattern::Name(name.clone()), None)], None),
            value: Expr {
                facts: js::Facts::NONE,
                kind: ExprKind::Await(import.clone()),
                span: value.span,
            },
            mutable: false,
        };
    }
}

/// `const { currentIndex: _currentIndex, ...meta } = ctx;`, the one field
/// a rest's pattern leaves out, which nothing reads, is `currentIndex: _`,
/// as a person writes it (ADR 0205); where it leaves out more, or its block
/// declares a `_` already, each keeps its name. A parameter's too
/// (`left_out_params`).
fn left_out(body: &mut [Stmt]) {
    for i in 0..body.len() {
        let StmtKind::Destructure {
            pattern: js::Pattern::Object(fields, Some(_)),
            ..
        } = &body[i].kind
        else {
            continue;
        };
        let Some(k) = lone_left_out(fields, |name| js::mentions_in(&body[i + 1..], name) > 0) else {
            continue;
        };
        if declares_underscore(body) {
            continue;
        }
        if let StmtKind::Destructure {
            pattern: js::Pattern::Object(fields, _),
            ..
        } = &mut body[i].kind
        {
            fields[k].1 = js::Pattern::Name("_".to_string());
        }
    }
}

/// A function's parameters, as `left_out` a body's patterns:
/// `function Plain({ className: _, ...props })`.
fn left_out_params(params: &mut [js::Pattern], body: &[Stmt]) {
    for i in 0..params.len() {
        let js::Pattern::Object(fields, Some(_)) = &params[i] else {
            continue;
        };
        let Some(k) = lone_left_out(fields, |name| js::mentions_in(body, name) > 0) else {
            continue;
        };
        if declares_underscore(body) || params.iter().any(|p| p.names().contains(&"_")) {
            continue;
        }
        if let js::Pattern::Object(fields, _) = &mut params[i] {
            fields[k].1 = js::Pattern::Name("_".to_string());
        }
    }
}

/// The one field of an object pattern with a rest that it leaves out, a
/// `_name` nothing reads, as `read` says; `None` of none, or of more.
fn lone_left_out(fields: &[(String, js::Pattern, Option<Expr>)], read: impl Fn(&str) -> bool) -> Option<usize> {
    let unread: Vec<usize> = (fields.iter().enumerate())
        .filter(
            |(_, (_, bound, _))| matches!(bound, js::Pattern::Name(n) if n.starts_with('_') && n != "_" && !read(n)),
        )
        .map(|(k, _)| k)
        .collect();
    match unread[..] {
        [k] => Some(k),
        _ => None,
    }
}

/// Whether `body` declares a `_` of its own, where another would clash.
fn declares_underscore(body: &[Stmt]) -> bool {
    body.iter().any(|stmt| match &stmt.kind {
        StmtKind::Const(name, _) | StmtKind::Let(name, _) => name == "_",
        StmtKind::Destructure { pattern, .. } => pattern.names().contains(&"_"),
        _ => false,
    })
}

/// A function's body, a closure's too: its last `return;` isn't written, as
/// it falls off its end there, as a person ends one (ADR 0299); and its
/// parameters' patterns, as its body's (`left_out_params`).
fn function_body(params: &mut [js::Pattern], body: &mut Vec<Stmt>) {
    block(body);
    left_out_params(params, body);
    catches::tail_tries(body);
    if body
        .last()
        .is_some_and(|stmt| matches!(stmt.kind, StmtKind::Return(None)))
    {
        body.pop();
    }
}

fn block(body: &mut Vec<Stmt>) {
    catches::try_catches(body);
    left_out(body);
    for stmt in body.iter_mut() {
        if let Some(coalesced) = coalescing(stmt) {
            stmt.kind = coalesced;
        }
        // `else {}`, of nothing but Rust's comments, is no `else`.
        if let StmtKind::If(_, _, els) = &mut stmt.kind
            && els.as_ref().is_some_and(Vec::is_empty)
        {
            *els = None;
        }
        // `let timeout = undefined;` is `let timeout;`, as a person writes it:
        // a `let` starts as `undefined`, each time it's run; and `return
        // undefined;` is `return;`, as JS ends a function with nothing to give
        // (ADR 0299).
        if let StmtKind::Let(_, value) | StmtKind::Return(value) = &mut stmt.kind
            && value.as_ref().is_some_and(|v| matches!(v.kind, ExprKind::Undefined))
        {
            *value = None;
        }
    }
    for stmt in body.iter_mut() {
        match &mut stmt.kind {
            StmtKind::Const(_, e)
            | StmtKind::Expr(e)
            | StmtKind::Throw(e)
            | StmtKind::Return(Some(e))
            | StmtKind::Let(_, Some(e))
            | StmtKind::Destructure { value: e, .. } => expr(e),
            StmtKind::Assign(a, b) => {
                expr(a);
                expr(b);
            }
            StmtKind::If(e, a, b) => {
                // Its test as a test, `!!x` the `x` it tests.
                *e = e.tested();
                expr(e);
                block(a);
                if let Some(b) = b {
                    block(b);
                }
            }
            StmtKind::While { cond, body, .. } => {
                // Its test as a test, as an `if`'s.
                *cond = cond.tested();
                expr(cond);
                block(body);
            }
            StmtKind::Labeled(_, body) => block(body),
            StmtKind::Switch {
                discriminant,
                cases,
                default,
            } => {
                expr(discriminant);
                for js::Case { values, body, .. } in cases.iter_mut() {
                    values.iter_mut().for_each(expr);
                    block(body);
                }
                if let Some(default) = default {
                    block(default);
                }
            }
            StmtKind::Try(body, finally) | StmtKind::TryCatch(body, _, finally) => {
                block(body);
                block(finally);
            }
            StmtKind::ForOf { iterable, body, .. } => {
                expr(iterable);
                block(body);
            }
            StmtKind::For { start, test, body, .. } => {
                expr(start);
                expr(test);
                block(body);
            }
            // A function declared in a body (ADR 0308), as the one it's in.
            StmtKind::Function(function) => function_body(&mut function.params, &mut function.body),
            StmtKind::Compound(_, a, b) => {
                expr(a);
                expr(b);
            }
            StmtKind::Let(_, None)
            | StmtKind::Return(None)
            | StmtKind::Break(_)
            | StmtKind::Continue(_)
            | StmtKind::Directive(_)
            | StmtKind::Comments => {}
        }
    }
    truths::truth_tested(body);
    fallen_held(body);
    fallen_declared(body);
}

/// `let name = error.name; if (!name) { name = "Error"; }` is `const name =
/// error.name || "Error";`: a `let`, then its fallback where its value is
/// falsy, the very `!x` JS tests its truth by, is `e || c`, as React
/// writes it; and of its fallback where it's none, `x ??= c` of `if (x ==
/// null)` (`coalescing`), `e ?? c` (ADR 0651). Each makes `c` only where
/// `e` is falsy, or none, as the `if` did. Not of a `c` that reads `x`,
/// which the declaration would read before it's made; a `const` where
/// nothing sets `x` after.
fn fallen_declared(body: &mut Vec<Stmt>) {
    let mut i = 0;
    while i + 1 < body.len() {
        if let StmtKind::Let(name, Some(value)) = &body[i].kind
            && let Some((op, fallback)) = fallback_of(&body[i + 1], name)
            && !fallback.mentions_var(name)
        {
            let name = name.clone();
            let value = Expr {
                span: value.span,
                ..Expr::bin(op, value.clone(), fallback)
            };
            body[i].kind = match js::Writes::of(&body[i + 2..]).may_replace(&name) {
                true => StmtKind::Let(name, Some(value)),
                false => StmtKind::Const(name, value),
            };
            body.remove(i + 1);
        }
        i += 1;
    }
}

/// The fallback `stmt` gives the variable `name`: `c` of `if (!x) { x =
/// c; }`, by `||`, or of `x = x ?? c`, by `??`.
fn fallback_of(stmt: &Stmt, name: &str) -> Option<(Op, Expr)> {
    let is = |e: &Expr| matches!(&e.kind, ExprKind::Var(v) if v == name);
    match &stmt.kind {
        StmtKind::If(test, then, None) if matches!(&test.kind, ExprKind::Unary(UnaryOp::Not, x) if is(x)) => {
            match then.as_slice() {
                [
                    Stmt {
                        kind: StmtKind::Assign(target, fallback),
                        ..
                    },
                ] if is(target) => Some((Op::Or, fallback.clone())),
                _ => None,
            }
        }
        StmtKind::Assign(target, value) if is(target) => match &value.kind {
            ExprKind::Binary(Op::Coalesce, x, fallback) if is(x) => Some((Op::Coalesce, (**fallback).clone())),
            _ => None,
        },
        _ => None,
    }
}

/// `const type = a || b || undefined; return type ?? "Object";` is `const
/// type = a || b || "Object"; return type;`: a chain held, whose fallback
/// the next statement gives where it reads it, its one read, is the
/// chain's, `fallen_to`'s, as a person writes it (ADR 0646). Only a
/// constant fallback, which nothing after the `const` could change, nor
/// its being made earlier.
fn fallen_held(body: &mut [Stmt]) {
    for at in 1..body.len() {
        let (before, after) = body.split_at_mut(at);
        let StmtKind::Const(name, chain) = &mut before[at - 1].kind else {
            continue;
        };
        if js::mentions_in(&after[1..], name) != 0 || js::mentions_in(&after[..1], name) != 1 {
            continue;
        }
        let (StmtKind::Return(Some(read)) | StmtKind::Const(_, read) | StmtKind::Expr(read)) = &mut after[0].kind
        else {
            continue;
        };
        let ExprKind::Binary(js::Op::Coalesce, held, fallback) = &read.kind else {
            continue;
        };
        if !matches!(&held.kind, ExprKind::Var(v) if v == name) || !fallback.is_constant() {
            continue;
        }
        let Some(fallen) = fallen_to(chain, fallback) else {
            continue;
        };
        *chain = Expr {
            span: chain.span,
            ..fallen
        };
        *read = Expr {
            span: read.span,
            ..(**held).clone()
        };
    }
}

/// `const match = $try(() => f()); if (match.TAG === "Ok") { x = match._0; }
/// else { .. }`, a `match` of what a JS call threw whose `Ok` only keeps
/// the value and whose `Err` reads no error, is `try { x = f(); } catch {
/// .. }`, as JS writes it (ADR 0035): only the call is in the `try`, as only
/// it is in `$try`.
/// A `const` only tests read, `const hide = !!error || !ready`, is a test
/// itself: what it's made of is read by its truth, `error || !ready`, as
/// a person writes it (ADR 0298). One read as a value keeps its boolean.
fn tested_constants(body: &mut Vec<Stmt>) {
    let mut candidates = Vec::new();
    js::each_block_mut(body, &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Const(name, value) = &stmt.kind
                && value.retested().is_some()
            {
                candidates.push(name.clone());
            }
        }
    });
    for name in candidates {
        // Each read a test makes, marked: none is left if they all are.
        let mut marked = body.clone();
        js::each_block_mut(&mut marked, &mut |stmts| {
            for stmt in stmts.iter_mut() {
                match &mut stmt.kind {
                    StmtKind::If(cond, ..) | StmtKind::While { cond, .. } => mark_tested(cond, &name),
                    StmtKind::For { test, .. } => mark_tested(test, &name),
                    _ => {}
                }
            }
        });
        js::each_expr_mut(&mut marked, &mut |e| {
            if let ExprKind::Cond(test, ..) = &mut e.kind {
                mark_tested(test, &name);
            }
        });
        if js::mentions_in(&marked, &name) != 0 {
            continue;
        }
        js::each_block_mut(body, &mut |stmts| {
            for stmt in stmts.iter_mut() {
                if let StmtKind::Const(declared, value) = &mut stmt.kind
                    && *declared == name
                    && let Some(tested) = value.retested()
                {
                    *value = tested;
                }
            }
        });
    }
}

/// `name`'s reads in `test`, a test, and in the parts of it a test reads by
/// their truth, `!a`, `a && b`, `a || b`, renamed apart.
fn mark_tested(test: &mut Expr, name: &str) {
    match &mut test.kind {
        ExprKind::Var(var) if var == name => *var = String::from("$tested"),
        ExprKind::Unary(js::UnaryOp::Not, a) => mark_tested(a, name),
        ExprKind::Binary(js::Op::And | js::Op::Or, a, b) => {
            mark_tested(a, name);
            mark_tested(b, name);
        }
        _ => {}
    }
}

/// `a` of `a || b`, whose value is kept only where it's truthy, as one
/// that's the same where it is and falsy where it isn't (ADR 0647): `t ? x :
/// undefined` is `t && x`, and of a chain `x != null && .. && x[k]`, whose
/// other tests do nothing, `.. && x?.[k]`, as React writes
/// `(hasToStringTag && value[Symbol.toStringTag]) || ..`.
fn kept_by_truth(a: &Expr) -> Option<Expr> {
    let ExprKind::Cond(test, x, none) = &a.kind else {
        return None;
    };
    if !matches!(none.kind, ExprKind::Undefined) {
        return None;
    }
    let mut parts = Vec::new();
    fn conjuncts(e: &Expr, out: &mut Vec<Expr>) {
        match &e.kind {
            ExprKind::Binary(js::Op::And, a, b) => {
                conjuncts(a, out);
                conjuncts(b, out);
            }
            _ => out.push(e.clone()),
        }
    }
    conjuncts(test, &mut parts);
    let mut kept = (**x).clone();
    // The first test, `x != null`, of what the value reads a part of.
    if let Some(present) = js::null_test(&parts[0], true)
        && parts[1..].iter().all(|p| !p.has_effects())
        && let Some(chained) = optional_first(&kept, present)
    {
        parts.remove(0);
        kept = chained;
    }
    parts.push(kept);
    parts.into_iter().reduce(|a, b| Expr::bin(js::Op::And, a, b))
}

/// `x[k]` or `x.p` of `x`, its first part optional: `x?.[k]`, `x?.p`.
fn optional_first(part: &Expr, x: &Expr) -> Option<Expr> {
    match &part.kind {
        ExprKind::Index(of, key) if js::same_path(of, x) => Some(Expr::optional_index((**of).clone(), (**key).clone())),
        ExprKind::Member(of, name) if js::same_path(of, x) => Some(Expr::optional_member((**of).clone(), name.clone())),
        _ => None,
    }
}

/// `chain ?? y`, of a chain of `||`, `x || z`: `x` where it's truthy, never
/// `null` or `undefined`, else `z ?? y`. Of one that ends in `undefined`, a
/// `filter` by truth's, or `null`, `x || y`; of one that ends in another
/// literal, which `??` keeps, the chain, `a || "Memo"` (ADR 0635). Each
/// `||` after the one before, `a || b || y`, as JS reads them.
fn fallen_to(chain: &Expr, y: &Expr) -> Option<Expr> {
    fn parts(e: &Expr, out: &mut Vec<Expr>) {
        match &e.kind {
            ExprKind::Binary(js::Op::Or, a, b) => {
                parts(a, out);
                parts(b, out);
            }
            _ => out.push(e.clone()),
        }
    }
    let mut chained = Vec::new();
    parts(chain, &mut chained);
    match chained.pop()? {
        _ if chained.is_empty() => return None,
        last if matches!(
            last.kind,
            ExprKind::Undefined | ExprKind::GivenUndefined | ExprKind::Null
        ) =>
        {
            parts(y, &mut chained)
        }
        last if last.is_constant() => chained.push(last),
        _ => return None,
    }
    chained.into_iter().reduce(|a, b| Expr::bin(js::Op::Or, a, b))
}

/// `if (x == null) { x = e; }` is `x = x ?? e`, printed `x ??= e`: the
/// same test, and `e` made only where it's none.
fn coalescing(stmt: &Stmt) -> Option<StmtKind> {
    let StmtKind::If(test, then, None) = &stmt.kind else {
        return None;
    };
    let tested = js::null_test(test, false)?;
    let [
        Stmt {
            kind: StmtKind::Assign(target, value),
            ..
        },
    ] = then.as_slice()
    else {
        return None;
    };
    if !js::same_path(tested, target) {
        return None;
    }
    Some(StmtKind::Assign(
        target.clone(),
        Expr::bin(js::Op::Coalesce, target.clone(), value.clone()),
    ))
}

/// A `const` nothing reads, of a value whose making calls nothing and reads
/// only variables, is no statement: `const lock = { value: 1 };`. Not one
/// of a property, which a binding's getter may be, `el.offsetWidth`, that
/// lays the page out (ADR 0356).
fn unread(body: &mut Vec<Stmt>) {
    let mut read = HashSet::new();
    js::visit_stmts(body, &mut |name| {
        read.insert(name.to_string());
    });
    js::each_block_mut(body, &mut |stmts| {
        stmts.retain(|s| {
            !matches!(&s.kind, StmtKind::Const(name, value)
                if !read.contains(name) && value.reads_only_vars())
        });
    });
}

/// `const n$1 = n;`, Rust's `let n = n;`, a variable shadowed by its own
/// value, is `n` itself: `<Type />` for `let Type = from_unknown(Type)`,
/// `content` for `let content = content.clone()`. Only where `n` holds
/// that value wherever `n$1` is read: nothing sets `n` after it, nor in a
/// closure, nor where a loop runs it again. In a closure, `own` are its
/// parameters, the only ones aliased: what's outside it, it doesn't see set.
fn shadows(body: &mut Vec<Stmt>, own: Option<&HashSet<String>>) {
    while let Some((alias, of)) = shadow(body, own) {
        js::each_expr_mut(body, &mut |e| {
            if matches!(&e.kind, ExprKind::Var(name) if *name == alias) {
                e.kind = ExprKind::Var(of.clone());
            }
        });
        js::each_block_mut(body, &mut |stmts| {
            stmts.retain(|s| {
                !matches!(&s.kind, StmtKind::Const(name, value)
                if *name == alias && matches!(&value.kind, ExprKind::Var(v) if *v == of))
            });
        });
    }
}

/// A `const n$1 = n;` that `shadows` makes `n`, and `n`.
fn shadow(body: &[Stmt], own: Option<&HashSet<String>>) -> Option<(String, String)> {
    // What closures set, and what their parameters are named, which a
    // name read in them would mean instead.
    let mut in_closures = js::Writes::of(body).replaced_in_closures;
    js::each_expr(body, &mut |e| {
        if let ExprKind::Arrow(params, _) | ExprKind::AsyncArrow(params, _) = &e.kind {
            in_closures.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
        }
    });
    let mut walk = Walk::default();
    walk.stmts(body, false);
    walk.shadows.into_iter().find_map(|(at, looped, alias, of)| {
        let sets: Vec<usize> = walk
            .sets
            .iter()
            .filter(|(_, name)| *name == of)
            .map(|&(set, _)| set)
            .collect();
        let kept = !in_closures.contains(&of) && (sets.is_empty() || !looped && sets.iter().all(|&set| set < at));
        let owned = own.is_none_or(|own| own.contains(&of));
        (kept && owned).then_some((alias, of))
    })
}

/// The function's statements in order, outside its closures: each
/// `const n$1 = n;`, where, and whether in a loop, and each variable set.
#[derive(Default)]
struct Walk {
    at: usize,
    shadows: Vec<(usize, bool, String, String)>,
    sets: Vec<(usize, String)>,
}

impl Walk {
    fn stmts(&mut self, stmts: &[Stmt], looped: bool) {
        for stmt in stmts {
            self.at += 1;
            match &stmt.kind {
                StmtKind::Const(alias, value) => {
                    if let ExprKind::Var(of) = &value.kind
                        && crate::names::apart(alias).is_some_and(|(base, _)| base == of)
                    {
                        self.shadows.push((self.at, looped, alias.clone(), of.clone()));
                    }
                }
                StmtKind::Assign(target, _) | StmtKind::Compound(_, target, _) => {
                    if let ExprKind::Var(name) = &target.kind {
                        self.sets.push((self.at, name.clone()));
                    }
                }
                StmtKind::If(_, a, b) => {
                    self.stmts(a, looped);
                    if let Some(b) = b {
                        self.stmts(b, looped);
                    }
                }
                StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } => self.stmts(body, true),
                StmtKind::For { name, body, .. } => {
                    self.sets.push((self.at, name.clone()));
                    self.stmts(body, true);
                }
                StmtKind::Labeled(_, body) => self.stmts(body, looped),
                StmtKind::Switch { cases, default, .. } => {
                    js::case_lists(cases, default).for_each(|body| self.stmts(body, looped));
                }
                StmtKind::Try(a, b) | StmtKind::TryCatch(a, _, b) => {
                    self.stmts(a, looped);
                    self.stmts(b, looped);
                }
                StmtKind::Let(..)
                | StmtKind::Destructure { .. }
                | StmtKind::Expr(_)
                | StmtKind::Break(_)
                | StmtKind::Continue(_)
                | StmtKind::Directive(_)
                | StmtKind::Comments
                | StmtKind::Return(_)
                | StmtKind::Throw(_)
                | StmtKind::Function(_) => {}
            }
        }
    }
}

/// `Object.fromEntries([["/index.html", code], ..])`, each key a string
/// written out, is the object literal of them, `{ "/index.html": code }`:
/// the same properties, made in the same order. Not of a key given twice,
/// which a literal would have twice, nor of `__proto__`, which a literal
/// makes the prototype, nor of an `undefined` value, which an object's
/// field leaves out (ADR 0280) and an entry keeps.
fn entries_written_out(e: &Expr) -> Option<Expr> {
    let ExprKind::Call(f, args) = &e.kind else {
        return None;
    };
    let (ExprKind::Member(object, name), [entries]) = (&f.kind, args.as_slice()) else {
        return None;
    };
    let (ExprKind::Var(object), ExprKind::Array(entries)) = (&object.kind, &entries.kind) else {
        return None;
    };
    if object != "Object" || name != "fromEntries" {
        return None;
    }
    let mut fields = Vec::new();
    for entry in entries {
        let ExprKind::Array(pair) = &entry.kind else {
            return None;
        };
        let [key, value] = pair.as_slice() else {
            return None;
        };
        let ExprKind::Str(key) = &key.kind else {
            return None;
        };
        let given = fields
            .iter()
            .any(|field| matches!(field, Prop::Field(k, _) if k == key));
        if given || key == "__proto__" || matches!(value.kind, ExprKind::Undefined) {
            return None;
        }
        fields.push(Prop::Field(key.clone(), value.clone()));
    }
    Some(Expr {
        facts: js::Facts::NONE,
        kind: ExprKind::Object(fields, js::ObjectLayout::default()),
        span: e.span,
    })
}

/// A field that's a closure of statements, `then: (resolve, reject) => {
/// .. }`, as a method, `then(resolve, reject) { .. }`, as React's ReactAct
/// writes its thenable (ADR 0882). An arrow and a method differ only in
/// their `this` (and `arguments`, which no Rust reads), so not one that
/// reads `this`, a class's. One named `__proto__` is a property as any
/// method is: only `__proto__: value` sets the prototype.
fn method(prop: &Prop) -> Option<Prop> {
    let Prop::Field(name, value) = prop else {
        return None;
    };
    let (ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body)) = &value.kind else {
        return None;
    };
    let mut this = false;
    js::each_expr(body, &mut |e| this |= matches!(&e.kind, ExprKind::This));
    if this {
        return None;
    }
    // One of a value alone, `double: (x) => x * 2`, reads as it is, as a
    // person writes a WebAssembly import's.
    if matches!(
        body.as_slice(),
        [Stmt {
            kind: StmtKind::Return(Some(_)),
            ..
        }]
    ) {
        return None;
    }
    Some(Prop::Method(name.clone(), value.clone()))
}

fn expr(e: &mut Expr) {
    // A `&mut`'s handle on a place (ADR 0099), and a `dyn`'s of one with its
    // dictionary, as JS writes it: `{ get value() { return n; }, set
    // value(value) { n = value; } }`, its parameter a name the place
    // doesn't read; what it stores where its storage says, `value ?? null`
    // (ADR 0772).
    if let ExprKind::Handle(place, _) | ExprKind::Pair(place, _) = &e.kind {
        let mut param = "value".to_string();
        if place.mentions_var(&param) {
            param = "newValue".to_string();
        }
        while place.mentions_var(&param) {
            param.push('_');
        }
        let get = Expr::arrow(
            Vec::new(),
            vec![StmtKind::Return(Some((**place).clone())).at(place.span)],
        );
        let written = match &e.kind {
            ExprKind::Handle(_, storage) => storage.written(Expr::var(&param), false),
            _ => Expr::var(&param),
        };
        let set = Expr::arrow(
            vec![param.as_str().into()],
            vec![StmtKind::Assign((**place).clone(), written).at(place.span)],
        );
        let dictionary = match &e.kind {
            ExprKind::Pair(_, dictionary) => Some(Prop::Field(js::DICTIONARY.into(), (**dictionary).clone())),
            _ => None,
        };
        let props = dictionary
            .into_iter()
            .chain([Prop::Getter(js::HELD.into(), get), Prop::Setter(js::HELD.into(), set)])
            .collect();
        *e = Expr {
            span: e.span,
            ..Expr::object(props)
        };
    }
    // `(x) => !!x` is `Boolean`, as react.dev's `.filter(Boolean)` is:
    // `Boolean(x)` is `!!x`, and it reads no other argument.
    if let ExprKind::Arrow(params, body) = &e.kind
        && let [js::Pattern::Name(param)] = params.as_slice()
        && let [
            Stmt {
                kind: StmtKind::Return(Some(value)),
                ..
            },
        ] = body.as_slice()
        && let ExprKind::Unary(js::UnaryOp::Not, not) = &value.kind
        && let ExprKind::Unary(js::UnaryOp::Not, tested) = &not.kind
        && matches!(&tested.kind, ExprKind::Var(name) if name == param)
    {
        *e = Expr {
            facts: js::Facts::NONE,
            kind: ExprKind::Var("Boolean".into()),
            span: e.span,
        };
        return;
    }
    // `x == null || !x` is `!x`: `null` and `undefined` are falsy (ADR 0313).
    if let ExprKind::Binary(js::Op::Or, null_test, not) = &e.kind
        && let Some(x) = js::null_test(null_test, false)
        && let ExprKind::Unary(js::UnaryOp::Not, y) = &not.kind
        && js::same_path(x, y)
    {
        *e = (**not).clone();
    }
    // `() => (async () => { .. })()`, a closure of an `async` block, is
    // `async () => { .. }`, as react.dev's `loadLinter`: calling either runs
    // the block up to its first `await` and gives its promise (ADR 0029).
    if let ExprKind::Arrow(params, body) = &mut e.kind
        && let [
            Stmt {
                kind: StmtKind::Return(Some(value)),
                ..
            },
        ] = body.as_mut_slice()
        && let ExprKind::Call(callee, args) = &mut value.kind
        && args.is_empty()
        && let ExprKind::AsyncArrow(own, block) = &mut callee.kind
        && own.is_empty()
    {
        e.kind = ExprKind::AsyncArrow(std::mem::take(params), std::mem::take(block));
    }
    match &mut e.kind {
        ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => {
            // `() => undefined`, the value it's given: its body is that value,
            // not `return;`.
            let concise = matches!(
                body.as_slice(),
                [Stmt {
                    kind: StmtKind::Return(Some(_)),
                    ..
                }]
            );
            match concise {
                true => {
                    block(body);
                    left_out_params(params, body);
                }
                false => function_body(params, body),
            }
            if concise
                && let [
                    Stmt {
                        kind: StmtKind::Return(value @ None),
                        ..
                    },
                ] = body.as_mut_slice()
            {
                *value = Some(Expr::undefined());
            }
        }
        ExprKind::Function(function) => function_body(&mut function.params, &mut function.body),
        ExprKind::Jsx(jsx) => {
            if let JsxTag::Component(e) = &mut jsx.tag {
                expr(e);
            }
            let element = matches!(jsx.tag, JsxTag::Intrinsic(_));
            // An attribute that's `undefined`, a `None` say, is none, as JSX
            // leaves it out.
            jsx.props.retain(|p| {
                !matches!(p, Prop::Field(_, value) | Prop::Getter(_, value) | Prop::Setter(_, value) | Prop::Method(_, value) if matches!(value.kind, ExprKind::Undefined))
            });
            for prop in &mut jsx.props {
                match prop {
                    Prop::Field(name, value) => {
                        // React ignores what a DOM element's handler returns: keep
                        // single-call ones as concise arrow expressions (ADR 0040).
                        // A component may read what its callback returns, which
                        // lowering says of one whose call gives `undefined`.
                        if element && js::is_handler_name(name) {
                            js::returning_its_call(value);
                        }
                        expr(value);
                    }
                    Prop::Getter(_, value) | Prop::Setter(_, value) | Prop::Method(_, value) | Prop::Spread(value) => {
                        expr(value)
                    }
                }
            }
            for child in &mut jsx.children {
                expr(child);
            }
        }
        // What `!` turns it reads as a test, wherever it is.
        ExprKind::Unary(js::UnaryOp::Not, a) => {
            **a = a.tested();
            expr(a);
        }
        ExprKind::Member(a, _)
        | ExprKind::OptionalMember(a, _)
        | ExprKind::Unary(_, a)
        | ExprKind::Await(a)
        | ExprKind::Spread(a)
        | ExprKind::Stringed(a)
        | ExprKind::Handle(a, _)
        | ExprKind::Assigned(_, a)
        | ExprKind::DropArgument(_, _, a) => expr(a),
        ExprKind::Binary(js::Op::Coalesce, a, b) => {
            expr(a);
            expr(b);
            if let Some(chain) = fallen_to(a, b) {
                *e = Expr { span: e.span, ..chain };
            }
        }
        ExprKind::Binary(js::Op::Or, a, b) => {
            expr(a);
            expr(b);
            if let Some(truth) = kept_by_truth(a) {
                **a = Expr { span: a.span, ..truth };
            }
        }
        ExprKind::Index(a, b) | ExprKind::OptionalIndex(a, b) | ExprKind::Binary(_, a, b) | ExprKind::Pair(a, b) => {
            expr(a);
            expr(b);
        }
        // A template in a template, a value's text, \`${`${x}`}\`, is spliced
        // into it: \`${x}\` (ADR 0066).
        ExprKind::Template(_, values, _) => {
            values.iter_mut().for_each(expr);
            e.splice_templates();
        }
        ExprKind::OptionalCall(f, args) => {
            expr(f);
            args.iter_mut().for_each(expr);
        }
        ExprKind::Cond(a, b, c) => {
            // Its test as a test, `!!x` the `x` it tests.
            **a = a.tested();
            expr(a);
            expr(b);
            expr(c);
            // `a ? a : b` is `a || b`: `a` where it's truthy, else `b`; and
            // `a ? a : undefined`, `a || undefined`, which a fallback after
            // makes `a || y` (`fallen_to`).
            if js::same_path(a, b) {
                let (then, els) = (
                    std::mem::replace(&mut **b, Expr::undefined()),
                    std::mem::replace(&mut **c, Expr::undefined()),
                );
                *e = Expr {
                    span: e.span,
                    ..Expr::bin(js::Op::Or, then, els)
                };
            // `a ? b : a` is `a && b`: `b` where `a` is truthy, else `a`, as
            // upstream's `module && module[requireString]` is of `if
            // js::truthy(module) { js::get(module, key) } else { module }`.
            // Of what reads the same each time, which `&&` reads once.
            } else if js::same_path(a, c) && a.reads_same() {
                let (test, then) = (
                    std::mem::replace(&mut **a, Expr::undefined()),
                    std::mem::replace(&mut **b, Expr::undefined()),
                );
                *e = Expr {
                    span: e.span,
                    ..Expr::bin(js::Op::And, test, then)
                };
            }
        }
        ExprKind::Call(f, args) | ExprKind::New(f, args) => {
            expr(f);
            args.iter_mut().for_each(expr);
            // A trailing `undefined` after another argument, a `None` say, is
            // no argument, `f(x)`, as a person leaves an optional one out:
            // only `arguments.length` tells (ADR 0313). One given alone is a
            // value, `setProgram(undefined)`.
            while args.len() > 1 && args.last().is_some_and(|a| matches!(a.kind, ExprKind::Undefined)) {
                args.pop();
            }
            if let Some(object) = entries_written_out(e) {
                *e = object;
            }
        }
        ExprKind::Array(items) => items.iter_mut().for_each(expr),
        ExprKind::Object(props, layout) => {
            // A field that's `undefined` is no key, `{ code }` (ADR 0280).
            undefined_keys::left_out(props, layout);
            for prop in props.iter_mut() {
                let (Prop::Field(_, value)
                | Prop::Getter(_, value)
                | Prop::Setter(_, value)
                | Prop::Method(_, value)
                | Prop::Spread(value)) = prop;
                expr(value);
                if let Some(method) = method(prop) {
                    *prop = method;
                }
            }
        }
        // Every variant with an expression in it is above, so a rule of
        // what an expression is, ADR 0280's say, holds wherever it's written.
        ExprKind::Num(_)
        | ExprKind::BigInt(_)
        | ExprKind::BigUint(_)
        | ExprKind::Bool(_)
        | ExprKind::Str(_)
        | ExprKind::Written(_)
        | ExprKind::Undefined
        | ExprKind::GivenUndefined
        | ExprKind::Null
        | ExprKind::This
        | ExprKind::Var(_)
        | ExprKind::Symbol(_)
        | ExprKind::ItemHole(_)
        | ExprKind::Import(_)
        | ExprKind::Regex(_) => {}
    }
}
