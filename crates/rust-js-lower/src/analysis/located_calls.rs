//! Which of generic code's calls through a dictionary give their entry the
//! caller's location (#2457): a call of a method whose model here raises a
//! panic at its caller (`recognition::model_raises_at_caller`), where some
//! instance the call's callers give may bind its arguments to types whose
//! impl tracks its caller (`recognition::impl_raises_at_caller`), a
//! `RefCell`'s `clone`, an integer's `div`; or any type, where they can't all
//! be seen (`instances_of`). Another gives none, and its entry takes none.
//! The callers' instances are `instances`'s, the one owner of what generic
//! code's type parameters may be (ADR 1115).

use crate::Body;
use rustc_middle::ty::{self, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::DefId;

/// What generic code's calls are given of their callers' instances,
/// `gives_location`'s question.
pub struct Located<'tcx> {
    callers: crate::instances::Callers<'tcx>,
}

impl<'tcx> Located<'tcx> {
    pub(super) fn new(
        tcx: TyCtxt<'tcx>,
        (all_bodies, names): (&[&Body<'tcx>], &crate::call_graph::Names<'tcx>),
    ) -> Self {
        Located {
            callers: crate::instances::callers_of(tcx, names, all_bodies),
        }
    }

    /// Whether `owner`'s call of `callee`, a trait method its dictionary is
    /// called for, given `args`, gives its entry its location: of a method
    /// whose model raises at its caller, where an instance of `args` its
    /// callers may give is of an impl that tracks it, or any, where its
    /// callers can't all be seen; of concrete `args`, where theirs does.
    pub(crate) fn gives_location(
        &self,
        tcx: TyCtxt<'tcx>,
        owner: DefId,
        (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
    ) -> bool {
        if !crate::recognition::model_raises_at_caller(tcx, callee) {
            return false;
        }
        let tracks = |args: ty::GenericArgsRef<'tcx>| {
            crate::recognition::impl_raises_at_caller(tcx, ty::TypingEnv::fully_monomorphized(), callee, args)
        };
        let args = tcx.erase_and_anonymize_regions(args);
        if !args.has_non_region_param() {
            return tracks(args);
        }
        // Its type arguments as one type, whose instances are each choice
        // its callers give its parameters.
        let types: Vec<ty::Ty<'tcx>> = args.types().collect();
        let tuple = ty::Ty::new_tup(tcx, &types);
        let owner = tcx.typeck_root_def_id(owner);
        let seen = crate::instances::instances_of(tcx, (&self.callers, crate::instances::Fact::Shared), tuple, owner);
        // Of callers it can't all see, whether a tracking impl may be it at
        // all, by `owner`'s own bounds (`tracking_impl_may_be`).
        let Some(instances) = seen.listed_or_any() else {
            return crate::recognition::tracking_impl_may_be(tcx, owner, callee, args);
        };
        instances.into_iter().any(|instance| {
            let ty::Tuple(parts) = *instance.kind() else {
                return true;
            };
            let mut parts = parts.iter();
            let given = tcx.mk_args_from_iter(args.iter().map(|arg| match arg.as_type() {
                Some(_) => parts.next().map_or(arg, Into::into),
                None => arg,
            }));
            tracks(given)
        })
    }
}
