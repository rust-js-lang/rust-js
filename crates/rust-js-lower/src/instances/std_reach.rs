//! The check that `recognition::crate_impl_called_by` is whole (#2376): of
//! each call the crate makes of std's generic code given something of its
//! own, the crate's impls std's own body reaches that take an iterator,
//! `from_iter`, `extend`, `sum` and `product` of a collection of its,
//! each one that call's lowering calls as the answer says, so the callers'
//! carrier sees what each is given. One std's body reaches otherwise, a
//! path no answer covers, is a bug of rust-js's, said at once, in the
//! debug build every test runs: a future std path fails a test, not a
//! program. std's MIR is followed where it's there, an instance once,
//! only while what it's given holds something of the crate's, as std's
//! code can reach the crate's no other way.

use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::mir::TerminatorKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;

use crate::Body;

/// What a std instance reaches of the crate's, each impl's method that
/// takes an iterator and what it's given.
type Reached<'tcx> = IndexSet<(DefId, ty::GenericArgsRef<'tcx>)>;

/// Each call of `all_bodies`, as `names` gives them, of std's generic code
/// given something of the crate's: what std's body reaches of the crate's
/// impls that take an iterator is what `crate_impl_called_by` answers of
/// it, or a bug, kept till the compile ends (`std_reach_answered`).
pub(crate) fn check_std_reach<'tcx>(
    tcx: TyCtxt<'tcx>,
    names: &crate::call_graph::Names<'tcx>,
    all_bodies: &[&Body<'tcx>],
) {
    let mut reaching: IndexMap<ty::Instance<'tcx>, Reached<'tcx>> = IndexMap::default();
    // Of this analysis alone, the crate's again once it's given instances.
    crate::session::caches(tcx).std_reach_unanswered.borrow_mut().clear();
    for body in all_bodies {
        let typing_env = ty::TypingEnv::post_analysis(tcx, body.def_id);
        for &(callee, args, span, _) in names.names_of(body.def_id) {
            if callee.is_local() || !mentions_the_crate(args) {
                continue;
            }
            let Ok(Some(instance)) = crate::traits::resolve(tcx, typing_env, callee, args) else {
                continue;
            };
            let reached = reach(tcx, typing_env, instance, &mut reaching);
            let answered: Vec<_> = (crate::recognition::crate_impl_called_by(tcx, typing_env, callee, args)
                .into_iter())
            .filter_map(|(called, called_args)| crate::traits::resolve(tcx, typing_env, called, called_args).ok()?)
            .map(|instance| (instance.def_id(), instance.args))
            .collect();
            let unanswered = reached.iter().find(|each| !answered.contains(each));
            if !reached.is_empty() {
                let answer = if unanswered.is_some() { "unanswered" } else { "answered" };
                rust_js_program::decisions::record("std-reach", answer);
            }
            if let Some(&(def, def_args)) = unanswered {
                (crate::session::caches(tcx).std_reach_unanswered.borrow_mut()).push(format!(
                    "rust-js: std's `{}` at {span:?} calls the crate's `{}` given `{def_args:?}`, which \
                     `recognition::crate_impl_called_by` doesn't answer, so the callers' carrier misses \
                     what it's given (#2376): add that std method to `crate_impl_called_by`, as its \
                     lowering calls the impl, or refuse the call where it's lowered, naming its issue",
                    tcx.def_path_str(callee),
                    tcx.def_path_str(def),
                ));
            }
        }
    }
}

/// What `instance`, std's, reaches of the crate's impls that take an
/// iterator, through its own MIR and each std instance it calls given
/// something of the crate's, each once (`reaching`; one met again on its
/// own way, a recursion's, adds what it's found so far).
fn reach<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    instance: ty::Instance<'tcx>,
    reaching: &mut IndexMap<ty::Instance<'tcx>, Reached<'tcx>>,
) -> Reached<'tcx> {
    if let Some(reached) = reaching.get(&instance) {
        return reached.clone();
    }
    reaching.insert(instance, Reached::default());
    let mut reached = Reached::default();
    let ty::InstanceKind::Item(def) = instance.def else {
        return reached;
    };
    if def.is_local() || !tcx.is_mir_available(def) {
        return reached;
    }
    for data in tcx.optimized_mir(def).basic_blocks.iter() {
        let TerminatorKind::Call { func, .. } = &data.terminator().kind else {
            continue;
        };
        let Some((callee, callee_args)) = func.const_fn_def() else {
            continue;
        };
        // Of generic code's own call, what its parameters' bounds can't
        // normalize is no instance std's code reaches.
        let Ok(callee_args) = instance.try_instantiate_mir_and_normalize_erasing_regions(
            tcx,
            typing_env,
            ty::EarlyBinder::bind(tcx, callee_args),
        ) else {
            continue;
        };
        if !mentions_the_crate(callee_args) {
            continue;
        }
        let Ok(Some(called)) = crate::traits::resolve(tcx, typing_env, callee, callee_args) else {
            continue;
        };
        match called.def {
            ty::InstanceKind::Item(local) if local.is_local() => {
                if crate::generic_facts::takes_an_iterator(tcx, local) {
                    reached.insert((local, called.args));
                }
            }
            _ => reached.extend(reach(tcx, typing_env, called, reaching)),
        }
    }
    reaching.insert(instance, reached.clone());
    reached
}

/// Whether `args` hold something of the crate's: a type, a closure or a
/// function it defines, by which std's code may reach its own.
fn mentions_the_crate(args: ty::GenericArgsRef<'_>) -> bool {
    args.iter()
        .flat_map(|arg| arg.walk())
        .filter_map(|part| part.as_type())
        .any(|part| match *part.kind() {
            ty::Adt(adt, _) => adt.did().is_local(),
            ty::Closure(def, _) | ty::FnDef(def, _) | ty::Coroutine(def, _) | ty::CoroutineClosure(def, _) => {
                def.is_local()
            }
            _ => false,
        })
}

/// That the compile, which ends without an error, left nothing std's code
/// reaches unanswered: one its lowering refuses, a `collect()` into a tuple
/// of the crate's `Extend`s, is said as that refusal, never as this (#2376).
pub fn std_reach_answered(tcx: TyCtxt<'_>) {
    let caches = crate::session::caches(tcx);
    if let Some(said) = said(&caches.std_reach_unanswered.borrow(), tcx.dcx().has_errors().is_some()) {
        panic!("{said}");
    }
}

/// What a compile says of what it left `unanswered`: each, where it ends
/// without an error, `errored`; nothing of one a refusal ended, which says
/// itself.
fn said(unanswered: &[String], errored: bool) -> Option<String> {
    (!errored && !unanswered.is_empty()).then(|| unanswered.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reach no answer covers is said where the compile succeeds, and
    /// only there.
    #[test]
    fn an_unanswered_reach_is_said_of_a_compile_that_succeeds() {
        let unanswered = ["rust-js: std's `partition` calls the crate's `extend`".to_string()];
        assert_eq!(said(&unanswered, false).as_deref(), Some(unanswered[0].as_str()));
        assert_eq!(said(&unanswered, true), None);
        assert_eq!(said(&[], false), None);
    }
}
