//! What the crate's modules import of each other, of what their bodies
//! import (`module_imports`), and what a module's load reads of another
//! that may load after it, in an import cycle: a `const` read by its name,
//! which its body reads as its value instead (ADR 0031, amended); what a
//! thread-local's initializer runs or reads, which then makes a private one
//! where it's first used and refuses a `pub` one (ADR 1006, amended); and a
//! dictionary that's its module's `const`, refused there (ADR 0592).

use super::Pass;
use crate::js;
use crate::lower::FnInfo;
use crate::lower::bindings::Export;
use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalDefId, LocalModId};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Each body run where its module loads, of `loads`, that reads another of
/// the crate's modules' `const` or numbered enum's variant by its name,
/// where that module may load after its own: one it reaches by what the
/// crate's modules import of each other, `imports`, an import cycle, through
/// a JS file of the program's own too. JS would read it before its module
/// made it, so that body reads its value (`FnCx::reads_by_name`, ADR 0031,
/// amended). What it imports by name is an edge, so the cycles found are
/// each one's that may be.
pub(super) fn load_cycles(
    tcx: TyCtxt<'_>,
    lowered: &[(DefId, crate::lower::LoweredFn)],
    fns: &HashMap<DefId, FnInfo>,
    loads: &HashSet<DefId>,
    imports: &crate::lower::load_order::ModuleImports,
) -> Vec<DefId> {
    let made_where_loaded = |id: DefId| {
        matches!(
            tcx.def_kind(id),
            DefKind::Const { .. } | DefKind::AssocConst { .. } | DefKind::Variant | DefKind::Ctor(..)
        )
    };
    (lowered.iter())
        .filter(|(id, _)| loads.contains(id))
        .filter(|(id, item)| {
            let own = fns[id].module;
            (item.dependencies.references.iter())
                .any(|&(from, read)| from == own && made_where_loaded(read) && imports.reaches(fns[&read].module, own))
        })
        .map(|&(id, _)| id)
        .collect()
}

/// Each thread-local made as its module loads, and what its initializer may
/// run and read there; `None`, what rust-js can't follow (ADR 1006,
/// amended).
type AtLoad = HashMap<LocalDefId, Option<Vec<DefId>>>;

/// Each private thread-local, of `at_load`, its module's load can't make
/// before what its initializer runs and reads (`out_of_order`): made where
/// it's first used, as Rust makes it, the crate lowered again so (the
/// user's decision, ADR 1006, amended). Where none is, each `pub` one such
/// is refused, as what JS's importers read is an `export const`.
pub(super) fn made_in_cycles(
    tcx: TyCtxt<'_>,
    at_load: &AtLoad,
    fns: &HashMap<DefId, FnInfo>,
    imports: &crate::lower::load_order::ModuleImports,
) -> Vec<LocalDefId> {
    let mut keys: Vec<_> = at_load.keys().copied().collect();
    keys.sort_by_key(|&key| tcx.def_span(key));
    let out_of_order = |key: LocalDefId| out_of_order(tcx, (key, at_load[&key].as_deref()), fns, imports);
    let (exported, private): (Vec<_>, Vec<_>) = keys.into_iter().partition(|&key| tcx.visibility(key).is_public());
    let lazy: Vec<_> = private.into_iter().filter(|&key| out_of_order(key).is_some()).collect();
    if lazy.is_empty() {
        for key in exported {
            if let Some(why) = out_of_order(key) {
                refuse_exported(tcx, key, why);
            }
        }
    }
    lazy
}

/// Why a thread-local's module's load can't make it before what its
/// initializer runs and reads.
enum OutOfOrder {
    /// It may run what rust-js can't follow.
    Unfollowed,
    /// It reads itself, which it would before it's made.
    ReadsItself,
    /// It runs or reads code of the first module, which reaches its own,
    /// the second, by what the crate's modules import of each other: an
    /// import cycle, so it may not have loaded yet.
    InCycle(LocalModId, LocalModId),
}

