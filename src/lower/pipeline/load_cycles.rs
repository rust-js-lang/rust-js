//! What the crate's modules import of each other, of what their bodies
//! import (`module_imports`), and what a module's load reads of another
//! that may load after it, in an import cycle: a `const` read by its name,
//! which its body reads as its value instead (ADR 0031, amended); a
//! thread-local's maker, which then makes it where it's first used (ADR
//! 0037, amended); and a dictionary that's its module's `const`, refused
//! there (ADR 0592).

use super::Pass;
use crate::js;
use crate::lower::FnInfo;
use crate::lower::bindings::Export;
use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalModId};
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

/// Each thread-local, of `makers`, made as its module loads by functions of
/// the crate's that only make a value (ADR 0037, amended), one of which is
/// of a module that may load after its own: of another module that reaches
/// its own by what the crate's modules import of each other, `imports`, an
/// import cycle. Its maker may read what that module hasn't made yet, a
/// `const`, so it's made where it's first used.
pub(super) fn made_in_cycles(
    tcx: TyCtxt<'_>,
    makers: &HashMap<rustc_span::def_id::LocalDefId, Vec<DefId>>,
    fns: &HashMap<DefId, FnInfo>,
    imports: &crate::lower::load_order::ModuleImports,
) -> Vec<rustc_span::def_id::LocalDefId> {
    let mut cyclic: Vec<_> = (makers.iter())
        .filter(|&(&key, makers)| {
            let own = fns[&key.to_def_id()].module;
            (makers.iter()).any(|maker| {
                let module = fns[maker].module;
                module != own && imports.reaches(module, own)
            })
        })
        .map(|(&key, _)| key)
        .collect();
    cyclic.sort_by_key(|&key| tcx.def_span(key));
    cyclic
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

/// What the crate's modules import of each other (`load_order`), of what
/// their bodies import: another module's items each reads, the bindings it
/// reads through another that holds them (ADR 0633), and the JS modules it
/// imports, a binding's, as `imported` says of each too, and those it
/// imports for their effects.
pub(super) fn module_imports<'d>(
    places: &[ModuleJs],
    fns: &HashMap<DefId, FnInfo>,
    (references, held, package_uses): (
        impl Iterator<Item = &'d (LocalModId, DefId)>,
        impl Iterator<Item = &'d (LocalModId, LocalModId, String, bool, DefId)>,
        impl Iterator<Item = &'d (LocalModId, Export)>,
    ),
    imported: &BTreeMap<Export, HashSet<LocalModId>>,
    js_modules: &dyn crate::program::JsModules,
) -> crate::lower::load_order::ModuleImports {
    let laid_out: Vec<_> = (places.iter())
        .map(|js| (js.module, (js.path.as_slice(), js.file.as_deref(), js.located)))
        .collect();
    let referenced = references.map(|&(from, id)| (from, fns[&id].module));
    let held = held.map(|&(from, holder, ..)| (from, holder));
    let used = package_uses.map(|(from, (module, _))| (*from, module.as_str()));
    let users =
        (imported.iter()).flat_map(|((module, _), users)| users.iter().map(move |&from| (from, module.as_str())));
    let effects = (places.iter()).flat_map(|js| js.effects.iter().map(|from| (js.module, from.as_str())));
    let bound = used.chain(users).chain(effects);
    crate::lower::load_order::ModuleImports::new(&laid_out, referenced.chain(held), bound, js_modules)
}
