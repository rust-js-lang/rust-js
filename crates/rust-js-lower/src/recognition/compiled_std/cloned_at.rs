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

/// Of each std item a model of the catalog stands for that clones what it's
/// given, where it raises a borrowed `RefCell`'s panic, by the same walk: a
/// function or a method whose type parameter `Clone` bounds, `Vec::resize`'s
/// `T`, its row by its path; an iterator whose `Iterator` impl does,
/// `Repeat<A>`'s `A`, a `Cloned`'s `T`, a row of each step std's catalog
/// names, by its path and the step's name. The rows are the catalog's
/// (`models::bound_paths`), never a list of their own (#2577), each of `clonedAt`, its item
/// at 0, which the lowering reads of the std item a call it lowers is of
/// (`clone_row_of`, #2509).
pub(super) fn cloned_methods(tcx: TyCtxt<'_>) -> Result<IndexMap<String, Vec<Option<String>>>, String> {
    let refcell = registry::by_path(tcx, StdCrate::Core, &["cell", "RefCell"]).ok_or("std has no `RefCell`")?;
    let cell = Ty::new_adt(tcx, tcx.adt_def(refcell), tcx.mk_args(&[tcx.types.u8.into()]));
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
    for path in crate::recognition::models::bound_paths() {
        let Some(item) = crate::recognition::models::resolved(tcx, path) else {
            continue;
        };
        match tcx.def_kind(item) {
            DefKind::Struct | DefKind::Enum => {
                let Some(self_ty) = iterator_of_cells(tcx, item, cell) else {
                    continue;
                };
                // Each step's, where any clones a cell; of one none does, a
                // `Cycle`'s of a slice's iterator, no rows.
                let mut steps = Vec::new();
                for step in crate::recognition::Steps::ALL.names() {
                    let Some(method) = (traits.iter().flatten()).find_map(|&trait_| {
                        (tcx.associated_items(trait_)
                            .filter_by_name_unhygienic(rustc_span::Symbol::intern(step)))
                        .find(|each| matches!(tcx.def_kind(each.def_id), DefKind::AssocFn))
                        .map(|each| each.def_id)
                    }) else {
                        continue;
                    };
                    let key = format!("{path}::{step}");
                    let sites = method_sites(tcx, method, (self_ty, None), cell)?;
                    let site = settled(&key, sites, &mut unsettled);
                    steps.push((key, site));
                }
                if steps.iter().any(|(_, site)| site.is_some()) {
                    kept.extend(steps.into_iter().map(|(key, site)| (key, vec![site])));
                }
            }
            DefKind::Fn | DefKind::AssocFn if clones_a_parameter(tcx, item) => {
                let (self_ty, impl_args) = function_of_cells(tcx, item, cell);
                let sites = method_sites(tcx, item, (self_ty, Some(impl_args)), cell)?;
                if let Some(site) = settled(path, sites, &mut unsettled) {
                    kept.insert(path.to_string(), vec![Some(site)]);
                }
            }
            _ => {}
        }
    }
    if !unsettled.is_empty() {
        return Err(format!(
            "these clone a cell at several sites in std's MIR, and test/std-cloned-at.rs's oracle has none of them (#2509): {}",
            unsettled.join("; ")
        ));
    }
    Ok(kept)
}

/// Whether a type parameter of `item`, a function's or its impl's, is
/// bounded by `Clone` in the clauses it's called under: what it's given it
/// may clone.
fn clones_a_parameter(tcx: TyCtxt<'_>, item: DefId) -> bool {
    let clone = tcx.lang_items().clone_trait();
    (tcx.clauses_of(item).instantiate_identity(tcx).into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause()?.no_bound_vars())
        .any(|tr| Some(tr.def_id()) == clone && type_parameter_of(tr.self_ty()).is_some())
}

