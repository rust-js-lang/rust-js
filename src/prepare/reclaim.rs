//! A name given apart from another, `label$1` of `label`, or `Error$` of
//! JS's global, is that name where nothing it would shadow is read (ADR
//! 0352): an import's, or a function's or `const`'s of the module's own
//! that it doesn't export, where its module never mentions the name; a local's,
//! where every read in its item is of what it was before, as JS's scopes
//! resolve them (ADR 0357): `const n` in a block beside another `n`.

use crate::collections::{IndexMap, IndexSet};
use crate::js::scopes;
use crate::js::{self, Expr, ExprKind, Stmt, StmtKind};

pub(super) fn module(module: &mut js::Module) {
    aliased_imports(module);
    imports(module);
    own_items(module);
    for item in &mut module.items {
        locals(item);
    }
}

/// Each name lowering made not spelled yet, `value#1` (`names::unspelled`),
/// spelled: its word, or given apart from it, `value$1`, the first that
/// JS's scopes let it be, which takes no other's read and lets none take
/// its own, as `locals` reclaims a name (ADR 0357); the program's names,
/// all made by now, keep theirs. Of an item whose scopes declare a name
/// twice, which has no tree to ask, given apart from every name the module
/// has, which no scope can mistake for another's. Not one of the passes
/// `--prepare-without` leaves out: a module is never printed with one
/// (`js::validate`, #1681 change 3, M2).
pub(super) fn spelled(module: &mut js::Module) {
    let mut taken = mentions(module);
    for item in &mut module.items {
        let mut unspelled = Vec::new();
        names_mut(item, true, &mut |name| {
            if crate::names::unspelled_base(name).is_some() && !unspelled.contains(name) {
                unspelled.push(name.clone());
            }
        });
        if unspelled.is_empty() {
            continue;
        }
        let mut tree = scopes::Tree::of(item);
        let mut spelled: IndexMap<String, String> = IndexMap::default();
        for name in unspelled {
            let base = crate::names::unspelled_base(&name).expect("unspelled");
            let in_scope = tree.as_mut().and_then(|tree| {
                let to = std::iter::once(base.to_string())
                    .chain((1..).map(|k| crate::names::given_apart(base, k)))
                    .filter(|to| !crate::names::kept_word(to))
                    .take(64)
                    .find(|to| tree.renames(&name, to))?;
                tree.rename(&name, &to);
                Some(to)
            });
            let to = in_scope.unwrap_or_else(|| crate::names::fresh_apart(&mut taken, base));
            taken.insert(to.clone());
            spelled.insert(name, to);
        }
        names_mut(item, true, &mut |name| {
            if let Some(to) = spelled.get(name) {
                name.clone_from(to);
            }
        });
    }
}

