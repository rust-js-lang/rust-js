//! MIR's lowering, as the base asks it (`mir_hooks::MirHooks`): each hook
//! the lowering's own method, which the base can't name (#2299).

use rustc_middle::ty;
use rustc_span::def_id::DefId;

use crate::lower::analysis::Mir;
use crate::lower::mir_hooks::MirHooks;
use crate::lower::{Body, FnCx, R};
use rust_js_program::js::{self, Expr, Stmt};

/// MIR's lowering's hooks, which the driver gives each session.
pub(in crate::lower) struct Hooks;

impl MirHooks for Hooks {
    fn mir_signature<'a, 'tcx>(
        &self,
        cx: &mut FnCx<'a, 'tcx>,
        body: &Body<'tcx>,
        mir: &Mir<'tcx>,
        into: &mut Vec<Stmt>,
    ) -> R<(Vec<js::Pattern>, bool)> {
        cx.mir_signature(body, mir, into)
    }

    fn crate_const_as_code<'a, 'tcx>(
        &self,
        cx: &mut FnCx<'a, 'tcx>,
        def_id: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> R<Expr> {
        cx.crate_const_as_code(def_id, args)
    }

    fn settle_unthrown_catches(&self, body: &mut Vec<Stmt>) {
        super::tidy::settle_unthrown_catches(body)
    }
}
