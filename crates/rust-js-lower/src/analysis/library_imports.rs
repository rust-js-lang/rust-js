//! What the crate's libraries export, each import of it named before
//! lowering (ADR 0100), as any module of the crate may turn out to use it.

use super::naming::JsUses;

/// Each library's export, named in `uses`: by its file, by its package's
/// root where that re-exports it, and through a library's module that
/// re-exports it.
pub(super) fn named(uses: &mut JsUses, foreign: &crate::library::Foreign<'_, '_>) {
    // What the crate's libraries export (ADR 0100) is named before lowering,
    // after its name there, a module's default too, `import noop from
    // "../shared/noop.js"` (ADR 0773), and imported by the modules that turn
    // out to use it.
    // By its package's name too, where its root re-exports it, as a
    // module naming it through its root imports it (ADR 1008, amended).
    for imported in foreign.all() {
        let by_file = (imported.from.clone(), imported.export.clone());
        for export in std::iter::once(by_file).chain(imported.rooted.clone()) {
            uses.bound_to.entry(export.clone()).or_default();
            uses.imported.entry(export.clone()).or_default();
            uses.named.insert(export, imported.name.clone());
        }
    }
    // And through a library's module that re-exports it, by what that
    // module's file exports it as (rd704).
    for export in foreign.all_reexported() {
        uses.bound_to.entry(export.clone()).or_default();
        uses.imported.entry(export.clone()).or_default();
        uses.named.insert(export.clone(), export.1.clone());
    }
}