/// `import IS from "./objectIs.js"; .. const is = IS;`, an import its module
/// reads once, a `const` at a function's top: the import by that name,
/// `import is from "./objectIs.js";`, where the module declares the name
/// nowhere else (ADR 0616). Not another module's variable, `Known::Live`,
/// which its module may set before a later read of the `const` (ADR 0270,
/// amended).
fn aliased_imports(module: &mut js::Module) {
    let locals: Vec<String> = (module.packages.iter())
        .flat_map(|p| p.default.iter().chain(p.named.iter().map(|(_, local)| local)))
        .chain(module.imports.iter().flat_map(js::Import::locals))
        .cloned()
        .collect();
    for local in locals {
        // How often the module reads it.
        let mut reads = 0;
        js::each_item_expr_mut(&mut module.items, &mut |e| {
            reads += usize::from(matches!(&e.kind, ExprKind::Var(v) if *v == local));
        });
        if reads != 1 {
            continue;
        }
        // The `const` that reads it, at a function's top.
        let mut alias = None;
        for item in &mut module.items {
            for function in item.functions_mut() {
                if let Some(k) = function.body.iter().position(|s| {
                    matches!(&s.kind, StmtKind::Const(_, value)
                        if !value.facts.has(js::Known::Live) && matches!(&value.kind, ExprKind::Var(v) if *v == local))
                }) && let StmtKind::Const(name, _) = &function.body[k].kind
                {
                    alias = Some(name.clone());
                }
            }
        }
        let Some(alias) = alias else { continue };
        // Declared nowhere else in the module, nor its name otherwise.
        let mut declared = 0;
        for item in &mut module.items {
            names_mut(item, false, &mut |name| declared += usize::from(*name == alias));
            if item.declared() == Some(alias.as_str()) {
                declared += 1;
            }
        }
        let imported = (module.packages.iter())
            .flat_map(|p| {
                p.default
                    .iter()
                    .chain(p.named.iter().map(|(_, l)| l))
                    .chain(&p.namespace)
            })
            .chain(module.imports.iter().flat_map(js::Import::locals))
            .any(|l| *l == alias);
        if declared != 1 || imported {
            continue;
        }
        for item in &mut module.items {
            for function in item.functions_mut() {
                function.body.retain(|s| {
                    !matches!(&s.kind, StmtKind::Const(name, value)
                        if *name == alias && matches!(&value.kind, ExprKind::Var(v) if *v == local))
                });
            }
        }
        let rename = |name: &mut String| {
            if *name == local {
                name.clone_from(&alias);
            }
        };
        for package in &mut module.packages {
            package
                .default
                .iter_mut()
                .chain(package.named.iter_mut().map(|(_, l)| l))
                .for_each(rename);
        }
        for import in &mut module.imports {
            import.locals_mut().for_each(rename);
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
        .chain(module.imports.iter().flat_map(js::Import::locals))
        .cloned()
        .collect();
    // What the module mentions, read once: a name reclaimed is one more
    // base mentioned, and the name it was, `label$1`, is no base, as a base
    // has no `$` (`reclaimable`).
    let mut mentioned = mentions(module);
    for local in locals {
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
            import.locals_mut().for_each(&mut rename);
        }
        module.default_export.iter_mut().for_each(&mut rename);
        for item in &mut module.items {
            names_mut(item, true, &mut rename);
        }
        mentioned.insert(base);
    }
}

/// Each function or `const` of the module's own, not exported, of a name
/// its module never mentions: another module reads an exported one by the
/// name it has.
fn own_items(module: &mut js::Module) {
    let own: Vec<String> = (module.items.iter())
        .filter_map(|item| match item {
            js::Item::Function(function) if !function.export => Some(function.name.clone()),
            js::Item::Const(constant) if !constant.export => Some(constant.name.clone()),
            _ => None,
        })
        .collect();
    let mut mentioned = mentions(module);
    for local in own {
        let Some(base) = reclaimable(&local, &mentioned).map(str::to_string) else {
            continue;
        };
        let mut rename = |name: &mut String| {
            if *name == local {
                name.clone_from(&base);
            }
        };
        module.default_export.iter_mut().for_each(&mut rename);
        // One its `export { .. as .. }` names, exported by that alone (ADR
        // 0596, amended).
        module.aliases.iter_mut().map(|(local, _)| local).for_each(&mut rename);
        for item in &mut module.items {
            if let js::Item::Const(constant) = item {
                rename(&mut constant.name);
            }
            names_mut(item, true, &mut rename);
        }
        mentioned.insert(base);
    }
    // One exported by an alias of the name it has now, `fetch` of `export {
    // fetch$1 as fetch }` (#338): exported where it's declared, `export
    // function fetch()`, as one never named apart is.
    // Not a module variable's, `let getTime`, which `export { getTime }`
    // exports.
    let own_name = |(local, exported): &(String, String)| local == exported;
    let mut folded = IndexSet::default();
    for item in &mut module.items {
        let (name, export) = match item {
            js::Item::Function(function) => (&function.name, &mut function.export),
            js::Item::Const(constant) => (&constant.name, &mut constant.export),
            _ => continue,
        };
        if module.aliases.iter().any(|alias| own_name(alias) && alias.0 == *name) {
            *export = true;
            folded.insert(name.clone());
        }
    }
    module
        .aliases
        .retain(|alias| !(own_name(alias) && folded.contains(&alias.0)));
}

/// Each local of `item`'s, of a name where each read is still of what it
/// was: its base, `n` of `n$2`, or else the first of `n$1` on that is.
/// The scope tree is asked and renamed as each is decided, and the item
/// written once at the end, each name as what the renames in their order
/// made of it: written after each, a function of four thousand lines with
/// six hundred such names took 60 ms being walked, and as long again in
/// the tree.
fn locals(item: &mut js::Item) {
    let Some(mut tree) = scopes::Tree::of(item) else { return };
    let mut declared = Vec::new();
    names_mut(item, false, &mut |name| {
        if !declared.contains(name) {
            declared.push(name.clone());
        }
    });
    // Each name renamed, as it's written in the item, to what it is now.
    let mut renamed: IndexMap<String, String> = IndexMap::default();
    for local in declared {
        let Some(base) = reclaimable(&local, &IndexSet::default()) else {
            continue;
        };
        let suffix: usize = local[base.len() + 1..].parse().unwrap_or(0);
        let Some(to) = std::iter::once(base.to_string())
            .chain((1..suffix).map(|i| crate::names::given_apart(base, i)))
            .find(|to| tree.renames(&local, to))
        else {
            continue;
        };
        tree.rename(&local, &to);
        // What was renamed to `local` before is `to` now, as writing the
        // item each time would have made it.
        for now in renamed.values_mut() {
            if *now == local {
                now.clone_from(&to);
            }
        }
        renamed.insert(local, to);
    }
    if !renamed.is_empty() {
        names_mut(item, true, &mut |name| {
            if let Some(to) = renamed.get(name) {
                name.clone_from(to);
            }
        });
    }
}

/// The name `name`, `label$1` or `Error$`, was given apart from, where it
/// may be that name: one none of `mentioned` is, nor a word JS keeps.
fn reclaimable<'a>(name: &'a str, mentioned: &IndexSet<String>) -> Option<&'a str> {
    // `label$1`, given apart, or `Error$`, named apart for good.
    let base = match crate::names::apart(name) {
        Some((base, _)) => base,
        None => name.strip_suffix('$')?,
    };
    if base.is_empty() || base.contains('$') || mentioned.contains(base) {
        return None;
    }
    // A word JS keeps, `class`, is named apart for good.
    if crate::names::kept_word(base) {
        return None;
    }
    Some(base)
}

