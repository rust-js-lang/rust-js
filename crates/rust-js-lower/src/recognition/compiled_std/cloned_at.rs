//! Where std raises a `RefCell`'s panic as std's own clone of a type clones
//! it, mutably borrowed (#2509): of each std `Clone` impl, read from std's
//! monomorphic MIR, as `bun run std` writes `$stdSites.clonedAt`; and of a
//! clone the lowering makes, the impl rustc resolves it to, whose row it
//! reads (`cloned_by`). Split from raised.rs by concept, as a file is at
//! most 1500 lines.

use super::super::*;
use super::raised::{located, located_through};
use crate::generic_facts::type_parameter_of;
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::mir;

/// Where std raises a `RefCell`'s panic, cloned mutably borrowed by a
/// std type's `clone`, run natively of the types whose clone reaches it by
/// more than one path (test/std-cloned-at.rs): `<type> <position> <site>`.
const CLONED_AT_ORACLE: &str = include_str!("cloned_at_oracle.txt");

/// The key `clonedAt` names a std `Clone` impl by, of its self type as the
/// impl writes it, `self_ty`: the type's path from its crate,
/// `alloc::collections::btree::map::BTreeMap`, and where its arguments
/// aren't each a parameter of the impl, their shapes, `alloc::boxed::Box<[_],
/// _>` apart from `Box<T>`'s; of an array, `[T; N]`. The lowering asks it of
/// the impl rustc resolves a clone to, so both name it alike (#2509).
fn cloned_at_key(tcx: TyCtxt<'_>, self_ty: Ty<'_>) -> Option<String> {
    let shape = |arg: ty::GenericArg<'_>| match arg.as_type() {
        None => "_".to_string(),
        Some(of) if type_parameter_of(of).is_some() => "_".to_string(),
        Some(of)
            if let ty::Slice(item) = *of.kind()
                && type_parameter_of(item).is_some() =>
        {
            "[_]".to_string()
        }
        Some(_) => "?".to_string(),
    };
    match self_ty.kind() {
        ty::Adt(adt, args) => {
            let path = format!(
                "{}{}",
                tcx.crate_name(adt.did().krate),
                tcx.def_path(adt.did()).to_string_no_crate_verbose()
            );
            let shapes: Vec<String> = args.iter().map(shape).collect();
            Some(match shapes.iter().all(|shape| shape == "_") {
                true => path,
                false => format!("{path}<{}>", shapes.join(", ")),
            })
        }
        ty::Array(..) => Some("[T; N]".to_string()),
        _ => None,
    }
}

/// Of a std `Clone` impl's self type as the impl writes it, `self_ty`, the
/// impl's type parameter each of its arguments holds, by position: `Box<[T],
/// A>`'s `T` at 0. The positions `clonedAt` indexes (#2509).
fn cloned_at_positions(self_ty: Ty<'_>) -> Vec<Option<u32>> {
    let held = |arg: Ty<'_>| (arg.walk()).find_map(|part| part.as_type().and_then(type_parameter_of));
    match self_ty.kind() {
        ty::Adt(_, of) => of.iter().map(|arg| arg.as_type().and_then(held)).collect(),
        ty::Array(item, _) => vec![held(*item)],
        _ => Vec::new(),
    }
}

/// Of each std `Clone` impl whose clone of a `RefCell`, at one of its type's
/// positions, panics, where it raises it (`cloned_at_site`), `clonedAt[its
/// key][its position]`, `null` where no `RefCell` reaches (#2509).
pub(super) fn cloned_at(tcx: TyCtxt<'_>) -> Result<IndexMap<String, Vec<Option<String>>>, String> {
    let clone = tcx.lang_items().clone_trait().ok_or("std has no `Clone`")?;
    let mut kept: IndexMap<String, Vec<Option<String>>> = IndexMap::default();
    for imp in tcx.all_impls(clone) {
        if crate::recognition::std_crate(tcx, imp.krate).is_none() {
            continue;
        }
        let identity = tcx.type_of(imp).instantiate_identity().skip_normalization();
        let (Some(key), positions) = (cloned_at_key(tcx, identity), cloned_at_positions(identity)) else {
            continue;
        };
        let sites = (0..positions.len())
            .map(|at| cloned_at_site(tcx, imp, at))
            .collect::<Result<Vec<_>, _>>()?;
        if sites.iter().all(Option::is_none) {
            continue;
        }
        if kept.insert(key.clone(), sites).is_some() {
            return Err(format!("two of std's `Clone` impls of `{key}` clone a `RefCell`"));
        }
    }
    Ok(kept)
}

