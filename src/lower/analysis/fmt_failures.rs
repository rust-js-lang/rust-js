//! Which of the crate's functions may return a write's result's `Err`, a
//! `fmt::Error` (ADR 0187) or an `io::Error` (ADR 0462): one that makes one,
//! as chrono's formatting does, `?` of a value's `Result` of a write's
//! error, or that calls or formats what may, of the crate's or a library's,
//! a crate's own writer through std's `write_all`. Only one that returns a
//! write's result passes one on; a crate none of whose functions may, nor
//! its `io::Write` impls' methods (#1072), is lowered as one whose writes
//! never fail (ADRs 0054, 0132). In a library, a write or a format through
//! what no consumer compiles for its own, a `dyn`'s or a trait method's own
//! type parameter's, may fail whatever the library writes (#1392); one
//! through another type parameter is its consumer's to compile, where what
//! it gives may fail (`call_may_fail`, ADR 0914).

use crate::collections::IndexSet;
use crate::lower::Body;
use crate::lower::library::Foreign;
use crate::lower::recognition::{
    StdItem, fmt_trait_called, formatted_parts, is_err_variant, is_generic_fmt_result, is_std_def,
    is_write_result_type, of_files, of_io_reading,
};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::DefId;

/// What a body calls, as its typing environment resolves it.
#[derive(Clone, Copy)]
enum Callee {
    /// The crate's own function, or a library's.
    Known(DefId),
    /// Through a dictionary: a generic `T`'s, which may be any.
    Unknown,
    /// Through what no consumer compiles for its own: a `dyn`'s, or a
    /// trait method's own type parameter's, `render<W: Write>`'s `W`, which
    /// a dictionary's callers give (ADR 0914). A library's may fail
    /// whatever it writes itself, as its consumer's may (#1392); a
    /// program's, as a dictionary's.
    Unseen,
}

/// What `failing_fns` found of a crate: its functions that may give back a
/// write's error, whether any write of the crate's or its libraries' may
/// (ADR 0187), and whether one through a dictionary may, a generic `T`'s:
/// any's, but in a library, not where only what no consumer compiles may,
/// a `dyn`'s or a trait method's own type parameter's (#1392).
#[derive(Clone, Copy)]
pub(in crate::lower) struct Failing<'a> {
    pub(in crate::lower) fns: &'a IndexSet<DefId>,
    pub(in crate::lower) any: bool,
    pub(in crate::lower) dictionaries: bool,
}

