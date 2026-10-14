//! A `for` over the crate's own `IntoIterator` (`user_into_iter`): its
//! `into_iter`, then a JS iterator of it. Loops themselves are MIR's,
//! made `for` and `while` by tidy (`mir/tidy/loops.rs`).

use super::recognition::{StdItem, std_item, trait_method};

use super::FnCx;

use rustc_hir as hir;

use rustc_middle::ty::{self, TypeVisitableExt};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `IntoIterator::into_iter` of `ty`, if it's an impl of the crate's: a
    /// collection of its own, whose items are what that gives (ADR 0160).
    pub(super) fn user_into_iter(&self, ty: ty::Ty<'tcx>) -> Option<(hir::def_id::DefId, ty::GenericArgsRef<'tcx>)> {
        let trait_id = std_item(self.tcx, StdItem::IntoIterator);
        let into_iter = trait_method(self.tcx, trait_id, "into_iter");
        if ty.has_escaping_bound_vars() {
            return None;
        }
        let args = self.tcx.mk_args(&[ty.into()]);
        let instance = self.resolve_instance(into_iter, args).ok()??;
        self.is_rust_fn(instance.def_id()).then_some((into_iter, args))
    }
}
