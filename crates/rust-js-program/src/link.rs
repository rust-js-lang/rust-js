//! Resolve symbolic module references after lowering and reachability:
//! what each module imports, each import a `Name` of its own, which every
//! read of what it imports is. It reads no words: preparation spells each
//! import apart from the module's names, and a local apart from a runtime
//! helper's read (`prepare::import_names`, ADR 1150, #1975).

use crate::collections::{IndexMap, IndexSet};
use crate::js::{ExprKind, Name};
use crate::program::{ImportRequest, Lowered, LoweredImport, LoweredModule, Unlinked};
use std::collections::BTreeMap;

/// Proof that import resolution and runtime dependency closure completed.
/// The private field prevents other phases from constructing this directly.
pub struct Linked(Lowered);

impl Linked {
    pub fn into_program(self) -> Lowered {
        self.0
    }
}

/// Consume symbolic output; printing only receives the completed linked crate.
pub fn link(unlinked: Unlinked) -> Linked {
    let mut modules: Vec<LoweredModule> = unlinked
        .modules
        .into_iter()
        .map(|mut item| {
            resolve(&mut item.module, &item.imports, item.reserved_names);
            // Each helper its tree reads, asked for by being written.
            let mut written = Vec::new();
            let mut finds_places_by_address = false;
            crate::js::each_item_expr_mut(&mut item.module.items, &mut |e| {
                finds_places_by_address |= matches!(e.helper_name(), Some("$ptrFrom" | "$ptrAt"));
                // And what a handle's storage writes with, which preparation
                // writes out (`Storage::helper`, #968).
                let stored = match &e.kind {
                    crate::js::ExprKind::Handle(_, storage) | crate::js::ExprKind::Pair(_, _, storage) => {
                        storage.helper()
                    }
                    _ => None,
                };
                for name in e.helper_name().into_iter().chain(stored) {
                    written.extend(crate::runtime::helper_of(name));
                }
            });
            // A module that finds places by their addresses exposes each as
            // it's asked, from when it loads, before anything asks one
            // (ADR 0562, amended, #1426).
            if finds_places_by_address {
                let expose = crate::js::Expr::call_helper("$exposeAddresses", Vec::new());
                let statement = crate::js::StmtKind::Expr(expose).at(crate::js::Span::NONE);
                item.module
                    .items
                    .insert(0, crate::js::Item::Statements(vec![statement]));
                written.extend(crate::runtime::helper_of("$exposeAddresses"));
            }
            written.sort();
            written.dedup();
            item.module.runtime = written;
            item.module
        })
        .collect();
    keep_imported(&mut modules);
    Linked(Lowered {
        library: unlinked.library,
        offered_paths: unlinked.offered_paths,
        sources: unlinked.sources,
        modules,
        tests: unlinked.tests,
    })
}

/// A module of types alone that another module's JS imports, or
/// re-exports, `export * as bar from "./foo/bar.js"`, keeps its JS, though
/// nothing is in it: only a declaration's `import type` may name a module
/// whose JS isn't written (ADR 1005, amended).
fn keep_imported(modules: &mut [LoweredModule]) {
    let imported: IndexSet<Vec<String>> = (modules.iter())
        .flat_map(|module| {
            let reexported = module.reexports.iter().filter_map(|reexport| match &reexport.from {
                crate::js::ImportFrom::Module(path) => Some(path.clone()),
                crate::js::ImportFrom::Specifier(_) => None,
            });
            module
                .imports
                .iter()
                .map(|import| import.path.clone())
                .chain(reexported)
        })
        .collect();
    for module in modules.iter_mut().filter(|module| imported.contains(&module.path)) {
        module.types_only = false;
    }
}

fn resolve(module: &mut LoweredModule, imports: &[ImportRequest], reserved: IndexSet<String>) {
    // The other modules' items the module reads, by their symbols.
    let mut read: IndexSet<crate::js::Symbol> = IndexSet::default();
    crate::js::each_item_expr_mut(&mut module.items, &mut |e| {
        if let ExprKind::Symbol(symbol) = &e.kind {
            read.insert(symbol.clone());
        }
    });
    // One import of each `use` of a module, where it's written (rd649): by
    // its path and its `use`'s place.
    type Written = (Vec<String>, (u32, u32));
    let mut grouped: BTreeMap<Written, Vec<(String, crate::js::Name)>> = BTreeMap::new();
    let mut namespaces: IndexMap<Written, crate::js::Name> = IndexMap::default();
    // An import of what the module reads: one its lowering asked for whose
    // read tidy took out, a test of a numbered enum's variant an
    // exhaustive `match` needn't write, `Dark` (ADR 0722), isn't written.
    let mut apart = Vec::new();
    let replacements: IndexMap<_, _> = imports
        .iter()
        .filter(|request| request.always || read.contains(&request.symbol))
        .map(|request| {
            // Its own `Name`, the word its `use` gives it, which preparation
            // spells apart from the module's other names; one the module
            // reads by its name already has it, as it is.
            let alias = Name::imported(&request.local);
            if !request.always {
                apart.push(alias.clone());
            }
            let key = (request.path.clone(), (request.at.lo, request.at.hi));
            let named = grouped.entry(key.clone()).or_default();
            // The module itself, its own `import * as` (rd114, ADR 1018).
            match request.export == crate::js::NAMESPACE {
                true => _ = namespaces.insert(key, alias.clone()),
                false => named.push((request.export.clone(), alias.clone())),
            }
            (request.symbol.clone(), alias)
        })
        .collect();
    module.imports = grouped
        .into_iter()
        .map(|(key, named)| {
            let namespace = namespaces.swap_remove(&key);
            let (path, (lo, hi)) = key;
            LoweredImport {
                path,
                named,
                namespace,
                span: crate::js::Span { lo, hi },
            }
        })
        .collect();
    // Each read of another module's item, of its import's `Name`.
    crate::js::each_item_expr_mut(&mut module.items, &mut |e| {
        if let ExprKind::Symbol(symbol) = &e.kind {
            let alias = replacements
                .get(symbol)
                .expect("every symbolic module reference has a dependency");
            e.kind = ExprKind::Var(alias.clone());
        }
    });
    module.import_names = crate::js::ImportNames { reserved, apart };
}