/// Where std's `Clone` impl `imp` raises a `RefCell`'s panic, cloned
/// mutably borrowed at its self type's `position`, the frame nearest the
/// `RefCell`'s `clone` that doesn't track its caller: by a walk of std's
/// monomorphic MIR (`raised_through`), that position's parameter a
/// `RefCell` of a `u8`, another its type's default or a `u8`. `None` where no
/// `RefCell` may be there, its impl's clauses not holding of one, or none
/// of its calls reaches one's `clone`; of several, the native oracle's
/// (`CLONED_AT_ORACLE`), a `BTreeMap`'s leaf's line of a few entries, which
/// ADR 0262's exception names of any; else an error. `bun run std` writes
/// each, and the lowering asks it of the impl it resolves a clone to, so a
/// row it reads is there (#2509).
fn cloned_at_site(tcx: TyCtxt<'_>, imp: DefId, position: usize) -> Result<Option<String>, String> {
    use rustc_middle::ty::consts::ConstExt as _;
    use rustc_trait_selection::infer::TyCtxtInferExt;
    use rustc_trait_selection::traits::query::evaluate_obligation::InferCtxtExt;
    let identity = tcx.type_of(imp).instantiate_identity().skip_normalization();
    let (Some(key), positions) = (cloned_at_key(tcx, identity), cloned_at_positions(identity)) else {
        return Ok(None);
    };
    let Some(at) = positions.get(position).copied().flatten().map(|index| index as usize) else {
        return Ok(None);
    };
    let refcell = registry::by_path(tcx, StdCrate::Core, &["cell", "RefCell"]).ok_or("std has no `RefCell`")?;
    let cell = Ty::new_adt(tcx, tcx.adt_def(refcell), tcx.mk_args(&[tcx.types.u8.into()]));
    let clone_fn = tcx.require_lang_item(LangItem::CloneFn, rustc_span::DUMMY_SP);
    let generics = tcx.generics_of(imp);
    // May this parameter be a `RefCell`, whatever the others are?
    let infcx = tcx.infer_ctxt().build(ty::TypingMode::non_body_analysis());
    let fresh = infcx.fresh_args_for_item(rustc_span::DUMMY_SP, imp);
    let probe: Vec<ty::GenericArg<'_>> = (fresh.iter().enumerate())
        .map(|(k, arg)| if k == at { cell.into() } else { arg })
        .collect();
    let probe = tcx.mk_args(&probe);
    let may_be = (tcx.clauses_of(imp).instantiate(tcx, probe).into_iter()).all(|(clause, _)| {
        let obligation = rustc_trait_selection::traits::Obligation::new(
            tcx,
            rustc_trait_selection::traits::ObligationCause::dummy(),
            ty::ParamEnv::empty(),
            clause.skip_normalization(),
        );
        infcx.predicate_may_hold(&obligation)
    });
    if !may_be {
        return Ok(None);
    }
    // Concretely: this one the cell; another, where the self type gives a
    // parameter with a default, `Vec`'s allocator, that default; else a
    // `u8`, or of a const, `2`.
    let mut args: Vec<ty::GenericArg<'_>> = (0..generics.count())
        .map(|k| match generics.param_at(k, tcx).kind {
            _ if k == at => cell.into(),
            ty::GenericParamDefKind::Lifetime => tcx.lifetimes.re_erased.into(),
            ty::GenericParamDefKind::Const { .. } => {
                let two = ty::ScalarInt::try_from_target_usize(2u64, tcx).expect("2 is a usize");
                ty::Const::new_value(tcx, ty::ValTree::from_scalar_int(tcx, two), tcx.types.usize).into()
            }
            ty::GenericParamDefKind::Type { .. } => tcx.types.u8.into(),
        })
        .collect();
    if let ty::Adt(adt, of) = identity.kind() {
        let adt_generics = tcx.generics_of(adt.did());
        for (each, arg) in of.iter().enumerate() {
            if let Some(index) = arg.as_type().and_then(type_parameter_of)
                && index as usize != at
                && let ty::GenericParamDefKind::Type { has_default: true, .. } = adt_generics.param_at(each, tcx).kind
            {
                let default = adt_generics.param_at(each, tcx).def_id;
                args[index as usize] = tcx.type_of(default).instantiate_identity().skip_normalization().into();
            }
        }
    }
    let self_ty = tcx
        .type_of(imp)
        .instantiate(tcx, tcx.mk_args(&args))
        .skip_normalization();
    // One no concrete type of it resolves, a `FlatMap`'s `I` a `RefCell`,
    // which isn't an iterator: never one.
    let env = ty::TypingEnv::fully_monomorphized();
    let Ok(Some(instance)) = crate::traits::resolve(tcx, env, clone_fn, tcx.mk_args(&[self_ty.into()])) else {
        return Ok(None);
    };
    let mut found = IndexSet::default();
    raised_through(tcx, instance, cell, 0, &mut IndexSet::default(), &mut found, None);
    let mut found = found
        .into_iter()
        .map(|(file, line, column)| format!("{file}:{line}:{column}"));
    Ok(match (found.next(), found.next()) {
        (None, _) => None,
        (Some(only), None) => Some(only),
        (Some(_), Some(_)) => Some(cloned_at_oracle(&key, position).ok_or_else(|| {
            format!("`{key}`'s clone of a `RefCell` at its position {position} raises at several sites in std's MIR, and test/std-cloned-at.rs's oracle has none of it")
        })?),
    })
}

