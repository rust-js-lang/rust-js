//! Which of the crate's functions may return `Err(fmt::Error)` (ADR 0187):
//! one that makes one, as chrono's formatting does, or that calls or formats
//! what may, of the crate's or a library's. Only one that returns a
//! `fmt::Result` passes one on; a crate none of whose functions do is
//! lowered as one whose writes never fail (ADR 0054).

use crate::lower::Body;
use crate::lower::library::Foreign;
use crate::lower::recognition::{fmt_trait_called, is_err_variant, is_fmt_result_type};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::DefId;
use std::collections::HashSet;

/// What a body calls, as its typing environment resolves it.
enum Callee {
    /// The crate's own function, or a library's.
    Known(DefId),
    /// Through a dictionary: a generic `T`'s, which may be any.
    Unknown,
}

pub(super) fn failing_fns<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    foreign: &Foreign<'_, 'tcx>,
) -> HashSet<DefId> {
    let mut failing: HashSet<DefId> = HashSet::new();
    let mut calls: Vec<(DefId, Callee)> = Vec::new();
    for body in all_bodies {
        let caller = tcx.typeck_root_def_id(body.def_id.to_def_id());
        let typing_env = ty::TypingEnv::post_analysis(tcx, caller);
        for expr in body.thir.exprs.iter() {
            match expr.kind {
                ExprKind::Adt(ref adt)
                    if is_fmt_result_type(tcx, expr.ty)
                        && is_err_variant(tcx, adt.adt_def.variant(adt.variant_index).def_id) =>
                {
                    failing.insert(caller);
                }
                ExprKind::ZstLiteral { .. } => {
                    let &ty::FnDef(callee, args) = expr.ty.kind() else {
                        continue;
                    };
                    // `{}` of a value, or its `to_string()`: its trait's `fmt`,
                    // and that of each type it holds, as std's `Vec<T>` calls `T`'s.
                    if let Some(fmt_trait) = fmt_trait_called(tcx, callee) {
                        let fmt = tcx.associated_item_def_ids(fmt_trait)[0];
                        for part in args
                            .types()
                            .next()
                            .into_iter()
                            .flat_map(|ty| ty.walk())
                            .filter_map(|part| part.as_type())
                        {
                            if let Some(callee) = resolve(tcx, typing_env, fmt, tcx.mk_args(&[part.into()])) {
                                calls.push((caller, callee));
                            }
                        }
                        continue;
                    }
                    let output = tcx
                        .fn_sig(callee)
                        .instantiate(tcx, args)
                        .skip_normalization()
                        .skip_binder()
                        .output();
                    if is_fmt_result_type(tcx, output)
                        && let Some(callee) = resolve(tcx, typing_env, callee, args)
                    {
                        calls.push((caller, callee));
                    }
                }
                _ => {}
            }
        }
    }
    // A generic `T`'s may fail where any of the crate's or a library's does.
    let any = !failing.is_empty() || foreign.any_fails();
    let mut changed = true;
    while changed {
        changed = false;
        for (caller, callee) in &calls {
            let fails = match *callee {
                Callee::Known(id) => failing.contains(&id) || foreign.fails(id),
                Callee::Unknown => any,
            };
            if fails && failing.insert(*caller) {
                changed = true;
            }
        }
    }
    // Only one that returns a `fmt::Result` passes its error on; another
    // that formats one panics, as std's `format!` does.
    failing.retain(|&id| {
        tcx.def_kind(id).is_fn_like()
            && is_fmt_result_type(
                tcx,
                tcx.fn_sig(id)
                    .instantiate_identity()
                    .skip_normalization()
                    .skip_binder()
                    .output(),
            )
    });
    failing
}

/// `def_id` of `args`, as the caller's typing environment resolves it: a
/// function or an impl's method, or `Unknown` where a dictionary decides, a
/// type parameter's or a `dyn`'s. `None` where it's of nothing: a type that
/// doesn't implement the trait.
fn resolve<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<Callee> {
    match ty::Instance::try_resolve(tcx, typing_env, def_id, args) {
        Ok(Some(instance)) if matches!(instance.def, ty::InstanceKind::Virtual(..)) => Some(Callee::Unknown),
        Ok(Some(instance)) => Some(Callee::Known(instance.def_id())),
        Ok(None) if args.has_param() => Some(Callee::Unknown),
        _ => None,
    }
}
