//! What the base asks of MIR's lowering that only MIR's lowering can do,
//! through one trait, `MirHooks`, as the base can't name `mir` (#2299's
//! commit 5, #2294 step 4): `mir` implements it (`mir::Hooks`), and the
//! driver gives it to each lowering session (`lower::start_session`),
//! which keeps it beside its caches (`Caches::hooks`).
//!
//! Each method is here only because the base can't name `mir`: a question
//! that reads no MIR lowering state moves down to the base instead, beside
//! what asks it, never behind a hook. A review of the crate cut checks
//! that none was added for less.

use rustc_middle::ty;
use rustc_span::def_id::DefId;

use crate::lower::analysis::Mir;
use crate::lower::{Body, FnCx, R};
use rust_js_program::js::{self, Expr, Stmt};

/// MIR's lowering, as the base asks it.
pub(in crate::lower) trait MirHooks {
    /// A trait method's default body's parameters, its MIR lowered in
    /// `cx` (`FnCx::mir_signature`): what `traits` writes a default's and
    /// an impl's call of it with, the parameters MIR's lowering binds.
    fn mir_signature<'a, 'tcx>(
        &self,
        cx: &mut FnCx<'a, 'tcx>,
        body: &Body<'tcx>,
        mir: &Mir<'tcx>,
        into: &mut Vec<Stmt>,
    ) -> R<(Vec<js::Pattern>, bool)>;

    /// The crate's constant `def_id` of `args` as code, its MIR lowered
    /// (`FnCx::crate_const_as_code`): what the root reads a generic
    /// constant as.
    fn crate_const_as_code<'a, 'tcx>(
        &self,
        cx: &mut FnCx<'a, 'tcx>,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> R<Expr>;

    /// `body` with each `try` that can't throw its body, tidy's pass
    /// (`mir::settle_unthrown_catches`): what `std_types` asks of a
    /// thread-local read once its module is placed.
    fn settle_unthrown_catches(&self, body: &mut Vec<Stmt>);
}

impl<'tcx> FnCx<'_, 'tcx> {
    /// The hooks of the session `cx` lowers in (`MirHooks`).
    pub(in crate::lower) fn mir_hooks(&self) -> &'static dyn MirHooks {
        crate::lower::session::caches(self.tcx).hooks
    }
}
