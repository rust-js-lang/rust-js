//! A name given apart from another, `label$1` of `label`, or `Error$` of
//! JS's global, is that name where nothing it would shadow is read (ADR
//! 0352): an import's, where its module never mentions the name; a local's,
//! where every read in its item is of what it was before, as JS's scopes
//! resolve them (ADR 0357): `const n` in a block beside another `n`.

use std::collections::HashSet;

use super::scopes;
use crate::js::{self, Expr, ExprKind, Stmt, StmtKind};

pub(super) fn module(module: &mut js::Module) {
    imports(module);
    // Not an `on_load!` body's, whose names are the module's (ADR 0267).
    for item in &mut module.items {
        if !matches!(item, js::Item::Statements(_)) {
            locals(item);
        }
    }
}

/// Each import's, of a name its module never mentions.
fn imports(module: &mut js::Module) {
    let locals: Vec<String> = (module.packages.iter())
        .flat_map(|p| {
            p.default
                .iter()
                .chain(p.named.iter().map(|(_, local)| local))
                .chain(&p.namespace)
        })
        .chain(
            module
                .imports
                .iter()
                .flat_map(|i| i.named.iter().map(|(_, local)| local)),
        )
        .cloned()
        .collect();
    for local in locals {
        let mentioned = mentions(module);
        let Some(base) = reclaimable(&local, &mentioned).map(str::to_string) else {
            continue;
        };
        let mut rename = |name: &mut String| {
            if *name == local {
                name.clone_from(&base);
            }
        };
        for package in &mut module.packages {
            (package.default.iter_mut())
                .chain(package.named.iter_mut().map(|(_, l)| l))
                .chain(&mut package.namespace)
                .for_each(&mut rename);
        }
        for import in &mut module.imports {
            import.named.iter_mut().map(|(_, l)| l).for_each(&mut rename);
        }
        module.default_export.iter_mut().for_each(&mut rename);
        for item in &mut module.items {
            names_mut(item, true, &mut rename);
        }
    }
}

/// Each local of `item`'s, of a name where each read is still of what it
/// was: its base, `n` of `n$2`, or else the first of `n$1` on that is.
fn locals(item: &mut js::Item) {
    let mut tried = HashSet::new();
    loop {
        let Some(before) = reads(item, None) else { return };
        let mut declared = Vec::new();
        names_mut(item, false, &mut |name| declared.push(name.clone()));
        let renamed = (declared.iter())
            .filter(|d| tried.insert((*d).clone()))
            .find_map(|local| {
                let base = reclaimable(local, &HashSet::new())?;
                let suffix: usize = local[base.len() + 1..].parse().unwrap_or(0);
                std::iter::once(base.to_string())
                    .chain((1..suffix).map(|i| format!("{base}${i}")))
                    .find(|to| reads(item, Some((local, to))).as_ref() == Some(&before))
                    .map(|to| (local.clone(), to))
            });
        let Some((local, to)) = renamed else { return };
        names_mut(item, true, &mut |name| {
            if *name == local {
                name.clone_from(&to);
            }
        });
    }
}

/// Each read in `item`, and what it's of, were `renamed`'s first name its
/// second.
fn reads(item: &js::Item, renamed: Option<(&str, &str)>) -> Option<Vec<scopes::Of>> {
    match item {
        js::Item::Function(function) => scopes::reads(function, renamed),
        js::Item::Namespace(namespace) => {
            let methods = namespace.methods.iter().map(|m| scopes::reads(m, renamed));
            methods
                .collect::<Option<Vec<_>>>()
                .map(|all| all.into_iter().flatten().collect())
        }
        js::Item::Const(constant) => scopes::reads_of(&constant.value, renamed),
        js::Item::Statements(_) => None,
    }
}

/// The name `name`, `label$1` or `Error$`, was given apart from, where it
/// may be that name: one none of `mentioned` is, nor a word JS keeps.
fn reclaimable<'a>(name: &'a str, mentioned: &HashSet<String>) -> Option<&'a str> {
    // The globals rust-js names apart from a Rust name, which JS lets a
    // variable be.
    const GLOBALS: &[&str] = &[
        "Math",
        "Error",
        "String",
        "WeakMap",
        "DataView",
        "ArrayBuffer",
        "Number",
        "BigInt",
        "Object",
    ];
    let (base, suffix) = name.rsplit_once('$')?;
    if base.is_empty() || base.contains('$') || !suffix.bytes().all(|b| b.is_ascii_digit()) || mentioned.contains(base)
    {
        return None;
    }
    // A word JS keeps, `class`, is named apart for good.
    if crate::names::js_ident(base) != base && !GLOBALS.contains(&base) {
        return None;
    }
    Some(base)
}

