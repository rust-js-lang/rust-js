//! Which of the crate's functions may return a write's result's `Err`, a
//! `fmt::Error` (ADR 0187) or an `io::Error` (ADR 0462): one that makes one,
//! as chrono's formatting does, `?` of a value's `Result` of a write's
//! error, or that calls or formats what may, of the crate's or a library's,
//! a crate's own writer through std's `write_all`. Only one that returns a
//! write's result passes one on; a crate none of whose functions may, nor
//! its `io::Write` impls' methods (#1072), is lowered as one whose writes
//! never fail (ADRs 0054, 0132).

use crate::collections::IndexSet;
use crate::lower::Body;
use crate::lower::library::Foreign;
use crate::lower::recognition::{
    StdItem, fmt_trait_called, is_err_variant, is_generic_fmt_result, is_std_def, is_write_result_type,
};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::DefId;

/// What a body calls, as its typing environment resolves it.
enum Callee {
    /// The crate's own function, or a library's.
    Known(DefId),
    /// Through a dictionary: a generic `T`'s, which may be any.
    Unknown,
}

/// The functions that return a write's result and may give its `Err`; and
/// whether any write of the crate's or its libraries' may fail: one of
/// those may, or a writer's own `write` may, whose `io::Result<usize>`'s
/// `Err` std's `write_all` gives on, where nothing of the crate's that
/// returns a write's result does (#1072).
pub(super) fn failing_fns<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    foreign: &Foreign<'_, 'tcx>,
) -> (IndexSet<DefId>, bool) {
    let mut failing: IndexSet<DefId> = IndexSet::default();
    let mut calls: Vec<(DefId, Callee)> = Vec::new();
    // What makes an `Err` of a value's `Result` of a write's error, an
    // `io::Result<usize>`'s, and what calls what gives one, each as its
    // caller has it: of which a writer's `write` may give its error to
    // std's `write_all` (#1072).
    let mut erring: IndexSet<DefId> = IndexSet::default();
    let mut gives_errors: Vec<(DefId, DefId)> = Vec::new();
    for body in all_bodies {
        // A closure's own, which its call resolves to, not its maker's (#1042).
        let caller = body.def_id;
        let typing_env = ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(caller));
        // A write's error made, `Err(fmt::Error)`.
        if makes_err(tcx, body, |ty| is_write_result_type(tcx, ty)) {
            failing.insert(caller);
        }
        // Of a value's `Result`, `Err(io::Error::other(..))` of an
        // `io::Result<usize>`.
        if makes_err(tcx, body, |ty| of_write_error(tcx, ty)) {
            erring.insert(caller);
        }
        // Each function it names, as its THIR names it, or a body of MIR
        // alone, a library's the crate compiles (ADR 0914), its MIR.
        for (callee, args, _) in crate::lower::instances::calls_of(body) {
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
            // As the caller has it, a closure's `<F as FnOnce>::Output` the
            // `fmt::Result` its bound says (#1042).
            let output = tcx
                .fn_sig(callee)
                .instantiate(tcx, args)
                .skip_normalization()
                .skip_binder()
                .output();
            let output =
                (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(output))).unwrap_or(output);
            if !is_write_result_type(tcx, output)
                && of_write_error(tcx, output)
                && let Some(Callee::Known(id)) = resolve(tcx, typing_env, callee, args)
            {
                gives_errors.push((caller, id));
            }
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
                // std's own, `write_all`, of a writer rust-js compiled, the
                // crate's or a library's, whose `write` may fail as any of
                // theirs may.
                let own_writer = matches!(callee, Callee::Known(id) if !id.is_local())
                    && args.types().next().is_some_and(|this| {
                        matches!(this.peel_refs().kind(), ty::Adt(adt, _) if foreign.compiled_by_rust_js(adt.did()))
                    });
                calls.push((caller, if own_writer { Callee::Unknown } else { callee }));
            }
        }
    }
    // A writer's `write` may fail where it makes an `Err`, or calls what
    // gives one, as a function that may fail may.
    erring.extend(failing.iter().copied());
    let mut changed = true;
    while changed {
        changed = false;
        for &(caller, callee) in &gives_errors {
            if erring.contains(&callee) && erring.insert(caller) {
                changed = true;
            }
        }
    }
    let writer_fails = erring.iter().any(|&id| {
        (tcx.trait_impl_of_assoc(id)).is_some_and(|imp| is_std_def(tcx, tcx.impl_trait_id(imp), StdItem::IoWrite))
    });
    // A generic `T`'s may fail where any of the crate's or a library's does.
    let any = !failing.is_empty() || writer_fails || foreign.writes_may_fail();
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
    let any = any || !failing.is_empty();
    // Only one that returns a write's result passes its error on; another
    // that formats one panics, as std's `format!` does.
    failing.retain(|&id| {
        tcx.def_kind(id).is_fn_like() && output_of(tcx, id).is_some_and(|output| is_write_result_type(tcx, output))
    });
    (failing, any)
}

/// What the function or closure `id` gives back, as it's declared; `None`
/// of a coroutine's body, an `async` block's.
fn output_of<'tcx>(tcx: TyCtxt<'tcx>, id: DefId) -> Option<ty::Ty<'tcx>> {
    match tcx.def_kind(id) {
        rustc_hir::def::DefKind::Closure => match *tcx.type_of(id).instantiate_identity().skip_normalization().kind() {
            ty::Closure(_, args) => Some(args.as_closure().sig().skip_binder().output()),
            _ => None,
        },
        _ => Some(
            tcx.fn_sig(id)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder()
                .output(),
        ),
    }
}

/// Whether `body` makes an `Err` of a `Result` that `of` says, of its THIR:
/// a write's error, `Err(fmt::Error)`, or a value's `Result` of one. A body
/// of MIR alone, a library's the crate compiles (ADR 0914), has none
/// rust-js lowers: optimized MIR makes it of a constant `fmt::Error`, which
/// is refused.
fn makes_err<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>, of: impl Fn(ty::Ty<'tcx>) -> bool) -> bool {
    (body.thir.exprs.iter()).any(|expr| {
        matches!(expr.kind, ExprKind::Adt(ref adt) if of(expr.ty)
            && is_err_variant(tcx, adt.adt_def.variant(adt.variant_index).def_id))
    })
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
