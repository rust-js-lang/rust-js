//! What a module's load reads of another that may load after it, in an
//! import cycle: a `const` read by its name, which its body reads as its
//! value instead (ADR 0031, amended), and a dictionary that's its module's
//! `const`, refused there (ADR 0592).

use super::Pass;
use crate::js;
use crate::lower::FnInfo;
use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalModId};
use std::collections::{HashMap, HashSet};

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
