//! What a compiled std struct a call is given as it's held asks of its
//! caller: the steps lowering makes it knowing its back and its length
//! with (`KNOWING_STEPS`, #2042). Its own concept beside the relay of
//! what a callee asks (`relay.rs`), as a file is at most 1500 lines.

use super::*;

/// Each of `knowing`'s steps (`KNOWING_STEPS`) of each compiled std struct
/// a call's `args` hold that is its step's trait, at its type (what one
/// that names none of the caller's parameters asks reaches no caller): its own parameters, `rfold`'s `B` and `F`,
/// themselves, which pick no impl. Only what fills a parameter the callee
/// may step, one bound by `Iterator`, an extension of it or
/// `IntoIterator`, not `format!`'s `Debug` of it. Not of the struct whose own
/// method the call is, `Fuse`'s `rfold` or `len` of it as it is: lowering
/// calls that method's compiled body, which asks what it asks, and makes
/// nothing of the struct knowing.
pub(super) fn knowing_asked<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    knowing: &[(DefId, &str)],
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
) -> Vec<(DefId, ty::GenericArgsRef<'tcx>)> {
    let own_method = tcx.trait_of_assoc(callee).is_some()
        && crate::lower::items::resolve_instance_in(tcx, typing_env, callee, args)
            .is_ok_and(|instance| instance.is_some_and(|i| tcx.impl_of_assoc(i.def_id()).is_some()));
    let called_on = args.types().next().filter(|_| own_method);
    let iterator = std_item(tcx, StdItem::Iterator);
    let into_iterator = std_item(tcx, StdItem::IntoIterator);
    // What the callee's iterator bounds are of, in its own terms: an
    // argument is stepped where its place in them is one.
    let bounded: IndexSet<ty::Ty<'tcx>> = (tcx.clauses_of(callee).instantiate_identity(tcx).into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
        .filter(|bound| {
            let of = bound.def_id();
            of == iterator || of == into_iterator || crate::lower::recognition::is_iterator_extension(tcx, of)
        })
        .map(|bound| bound.self_ty().skip_binder())
        .collect();
    let own = ty::GenericArgs::identity_for_item(tcx, callee);
    let held: IndexSet<ty::Ty<'tcx>> = (args.iter().zip(own.iter()))
        .filter(|&(_, own)| own.as_type().is_some_and(|own| bounded.contains(&own)))
        .filter_map(|(arg, _)| arg.as_type())
        .flat_map(|ty| ty.walk())
        .filter_map(|part| part.as_type())
        .filter(|&ty| Some(ty) != called_on)
        .filter(|&ty| crate::lower::recognition::is_compiled_std_iterator(tcx, ty))
        .collect();
    let mut steps = Vec::new();
    for held in held {
        for &(trait_id, method) in knowing {
            let tr = tcx.erase_and_anonymize_regions(ty::TraitRef::new(tcx, trait_id, [held]));
            if crate::lower::traits::select(tcx, typing_env, tr).is_err() {
                continue;
            }
            let step = trait_method(tcx, trait_id, method);
            let step_args = ty::GenericArgs::for_item(tcx, step, |param, _| match param.index {
                0 => held.into(),
                _ => tcx.mk_param_from_def(param),
            });
            // The impl's method it resolves to, as `STRUCT_STEPS`' are
            // followed: never the trait's, whose own parameters stand in for
            // nothing a caller's copy of a default has.
            if let Ok(Some(instance)) = crate::lower::items::resolve_instance_in(tcx, typing_env, step, step_args)
                && instance.def_id() != step
            {
                steps.push((instance.def_id(), instance.args));
            }
        }
    }
    steps
}