/// Each name the module mentions: its items' own, what they declare and
/// read, and its imports'.
fn mentions(module: &mut js::Module) -> HashSet<String> {
    let mut names = HashSet::new();
    for item in &mut module.items {
        if let Some(name) = item.declared() {
            names.insert(name.to_string());
        }
        names_mut(item, true, &mut |name| {
            names.insert(name.clone());
        });
    }
    for package in &module.packages {
        names.extend(
            (package.default.iter())
                .chain(package.named.iter().map(|(_, l)| l))
                .chain(&package.namespace)
                .cloned(),
        );
    }
    names.extend(
        module
            .imports
            .iter()
            .flat_map(|i| i.named.iter().map(|(_, l)| l.clone())),
    );
    names.extend(module.caches.iter().cloned());
    names.extend(module.default_export.iter().cloned());
    names
}

/// The names `item` declares, anywhere in it, and with `reads` those it
/// reads too, its own name a function's.
fn names_mut(item: &mut js::Item, reads: bool, f: &mut dyn FnMut(&mut String)) {
    let function = |function: &mut js::Function, f: &mut dyn FnMut(&mut String)| {
        if reads {
            f(&mut function.name);
        }
        function.params.iter_mut().for_each(|p| pattern_mut(p, f));
        declared_mut(&mut function.body, f);
        js::each_expr_mut(&mut function.body, &mut |e| expr_mut(e, reads, f));
    };
    match item {
        js::Item::Function(item) => function(item, f),
        js::Item::Namespace(namespace) => namespace.methods.iter_mut().for_each(|m| function(m, f)),
        js::Item::Const(constant) => constant.value.each_mut(&mut |e| expr_mut(e, reads, f)),
        js::Item::Statements(stmts) => {
            declared_mut(stmts, f);
            js::each_expr_mut(stmts, &mut |e| expr_mut(e, reads, f));
        }
    }
}

/// An expression's names: a variable or a component tag it reads, with
/// `reads`, `String` of a template's conversion; a closure's parameters,
/// and what its body declares.
fn expr_mut(e: &mut Expr, reads: bool, f: &mut dyn FnMut(&mut String)) {
    match &mut e.kind {
        ExprKind::Var(name) if reads => f(name),
        ExprKind::Stringed(_) if reads => f(&mut "String".to_string()),
        ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => {
            params.iter_mut().for_each(|p| pattern_mut(p, f));
            declared_mut(body, f);
        }
        ExprKind::Function(function) => {
            f(&mut function.name);
            function.params.iter_mut().for_each(|p| pattern_mut(p, f));
            declared_mut(&mut function.body, f);
        }
        _ => {}
    }
}

/// What `stmts` declare, in each of their blocks and the functions written
/// in them; not in their closures, which their expressions hold.
fn declared_mut(stmts: &mut [Stmt], f: &mut dyn FnMut(&mut String)) {
    for stmt in stmts {
        match &mut stmt.kind {
            StmtKind::Const(name, _) | StmtKind::Let(name, _) => f(name),
            StmtKind::Destructure { pattern, .. } => pattern_mut(pattern, f),
            StmtKind::ForOf { pattern, body, .. } => {
                pattern_mut(pattern, f);
                declared_mut(body, f);
            }
            StmtKind::For { name, body, .. } => {
                f(name);
                declared_mut(body, f);
            }
            StmtKind::If(_, a, b) => {
                declared_mut(a, f);
                if let Some(b) = b {
                    declared_mut(b, f);
                }
            }
            StmtKind::While { body, .. } | StmtKind::Labeled(_, body) => declared_mut(body, f),
            StmtKind::Try(a, b) => {
                declared_mut(a, f);
                declared_mut(b, f);
            }
            StmtKind::TryCatch(a, name, b) => {
                declared_mut(a, f);
                if let Some(name) = name {
                    f(name);
                }
                declared_mut(b, f);
            }
            StmtKind::Function(function) => {
                f(&mut function.name);
                function.params.iter_mut().for_each(|p| pattern_mut(p, f));
                declared_mut(&mut function.body, f);
            }
            StmtKind::Assign(..)
            | StmtKind::Expr(_)
            | StmtKind::Break(_)
            | StmtKind::Continue(_)
            | StmtKind::Return(_)
            | StmtKind::Throw(_)
            | StmtKind::Directive(_) => {}
        }
    }
}

fn pattern_mut(pattern: &mut js::Pattern, f: &mut dyn FnMut(&mut String)) {
    match pattern {
        js::Pattern::Name(name) => f(name),
        js::Pattern::Array(items) => items.iter_mut().flatten().for_each(f),
        js::Pattern::Object(fields, rest) => {
            fields.iter_mut().for_each(|(_, name, _)| f(name));
            rest.iter_mut().for_each(f);
        }
    }
}
