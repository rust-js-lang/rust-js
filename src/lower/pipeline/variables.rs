//! A module's variables: its thread-locals of a value and its `static
//! mut`s, each its module's variable where it's plain, by its name (ADR
//! 0096, amended), those it exports `export let x` (ADR 0270, amended);
//! and a library's it reads, by its name or through its namespace (rd114a,
//! ADR 1018).

use super::super::FnInfo;
use super::super::bindings::Export;
use super::super::library::Foreign;
use super::super::representation::{NonePlace, none_form};
use crate::collections::{IndexMap, IndexSet};
use crate::js;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalDefId, LocalModId};

/// Make `module`'s plain variables of its `items`, a library's it reads
/// among them, by `names`, its imports' names: each of its own made a
/// variable, and whether its JS sets it.
pub(super) fn place(
    tcx: TyCtxt<'_>,
    module: LocalModId,
    items: &mut [js::Item],
    (fns, consts, foreign): (&IndexMap<DefId, FnInfo>, &[LocalDefId], &Foreign<'_, '_>),
    (quiet_locals, thread_local_inits): (&IndexSet<LocalDefId>, &IndexMap<LocalDefId, LocalDefId>),
    names: Option<&IndexMap<Export, String>>,
) -> IndexMap<DefId, bool> {
    // Its `static mut`s, each its module's variable as a
    // thread-local of a value is (ADR 0096, amended).
    let statics = super::super::analysis::module_statics(tcx, consts, module);
    let quiet: IndexSet<&str> = (quiet_locals.iter())
        .filter(|&&key| tcx.parent_module_from_def_id(key) == module)
        .chain(&statics)
        .map(|key| fns[&key.to_def_id()].name.as_str())
        .collect();
    // Its thread-locals of a value, and its `static mut`s, each its
    // module's variable where it's plain, by its name.
    let held_values: IndexMap<&str, LocalDefId> = (thread_local_inits.values())
        .filter(|&&key| {
            tcx.parent_module_from_def_id(key) == module && super::super::analysis::holds_a_value(tcx, key.to_def_id())
        })
        .chain(&statics)
        .filter_map(|&key| Some((fns.get(&key.to_def_id())?.name.as_str(), key)))
        .collect();
    // Those it exports as its variables, `export let x`, which their
    // importers read by name (ADR 0270, amended); one it exports still
    // its `{ value }`, which a JS module importing it would read as the
    // box, refused. And its `#[rust_js::nullable]` thread-locals of a
    // cell, whose `None` is `null` however it's written (#541).
    let nullable: IndexSet<&str> = (thread_local_inits.values())
        .filter(|&&key| {
            tcx.parent_module_from_def_id(key) == module
                && none_form(tcx, NonePlace::LocalCell(key.to_def_id())).stores_null()
        })
        .filter_map(|&key| Some(fns.get(&key.to_def_id())?.name.as_str()))
        .collect();
    let (made, boxed) =
        super::super::std_types::cell::plain_statics(items, &quiet, &held_values.keys().copied().collect(), &nullable);
    for name in boxed {
        super::super::analysis::refuse_boxed_export(tcx, held_values[name.as_str()].to_def_id());
    }
    let variables: IndexMap<DefId, bool> = (quiet_locals.iter())
        .filter(|&&key| tcx.parent_module_from_def_id(key) == module)
        .chain(&statics)
        .filter_map(|key| {
            let name = &fns[&key.to_def_id()].name;
            let &(_, set) = made.iter().find(|(made, _)| made == name)?;
            Some((key.to_def_id(), set))
        })
        .collect();
    // A library's, by the name the module imports it as, `names`, or through its module's namespace, `Fiber.current` (rd114a, ADR
    // 1018), by what that exports it as.
    let exports: Vec<(String, String)> = (foreign.all())
        .filter(|item| item.calling.has(crate::library::ItemFact::Variable) && item.member.is_none())
        .flat_map(|item| std::iter::once((item.from.clone(), item.export.clone())).chain(item.rooted.clone()))
        .collect();
    let local = |export: &(String, String)| names?.get(export).map(String::as_str);
    let foreign_variables: IndexSet<&str> = exports.iter().filter_map(local).collect();
    let namespaced: IndexSet<(&str, &str)> = (exports.iter())
        .filter_map(|(from, export)| Some((local(&(from.clone(), js::NAMESPACE.into()))?, export.as_str())))
        .collect();
    if !foreign_variables.is_empty() || !namespaced.is_empty() {
        super::super::std_types::cell::imported_variables_through(
            items,
            &|e| matches!(&e.kind, js::ExprKind::Var(name) if foreign_variables.contains(name.as_str())),
            &|namespace, export| {
                matches!(&namespace.kind,
                js::ExprKind::Var(namespace) if namespaced.contains(&(namespace.as_str(), export)))
            },
        );
    }
    variables
}
