//! What JS may call of the crate's (#2403 lever (3), Tu's decision): what
//! each module's JS re-exports gives JS, as `reexports` resolves it for the
//! export it writes, and what that reaches from the crate's root and each
//! module a hand-written JS file imports. The one owner of both, so the
//! root's exports and what's checked can't disagree.

use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalModId};

/// What a module's JS re-exports of the crate's, its `pub use`s and its
/// default, as `reexports` resolves them for the export it writes: each
/// function, module whose namespace, and class it gives JS. What JS can call
/// through the crate's root, and each module a hand-written JS file imports,
/// is these, the one owner of both (#2403 lever (3), ADR 1008).
#[derive(Default)]
pub struct JsReached {
    pub functions: IndexSet<DefId>,
    pub modules: IndexSet<LocalModId>,
    pub classes: IndexSet<DefId>,
    /// Each library function a module re-exports, by the name it gives it
    /// there: where JS may call it and its own JS checks nothing JS gives it
    /// (`Item::checked`), the module's own checked forwarder in its place
    /// (#2403 (Y)).
    pub library: Vec<(LocalModId, DefId, String)>,
    /// Each library module a module re-exports as a namespace, `pub use
    /// dep::m as ns`, by the name it gives it there and the `use`'s own:
    /// where JS may call it and it has a function its own JS doesn't check,
    /// a module of the crate's in its place (#2403 (Y), #2536).
    pub namespaces: Vec<(LocalModId, DefId, String, String)>,
}

/// What JS may call of the crate's, from `seeds`, the modules it imports,
/// the crate's root and each a hand-written JS file imports: each, and
/// what each gives JS in turn, a module's namespace with all it gives.
pub fn js_reached(seeds: impl IntoIterator<Item = LocalModId>, by: &IndexMap<LocalModId, JsReached>) -> JsReached {
    let mut all = JsReached::default();
    let mut pending: Vec<LocalModId> = seeds.into_iter().collect();
    while let Some(module) = pending.pop() {
        if !all.modules.insert(module) {
            continue;
        }
        let Some(reached) = by.get(&module) else { continue };
        all.functions.extend(reached.functions.iter().copied());
        all.classes.extend(reached.classes.iter().copied());
        all.library.extend(reached.library.iter().cloned());
        all.namespaces.extend(reached.namespaces.iter().cloned());
        pending.extend(reached.modules.iter().copied());
    }
    all
}

impl JsReached {
    /// And each function a forwarder JS may call passes its parameters on
    /// to, which its module re-exports in its place (rd575, ADR 1064), so JS
    /// calls it there: of `forwarders`, each forwarder's, until no more.
    pub fn through_forwarders(&mut self, tcx: TyCtxt<'_>, forwarders: &IndexMap<DefId, DefId>) {
        loop {
            let more: Vec<DefId> = (forwarders.iter())
                .filter(|&(&forwarder, &target)| {
                    target.is_local()
                        && !self.functions.contains(&target)
                        && forwarder
                            .as_local()
                            .is_some_and(|forwarder| self.may_call(tcx, forwarder))
                })
                .map(|(_, &target)| target)
                .collect();
            if more.is_empty() {
                return;
            }
            self.functions.extend(more);
        }
    }

    /// Whether JS may call `item`, a function of the crate's: one of a
    /// module it reaches, one given it by name, or a member of a class given
    /// it, or that rustc's effective visibility exports, which another crate
    /// may re-export to JS, keeping its `instanceof`, so no consumer can
    /// check it in its place (#2403 (Y), the lead's decision).
    pub fn may_call(&self, tcx: TyCtxt<'_>, item: rustc_span::def_id::LocalDefId) -> bool {
        self.modules.contains(&tcx.parent_module_from_def_id(item))
            || self.functions.contains(&item.to_def_id())
            || crate::classes::class_of(tcx, item.to_def_id()).is_some_and(|class| {
                self.classes.contains(&class) || (class.is_local() && crate::library::reachable(tcx, class))
            })
    }

    /// Of a `use` that resolved to `(kind, id)`, which wrote an export
    /// where `found` grew past `before`: what it gives JS.
    pub(super) fn settle(&mut self, tcx: TyCtxt<'_>, pending: &mut Option<(usize, DefKind, DefId)>, now: usize) {
        let Some((before, kind, id)) = pending.take() else {
            return;
        };
        if now <= before {
            return;
        }
        match kind {
            DefKind::Fn => {
                self.functions.insert(id);
            }
            DefKind::Mod => {
                if let Some(local) = id.as_local() {
                    self.modules.insert(LocalModId::new_unchecked(local));
                }
            }
            DefKind::Struct if crate::classes::is_class(tcx, id) => {
                self.classes.insert(id);
            }
            _ => {}
        }
    }
}
