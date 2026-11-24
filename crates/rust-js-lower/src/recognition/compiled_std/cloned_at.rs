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

/// Of each std method a runtime model stands for that clones an item,
/// where it raises a borrowed `RefCell`'s panic, by the same walk: each
/// step std's catalog names of a `cloned()`, `Cloned`'s, by its path,
/// `core::iter::adapters::cloned::Cloned::next`, a fold's through
/// `FnMut::call_mut`'s shim, `next`'s through `FnOnce::call_once`'s; an
/// `Option<&T>`'s `cloned`; and a `Vec`'s `extend_from_slice`. Each a row of
/// `clonedAt`, its item at 0, which the lowering reads by the steps a value
/// is asked (`cloned_steps_row`, #2509).
pub(super) fn cloned_methods(tcx: TyCtxt<'_>) -> Result<IndexMap<String, Vec<Option<String>>>, String> {
    let refcell = registry::by_path(tcx, StdCrate::Core, &["cell", "RefCell"]).ok_or("std has no `RefCell`")?;
    let cell = Ty::new_adt(tcx, tcx.adt_def(refcell), tcx.mk_args(&[tcx.types.u8.into()]));
    let lent = Ty::new_imm_ref(tcx, tcx.lifetimes.re_erased, cell);
    let mut kept: IndexMap<String, Vec<Option<String>>> = IndexMap::default();
    // Of one whose walk finds several, the oracle's; each it has none of,
    // said at once.
    let mut unsettled: Vec<String> = Vec::new();
    let settled = |key: &str, sites: Vec<String>, unsettled: &mut Vec<String>| match sites.len() {
        0 => None,
        1 => sites.into_iter().next(),
        _ => cloned_at_oracle(key, 0).or_else(|| {
            unsettled.push(format!("{key}: {}", sites.join(", ")));
            None
        }),
    };
    // The steps of a `Cloned` of a `slice::Iter` of the cell.
    let iter = registry::by_path(tcx, StdCrate::Core, &["slice", "iter", "Iter"]).ok_or("std has no `slice::Iter`")?;
    let iter = Ty::new_adt(
        tcx,
        tcx.adt_def(iter),
        tcx.mk_args(&[tcx.lifetimes.re_erased.into(), cell.into()]),
    );
    let cloned = registry::by_path(tcx, StdCrate::Core, &["iter", "adapters", "cloned", "Cloned"])
        .ok_or("std has no `Cloned`")?;
    let cloned = Ty::new_adt(tcx, tcx.adt_def(cloned), tcx.mk_args(&[iter.into()]));
    let traits = [
        Some(std_item(tcx, StdItem::Iterator)),
        registry::by_path(
            tcx,
            StdCrate::Core,
            &["iter", "traits", "double_ended", "DoubleEndedIterator"],
        ),
        registry::by_path(
            tcx,
            StdCrate::Core,
            &["iter", "traits", "exact_size", "ExactSizeIterator"],
        ),
    ];
    for step in crate::recognition::Steps::ALL.names() {
        let Some(method) = (traits.iter().flatten()).find_map(|&trait_| {
            (tcx.associated_items(trait_)
                .filter_by_name_unhygienic(rustc_span::Symbol::intern(step)))
            .find(|each| matches!(tcx.def_kind(each.def_id), DefKind::AssocFn))
            .map(|each| each.def_id)
        }) else {
            continue;
        };
        let key = format!("core::iter::adapters::cloned::Cloned::{step}");
        let sites = method_sites(tcx, method, (cloned, None), cell)?;
        kept.insert(key.clone(), vec![settled(&key, sites, &mut unsettled)]);
    }
    // `Option<&T>::cloned` and `Vec::extend_from_slice`, each of the impl
    // whose self type it is.
    let option = registry::by_path(tcx, StdCrate::Core, &["option", "Option"]).ok_or("std has no `Option`")?;
    let vec = registry::by_path(tcx, StdCrate::Alloc, &["vec", "Vec"]).ok_or("std has no `Vec`")?;
    for (key, adt, name, self_ty) in [
        (
            "core::option::Option<&_>::cloned",
            option,
            "cloned",
            Ty::new_adt(tcx, tcx.adt_def(option), tcx.mk_args(&[lent.into()])),
        ),
        ("alloc::vec::Vec::extend_from_slice", vec, "extend_from_slice", {
            // Its allocator its parameter's default, `Global`.
            let allocator = tcx.generics_of(vec).param_at(1, tcx);
            let allocator = tcx
                .type_of(allocator.def_id)
                .instantiate_identity()
                .skip_normalization();
            Ty::new_adt(tcx, tcx.adt_def(vec), tcx.mk_args(&[cell.into(), allocator.into()]))
        }),
    ] {
        let name = rustc_span::Symbol::intern(name);
        // The impl whose self type `self_ty` is, its arguments as they make
        // it, `impl<T> Option<&T>`'s `T` the cell.
        let (method, impl_args) = (tcx.inherent_impls(adt).iter())
            .find_map(|&imp| {
                use rustc_trait_selection::infer::TyCtxtInferExt;
                let infcx = tcx.infer_ctxt().build(ty::TypingMode::non_body_analysis());
                let fresh = infcx.fresh_args_for_item(rustc_span::DUMMY_SP, imp);
                let of = tcx.type_of(imp).instantiate(tcx, fresh).skip_normalization();
                let _unified = (infcx.at(&rustc_middle::traits::ObligationCause::dummy(), ty::ParamEnv::empty()))
                    .eq(rustc_trait_selection::infer::DefineOpaqueTypes::No, of, self_ty)
                    .ok()?;
                let method = (tcx.associated_items(imp).filter_by_name_unhygienic(name))
                    .next()?
                    .def_id;
                let args: Vec<ty::GenericArg<'_>> = (fresh.iter())
                    .map(|arg| tcx.erase_and_anonymize_regions(infcx.deeply_resolve_ignoring_regions(arg)))
                    .collect();
                Some((method, args))
            })
            .ok_or_else(|| format!("std has no `{key}`"))?;
        let sites = method_sites(tcx, method, (self_ty, Some(impl_args)), cell)?;
        kept.insert(key.to_string(), vec![settled(key, sites, &mut unsettled)]);
    }
    if !unsettled.is_empty() {
        return Err(format!(
            "these clone a cell at several sites in std's MIR, and test/std-cloned-at.rs's oracle has none of them (#2509): {}",
            unsettled.join("; ")
        ));
    }
    Ok(kept)
}

