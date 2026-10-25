//! Which thread-locals are made where they're first used, as Rust makes
//! each (ADR 0037, amended): those whose initializer runs code of the
//! crate's, or may panic. One that runs only what std's and bindings'
//! calls make is made as its module loads, as JS writes it,
//! `createContext([])`: the guide's semantics list that difference.

use std::collections::{HashMap, HashSet};

use rustc_middle::mir::{self, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::{DefId, LocalDefId};

use super::fn_pointers::FnPointers;

use crate::lower::Body;

/// The thread-locals, by their keys, whose initializer (`inits`, each
/// initializer's key) runs the crate's code or may panic; a fn pointer's
/// call runs what `pointers` says it may be (ADR 0907).
pub(super) fn lazy_locals<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[&Body<'tcx>],
    inits: &HashMap<LocalDefId, LocalDefId>,
    pointers: &FnPointers<'tcx>,
) -> HashSet<LocalDefId> {
    (bodies.iter())
        .filter_map(|body| {
            let init = body.def_id.as_local()?;
            let key = *inits.get(&init)?;
            let mir = body.mir.as_ref()?;
            runs_the_crates(tcx, init, &mir.body, pointers).then_some(key)
        })
        .collect()
}

/// Whether `body`, `init`'s, runs code of the crate's or may panic: a call
/// of a body of the crate's, a function, a method of its impl or a closure,
/// or of what rust-js can't see the body of; a fn pointer's call of what it
/// may be (`pointers`), where any is the crate's or it may be anything; a
/// drop of a value of the crate's with a destructor; a check that may
/// fail; or a call that never returns, a panic's.
fn runs_the_crates<'tcx>(
    tcx: TyCtxt<'tcx>,
    init: LocalDefId,
    body: &mir::Body<'tcx>,
    pointers: &FnPointers<'tcx>,
) -> bool {
    let typing_env = ty::TypingEnv::post_analysis(tcx, init.to_def_id());
    // Whether calling `def_id`, of `args`, runs the crate's code.
    let runs = |def_id: DefId, args: ty::GenericArgsRef<'tcx>| {
        if tcx.is_closure_like(def_id) {
            return def_id.is_local();
        }
        !crate::lower::bindings::is_binding(tcx, def_id)
            && match crate::lower::items::resolve_instance_in(tcx, typing_env, def_id, args) {
                Ok(Some(instance)) => match instance.def {
                    ty::InstanceKind::Item(id) => id.is_local(),
                    // A shim of std's, a clone's or a drop's, runs what it's
                    // of: the crate's where that is.
                    _ => (instance.args.types()).any(|ty| ty.ty_adt_def().is_some_and(|adt| adt.did().is_local())),
                },
                _ => true,
            }
    };
    body.basic_blocks.iter().any(|data| match &data.terminator().kind {
        TerminatorKind::Assert { .. } | TerminatorKind::Call { target: None, .. } => true,
        TerminatorKind::Call { func, .. } => {
            let ty = func.ty(&body.local_decls, tcx);
            match crate::lower::fn_def(ty) {
                Some((def_id, args)) => runs(def_id, args),
                // A fn pointer's runs what it may be (ADR 0907): of a
                // binding's alone, `js::date::now`, none of the crate's.
                None if ty.is_fn_ptr() => (pointers.may_be(tcx, ty))
                    .is_none_or(|may_be| may_be.iter().any(|&(def_id, args)| runs(def_id, args))),
                // A closure's or a `dyn Fn`'s, which may be the crate's.
                None => true,
            }
        }
        // A drop of a value of the crate's that has a destructor runs it.
        TerminatorKind::Drop { place, .. } => place.ty(&body.local_decls, tcx).ty.walk().any(|part| {
            part.as_type()
                .and_then(|ty| ty.ty_adt_def())
                .is_some_and(|adt| adt.did().is_local() && adt.has_dtor(tcx))
        }),
        _ => false,
    })
}