/// The functions that return a write's result and may give its `Err`;
/// whether any write of the crate's or its libraries' may fail: one of
/// those may, or a writer's own `write` may, whose `io::Result<usize>`'s
/// `Err` std's `write_all` gives on, where nothing of the crate's that
/// returns a write's result does (#1072); and whether a dictionary's may,
/// `Failing::dictionaries`.
pub(super) fn failing_fns<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    foreign: &Foreign<'_, 'tcx>,
    library: bool,
) -> (IndexSet<DefId>, bool, bool) {
    let mut failing: IndexSet<DefId> = IndexSet::default();
    // A library's that write or format through what no consumer compiles,
    // which may fail whatever the library writes itself (#1392).
    let mut unseen: Vec<DefId> = Vec::new();
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
            // `?` of a `Result` of another error, given back as a write's
            // result's `Err` by its `From`, `Err(RandError(4))?` of a
            // function giving an `io::Result<()>`: that `Err`, thrown.
            if tcx.is_lang_item(callee, rustc_hir::attrs::lang_items::LangItem::TryTraitFromResidual)
                && let [returned, residual] = args.types().collect::<Vec<_>>()[..]
                && is_write_result_type(tcx, returned)
                && matches!(residual.kind(), ty::Adt(_, parts) if parts.len() == 2)
                && !of_write_error(tcx, residual)
            {
                failing.insert(caller);
                continue;
            }
            // `{}` of a value, or its `to_string()`: its trait's `fmt`,
            // and that of each type it holds, as std's `Vec<T>` calls `T`'s.
            if let Some(fmt_trait) = fmt_trait_called(tcx, callee) {
                let fmt = tcx.associated_item_def_ids(fmt_trait)[0];
                for part in args.types().next().into_iter().flat_map(formatted_parts) {
                    match resolve_in(tcx, typing_env, caller, fmt, tcx.mk_args(&[part.into()])) {
                        Some(Callee::Unseen) if library => unseen.push(caller),
                        Some(callee) => calls.push((caller, callee)),
                        None => {}
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
            // A byte source's, `read_exact` or `rewind`, of `io::Read`,
            // `BufRead` or `Seek`, its impl's or what the trait provides,
            // of a type that's known or a dictionary's: std's sources'
            // fail, `read_exact` past their end (#1156).
            if is_write_result_type(tcx, output) && of_io_reading(tcx, callee) {
                failing.insert(caller);
                continue;
            }
            // A call of std's files, which the OS may refuse, a `File`'s
            // write or `fs::remove_file`, or that opens one, which generic
            // code may be given to write (#1156).
            if of_files(tcx, callee, args) {
                failing.insert(caller);
                continue;
            }
            if is_write_result_type(tcx, output)
                && let Some(callee) = resolve_in(tcx, typing_env, caller, callee, args)
            {
                match callee {
                    Callee::Unseen if library => unseen.push(caller),
                    // std's own, `write_all`, of a writer rust-js compiled,
                    // whose `write` may fail as any of theirs may.
                    Callee::Known(id) if of_own_writer(foreign, id, args) => calls.push((caller, Callee::Unknown)),
                    callee => calls.push((caller, callee)),
                }
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
    // A generic `T`'s may fail where any of the crate's or a library's does;
    // not where a reader's own method alone does, `Chain::read`'s `?` of
    // its first's `read`: a read's error is no write's, which a generic
    // `T`'s `fmt` or `write` may give (#1156). What calls it and fails is
    // counted itself.
    let writes_fail = |failing: &IndexSet<DefId>| failing.iter().any(|&id| !of_io_reading(tcx, id));
    let any = writes_fail(&failing) || writer_fails || foreign.writes_may_fail();
    // A dictionary's may fail as the library's own writes may, not as only
    // what its consumers give it through a `dyn` may: those are its
    // consumers' to compile, where what they give may fail.
    let dictionaries = any;
    failing.extend(unseen);
    let mut changed = true;
    while changed {
        changed = false;
        for (caller, callee) in &calls {
            let fails = match *callee {
                Callee::Known(id) => failing.contains(&id) || foreign.fails(id),
                Callee::Unknown | Callee::Unseen => any,
            };
            if fails && failing.insert(*caller) {
                changed = true;
            }
        }
    }
    let any = any || writes_fail(&failing);
    let dictionaries = if library { dictionaries } else { any };
    // Only one that returns a write's result passes its error on; another
    // that formats one panics, as std's `format!` does.
    failing.retain(|&id| {
        tcx.def_kind(id).is_fn_like() && output_of(tcx, id).is_some_and(|output| is_write_result_type(tcx, output))
    });
    (failing, any, dictionaries)
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

/// May a call in `caller` of `def_id` of `args`, as `typing_env` resolves
/// it, give back a write's error (ADR 0187), where `failing` is what the
/// crate's analysis found: a byte source's `read_exact` and a call of std's
/// files (#1156); a function of the crate's or a library's that may; std's
/// own, of a writer rust-js compiled; a dictionary's where one may; and a
/// `dyn`'s or a trait method's own type parameter's, which may be any. The
/// lowering's answer of each call, and the one a consumer asks of what a
/// library's function it calls would call, given what it gives it
/// (`instances::needed`, #1392).
pub(in crate::lower) fn call_may_fail<'tcx>(
    tcx: TyCtxt<'tcx>,
    (typing_env, caller): (ty::TypingEnv<'tcx>, DefId),
    foreign: &Foreign<'_, 'tcx>,
    failing: Failing<'_>,
    (def_id, args): (DefId, ty::GenericArgsRef<'tcx>),
) -> bool {
    if !failing.any {
        return false;
    }
    if of_io_reading(tcx, def_id) || of_files(tcx, def_id, args) {
        return true;
    }
    match resolve_in(tcx, typing_env, caller, def_id, args) {
        Some(Callee::Known(id)) => failing.fns.contains(&id) || foreign.fails(id) || of_own_writer(foreign, id, args),
        Some(Callee::Unknown) => failing.dictionaries,
        Some(Callee::Unseen) => true,
        None => false,
    }
}

/// Whether `id`, called with `args`, is std's own of a writer rust-js
/// compiled, the crate's or a library's: `write_all`, which calls its
/// `write`, which may fail as any of theirs may (#1072). Not a library's
/// own function, whose manifest says whether it fails (`Foreign::fails`).
fn of_own_writer(foreign: &Foreign<'_, '_>, id: DefId, args: ty::GenericArgsRef<'_>) -> bool {
    !foreign.compiled_by_rust_js(id)
        && args.types().next().is_some_and(
            |this| matches!(this.peel_refs().kind(), ty::Adt(adt, _) if foreign.compiled_by_rust_js(adt.did())),
        )
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
        Ok(Some(instance)) if matches!(instance.def, ty::InstanceKind::Virtual(..)) => Some(Callee::Unseen),
        Ok(Some(instance)) => Some(Callee::Known(instance.def_id())),
        Ok(None) if args.has_param() => Some(Callee::Unknown),
        _ => None,
    }
}

/// `resolve`'s, of a call in `caller`: `Unseen` too where a dictionary
/// decides by a type parameter of a trait method's own, `render<W:
/// Write>`'s `W`, of the trait's or an impl's method, whose callers no
/// consumer compiles for its own (ADR 0914); an impl's or a trait's own
/// type parameter is `Unknown`, as a function's is.
fn resolve_in<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    caller: DefId,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<Callee> {
    let root = tcx.typeck_root_def_id(caller);
    let of_trait_method = tcx.trait_of_assoc(root).is_some() || tcx.trait_impl_of_assoc(root).is_some();
    let parents = tcx.generics_of(root).parent_count as u32;
    let own_param = |args: ty::GenericArgsRef<'tcx>| {
        (args.types().flat_map(|ty| ty.walk()))
            .any(|part| matches!(part.as_type().map(|ty| *ty.kind()), Some(ty::Param(param)) if param.index >= parents))
    };
    match resolve(tcx, typing_env, def_id, args) {
        Some(Callee::Unknown) if of_trait_method && own_param(args) => Some(Callee::Unseen),
        callee => callee,
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
