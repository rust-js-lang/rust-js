//! Which thread-locals are made where they're first used, as Rust makes
//! each (ADR 0037, amended): those whose initializer runs code of the
//! crate's, or may panic. One that runs only what std's and bindings'
//! calls make is made as its module loads, as JS writes it,
//! `createContext([])`: the guide's semantics list that difference.

use std::collections::{HashMap, HashSet};

use rustc_middle::mir::{self, TerminatorKind};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::LocalDefId;

use crate::lower::Body;

/// The thread-locals, by their keys, whose initializer (`inits`, each
/// initializer's key) runs the crate's code or may panic.
pub(super) fn lazy_locals<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[&Body<'tcx>],
    inits: &HashMap<LocalDefId, LocalDefId>,
) -> HashSet<LocalDefId> {
    (bodies.iter())
        .filter_map(|body| {
            let init = body.def_id.as_local()?;
            let key = *inits.get(&init)?;
            let mir = body.mir.as_ref()?;
            runs_the_crates(tcx, init, &mir.body).then_some(key)
        })
        .collect()
}

/// Whether `body`, `init`'s, runs code of the crate's or may panic: a call
/// of a body of the crate's, a function, a method of its impl or a closure,
/// or of what rust-js can't see the body of; a drop of a value of the
/// crate's with a destructor; a check that may fail; or a call that never
/// returns, a panic's.
fn runs_the_crates<'tcx>(tcx: TyCtxt<'tcx>, init: LocalDefId, body: &mir::Body<'tcx>) -> bool {
    let typing_env = ty::TypingEnv::post_analysis(tcx, init.to_def_id());
    body.basic_blocks.iter().any(|data| match &data.terminator().kind {
        TerminatorKind::Assert { .. } | TerminatorKind::Call { target: None, .. } => true,
        TerminatorKind::Call { func, .. } => match crate::lower::fn_def(func.ty(&body.local_decls, tcx)) {
            Some((def_id, args)) => {
                !crate::lower::bindings::is_binding(tcx, def_id)
                    && match crate::lower::items::resolve_instance_in(tcx, typing_env, def_id, args) {
                        Ok(Some(instance)) => match instance.def {
                            ty::InstanceKind::Item(id) => id.is_local(),
                            // A shim of std's, a clone's or a drop's, runs
                            // what it's of: the crate's where that is.
                            _ => (instance.args.types())
                                .any(|ty| ty.ty_adt_def().is_some_and(|adt| adt.did().is_local())),
                        },
                        _ => true,
                    }
            }
            // A closure's or a function value's call, which may be the crate's.
            None => true,
        },
        // A drop of a value of the crate's that has a destructor runs it.
        TerminatorKind::Drop { place, .. } => place.ty(&body.local_decls, tcx).ty.walk().any(|part| {
            part.as_type()
                .and_then(|ty| ty.ty_adt_def())
                .is_some_and(|adt| adt.did().is_local() && adt.has_dtor(tcx))
        }),
        _ => false,
    })
}