/// Where `method`, of a `self_ty` over a `cell`, raises a borrowed cell's
/// panic, by the walk (`raised_through`): its own type parameters each a
/// type its clauses take, a closure's a function pointer of its bound's
/// signature, a `Try`'s an `Option` of its output, another `()`; an
/// inherent method's impl's, `impl_args`, a trait's method's `Self`.
fn method_sites<'tcx>(
    tcx: TyCtxt<'tcx>,
    method: DefId,
    (self_ty, impl_args): (Ty<'tcx>, Option<Vec<ty::GenericArg<'tcx>>>),
    cell: Ty<'tcx>,
) -> Result<Vec<String>, String> {
    let generics = tcx.generics_of(method);
    let mut args: Vec<ty::GenericArg<'tcx>> = vec![self_ty.into()];
    // A trait's method's `Self`, else an inherent one's impl's parameters,
    // as `self_ty` gives them.
    if let Some(impl_args) = impl_args {
        args = impl_args;
    }
    let own = generics.parent_count..generics.count();
    for _ in own.clone() {
        args.push(tcx.types.unit.into());
    }
    let clauses: Vec<ty::Clause<'tcx>> = (tcx.clauses_of(method).instantiate_identity(tcx).into_iter())
        .map(|(clause, _)| clause.skip_normalization())
        .collect();
    let fn_traits = [LangItem::Fn, LangItem::FnMut, LangItem::FnOnce].map(|item| tcx.lang_items().get(item));
    let try_trait = tcx.lang_items().get(LangItem::Try);
    for at in own {
        let param = generics.param_at(at, tcx);
        if !matches!(param.kind, ty::GenericParamDefKind::Type { .. }) {
            continue;
        }
        let is_param = |of: Ty<'tcx>| type_parameter_of(of) == Some(at as u32);
        let now = tcx.mk_args(&args);
        let instantiate = |of: Ty<'tcx>| {
            ty::EarlyBinder::bind(tcx, of)
                .instantiate(tcx, now)
                .skip_normalization()
        };
        for clause in &clauses {
            let Some(tr) = clause.as_trait_clause().and_then(|clause| clause.no_bound_vars()) else {
                continue;
            };
            if !is_param(tr.self_ty()) {
                continue;
            }
            if fn_traits.contains(&Some(tr.def_id())) {
                let ty::Tuple(inputs) = *tr.trait_ref.args.type_at(1).kind() else {
                    continue;
                };
                let inputs: Vec<Ty<'tcx>> = inputs.iter().map(instantiate).collect();
                let output = (clauses.iter())
                    .filter_map(|clause| clause.as_projection_clause()?.no_bound_vars())
                    .find(|projection| is_param(projection.self_ty()))
                    .and_then(|projection| projection.term.as_type())
                    .map(instantiate)
                    .unwrap_or(tcx.types.unit);
                let sig = tcx.mk_fn_sig(
                    inputs,
                    output,
                    ty::FnSigKind::default().set_safety(rustc_hir::Safety::Safe),
                );
                args[at] = Ty::new_fn_ptr(tcx, ty::Binder::dummy(sig)).into();
            } else if Some(tr.def_id()) == try_trait {
                let output = (clauses.iter())
                    .filter_map(|clause| clause.as_projection_clause()?.no_bound_vars())
                    .find(|projection| is_param(projection.self_ty()))
                    .and_then(|projection| projection.term.as_type())
                    .map(instantiate)
                    .unwrap_or(tcx.types.unit);
                let option = tcx.lang_items().get(LangItem::Option).ok_or("std has no `Option`")?;
                args[at] = Ty::new_adt(tcx, tcx.adt_def(option), tcx.mk_args(&[output.into()])).into();
            }
        }
    }
    let env = ty::TypingEnv::fully_monomorphized();
    let Ok(Some(instance)) = crate::traits::resolve(tcx, env, method, tcx.mk_args(&args)) else {
        return Ok(Vec::new());
    };
    let mut found = IndexSet::default();
    raised_through(tcx, instance, cell, 0, &mut IndexSet::default(), &mut found, None);
    Ok(found
        .into_iter()
        .map(|(file, line, column)| format!("{file}:{line}:{column}"))
        .collect())
}