/// Each name the module mentions: its items' own, what they declare and
/// read, and its imports'.
pub(super) fn mentions(module: &mut js::Module) -> IndexSet<String> {
    let mut names = IndexSet::default();
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
    names.extend(module.imports.iter().flat_map(|i| i.locals().cloned()));
    names.extend(module.default_export.iter().cloned());
    names.extend(module.aliases.iter().map(|(local, _)| local.clone()));
    names
}

/// The names `item` declares, anywhere in it, and with `reads` those it
/// reads too, its own name a function's.
pub(super) fn names_mut(item: &mut js::Item, reads: bool, f: &mut dyn FnMut(&mut String)) {
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
            StmtKind::While { body, .. } | StmtKind::DoWhile { body, .. } | StmtKind::Labeled(_, body) => {
                declared_mut(body, f)
            }
            StmtKind::Switch { cases, default, .. } => {
                js::case_lists_mut(cases, default).for_each(|body| declared_mut(body, f));
            }
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
            | StmtKind::Compound(..)
            | StmtKind::Expr(_)
            | StmtKind::Break(_)
            | StmtKind::Continue(_)
            | StmtKind::Return(_)
            | StmtKind::Throw(_)
            | StmtKind::Directive(_)
            | StmtKind::Comments => {}
        }
    }
}

fn pattern_mut(pattern: &mut js::Pattern, f: &mut dyn FnMut(&mut String)) {
    pattern.names_mut().into_iter().for_each(f);
}

#[cfg(test)]
mod tests {
    use crate::js::{self, Expr, ExprKind, Stmt, StmtKind};

