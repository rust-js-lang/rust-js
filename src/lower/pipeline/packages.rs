//! What a module imports of JS modules and libraries' files: one `import`
//! of each, of what the module uses of it (ADR 0028), and of what it imports
//! for what that does when loaded (ADR 0110); and, once its items are
//! made, none of what nothing reads.

use super::super::analysis::UsesWritten;
use super::super::bindings::Export;
use super::super::library::Foreign;
use crate::js;
use crate::program::LoweredModule;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalModId};
use std::collections::{BTreeMap, HashMap, HashSet};

/// What `module` imports of each JS module, `imported`'s of it and what its
/// lowering `used`, each by its local name in `names`, its `import` where
/// its `use` is written: one of each `use` of it, an import of its own
/// (rd649), what no `use` names in the first's, which is where the source
/// first names the module (`package_named_at`). And what it imports for its
/// effects, `js::import!("./App.css")`, the specifiers of which are the
/// second.
pub(super) fn of_module(
    tcx: TyCtxt<'_>,
    module: LocalModId,
    imported: &BTreeMap<Export, HashSet<LocalModId>>,
    used: &dyn Fn(&Export) -> bool,
    names: &HashMap<Export, String>,
    (uses, foreign): (&UsesWritten, &Foreign<'_, '_>),
    written: &dyn Fn(Option<rustc_span::Span>) -> js::Span,
) -> (Vec<js::Package>, HashSet<String>) {
    type Use = Option<(rustc_span::BytePos, rustc_span::BytePos)>;
    let mut packages: BTreeMap<(&str, Use), js::Package> = BTreeMap::new();
    for export in imported
        .iter()
        .filter(|(export, users)| users.contains(&module) || used(export))
        .map(|(export, _)| export)
    {
        let (from, name) = export;
        let first = uses.package_at(tcx, foreign, from);
        let at = uses.export_at(tcx, foreign, (from, name)).or(first);
        let key = (from.as_str(), at.map(|at| (at.lo(), at.hi())));
        let package = packages.entry(key).or_insert_with(|| js::Package {
            from: from.clone(),
            default: None,
            named: Vec::new(),
            namespace: None,
            span: written(at),
            named_at: match at == first {
                true => written(uses.package_named_at(tcx, foreign, from)),
                false => written(at),
            },
        });
        let local = names[export].clone();
        match name.as_str() {
            "default" => package.default = Some(local),
            "*" => package.namespace = Some(local),
            _ => package.named.push((name.clone(), local)),
        }
    }
    let mut packages: Vec<js::Package> = packages.into_values().collect();
    // `js::import!("./App.css");`: `import "./App.css";`, for what a
    // module does when loaded, as a bundler's CSS does (ADR 0110), its
    // span the `js::import!`, as a `use`'s is its `use`, so the comments
    // above it are before its import (rd666, ADR 1069, amended); placed
    // nowhere, so it loads where ADR 1063 orders it, after what's named.
    let mut loaded = HashSet::new();
    for attr in super::super::bindings::marks(tcx, module, "import") {
        let Some(from) = attr.value_str().map(|s| s.to_string()) else {
            tcx.dcx()
                .span_err(attr.span(), "rust-js: write it `js::import!(\"./file.css\");`");
            continue;
        };
        loaded.insert(from.clone());
        if !packages.iter().any(|p| p.from == from) {
            packages.push(js::Package {
                from,
                default: None,
                named: Vec::new(),
                namespace: None,
                span: written(Some(attr.span())),
                named_at: js::Span::NONE,
            });
        }
    }
    (packages, loaded)
}

/// `module`'s imports of JS modules, but what nothing of its `lowered`
/// items reads: a library's item its lowering asked for, where tidy took
/// out what read it (ADR 0722), as the linker imports none of the crate's
/// modules' the module doesn't read; and what only `forwarded`, the calls
/// of functions re-exported in their place, read (rd575, ADR 1064). What
/// it imports for its effects, `loaded`, stays.
pub(super) fn unread(
    lowered: &mut LoweredModule,
    (imported, module): (&BTreeMap<Export, HashSet<LocalModId>>, LocalModId),
    loaded: &HashSet<String>,
    forwarded: &HashSet<String>,
) {
    let mut read = HashSet::new();
    for item in &lowered.items {
        item.visit_vars(&mut |name| {
            read.insert(name.to_string());
        });
    }
    let analyzed = |from: &str, export: &str| {
        (imported.get(&(from.to_string(), export.to_string()))).is_some_and(|users| users.contains(&module))
    };
    let only_forwarded = |local: &str| forwarded.contains(local) && !read.contains(local);
    lowered.packages.retain_mut(|package| {
        let named = package.named.len();
        (package.named).retain(|(export, local)| {
            read.contains(local) || (analyzed(&package.from, export) && !only_forwarded(local))
        });
        let unread = [&mut package.default, &mut package.namespace].map(|held| {
            let unread = held.as_deref().is_some_and(only_forwarded);
            held.take_if(|_| unread).is_some()
        });
        named == package.named.len() && unread == [false; 2]
            || !package.named.is_empty()
            || package.default.is_some()
            || package.namespace.is_some()
            || loaded.contains(&package.from)
    });
}

/// Of `packages`, the imports of a library's items each of `through` says
/// the module names through another of the crate's modules that re-exports
/// it, taken out, each a package's export, `(from, export)`, the item, and
/// where it's imported instead (rd650, ADR 0874, amended): each, that
/// module, what it exports it as, the name the module reads it by, which
/// stays, and the item, whose `use` it's imported at (rd649). A package
/// left of nothing is gone, but one imported for its effects, `loaded`.
pub(super) fn through_reexports<'t>(
    packages: &mut Vec<js::Package>,
    loaded: &HashSet<String>,
    through: impl Iterator<Item = (Export, DefId, &'t super::super::reexports::Through)>,
) -> Vec<super::crate_imports::Reimported> {
    let (mut moved, mut emptied) = (Vec::new(), HashSet::new());
    for ((from, export), id, through) in through {
        // Of whichever of the file's imports has it, one of each `use`.
        let local =
            (packages.iter_mut().filter(|package| package.from == from)).find_map(|package| match export.as_str() {
                "default" => package.default.take(),
                "*" => None,
                _ => {
                    (package.named.iter().position(|(named, _)| *named == export)).map(|at| package.named.remove(at).1)
                }
            });
        if let Some(local) = local {
            moved.push((through.module, through.export.clone(), local, id));
            emptied.insert(from);
        }
    }
    packages.retain(|package| {
        !emptied.contains(&package.from)
            || !package.named.is_empty()
            || package.default.is_some()
            || package.namespace.is_some()
            || loaded.contains(&package.from)
    });
    moved
}
