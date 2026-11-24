//! What JS may call of the crate's (#2403 lever (3), Tu's decision): what
//! the crate's root gives JS, and what each module a hand-written JS file of
//! the program or its workspace imports gives, by what each re-exports in
//! turn (`reexports::JsReached`), and each forwarder's target there.

use rust_js_lower::reexports;
use rust_js_program::collections::IndexMap;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::{CRATE_DEF_ID, DefId, LocalModId};

use super::load_cycles::ModuleJs;

/// Of the crate's modules, `places`, what JS may call: from its root and
/// each module the program's hand-written JS imports (`js_modules`), through
/// what each module gives JS, `by`, and each forwarder's target.
pub(super) fn js_reached(
    tcx: TyCtxt<'_>,
    places: &[ModuleJs],
    js_modules: &dyn rust_js_program::program::JsModules,
    (by, forwarders): (&IndexMap<LocalModId, reexports::JsReached>, &IndexMap<DefId, DefId>),
) -> reexports::JsReached {
    let laid_out: Vec<rust_js_program::program::ModuleAt<'_>> = (places.iter())
        .map(|place| (&place.path[..], place.file.as_deref(), place.located))
        .collect();
    let hand_imported = (js_modules.hand_imported(&laid_out).into_iter()).map(|at| places[at].module);
    let mut reached = reexports::js_reached(
        std::iter::once(LocalModId::new_unchecked(CRATE_DEF_ID)).chain(hand_imported),
        by,
    );
    reached.through_forwarders(tcx, forwarders);
    reached
}
