//! Which of the crate's functions may return a write's result's `Err`, a
//! `fmt::Error` (ADR 0187) or an `io::Error` (ADR 0462): one that makes one,
//! as chrono's formatting does, `?` of a value's `Result` of a write's
//! error, or that calls or formats what may, of the crate's or a library's,
//! a crate's own writer through std's `write_all`. Only one that returns a
//! write's result passes one on; a crate none of whose functions do is
//! lowered as one whose writes never fail (ADRs 0054, 0132).

use crate::lower::library::Foreign;
use crate::lower::recognition::{fmt_trait_called, is_err_variant, is_generic_fmt_result, is_write_result_type};
use crate::lower::{Body, fn_def};
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
        let caller = tcx.typeck_root_def_id(body.def_id);
        let typing_env = ty::TypingEnv::post_analysis(tcx, caller);
        for expr in body.thir.exprs.iter() {
            match expr.kind {
                ExprKind::Adt(ref adt)
                    if is_write_result_type(tcx, expr.ty)
                        && is_err_variant(tcx, adt.adt_def.variant(adt.variant_index).def_id) =>
                {
                    failing.insert(caller);
                }
                ExprKind::ZstLiteral { .. } => {
                    let Some((callee, args)) = fn_def(expr.ty) else {
                        continue;
                    };
                    // `?` of a value's `Result` of a write's error,
                    // `parse(s)?` of an `io::Result<u32>`: its `Err`, given on.
                    if tcx.is_lang_item(callee, rustc_hir::attrs::lang_items::LangItem::TryTraitBranch)
                        && let Some(tried) = args.types().next()
                        && !is_write_result_type(tcx, tried)
                        && of_write_error(tcx, tried)
                    {
                        failing.insert(caller);
                        continue;
                    }
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
                            // Not one a binder inside holds, `&'a dyn Display` of a
                            // `dyn for<'a> FnMut(&'a dyn Display)`, a type only there.
                            .filter(|part| !part.has_escaping_bound_vars())
                        {
                            if let Some(callee) = resolve(tcx, typing_env, fmt, tcx.mk_args(&[part.into()])) {
                                calls.push((caller, callee));
                            }
                        }
                        continue;
                    }
                    let declared = tcx
                        .fn_sig(callee)
                        .instantiate_identity()
                        .skip_normalization()
                        .skip_binder()
                        .output();
                    // A generic `Result` that's a `fmt::Result` here, `encode<S:
                    // Sink>`'s `Result<(), S::Error>`: what its dictionary's
                    // calls give, which may be any.
                    if is_generic_fmt_result(tcx, typing_env, declared, args) {
                        calls.push((caller, Callee::Unknown));
                        continue;
                    }
                    let output = tcx
                        .fn_sig(callee)
                        .instantiate(tcx, args)
                        .skip_normalization()
                        .skip_binder()
                        .output();
                    // A `Result`'s method that makes one of a value's `Result`
                    // of a write's error, `map` of an `io::Result<usize>`: its
                    // `Err`, given on.
                    let receiver = (tcx
                        .fn_sig(callee)
                        .instantiate(tcx, args)
                        .skip_normalization()
                        .skip_binder())
                    .inputs()
                    .first()
                    .copied();
                    // Not `?`'s own `from_residual`, which `Try::branch` says.
                    if is_write_result_type(tcx, output)
                        && !callee.is_local()
                        && !tcx.is_lang_item(callee, rustc_hir::attrs::lang_items::LangItem::TryTraitFromResidual)
                        && receiver.is_some_and(|from| !is_write_result_type(tcx, from) && of_write_error(tcx, from))
                    {
                        failing.insert(caller);
                        continue;
                    }
                    if is_write_result_type(tcx, output)
                        && let Some(callee) = resolve(tcx, typing_env, callee, args)
                    {
                        // std's own, `write_all`, of a crate's writer, whose
                        // `write` may fail as any of the crate's may.
                        let own_writer = matches!(callee, Callee::Known(id) if !id.is_local())
                            && args.types().next().is_some_and(
                                |this| matches!(this.peel_refs().kind(), ty::Adt(adt, _) if adt.did().is_local()),
                            );
                        calls.push((caller, if own_writer { Callee::Unknown } else { callee }));
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
    // Only one that returns a write's result passes its error on; another
    // that formats one panics, as std's `format!` does.
    failing.retain(|&id| {
        tcx.def_kind(id).is_fn_like()
            && is_write_result_type(
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
    match crate::lower::items::resolve_instance_in(tcx, typing_env, def_id, args) {
        Ok(Some(instance)) if matches!(instance.def, ty::InstanceKind::Virtual(..)) => Some(Callee::Unknown),
        Ok(Some(instance)) => Some(Callee::Known(instance.def_id())),
        Ok(None) if args.has_param() => Some(Callee::Unknown),
        _ => None,
    }
}

/// Whether `ty` is a `Result` of a write's error, a `fmt::Error` or an
/// `io::Error`.
fn of_write_error<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> bool {
    let unit_result = |error: ty::Ty<'tcx>| match ty.kind() {
        ty::Adt(result, _) => ty::Ty::new_adt(tcx, *result, tcx.mk_args(&[tcx.types.unit.into(), error.into()])),
        _ => ty,
    };
    matches!(ty.kind(), ty::Adt(_, args) if args.len() == 2
        && args[1].as_type().is_some_and(|error| is_write_result_type(tcx, unit_result(error))))
}