/// Whether `key`'s module's load can't make it before what its initializer
/// may run or read there, `reached`: what rust-js can't follow (`None`);
/// itself; or code of another module that may load after its own, one that
/// reaches its own by what the crate's modules import of each other,
/// `imports`. Its own module's `const`s and thread-locals are made before
/// it where it reads them (`js::in_load_order`), and a module it imports,
/// of no cycle, has loaded.
fn out_of_order(
    tcx: TyCtxt<'_>,
    (key, reached): (LocalDefId, Option<&[DefId]>),
    fns: &HashMap<DefId, FnInfo>,
    imports: &crate::lower::load_order::ModuleImports,
) -> Option<OutOfOrder> {
    let module_of = |id: DefId| {
        (fns.get(&id).or_else(|| fns.get(&tcx.typeck_root_def_id(id))))
            .map(|info| info.module)
            .or_else(|| id.as_local().map(|id| tcx.parent_module_from_def_id(id)))
    };
    let Some(reached) = reached else {
        return Some(OutOfOrder::Unfollowed);
    };
    if reached.contains(&key.to_def_id()) {
        return Some(OutOfOrder::ReadsItself);
    }
    let own = fns[&key.to_def_id()].module;
    (reached.iter().filter_map(|&id| module_of(id)))
        .find(|&module| module != own && imports.reaches(module, own))
        .map(|module| OutOfOrder::InCycle(module, own))
}

/// A `pub` thread-local its module's load can't make first, `why`: refused,
/// as JS's importers read it as an `export const` (ADR 1006, amended).
fn refuse_exported(tcx: TyCtxt<'_>, key: LocalDefId, why: OutOfOrder) {
    let span = tcx.def_ident_span(key).unwrap_or_else(|| tcx.def_span(key));
    let refuse = |what: &str, why: &str| {
        crate::lower::refusal::refuse(tcx, span, crate::lower::refusal::Refused::Because(what, why));
    };
    let made = "it's made as its module loads, as JS's `export const` is";
    match why {
        OutOfOrder::Unfollowed => refuse(
            "a `pub` thread-local whose initializer may run what rust-js can't follow",
            &format!(
                "{made}, and its initializer calls a `dyn Fn`, a fn pointer that may be any function, or what a type parameter is given, which may run code of a module that hasn't loaded yet: call what it runs by its name (ADR 1006)"
            ),
        ),
        OutOfOrder::ReadsItself => refuse(
            "a `pub` thread-local whose initializer reads itself",
            &format!(
                "{made}, so it would read itself before it's made: make it of what it reads, not of itself (ADR 1006)"
            ),
        ),
        OutOfOrder::InCycle(module, own) => {
            refuse(
                &format!(
                    "a `pub` thread-local whose initializer runs or reads `{}` as its module loads, in an import cycle with `{}`",
                    tcx.def_path_str(module.to_def_id()),
                    tcx.def_path_str(own.to_def_id()),
                ),
                &format!(
                    "{made}, and that module may not have loaded yet: move one of them out of the cycle (ADR 1006)"
                ),
            );
        }
    }
}

/// A dictionary that's its module's `const` (ADR 0592) is made when its
/// module loads. Where its module and one whose load runs code, a
/// `js::on_load!` or a value made by a call, import each other, the other
/// may load first and reach it before it's made: refused, naming both, as
/// JS's would throw where Rust's runs.
pub(super) fn refuse_dictionaries_in_load_cycles(
    tcx: TyCtxt<'_>,
    pass: &Pass,
    const_items: &HashMap<LocalModId, Vec<js::Const>>,
    (modules, imports): (&[LocalModId], &crate::lower::load_order::ModuleImports),
) {
    // A dictionary, its cache and what its entries share run nothing of the
    // program's where they're made: not code a cycle's load runs.
    let runs_code = |module: &LocalModId| {
        pass.statements.get(module).is_some_and(|stmts| !stmts.is_empty())
            || const_items.get(module).is_some_and(|consts| {
                (consts.iter()).any(|c| !pass.dictionary_names.contains(&c.name) && c.value.has_effects())
            })
    };
    let reaches = |from, to| imports.reaches(from, to);
    let loaders: Vec<LocalModId> = modules.iter().copied().filter(runs_code).collect();
    for &(module, impl_id) in &pass.dictionary_consts {
        if let Some(&other) = loaders
            .iter()
            .find(|&&other| other != module && reaches(module, other) && reaches(other, module))
        {
            let what = format!(
                "this impl's dictionary, made where `{}` loads, in an import cycle with `{}`, which runs code where it loads",
                tcx.def_path_str(module.to_def_id()),
                tcx.def_path_str(other.to_def_id()),
            );
            let why = "that code may reach the dictionary before it's made: move one out of the cycle (ADR 0592)";
            crate::lower::refusal::refuse(
                tcx,
                tcx.def_span(impl_id),
                crate::lower::refusal::Refused::Because(&what, why),
            );
        }
    }
}

