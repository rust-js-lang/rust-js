//! A name given apart from another, `label$1` of `label`, or `Error$` of
//! JS's global, is that name where nothing it would shadow is read (ADR
//! 0352): an import's, or a function's or `const`'s of the module's own
//! that it doesn't export, where its module never mentions the name; a local's,
//! where every read in its item is of what it was before, as JS's scopes
//! resolve them (ADR 0357): `const n` in a block beside another `n`.

use crate::collections::{IndexMap, IndexSet};
use crate::js::scopes;
use crate::js::{self, Expr, ExprKind, Name, Stmt, StmtKind};

pub(super) fn module(module: &mut js::Module) {
    // Each item's locals first, then the imports of what's left: an import
    // given apart from a local the item reclaims, `add$2` of a closure's
    // `add$1` that's `add` now, is `add$1` (ADR 1150).
    // The module's own items' words, a split function's entry among them.
    let own: IndexSet<String> = (module.items.iter())
        .filter_map(|item| match item {
            js::Item::Function(function) => Some(&function.name),
            _ => item.declared_name(),
        })
        .map(|name| name.word().to_string())
        .collect();
    for item in &mut module.items {
        locals(item, &own);
    }
    aliased_imports(module);
    imports(module);
    own_items(module);
}

/// Each name lowering made not spelled yet, `value#1` (`js::Name::made`),
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
            // By its word, as it's spelled below: a made name's word is
            // its own in its item, each of its clones' too.
            // Not a loop's counter, spelled once the program's names are
            // reclaimed (`counters::spelled`, #2032).
            if name.made_base().is_some() && !name.is_counter() && !unspelled.iter().any(|u: &Name| u.is(name.word())) {
                unspelled.push(name.clone());
            }
        });
        if unspelled.is_empty() {
            continue;
        }
        // Of a holder of another variable's value (`Name::derived`), what
        // that variable is of, found by its id: `value` of `value$1`, or of
        // `value#1`, unspelled yet, which it's given apart from (ADR 1150).
        let mut sources: IndexMap<String, String> = IndexMap::default();
        names_mut(item, false, &mut |name| {
            // Not a flag (`Suffix`), made of its variable's root, which
            // nothing names again.
            for derived in unspelled
                .iter()
                .filter(|u| u.suffix_tail().is_none() && name.is_origin_of(u))
            {
                let root = name.made_base().unwrap_or_else(|| name.root());
                sources
                    .entry(derived.word().to_string())
                    .or_insert_with(|| root.to_string());
            }
        });
        let mut tree = scopes::Tree::of(item);
        let mut spelled: IndexMap<String, String> = IndexMap::default();
        for name in unspelled {
            // Its source's, else what it was made of.
            let base = sources
                .get(name.word())
                .map_or_else(|| name.made_base().expect("made"), String::as_str);
            let in_scope = tree.as_mut().and_then(|tree| {
                let to = std::iter::once(base.to_string())
                    .chain((1..).map(|k| crate::names::given_apart(base, k)))
                    .filter(|to| !crate::names::kept_word(to))
                    .take(64)
                    .find(|to| tree.renames(name.word(), to))?;
                tree.rename(name.word(), &to);
                Some(to)
            });
            let to = in_scope.unwrap_or_else(|| crate::names::fresh_apart(&mut taken, base));
            taken.insert(to.clone());
            spelled.insert(name.into_word(), to);
        }
        names_mut(item, true, &mut |name| {
            if let Some(to) = spelled.get(name.word()) {
                name.respell(to);
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
        .cloned()
        .chain(
            module
                .imports
                .iter()
                .flat_map(js::Import::locals)
                .map(|l| l.word().to_string()),
        )
        .collect();
    for local in locals {
        // How often the module reads it.
        let mut reads = 0;
        js::each_item_expr_mut(&mut module.items, &mut |e| {
            reads += usize::from(matches!(&e.kind, ExprKind::Var(v) if v.is(&local)));
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
                        if !value.facts.has(js::Known::Live) && matches!(&value.kind, ExprKind::Var(v) if v.is(&local)))
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
            // By its word: a declaration of it anywhere, another
            // function's own variable too, takes the import's name.
            names_mut(item, false, &mut |name| declared += usize::from(name.is(alias.word())));
            if item.declared_name().map(js::Name::word) == Some(alias.word()) {
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
            .any(|l| alias.is(l))
            || (module.imports.iter().flat_map(js::Import::locals)).any(|l| alias.is(l.word()));
        if declared != 1 || imported {
            continue;
        }
        for item in &mut module.items {
            for function in item.functions_mut() {
                function.body.retain(|s| {
                    !matches!(&s.kind, StmtKind::Const(name, value)
                        if *name == alias && matches!(&value.kind, ExprKind::Var(v) if v.is(&local)))
                });
            }
        }
        let rename = |name: &mut String| {
            if *name == local {
                *name = alias.word().to_string();
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
            import.locals_mut().for_each(|name| {
                if name.is(&local) {
                    name.respell(alias.word());
                }
            });
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
        .cloned()
        .chain(
            module
                .imports
                .iter()
                .flat_map(js::Import::locals)
                .map(|l| l.word().to_string()),
        )
        .collect();
    // What the module mentions, read once: a name reclaimed is one more
    // base mentioned, and the name it was, `label$1`, is no base, as a base
    // has no `$` (`reclaimable`).
    let mut mentioned = mentions(module);
    for local in locals {
        // Its base, or given apart from it less than it is, the first no
        // name of the module's is.
        let Some(base) = reclaimable(&local, &IndexSet::default()).and_then(|base| {
            let given = crate::names::apart(&local).map_or(0, |(_, k)| k);
            std::iter::once(base.to_string())
                .chain((1..given).map(|k| crate::names::given_apart(base, k)))
                .find(|to| !mentioned.contains(to))
        }) else {
            continue;
        };
        // The one rule, `local` named `base`, of a name and of the text an
        // import's local is still (`Name` in M3's later steps).
        let to = |name: &str| (name == local).then_some(base.as_str());
        let mut rename = |name: &mut String| {
            if let Some(to) = to(name) {
                *name = to.to_string();
            }
        };
        let mut respell = |name: &mut Name| {
            if let Some(to) = to(name.word()) {
                name.respell(to);
            }
        };
        for package in &mut module.packages {
            (package.default.iter_mut())
                .chain(package.named.iter_mut().map(|(_, l)| l))
                .chain(&mut package.namespace)
                .for_each(&mut rename);
        }
        for import in &mut module.imports {
            import.locals_mut().for_each(&mut respell);
        }
        module.default_export.iter_mut().for_each(&mut rename);
        for item in &mut module.items {
            names_mut(item, true, &mut respell);
        }
        // What it was is free now, for an import after it.
        mentioned.shift_remove(&local);
        mentioned.insert(base);
    }
}

/// Each function or `const` of the module's own, not exported, of a name
/// its module never mentions: another module reads an exported one by the
/// name it has.
fn own_items(module: &mut js::Module) {
    let own: Vec<String> = (module.items.iter())
        .filter_map(|item| match item {
            js::Item::Function(function) if !function.export => Some(function.name.word().to_string()),
            js::Item::Const(constant) if !constant.export => Some(constant.name.word().to_string()),
            _ => None,
        })
        .collect();
    let mut mentioned = mentions(module);
    for local in own {
        let Some(base) = reclaimable(&local, &mentioned).map(str::to_string) else {
            continue;
        };
        // The one rule, `local` named `base`, of a name and of the text an
        // import's local is still (`Name` in M3's later steps).
        let to = |name: &str| (name == local).then_some(base.as_str());
        let mut rename = |name: &mut String| {
            if let Some(to) = to(name) {
                *name = to.to_string();
            }
        };
        let mut respell = |name: &mut Name| {
            if let Some(to) = to(name.word()) {
                name.respell(to);
            }
        };
        module.default_export.iter_mut().for_each(&mut rename);
        // One its `export { .. as .. }` names, exported by that alone (ADR
        // 0596, amended).
        module.aliases.iter_mut().map(|(local, _)| local).for_each(&mut rename);
        for item in &mut module.items {
            if let js::Item::Const(constant) = item {
                respell(&mut constant.name);
            }
            names_mut(item, true, &mut respell);
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
        if module.aliases.iter().any(|alias| own_name(alias) && name.is(&alias.0)) {
            *export = true;
            folded.insert(name.word().to_string());
        }
    }
    module
        .aliases
        .retain(|alias| !(own_name(alias) && folded.contains(alias.0.as_str())));
}

/// Each variable `item` declares its own spelling: one whose word another
/// variable it declares has too, by its id, spelled apart from every name it
/// mentions, `item$9`, so `locals` reclaims each by itself, an arrow's
/// `(item$2) => ..` as `(item) => ..` though another `item$2` beside it
/// can't be (ADR 1150). The first declared keeps its word.
fn apart_by_variable(item: &mut js::Item) {
    // A closure's parameter of a name declared elsewhere in the item too,
    // a capture's of `let shared = Rc::clone(&shared); move || ..` that
    // lowering names as the variable it holds, is a binding of its own in
    // JS: its own variable inside the closure, by an id of its own.
    let mut declarations: IndexMap<Name, usize> = IndexMap::default();
    names_mut(item, false, &mut |name| {
        *declarations.entry(name.clone()).or_default() += 1
    });
    js::each_item_expr_mut(std::slice::from_mut(item), &mut |e| {
        let (ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body)) = &mut e.kind else {
            return;
        };
        let twice: Vec<Name> = (params.iter().flat_map(|p| p.bound()))
            .filter(|name| declarations.get(*name).is_some_and(|&n| n > 1))
            .cloned()
            .collect();
        for name in twice {
            let own = name.rebound();
            // Its holders too, of it by its id (`Name::derived`).
            let mut change = |n: &mut Name| {
                if *n == name {
                    *n = own.clone();
                } else {
                    n.rebind_origin(&name, &own);
                }
            };
            params.iter_mut().for_each(|p| p.each_name_mut(&mut change));
            declared_mut(body, &mut change);
            js::each_expr_mut(body, &mut |e| expr_mut(e, true, &mut change));
        }
    });
    let mut taken: IndexSet<String> = IndexSet::default();
    names_mut(item, true, &mut |name| _ = taken.insert(name.word().to_string()));
    let mut first: IndexMap<String, Name> = IndexMap::default();
    let mut apart: IndexMap<Name, String> = IndexMap::default();
    // A word given apart, `item$2`, which reclaiming may make plainer; one
    // that isn't, `knownKeys` of two functions' own, is as plain as it gets.
    names_mut(item, false, &mut |name| match first.get(name.word()) {
        None => _ = first.insert(name.word().to_string(), name.clone()),
        Some(kept) if kept == name || apart.contains_key(name) || crate::names::apart(name.word()).is_none() => {}
        Some(_) => {
            let to = crate::names::fresh_apart(&mut taken, name.word());
            apart.insert(name.clone(), to);
        }
    });
    if !apart.is_empty() {
        names_mut(item, true, &mut |name| {
            if let Some(to) = apart.get(name) {
                name.respell(to.as_str());
            }
        });
    }
}

/// Each local of `item`'s, of a name where each read is still of what it
/// was: its base, `n` of `n$2`, or else the first of `n$1` on that is.
/// The scope tree is asked and renamed as each is decided, and the item
/// written once at the end, each name as what the renames in their order
/// made of it: written after each, a function of four thousand lines with
/// six hundred such names took 60 ms being walked, and as long again in
/// the tree.
fn locals(item: &mut js::Item, own: &IndexSet<String>) {
    // Each variable spelled apart only where its item has scopes to reclaim
    // it in, else as it was.
    let mut apart = item.clone();
    apart_by_variable(&mut apart);
    let Some(mut tree) = scopes::Tree::of(&apart) else {
        return;
    };
    *item = apart;
    let mut declared = Vec::new();
    // The words of the locals the program named (`Name::is_the_programs`).
    let mut programs: IndexSet<String> = IndexSet::default();
    // By its word, as the scopes and the renames below are keyed: what's
    // reclaimed is a spelling (#1681 change 3).
    names_mut(item, false, &mut |name| {
        if name.is_the_programs() {
            programs.insert(name.word().to_string());
        }
        if !declared.iter().any(|d: &Name| d.is(name.word())) {
            declared.push(name.clone());
        }
    });
    // Each name renamed, as it's written in the item, to what it is now.
    let mut renamed: IndexMap<String, String> = IndexMap::default();
    // The program's names first: where a local lowering made has the
    // spelling a local the program named, given apart, would reclaim,
    // `state` of `state$3`, it gives way, spelled apart from every name the
    // item has, then reclaims what's left after the program's (ADR 1150).
    let wanted: IndexSet<&str> = (programs.iter())
        .filter_map(|word| crate::names::apart(word).map(|(base, _)| base))
        .collect();
    let mut taken: IndexSet<String> = tree.names().map(str::to_owned).collect();
    for local in &declared {
        if programs.contains(local.word()) || !wanted.contains(local.word()) {
            continue;
        }
        let aside = crate::names::fresh_in(&mut taken, local.word());
        if tree.renames(local.word(), &aside) {
            tree.rename(local.word(), &aside);
            renamed.insert(local.word().to_string(), aside);
        }
    }
    // An arrow's parameters first, before what its function declares
    // around it, which reclaims a name around it then, as `(item) => ..`.
    let mut arrows: IndexSet<String> = IndexSet::default();
    js::each_item_expr_mut(std::slice::from_mut(item), &mut |e| {
        if let ExprKind::Arrow(params, _) | ExprKind::AsyncArrow(params, _) = &e.kind {
            arrows.extend(params.iter().flat_map(|p| p.names()).map(str::to_string));
        }
    });
    declared.sort_by_key(|local| (!programs.contains(local.word()), !arrows.contains(local.word())));
    // Of the entries the item reads, the module's own, whose word its
    // module has: the item read it before the split, so no local reclaims
    // it. Not an import's, which an import alias gives way to a local of,
    // as it did before the split, `import { add as add$1 }`; nor its own
    // name's, a body's, `contentSegment$`, which never calls its entry:
    // its local takes that word, as main's did.
    let itself = match &*item {
        js::Item::Function(function) => function.name.word().strip_suffix('$').map(str::to_string),
        _ => None,
    };
    let entries_read: IndexSet<String> = (entries_read(item).into_iter())
        .filter(|entry| own.contains(entry) && itself.as_ref() != Some(entry))
        .collect();
    for local in declared {
        // As it's spelled now, of one given way above.
        let now = renamed
            .get(local.word())
            .cloned()
            .unwrap_or_else(|| local.word().to_string());
        let Some(base) = reclaimable(&now, &IndexSet::default()) else {
            continue;
        };
        let suffix: usize = now[base.len() + 1..].parse().unwrap_or(0);
        let Some(to) = std::iter::once(base.to_string())
            .chain((1..suffix).map(|i| crate::names::given_apart(base, i)))
            .find(|to| !entries_read.contains(to) && tree.renames(&now, to))
        else {
            continue;
        };
        tree.rename(&now, &to);
        // What was renamed to `local` before is `to` now, as writing the
        // item each time would have made it.
        for was in renamed.values_mut() {
            if *was == now {
                was.clone_from(&to);
            }
        }
        renamed.insert(local.into_word(), to);
    }
    if !renamed.is_empty() {
        names_mut(item, true, &mut |name| {
            if let Some(to) = renamed.get(name.word()) {
                name.respell(to);
            }
        });
    }
}

/// The name `name`, `label$1` or `Error$`, was given apart from, where it
/// may be that name: one none of `mentioned` is, nor a word JS keeps.
fn reclaimable<'a>(name: &'a str, mentioned: &IndexSet<String>) -> Option<&'a str> {
    // `label$1`, given apart, or `Error$`, named apart for good.
    let given = crate::names::apart(name);
    let base = match given {
        Some((base, _)) => base,
        None => name.strip_suffix('$')?,
    };
    // A word JS keeps given apart, `arguments$$1` of `arguments$`, is its
    // own spelling where nothing else is so, never the word (ADR 1150).
    if given.is_some() && base.strip_suffix('$').is_some_and(crate::names::kept_word) && !mentioned.contains(base) {
        return Some(base);
    }
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
        if let Some(name) = item.declared_name() {
            names.insert(name.word().to_string());
        }
        names_mut(item, true, &mut |name| {
            names.insert(name.word().to_string());
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
            .flat_map(|i| i.locals().map(|l| l.word().to_string())),
    );
    names.extend(module.default_export.iter().cloned());
    names.extend(module.aliases.iter().map(|(local, _)| local.clone()));
    names
}

/// Each entry whose split body `item` reads, `label` of `label$(false)`,
/// the module's own or one it imports, its own name's too, `parse` of
/// `parse$`'s: a word it reads, as it read the entry's before the split,
/// so no temporary is named for it (#2403, ADR 1193). React's `const value
/// = lanesToEventPriority$(..)` was `const lanesToEventPriority`, the
/// imported body's entry's word; the guide's `parse$` read `const parse =
/// $parseInt(..)`. `locals` reclaims fewer of them.
pub(super) fn entries_read(item: &mut js::Item) -> IndexSet<String> {
    let mut entries: IndexSet<String> = IndexSet::default();
    names_mut(item, true, &mut |name| {
        // A module's name, its own function's or an import's: a local's
        // given apart, `String$`, is no body.
        if name.is_module()
            && let Some(entry) = name.word().strip_suffix('$')
        {
            entries.insert(entry.to_string());
        }
    });
    entries
}

/// The names `item` declares, anywhere in it, and with `reads` those it
/// reads too, its own name a function's.
pub(super) fn names_mut(item: &mut js::Item, reads: bool, f: &mut dyn FnMut(&mut Name)) {
    let function = |function: &mut js::Function, f: &mut dyn FnMut(&mut Name)| {
        if reads {
            f(&mut function.name);
        }
        function.params.iter_mut().for_each(|p| pattern_mut(p, f));
        declared_mut(&mut function.body, f);
        js::each_expr_mut(&mut function.body, &mut |e| expr_mut(e, reads, f));
    };
    match item {
        js::Item::Function(item) => function(item, f),
        js::Item::Namespace(namespace) => {
            (namespace.constants.iter_mut()).for_each(|c| c.value.each_mut(&mut |e| expr_mut(e, reads, f)));
            namespace.methods.iter_mut().for_each(|m| function(m, f));
        }
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
fn expr_mut(e: &mut Expr, reads: bool, f: &mut dyn FnMut(&mut Name)) {
    match &mut e.kind {
        ExprKind::Var(name) if reads => f(name),
        ExprKind::Stringed(_) if reads => f(&mut Name::global("String")),
        ExprKind::Converted(conversion, _) if reads => f(&mut Name::global(conversion.global())),
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
fn declared_mut(stmts: &mut [Stmt], f: &mut dyn FnMut(&mut Name)) {
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

fn pattern_mut(pattern: &mut js::Pattern, f: &mut dyn FnMut(&mut Name)) {
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
    // An inner binding may shadow a name the module's scope declares, an
    // `on_load!` body's `const x`, as JS's scopes let it: `(x$1) => x$1` is
    // `(x) => x`. Never where its scope reads the module's own name of the
    // word, an import's `value` it calls: that one stays `value$1`. And the
    // module spelled so, the import's read its parameter's, `js::validate`
    // refuses (`Problem::ModuleShadowed`, ADR 1150).
    #[test]
    fn an_inner_name_shadows_a_modules_only_where_it_reads_none_of_it() {
        let at = |kind: StmtKind| kind.at(js::Span::NONE);
        let x = js::Name::declared("x".to_string());
        let alone = js::Name::declared("x$1".to_string());
        let imported = js::Name::module("value");
        let reading = js::Name::declared("value$1".to_string());
        let arrow = |param: &js::Name, reads: Vec<Expr>| {
            Expr::arrow(
                vec![js::Pattern::Name(param.clone())],
                vec![at(StmtKind::Return(Some(Expr::call(Expr::var(param), reads))))],
            )
        };
        let module = |reading: &js::Name| js::Module {
            items: vec![js::Item::Statements(vec![
                at(StmtKind::Const(x.clone(), Expr::num(1.0))),
                at(StmtKind::Const("f".into(), arrow(&alone, Vec::new()))),
                at(StmtKind::Const("g".into(), arrow(reading, vec![Expr::var(&imported)]))),
            ])],
            imports: vec![js::Import {
                named: vec![("value".to_string(), imported.clone())],
                namespace: None,
                from: "./value.js".to_string(),
                span: js::Span::NONE,
            }],
            ..js::Module::default()
        };
        let params = |module: &js::Module| -> Vec<String> {
            let js::Item::Statements(stmts) = &module.items[0] else {
                unreachable!("statements")
            };
            (stmts[1..].iter())
                .map(|s| match &s.kind {
                    StmtKind::Const(
                        _,
                        Expr {
                            kind: ExprKind::Arrow(params, _),
                            ..
                        },
                    ) => params[0].names()[0].to_string(),
                    _ => unreachable!("an arrow"),
                })
                .collect()
        };
        let mut prepared = module(&reading);
        crate::prepare::module(&mut prepared, crate::prepare::Passes::default());
        assert_eq!(params(&prepared), ["x", "value$1"]);
        assert_eq!(js::validate::module(&prepared), []);
        let wrong = module(&js::Name::declared("value".to_string()));
        assert_eq!(
            js::validate::module(&wrong),
            [js::validate::Problem::ModuleShadowed("value".to_string())]
        );
    }

    #[test]
    fn an_unspelled_name_is_spelled_apart_from_the_modules() {
        let at = |kind: StmtKind| kind.at(js::Span::NONE);
        let param = |name: &js::Name| vec![js::Pattern::Name(name.clone())];
        let item = js::Name::made("item", 1);
        let calls = Expr::arrow(
            param(&item),
            vec![at(StmtKind::Return(Some(Expr::call(
                Expr::var("item"),
                vec![Expr::var(&item)],
            ))))],
        );
        let value = js::Name::made("value", 1);
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
                function(
                    "item",
                    param(&"x".into()),
                    vec![at(StmtKind::Return(Some(Expr::var("x"))))],
                ),
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
        let value = js::Name::made("value", 1);
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
