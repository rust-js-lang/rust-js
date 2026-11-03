//! A function that only calls, with its parameters, a module variable's
//! function, or a function its module constant's value holds, as that
//! function: the variable by the function's name, or the value taken apart
//! where it's made (ADR 0610, amended).

use crate::js::{self, Expr, ExprKind, Stmt, StmtKind};
use std::collections::HashSet;

/// `function getTime(n) { return GET_TIME(n); }`, a function that only
/// calls a module variable's function with its parameters, a thread-local's
/// of a closure: that variable, by the function's name, `let getTime; if
/// (..) { getTime = (n) => .. }`, exported where the function was (ADR
/// 0610). Not where the module's load calls the function, which a
/// declaration's hoisting lets run before the variable is set.
pub(super) fn functions(module: &mut js::Module) {
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
        // Its call's value, `return GET_TIME(n);`, or none, `QUEUE(callback);`
        // of a function of no value.
        let returns = matches!(
            function.body.as_slice(),
            [Stmt {
                kind: StmtKind::Return(_),
                ..
            }]
        );
        // The module variable it calls with its parameters.
        let variable = match js::passes_on(&function.params, &function.body, true).map(|callee| &callee.kind) {
            Some(ExprKind::Var(v)) if *v != function.name => Some(v.clone()),
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
        if given.is_empty() || !(returns && given.iter().all(arrows) || called_only_privately(module, i)) {
            i += 1;
            continue;
        }
        if called_where_loaded(module, &name) {
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

/// Whether the module's load may call `name`, by a constant's value or its
/// code, which a declaration's hoisting lets run before a variable is set;
/// or it's the module's default.
fn called_where_loaded(module: &js::Module, name: &str) -> bool {
    let mut loaded = Vec::new();
    for item in &module.items {
        match item {
            js::Item::Const(constant) => loaded.push(StmtKind::Expr(constant.value.clone()).at(constant.span)),
            js::Item::Statements(stmts) => loaded.extend(stmts.iter().cloned()),
            _ => {}
        }
    }
    js::mentions_in(&loaded, name) > 0 || module.default_export.as_deref() == Some(name)
}

/// Whether the function at `at` is the module's own, not exported by its
/// name or another (ADR 0596), and the crate's code only calls it, by its
/// parameters: none of its callers gives it more, nor reads what a
/// function of no value gives, a `()` being `undefined` where it's read
/// (ADR 0610, amended).
fn called_only_privately(module: &js::Module, at: usize) -> bool {
    let js::Item::Function(function) = &module.items[at] else {
        return false;
    };
    !function.export
        && !module.aliases.iter().any(|(local, _)| *local == function.name)
        && only_called(&module.items, at, &function.name, function.params.len())
}

/// `function link(dep, sub, version) { system.link(dep, sub, version);
/// }`, each function of the module's own that's only called and only
/// calls, with its parameters, a function that reads no `this` its module
/// constant's value holds (`Known::ThisUnread`), a struct's field the
/// crate fills with its own functions: that value taken apart where it's
/// made, each field by its function's name, the functions gone, `const {
/// link, unlink, propagate } = createReactiveSystem(..);`, as the function
/// is its variable (ADR 0610, amended, #533). Only those functions read
/// the constant, so nothing sets it or writes its fields: each call read
/// the function the value was made with, which the destructuring reads
/// once.
/// Not one kept out of line (ADR 0649). One the module's load calls
/// before the value is made read it before it was made, as the function
/// did.
pub(super) fn fields(module: &mut js::Module) {
    // Each such function, by the constant it reads: where it is, the field
    // and its name.
    type Taken = (usize, String, String);
    let mut by_value: Vec<(String, Vec<Taken>)> = Vec::new();
    for (i, item) in module.items.iter().enumerate() {
        let js::Item::Function(function) = item else {
            continue;
        };
        let call = match function.body.as_slice() {
            [
                Stmt {
                    kind: StmtKind::Return(Some(call)) | StmtKind::Expr(call),
                    ..
                },
            ] => call,
            _ => continue,
        };
        let Some(ExprKind::Member(of, field)) = js::passes_on(&function.params, &function.body, true).map(|c| &c.kind)
        else {
            continue;
        };
        let ExprKind::Var(value) = &of.kind else {
            continue;
        };
        if !call.facts.has(js::Known::ThisUnread)
            || !crate::names::is_identifier(field)
            || function.noinline
            || !called_only_privately(module, i)
        {
            continue;
        }
        let taken = (i, field.clone(), function.name.clone());
        match by_value.iter_mut().find(|(v, _)| v == value) {
            Some((_, fields)) => fields.push(taken),
            None => by_value.push((value.clone(), vec![taken])),
        }
    }
    let mut gone = HashSet::new();
    for (value, fields) in by_value {
        let at = module
            .items
            .iter()
            .position(|item| matches!(item, js::Item::Const(c) if c.name == value && !c.export && c.fields.is_empty()));
        let mut reads = 0;
        for item in &module.items {
            item.visit_vars(&mut |v| reads += usize::from(v == value));
        }
        let Some(at) = at.filter(|_| reads == fields.len()) else {
            continue;
        };
        let js::Item::Const(constant) = &module.items[at] else {
            unreachable!("found")
        };
        let pattern = (fields.iter())
            .map(|(_, field, name)| (field.clone(), js::Pattern::Name(name.clone()), None))
            .collect();
        let destructure = StmtKind::Destructure {
            pattern: js::Pattern::Object(pattern, None),
            value: constant.value.clone(),
            mutable: false,
        };
        module.items[at] = js::Item::Statements(vec![destructure.at(constant.span)]);
        gone.extend(fields.iter().map(|&(i, ..)| i));
    }
    let mut i = 0;
    module.items.retain(|_| {
        i += 1;
        !gone.contains(&(i - 1))
    });
}

/// `const flightClient = ReactFlightClient({..});` of a private
/// thread-local, each of whose reads calls its member, a function that
/// reads no `this` (`Known::ThisUnread`), a namespace's method through
/// it, `flightClient.createResponse(..)`: that value taken apart where
/// it's made, each member by its name, called alone, `const {
/// createResponse, close } = ReactFlightClient({..});` and
/// `createResponse(..)`, as upstream's noop Flight client writes it (ADR
/// 0610, amended, #561). Nothing sets the constant or writes its members,
/// so each call read the function the value was made with, which the
/// destructuring reads once. Not where a member's name is no identifier or
/// any name the module has already, which its call would then read.
pub(super) fn members(module: &mut js::Module) {
    let mut names: HashSet<String> = HashSet::new();
    for item in &module.items {
        match item {
            js::Item::Const(constant) => {
                names.insert(constant.name.clone());
                js::names_in(&[StmtKind::Expr(constant.value.clone()).at(constant.span)], &mut names);
            }
            js::Item::Statements(stmts) => js::names_in(stmts, &mut names),
            js::Item::Function(function) => js::names_in(
                &[StmtKind::Function(Box::new(function.clone())).at(function.span)],
                &mut names,
            ),
            js::Item::Namespace(namespace) => {
                names.insert(namespace.name.clone());
                for method in &namespace.methods {
                    js::names_in(
                        &[StmtKind::Function(Box::new(method.clone())).at(method.span)],
                        &mut names,
                    );
                }
            }
        }
    }
    for import in &module.imports {
        names.extend(import.named.iter().map(|(_, local)| local.clone()));
        names.extend(import.namespace.clone());
    }
    for package in &module.packages {
        names.extend(package.named.iter().map(|(_, local)| local.clone()));
        names.extend(package.default.clone().into_iter().chain(package.namespace.clone()));
    }
    for at in 0..module.items.len() {
        let js::Item::Const(constant) = &module.items[at] else {
            continue;
        };
        let value = constant.name.clone();
        if constant.export
            || !constant.fields.is_empty()
            || module.aliases.iter().any(|(local, _)| *local == value)
            || module.default_export.as_deref() == Some(value.as_str())
        {
            continue;
        }
        let mut reads = 0;
        for item in &module.items {
            item.visit_vars(&mut |v| reads += usize::from(v == value));
        }
        // Each member it's called through, in the order they're first called.
        let mut members: Vec<String> = Vec::new();
        let mut calls = 0;
        js::each_item_expr_mut(&mut module.items, &mut |e| {
            if let Some(member) = member_called(e, &value) {
                calls += 1;
                if !members.iter().any(|m| m == member) {
                    members.push(member.to_string());
                }
            }
        });
        if reads == 0
            || calls != reads
            || members
                .iter()
                .any(|m| !crate::names::is_identifier(m) || names.contains(m))
        {
            continue;
        }
        js::each_item_expr_mut(&mut module.items, &mut |e| {
            if let Some(member) = member_called(e, &value).map(str::to_string)
                && let ExprKind::Call(callee, _) = &mut e.kind
            {
                **callee = Expr::var(&member);
            }
        });
        let js::Item::Const(constant) = &module.items[at] else {
            unreachable!("found")
        };
        let pattern = (members.iter())
            .map(|member| (member.clone(), js::Pattern::Name(member.clone()), None))
            .collect();
        let destructure = StmtKind::Destructure {
            pattern: js::Pattern::Object(pattern, None),
            value: constant.value.clone(),
            mutable: false,
        };
        module.items[at] = js::Item::Statements(vec![destructure.at(constant.span)]);
        names.extend(members);
    }
}

/// The member of `value` that `e` calls, `value.member(..)`, where its
/// function reads no `this` (`Known::ThisUnread`).
fn member_called<'e>(e: &'e Expr, value: &str) -> Option<&'e str> {
    let ExprKind::Call(callee, _) = &e.kind else {
        return None;
    };
    let ExprKind::Member(of, member) = &callee.kind else {
        return None;
    };
    (e.facts.has(js::Known::ThisUnread) && matches!(&of.kind, ExprKind::Var(v) if v == value)).then_some(member)
}

/// Whether each read of `name` in `items` but the function at `at` is a
/// call of it by at most `arity` arguments: one that leaves its last
/// `undefined`s out, `unlink(sub)`, gives what the function gives on, as
/// a call of them would, only `arguments.length` telling (ADR 0313).
fn only_called(items: &[js::Item], at: usize, name: &str, arity: usize) -> bool {
    let (mut reads, mut calls) = (0, 0);
    let mut each = |e: &Expr| {
        calls += usize::from(matches!(&e.kind, ExprKind::Call(callee, args)
            if args.len() <= arity && matches!(&callee.kind, ExprKind::Var(v) if v == name)));
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