/// A module's JS: where it's laid out, its module path, its Rust file and
/// whether a `#[path]` placed it (ADR 0273), and what it imports for its
/// effects alone, `js::import!("./setup.js")` (ADR 0110).
pub(super) struct ModuleJs {
    pub(super) module: LocalModId,
    pub(super) path: Vec<String>,
    pub(super) file: Option<std::path::PathBuf>,
    pub(super) located: bool,
    pub(super) effects: Vec<String>,
}

/// A module and where a `pub use` of it re-exports from, as `reexports`
/// writes it.
pub(super) type ReexportFrom = (LocalModId, js::ImportFrom);

/// What the crate's modules import of each other (`load_order`), of what
/// their bodies import: another module's items each reads, the bindings it
/// reads through another that holds them (ADR 0633), and the JS modules it
/// imports, a binding's, as `imported` says of each too, and those it
/// imports for their effects, and what each re-exports, `reexports`'s.
pub(super) fn module_imports<'d>(
    places: &[ModuleJs],
    fns: &HashMap<DefId, FnInfo>,
    (references, held, package_uses): (
        impl Iterator<Item = &'d (LocalModId, DefId)>,
        impl Iterator<Item = &'d (LocalModId, LocalModId, String, bool, DefId)>,
        impl Iterator<Item = &'d (LocalModId, Export)>,
    ),
    (imported, reexports): (&BTreeMap<Export, HashSet<LocalModId>>, &[ReexportFrom]),
    js_modules: &dyn crate::program::JsModules,
) -> crate::lower::load_order::ModuleImports {
    let laid_out: Vec<_> = (places.iter())
        .map(|js| (js.module, (js.path.as_slice(), js.file.as_deref(), js.located)))
        .collect();
    let referenced = references.map(|&(from, id)| (from, fns[&id].module));
    let held = held.map(|&(from, holder, ..)| (from, holder));
    // And what each module re-exports, as `reexports` writes it, `export {
    // one } from "./z.js"`: an import of that module, or of a binding's JS
    // file, so a cycle through it is one (#295). A type's, which has no JS,
    // is none.
    let module_of = |path: &[String]| (places.iter()).find(|js| js.path == path).map(|js| js.module);
    let reexported = (reexports.iter()).filter_map(|(from, of)| match of {
        js::ImportFrom::Module(path) => Some((*from, module_of(path)?)),
        js::ImportFrom::Specifier(_) => None,
    });
    let reexported_files = (reexports.iter()).filter_map(|(from, of)| match of {
        js::ImportFrom::Specifier(specifier) => Some((*from, specifier.as_str())),
        js::ImportFrom::Module(_) => None,
    });
    let used = package_uses.map(|(from, (module, _))| (*from, module.as_str()));
    let users =
        (imported.iter()).flat_map(|((module, _), users)| users.iter().map(move |&from| (from, module.as_str())));
    let effects = (places.iter()).flat_map(|js| js.effects.iter().map(|from| (js.module, from.as_str())));
    let bound = used.chain(users).chain(effects).chain(reexported_files);
    crate::lower::load_order::ModuleImports::new(&laid_out, referenced.chain(held).chain(reexported), bound, js_modules)
}
