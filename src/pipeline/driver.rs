//! What the driver asks of the lowering that names `mir`, the driver's
//! side of the cut (#2299's commit 6): a session begun with MIR's
//! lowering's hooks, and tidy's passes' names. The root, the base's,
//! names nothing of `mir` or of the pipeline.

use rustc_middle::ty::TyCtxt;

use rust_js_lower::session;

/// A lowering session of `tcx` begun, its flags `flags`, until the guard
/// it gives is dropped (`session::start`), given MIR's lowering's hooks,
/// which the base asks of it (`mir_hooks`, #2299): the driver's, which
/// names both.
pub(crate) fn start_session(tcx: TyCtxt<'_>, flags: session::Flags) -> session::Session {
    session::start(tcx, flags, &rust_js_mir::hooks::Hooks)
}

/// Each of tidy's passes, `stage/pass`, in the order they run (ADR 0659).
pub fn tidy_pass_names() -> Vec<String> {
    rust_js_mir::tidy_pass_names()
}