/// What a parameter of an impl, `at` of `generics`, is made of a `cell`, as
/// its clauses take it: one `Clone` bounds the cell; one that's an
/// `Iterator` of references to a parameter `Clone` bounds a slice's
/// iterator of cells; one the impl's self type gives a parameter of its
/// own that has a default, that default, `Vec<T, A>`'s `A`'s `Global`;
/// another `()`. A lifetime is erased.
fn of_cells<'tcx>(
    tcx: TyCtxt<'tcx>,
    (imp, clauses): (DefId, &[ty::Clause<'tcx>]),
    param: &ty::GenericParamDef,
    cell: Ty<'tcx>,
) -> ty::GenericArg<'tcx> {
    if !matches!(param.kind, ty::GenericParamDefKind::Type { .. }) {
        return tcx.lifetimes.re_erased.into();
    }
    let clone = tcx.lang_items().clone_trait();
    let iterator = tcx.get_diagnostic_item(rustc_span::sym::Iterator);
    let bounded = |by: Option<DefId>, index: u32| {
        (clauses.iter())
            .filter_map(|clause| clause.as_trait_clause()?.no_bound_vars())
            .any(|tr| Some(tr.def_id()) == by && type_parameter_of(tr.self_ty()) == Some(index))
    };
    // An iterator first, of one `Clone` bounds too, a `Cycle`'s `I`.
    if bounded(iterator, param.index)
        && let Some(iter) = registry::by_path(tcx, StdCrate::Core, &["slice", "iter", "Iter"])
    {
        return Ty::new_adt(
            tcx,
            tcx.adt_def(iter),
            tcx.mk_args(&[tcx.lifetimes.re_erased.into(), cell.into()]),
        )
        .into();
    }
    if bounded(clone, param.index) {
        return cell.into();
    }
    // The default of the self type's parameter it is, `Vec`'s `A = Global`.
    if let ty::Adt(adt, args) = tcx.type_of(imp).instantiate_identity().skip_normalization().kind() {
        let own = tcx.generics_of(adt.did());
        for (at, arg) in args.iter().enumerate() {
            if let Some(given) = arg.as_type()
                && type_parameter_of(given) == Some(param.index)
                && let ty::GenericParamDefKind::Type { has_default: true, .. } = own.param_at(at, tcx).kind
            {
                return (tcx
                    .type_of(own.param_at(at, tcx).def_id)
                    .instantiate_identity()
                    .skip_normalization())
                .into();
            }
        }
    }
    tcx.types.unit.into()
}

/// `item`, an iterator struct whose `Iterator` impl bounds a parameter by
/// `Clone`, made of a `cell` as that impl's clauses take it, `Repeat<A>`'s
/// `Repeat` of the cell, `Cloned<I>`'s of a slice's iterator of cells;
/// none of one whose impl bounds none, a `Map`'s, which clones nothing.
fn iterator_of_cells<'tcx>(tcx: TyCtxt<'tcx>, item: DefId, cell: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let iterator = std_item(tcx, StdItem::Iterator);
    let imp = (tcx.trait_impls_of(iterator).non_blanket_impls().values().flatten().copied()).find(|&imp| {
        matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(), ty::Adt(adt, _) if adt.did() == item)
    })?;
    if !clones_a_parameter(tcx, imp) {
        return None;
    }
    let clauses: Vec<ty::Clause<'_>> = (tcx.clauses_of(imp).instantiate_identity(tcx).into_iter())
        .map(|(clause, _)| clause.skip_normalization())
        .collect();
    let generics = tcx.generics_of(imp);
    let args: Vec<ty::GenericArg<'_>> = (0..generics.count())
        .map(|at| of_cells(tcx, (imp, &clauses), generics.param_at(at, tcx), cell))
        .collect();
    Some(
        tcx.type_of(imp)
            .instantiate(tcx, tcx.mk_args(&args))
            .skip_normalization(),
    )
}

/// `item`, a function, of a `cell`: a method's self type and its impl's
/// arguments, each as `of_cells` makes it of the clauses the method is
/// called under; a free function's, none, `()` its self type, unread.
fn function_of_cells<'tcx>(tcx: TyCtxt<'tcx>, item: DefId, cell: Ty<'tcx>) -> (Ty<'tcx>, Vec<ty::GenericArg<'tcx>>) {
    let Some(imp) = tcx.impl_of_assoc(item) else {
        return (tcx.types.unit, Vec::new());
    };
    let clauses: Vec<ty::Clause<'_>> = (tcx.clauses_of(item).instantiate_identity(tcx).into_iter())
        .map(|(clause, _)| clause.skip_normalization())
        .collect();
    let generics = tcx.generics_of(imp);
    let args: Vec<ty::GenericArg<'_>> = (0..generics.count())
        .map(|at| of_cells(tcx, (imp, &clauses), generics.param_at(at, tcx), cell))
        .collect();
    let self_ty = tcx
        .type_of(imp)
        .instantiate(tcx, tcx.mk_args(&args))
        .skip_normalization();
    (self_ty, args)
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
    let clone = tcx.lang_items().clone_trait();
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
            if Some(tr.def_id()) == clone {
                args[at] = cell.into();
            } else if fn_traits.contains(&Some(tr.def_id())) {
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