/// The native oracle's site of `key`'s `position` (`CLONED_AT_ORACLE`).
fn cloned_at_oracle(key: &str, position: usize) -> Option<String> {
    (CLONED_AT_ORACLE.lines().filter(|line| !line.starts_with('#'))).find_map(|line| {
        let mut cells = line.split(' ');
        (cells.next() == Some(key) && cells.next() == Some(position.to_string().as_str()))
            .then(|| cells.next().map(str::to_string))
            .flatten()
    })
}

/// Where `instance`'s run calls what tracks its caller with a value of
/// `needle`, a `RefCell`'s `clone`, each into `all`: through each call it
/// makes that holds `needle` in its arguments, resolved as rustc does,
/// specialized and through shims, at the frame nearest it that doesn't
/// track its caller, codegen's (`located_through`), a shim's where its
/// method is declared (#2509).
fn raised_through<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: ty::Instance<'tcx>,
    needle: Ty<'tcx>,
    depth: usize,
    seen: &mut IndexSet<ty::Instance<'tcx>>,
    all: &mut IndexSet<(String, usize, usize)>,
    called_at: Option<(String, usize, usize)>,
) {
    if depth > 24 || !seen.insert(instance) {
        return;
    }
    // Of one with no body here, an intrinsic, a `dyn`'s method or an item
    // whose MIR isn't kept, nothing to walk.
    match instance.def {
        ty::InstanceKind::Intrinsic(_) | ty::InstanceKind::Virtual(..) => return,
        ty::InstanceKind::Item(def) if !tcx.is_mir_available(def) => return,
        _ => {}
    }
    let env = ty::TypingEnv::fully_monomorphized();
    let body = tcx.instance_mir(instance.def);
    // A frame that tracks its caller places what it calls where it was
    // called, `called_at`: a `RefCell`'s own `clone` of one it holds, which
    // names its caller's line; at the walk's root, the program's, no site.
    let tracks = instance.def.requires_caller_location(tcx);
    for block in body.basic_blocks.iter() {
        let terminator = block.terminator();
        let mir::TerminatorKind::Call { func, fn_span, .. } = &terminator.kind else {
            continue;
        };
        let func_ty = instance.instantiate_mir_and_normalize_erasing_regions(
            tcx,
            env,
            ty::EarlyBinder::bind(tcx, func.ty(body, tcx)),
        );
        let ty::FnDef(callee, args) = *func_ty.kind() else {
            continue;
        };
        let Some(args) = args.no_bound_vars() else {
            continue;
        };
        let holds = args
            .types()
            .any(|t| t.walk().any(|part| part.as_type() == Some(needle)));
        if !holds {
            continue;
        }
        let Ok(Some(next)) = crate::traits::resolve(tcx, env, callee, args) else {
            continue;
        };
        // Where this call is placed: this frame's line of it, a shim's where
        // its method is declared, or of a frame that tracks its caller,
        // where it was called.
        let at = match (tracks, instance.def) {
            (true, _) => called_at.clone(),
            (false, ty::InstanceKind::Item(_)) => located_through(
                tcx,
                None,
                body,
                mir::SourceInfo {
                    span: *fn_span,
                    ..terminator.source_info
                },
            ),
            (false, _) => Some(located(tcx, tcx.def_span(instance.def_id()))),
        };
        // A call of what tracks its caller, a `RefCell`'s `clone` or a
        // frame that tracks on to it: its panic names where it's placed.
        if next.def.requires_caller_location(tcx) {
            all.extend(at);
            continue;
        }
        raised_through(tcx, next, needle, depth + 1, seen, all, at);
    }
}

/// What rustc resolves `ty`'s `clone` to, in `typing_env` (#2509): std's
/// impl, by its `clonedAt` key, with the part its type holds at each
/// position as the clone's instance has it, a `HashMap` of `u8` keys and
/// cells' `[u8, the cell, RandomState, Global]`; the crate's own derive; or
/// another, a hand-written impl's, whose clone places its own.
pub(crate) enum ClonedBy<'tcx> {
    Std { key: String, parts: Vec<Option<Ty<'tcx>>> },
    Derive(DefId),
    Other,
}

pub(crate) fn cloned_by<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> ClonedBy<'tcx> {
    let clone = tcx.require_lang_item(LangItem::CloneFn, rustc_span::DUMMY_SP);
    let Some(instance) = crate::traits::resolve(tcx, typing_env, clone, tcx.mk_args(&[ty.into()]))
        .ok()
        .flatten()
    else {
        return ClonedBy::Other;
    };
    let Some(imp) = tcx.impl_of_assoc(instance.def_id()) else {
        return ClonedBy::Other;
    };
    if crate::recognition::std_crate(tcx, imp.krate).is_none() {
        return match tcx.is_automatically_derived(imp) {
            true => ClonedBy::Derive(imp),
            false => ClonedBy::Other,
        };
    }
    let identity = tcx.type_of(imp).instantiate_identity().skip_normalization();
    let Some(key) = cloned_at_key(tcx, identity) else {
        return ClonedBy::Other;
    };
    let parts = (cloned_at_positions(identity).into_iter())
        .map(|at| at.map(|index| instance.args.type_at(index as usize)))
        .collect();
    ClonedBy::Std { key, parts }
}
