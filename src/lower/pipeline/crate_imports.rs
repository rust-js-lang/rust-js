//! What each module imports of the crate's other modules: each item by
//! what its module exports it as, by the importer's name for it, from its
//! module or one that re-exports it (rd650); and the requests the linker
//! names them by.

use super::Pass;
use crate::js;
use crate::lower::analysis::renamed_in;
use crate::lower::reexports::Through;
use crate::lower::{FnInfo, module_symbol};
use crate::program::ImportRequest;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalModId};
use std::collections::{BTreeMap, HashMap, HashSet};

/// An import of another of the crate's modules' items: the module whose
/// item it is, its name there, what it's imported as, the importer's name
/// for it, the module it's imported from, its own or one that re-exports it
/// (rd650), and what a `use` names of it, whose import it's in (rd649).
pub(super) type ImportTarget = (LocalModId, String, String, String, LocalModId, Option<DefId>);

/// A library's item a module reads by its name, imported from the crate's
/// module that re-exports it: that module, what it exports it as, the name
/// it's read by, and the item (rd650, rd649).
pub(super) type Reimported = (LocalModId, String, String, DefId);

/// What each module imports of another: what that module exports its item
/// as, its name, its default or the name a `pub use` gives it
/// (`default_exports::exported_as`), each by what the importer's `use`
/// names it, `import { SandpackClient as Sandpack }`, or by its name,
/// `import { unstable_now as getCurrentTime }`; from its module, or from
/// one that re-exports it, by what that exports it as (`through`, rd650).
/// And `copies`, each a copy of a function (ADR 0051, amended).
pub(super) fn of_crate(
    tcx: TyCtxt<'_>,
    (pass, fns): (&Pass, &HashMap<DefId, FnInfo>),
    copies: Vec<(LocalModId, (LocalModId, String, String, String))>,
    through: &HashMap<(LocalModId, DefId), Through>,
) -> HashMap<LocalModId, HashSet<ImportTarget>> {
    let mut targets: HashMap<LocalModId, HashSet<ImportTarget>> = HashMap::new();
    for &(from, id) in &pass.references {
        let info = &fns[&id];
        let name = info.owner.as_ref().unwrap_or(&info.name).clone();
        let (export, local, named) = match info.owner {
            // A class's member, by its class, which may be its module's
            // default (ADR 0761).
            Some(_) if let Some(class) = crate::lower::classes::class_of(tcx, id) => (
                crate::lower::default_exports::exported_as(tcx, class, &name),
                renamed_in(tcx, from).remove(&class).unwrap_or_else(|| name.clone()),
                Some(class),
            ),
            Some(_) => (name.clone(), name.clone(), None),
            None => (
                crate::lower::default_exports::exported_as(tcx, id, info.exported()),
                renamed_in(tcx, from).remove(&id).unwrap_or_else(|| name.clone()),
                Some(id),
            ),
        };
        let (export, of) = match through.get(&(from, id)) {
            Some(through) if info.owner.is_none() => (through.export.clone(), through.module),
            _ => (export, info.module),
        };
        (targets.entry(from).or_default()).insert((info.module, name, export, local, of, named));
    }
    for (from, (target, name, export, local)) in copies {
        targets
            .entry(from)
            .or_default()
            .insert((target, name, export, local, target, None));
    }
    // And a binding it reads through another that holds it, by its export
    // there, named as the importer's `use` names it (ADR 0633).
    for (from, holder, name, default, binding) in &pass.held {
        let local = renamed_in(tcx, *from).remove(binding).unwrap_or_else(|| name.clone());
        let export = if *default { "default".to_string() } else { name.clone() };
        let import = (*holder, name.clone(), export, local, *holder, Some(*binding));
        (targets.entry(*from).or_default()).insert(import);
    }
    targets
}

/// What `module` asks the linker to import of the crate's modules: each of
/// `imports`; each of `reimported`, a library's item it reads by its name,
/// imported from the module that re-exports it, `(module, export, local,
/// item)` (rd650); by what each imports, `import { MAX as LIMIT,
/// unstable_now as getCurrentTime }`. Then each module it imports of,
/// itself, which it reads where a path names a function through it,
/// `Plugin.extractEvents`, or one it re-exports, `reexporters`'s
/// `Plugin.registerEvents` (rd114, rd137, ADR 1018): after what it imports
/// by name, which keeps its name. Each where `at` says the `use` naming it
/// is, of what it's from and what it is, each `use` of a module an import
/// of its own (rd649).
pub(super) fn requests(
    tcx: TyCtxt<'_>,
    module: LocalModId,
    (imports, reimported, reexporters): (HashSet<ImportTarget>, Vec<Reimported>, impl Iterator<Item = LocalModId>),
    paths: &HashMap<LocalModId, Vec<String>>,
    at: &dyn Fn(LocalModId, Option<DefId>) -> js::Span,
) -> Vec<ImportRequest> {
    let targets = imports.iter().map(|&(target, ..)| target).chain(reexporters);
    let through: BTreeMap<&Vec<String>, LocalModId> = targets.map(|to| (&paths[&to], to)).collect();
    let namespaces: Vec<ImportTarget> = (through.into_values())
        .map(|target| {
            let name = crate::lower::analysis::namespace_in(tcx, module, target);
            (
                target,
                js::NAMESPACE.to_string(),
                js::NAMESPACE.to_string(),
                name,
                target,
                Some(target.to_def_id()),
            )
        })
        .collect();
    let request = |(target, name, export, local, of, named): ImportTarget, always| ImportRequest {
        symbol: module_symbol(target, &name),
        export,
        local,
        path: paths[&of].clone(),
        at: at(of, named),
        always,
    };
    let mut imports: Vec<ImportTarget> = imports.into_iter().collect();
    imports.sort_by(|(a, an, ae, _, af, _), (b, bn, be, _, bf, _)| {
        (&paths[af], ae, an, &paths[a]).cmp(&(&paths[bf], be, bn, &paths[b]))
    });
    let mut requests: Vec<ImportRequest> = (imports.into_iter().map(|import| request(import, false)))
        .chain(
            (reimported.into_iter())
                .map(|(of, export, local, id)| request((of, export.clone(), export, local, of, Some(id)), true)),
        )
        .collect();
    // A library's item imported from a module of the crate's among its
    // others, by what it imports.
    requests.sort_by(|a, b| (&a.path, &a.export).cmp(&(&b.path, &b.export)));
    requests.extend(namespaces.into_iter().map(|import| request(import, false)));
    requests
}
