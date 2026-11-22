//! What a library exports of its private state for its consumers' compiles
//! of its functions (#2017, ADR 0914, amended): a consumer compiles a
//! library's function itself, for whatever reason, and reads what that
//! reads, but a `static` or a `thread_local!` key is state, which a copy
//! would make a second of. Its library's JS exports it by an internal name,
//! `export { LIMIT as $$LIMIT }`, which its manifest lists and no
//! declaration names, and the consumer imports it as its library's public
//! ones.

use super::{FnInfo, escapes, reachable};
use crate::instances::{called, is_state, reads};
use crate::std_types::cell::is_box;
use rust_js_program::collections::{IndexMap, IndexSet};
use rust_js_program::js;
use rust_js_program::program::LoweredModule;
use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{DefId, LocalDefId, LocalModId};

/// Of a library, its private state, a `static` or a `thread_local!` key no
/// other crate can name (`reachable`), that a function another crate can
/// call reads (`escapes`), or a function of its own it calls, or a closure
/// of either: what a consumer that compiles that function itself reads too
/// (`instances::out_of_reach`). One answer for every reason a consumer
/// compiles one (`Foreign::compiles`, `Copying`), as any it compiles is one
/// it can call, and wide: where no consumer reads one, it's an export spent,
/// where the narrower answer's miss is a refusal (the user's decision).
pub fn internal_state(tcx: TyCtxt<'_>, library: bool) -> IndexSet<LocalDefId> {
    let function = |id: DefId| matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn | DefKind::Closure);
    let mut todo: Vec<DefId> = (tcx.hir_body_owners())
        .map(LocalDefId::to_def_id)
        .filter(|&id| function(id) && escapes(tcx, library, id))
        .collect();
    let mut seen: IndexSet<DefId> = IndexSet::default();
    let mut state = IndexSet::default();
    while let Some(id) = todo.pop() {
        let Some(local) = id.as_local() else { continue };
        if !seen.insert(id) || !function(id) || !tcx.is_mir_available(id) {
            continue;
        }
        let body = tcx.optimized_mir(id);
        state.extend(
            (reads(tcx, body).into_iter())
                .filter_map(DefId::as_local)
                .filter(|&def| {
                    let def = def.to_def_id();
                    is_state(tcx, def) && !tcx.is_foreign_item(def) && !reachable(tcx, def)
                }),
        );
        todo.extend(called(body).into_iter().map(|(callee, _)| callee));
        todo.extend(tcx.nested_bodies_within(local).iter().map(LocalDefId::to_def_id));
    }
    state
}

/// What a library's module exports of its private state (#2017).
pub struct InternalExport {
    pub(crate) id: DefId,
    /// Its internal name, `$$LIMIT`; `None` of one exported by its own, as
    /// another of the crate's modules imports it.
    pub(crate) alias: Option<String>,
    /// Whether its module made it its variable, which its importer reads by
    /// its name (`variables::place`).
    pub(crate) variable: bool,
}

/// Of `module`'s `internal` state, each a consumer can read as it reads its
/// library's public one, whose JS is never its cell's `{ value }` (ADR
/// 0270, amended): exported by an internal name given apart from what the
/// module exports, where it isn't by its own already. Not one its module
/// keeps as its `{ value }`, whose importer would read the box; nor a
/// `static` of what may change inside it, an atomic, nor a `let` it sets in
/// place that isn't its variable, which a consumer's copy would assign to
/// the import (a `TypeError`): a consumer's compile that reads one is
/// refused, as before. `made`: those its module made its variables, a
/// thread-local's or a `static mut`'s, whose sets a consumer refuses (ADR
/// 0270).
pub fn internal_exports(
    tcx: TyCtxt<'_>,
    (module, lowered): (LocalModId, &mut LoweredModule),
    (internal, fns, made): (&IndexSet<LocalDefId>, &IndexMap<DefId, FnInfo>, &IndexSet<DefId>),
) -> Vec<InternalExport> {
    let mut taken: IndexSet<String> = (lowered.items.iter())
        .filter_map(|item| match item {
            js::Item::Function(function) if function.export => Some(function.name.word()), // word: an export's, which importers name
            js::Item::Const(constant) if constant.export => Some(constant.name.word()), // word: an export's, which importers name
            js::Item::Namespace(namespace) if namespace.export => Some(namespace.name.word()), // word: an export's, which importers name
            _ => None,
        })
        .map(str::to_owned)
        .chain(lowered.aliases.iter().map(|(_, exported)| exported.clone()))
        .chain((lowered.reexports.iter()).flat_map(|reexport| reexport.named.iter().map(|(_, alias)| alias.clone())))
        .collect();
    let mut exports = Vec::new();
    for &key in internal
        .iter()
        .filter(|&&key| tcx.parent_module_from_def_id(key) == module)
    {
        let id = key.to_def_id();
        // A `static` of what may change inside it, an atomic, which its
        // module may unbox and set in place where no consumer's set of it is
        // refused (ADR 0270's are of thread-locals and `static mut`s).
        let ty = tcx.type_of(id).instantiate_identity().skip_normalization();
        let changes_inside = matches!(tcx.def_kind(id), DefKind::Static { .. })
            && !tcx.is_mutable_static(id)
            && !ty.is_freeze(tcx, rustc_middle::ty::TypingEnv::fully_monomorphized());
        let Some(info) = fns.get(&id).filter(|_| !changes_inside) else {
            continue;
        };
        let exported = lowered.items.iter().find_map(|item| match item {
            js::Item::Const(constant)
                if constant.name.is(&info.name)
                    && !is_box(&constant.value)
                    && (!constant.mutable || made.contains(&id)) =>
            {
                Some(constant.export)
            }
            _ => None,
        });
        let Some(exported) = exported else { continue };
        let alias = (!exported).then(|| {
            rust_js_program::names::fresh_in(
                &mut taken,
                &format!("{}{}", rust_js_program::library::INTERNAL, info.name),
            )
        });
        if let Some(alias) = &alias {
            lowered.aliases.push((info.name.clone(), alias.clone()));
        }
        exports.push(InternalExport {
            id,
            alias,
            variable: made.contains(&id),
        });
    }
    exports
}