    // A name lowering made not spelled yet, `item#1`, is spelled by every
    // preparation, its passes all left out too, apart from each name its
    // module has: beside a module's function `item` its arrow reads, never
    // `item`, which would call the parameter; where the module has none of
    // its word, its word; and reclaimed to its word where JS's scopes let it
    // be (#1681 change 3, M2). The module then validates.
    #[test]
    fn an_unspelled_name_is_spelled_apart_from_the_modules() {
        let at = |kind: StmtKind| kind.at(js::Span::NONE);
        let param = |name: &str| vec![js::Pattern::Name(name.into())];
        let item = crate::names::unspelled("item", 1);
        let calls = Expr::arrow(
            param(&item),
            vec![at(StmtKind::Return(Some(Expr::call(
                Expr::var("item"),
                vec![Expr::var(&item)],
            ))))],
        );
        let value = crate::names::unspelled("value", 1);
        let identity = Expr::arrow(param(&value), vec![at(StmtKind::Return(Some(Expr::var(&value))))]);
        let function = |name: &str, params: Vec<js::Pattern>, body: Vec<Stmt>| {
            js::Item::Function(js::Function {
                name: name.into(),
                params,
                body,
                export: false,
                is_async: false,
                noinline: false,
                span: js::Span::NONE,
                name_span: js::Span::NONE,
                param_spans: Vec::new(),
                set_apart: false,
            })
        };
        let mapped = |arrow: Expr| {
            at(StmtKind::Expr(Expr::call(
                Expr::member(Expr::var("value"), "map"),
                vec![arrow],
            )))
        };
        let lowered = js::Module {
            items: vec![
                function("item", param("x"), vec![at(StmtKind::Return(Some(Expr::var("x"))))]),
                function(
                    "main",
                    Vec::new(),
                    vec![
                        at(StmtKind::Const("value".into(), Expr::num(1.0))),
                        mapped(identity),
                        mapped(calls),
                    ],
                ),
            ],
            ..js::Module::default()
        };
        let params = |module: &js::Module| -> Vec<String> {
            let js::Item::Function(main) = &module.items[1] else {
                unreachable!("a function")
            };
            (main.body[1..].iter())
                .map(|s| match &s.kind {
                    StmtKind::Expr(Expr {
                        kind: ExprKind::Call(_, args),
                        ..
                    }) => match &args[0].kind {
                        ExprKind::Arrow(params, _) => params[0].names()[0].to_string(),
                        _ => unreachable!("an arrow"),
                    },
                    _ => unreachable!("a call"),
                })
                .collect()
        };
        assert!(!js::validate::module(&lowered).is_empty());
        let mut module = lowered.clone();
        crate::prepare::module(&mut module, crate::prepare::Passes::default());
        assert_eq!(params(&module), ["value", "item$1"]);
        assert_eq!(js::validate::module(&module), []);
        let mut unreclaimed = lowered.clone();
        crate::prepare::module(
            &mut unreclaimed,
            crate::prepare::Passes::without(
                &crate::prepare::passes::PASSES
                    .iter()
                    .map(|(pass, _)| *pass)
                    .collect::<Vec<_>>(),
                false,
            ),
        );
        assert_eq!(params(&unreclaimed), ["value", "item$1"]);
        assert_eq!(js::validate::module(&unreclaimed), []);
        // Of an item whose scopes declare a name twice, which has no tree to
        // ask, apart from every name its module has, `value$1`.
        let value = crate::names::unspelled("value", 1);
        let identity = Expr::arrow(param(&value), vec![at(StmtKind::Return(Some(Expr::var(&value))))]);
        let twice = vec![
            at(StmtKind::Const("value".into(), Expr::num(1.0))),
            at(StmtKind::Let("a".into(), None)),
            at(StmtKind::Let("a".into(), None)),
            mapped(identity),
        ];
        let mut module = js::Module {
            items: vec![function("main", Vec::new(), twice)],
            ..js::Module::default()
        };
        crate::prepare::module(&mut module, crate::prepare::Passes::default());
        let js::Item::Function(main) = &module.items[0] else {
            unreachable!("a function")
        };
        let StmtKind::Expr(Expr {
            kind: ExprKind::Call(_, args),
            ..
        }) = &main.body[3].kind
        else {
            unreachable!("a call")
        };
        let ExprKind::Arrow(params, _) = &args[0].kind else {
            unreachable!("an arrow")
        };
        assert_eq!(params[0].names(), ["value$1"]);
    }
}
