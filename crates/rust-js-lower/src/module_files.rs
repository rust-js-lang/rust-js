//! Where a module of the crate is: its path below the crate root, the
//! `.rs` file its code lives in, and whether a `#[path]` placed that file
//! (ADR 0273), which its JS's file and its importers' paths are of.

use std::sync::Arc;

use rustc_hir::def::DefKind;
use rustc_hir::find_attr;
use rustc_middle::ty::TyCtxt;
use rustc_span::SourceFile;
use rustc_span::def_id::{CRATE_MOD_ID, LocalModId};

use rust_js_program::js;

/// A module's path below the crate root, e.g. `["math", "stats"]`.
pub(crate) fn module_path(tcx: TyCtxt<'_>, module: LocalModId) -> Vec<String> {
    if module == CRATE_MOD_ID {
        return Vec::new();
    }
    let parent = tcx.parent_module_from_def_id(module.to_local_def_id());
    let mut path = module_path(tcx, parent);
    let name = tcx.item_name(module.to_def_id());
    // A module in a block, as bitflags' macro writes in a function, is one of
    // its own, though another of its parent's has its name: numbered in the
    // order they're written, after the one the parent declares, `names$1`.
    let in_block = |m: LocalModId| tcx.def_kind(tcx.local_parent(m.to_local_def_id())) != DefKind::Mod;
    let mut namesakes: Vec<LocalModId> = tcx
        .hir_crate_items(())
        .definitions()
        .filter(|&id| tcx.def_kind(id) == DefKind::Mod)
        .map(LocalModId::new_unchecked)
        .filter(|&m| {
            m != CRATE_MOD_ID
                && tcx.item_name(m.to_def_id()) == name
                && tcx.parent_module_from_def_id(m.to_local_def_id()) == parent
        })
        .collect();
    namesakes.sort_by_key(|&m| (in_block(m), m.to_local_def_id().local_def_index));
    match namesakes.iter().position(|&m| m == module) {
        Some(at) if at > 0 => {
            let apart = js::Name::declared(name.to_string()).given_apart(at);
            path.push(apart.into_word()); // word: a module's file, which its importers' paths name
        }
        _ => path.push(name.to_string()),
    }
    path
}

/// The `.rs` file a module's code lives in: its own file for `mod foo;`,
/// the parent's file for an inline `mod foo { .. }`.
pub fn module_file(tcx: TyCtxt<'_>, module: LocalModId) -> Arc<SourceFile> {
    let inner = tcx.hir_get_module(module).0.spans.inner_span;
    tcx.sess.source_map().lookup_source_file(inner.lo())
}

/// Whether a `#[path]` placed `module`'s file (ADR 0273), its own or an
/// enclosing module's, `sandpack-rsc/bridge.rs` of `#[path = "sandpack-rsc"]
/// mod sandpack_rsc { mod bridge; }`: where no module path can name it. Not
/// an inline module's, which is its parent's file.
pub fn located(tcx: TyCtxt<'_>, module: LocalModId) -> bool {
    if module == CRATE_MOD_ID {
        return false;
    }
    let parent = tcx.parent_module_from_def_id(module.to_local_def_id());
    if Arc::ptr_eq(&module_file(tcx, module), &module_file(tcx, parent)) {
        return false;
    }
    let mut at = module;
    loop {
        if find_attr!(tcx, at.to_def_id(), Path(..)) {
            return true;
        }
        if at == CRATE_MOD_ID {
            return false;
        }
        at = tcx.parent_module_from_def_id(at.to_local_def_id());
    }
}
