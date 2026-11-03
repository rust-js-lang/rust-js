//! A crate's `#[test]`s, and which test file each is in (ADR 1076, the
//! user's decision): one `<module>.test.js` beside each module's JS that
//! has tests, which every JS runner finds by its name.
//!
//! The one rule. A *module of tests* is a module declared under
//! `#[cfg(test)]`, its `cfg` needing `test`, that has `#[test]`s of its own,
//! within no other: `mod tests` as the Rust book writes it, inline or in its
//! own file. Each `#[test]` is registered, under its Rust path, in the test
//! file of its module, or, inside a module of tests, of that module's
//! parent. A module's one module of tests is its parent's test file, no file
//! of its own; of a module with several, each is its own file, which its
//! parent's test file registers the tests of.

use rustc_ast::attr::data_structures::CfgEntry;
use rustc_hir::find_attr;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::{CRATE_DEF_ID, DefId, LocalDefId, LocalModId};
use rustc_span::{Symbol, sym};
use std::collections::{HashMap, HashSet};

use super::super::recognition::{StdItem, is_std_def};
use super::super::{Body, FnInfo, module_path};
use super::namespace_of;
use crate::program::{TestFn, Tests};

/// The tests: each marker names a function beside it, of the same name.
/// One registered in another module's test file than its own module's is
/// imported there, so it's `exported` too; one its own module's file
/// registers is that module's own.
pub(super) fn collect_tests<'tcx>(
    tcx: TyCtxt<'tcx>,
    markers: &[(LocalDefId, Symbol)],
    bodies: &[&Body<'tcx>],
    fns: &HashMap<DefId, FnInfo>,
    exported: &mut HashSet<DefId>,
) -> Tests {
    let found: Vec<(LocalModId, Symbol, LocalDefId)> = markers
        .iter()
        .filter_map(|&(marker, label)| {
            let module = tcx.parent_module_from_def_id(marker);
            let name = tcx.item_name(marker.to_def_id());
            let test = bodies
                .iter()
                .filter_map(|body| body.def_id.as_local())
                .find(|&f| tcx.parent_module_from_def_id(f) == module && tcx.item_name(f.to_def_id()) == name)?;
            Some((module, label, test))
        })
        .collect();
    let with_tests: HashSet<LocalModId> = found.iter().map(|&(module, ..)| module).collect();
    let of_tests = modules_of_tests(tcx, &with_tests);
    // Each parent's modules of tests: its one is its test file.
    let mut by_parent: HashMap<LocalModId, Vec<LocalModId>> = HashMap::new();
    for &module in &of_tests {
        by_parent.entry(parent(tcx, module)).or_default().push(module);
    }
    let placed: HashSet<LocalModId> = (by_parent.values())
        .filter(|modules| modules.len() == 1)
        .flatten()
        .copied()
        .collect();
    let mut fns_found = Vec::new();
    for (module, label, test) in found {
        let home = home(tcx, module, &of_tests);
        // Its own module's file registers it: no other imports it.
        if !placed.contains(&module) {
            exported.insert(test.to_def_id());
        }
        let should_panic = find_attr!(tcx, test, ShouldPanic { reason, .. } => reason.map(|r| r.to_string()));
        let output = tcx
            .fn_sig(test)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder()
            .output();
        fns_found.push(TestFn {
            module: module_path(tcx, module),
            namespace: namespace_of(tcx, module),
            name: fns[&test.to_def_id()].name.clone(),
            label: label.to_string(),
            should_panic,
            ignore: find_attr!(tcx, test, Ignore { .. }),
            returns_result: matches!(output.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Result)),
            home: module_path(tcx, home),
        });
    }
    let mut placed: Vec<Vec<String>> = placed.into_iter().map(|module| module_path(tcx, module)).collect();
    placed.sort();
    Tests { fns: fns_found, placed }
}

/// The modules of tests among the modules with `#[test]`s of their own:
/// each declared under a `cfg` that needs `test`, within no other such.
fn modules_of_tests(tcx: TyCtxt<'_>, with_tests: &HashSet<LocalModId>) -> HashSet<LocalModId> {
    let declared: HashSet<LocalModId> = (with_tests.iter())
        .copied()
        .filter(|&module| module.to_local_def_id() != CRATE_DEF_ID && declared_for_tests(tcx, module))
        .collect();
    (declared.iter())
        .copied()
        .filter(|&module| !ancestors(tcx, module).any(|above| declared.contains(&above)))
        .collect()
}

/// Whether `module`'s declaration is compiled only for tests: a `cfg` of it,
/// as rustc keeps what it evaluated (`CfgTrace`), that needs `test`.
fn declared_for_tests(tcx: TyCtxt<'_>, module: LocalModId) -> bool {
    find_attr!(tcx, module.to_local_def_id(), CfgTrace(cfgs) => cfgs.iter().any(|(cfg, _)| needs_test(cfg)))
        .unwrap_or(false)
}

/// Whether a `cfg` holds only where `test` does: `test`, or an `all` of it.
fn needs_test(cfg: &CfgEntry) -> bool {
    match cfg {
        CfgEntry::NameValue { name, value: None, .. } => *name == sym::test,
        CfgEntry::All(all, _) => all.iter().any(needs_test),
        _ => false,
    }
}

/// The module whose test file registers the tests of `module`: the parent of
/// the module of tests it's in, or itself.
fn home(tcx: TyCtxt<'_>, module: LocalModId, of_tests: &HashSet<LocalModId>) -> LocalModId {
    std::iter::once(module)
        .chain(ancestors(tcx, module))
        .find(|m| of_tests.contains(m))
        .map_or(module, |within| parent(tcx, within))
}

fn parent(tcx: TyCtxt<'_>, module: LocalModId) -> LocalModId {
    tcx.parent_module_from_def_id(module.to_local_def_id())
}

/// The modules `module` is in, nearest first, the crate's root last.
fn ancestors(tcx: TyCtxt<'_>, module: LocalModId) -> impl Iterator<Item = LocalModId> + '_ {
    std::iter::successors(Some(module), move |&m| {
        (m.to_local_def_id() != CRATE_DEF_ID).then(|| parent(tcx, m))
    })
    .skip(1)
}
