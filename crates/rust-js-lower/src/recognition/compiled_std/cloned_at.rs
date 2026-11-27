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
            ty::GenericParamDefKind::Const { .. } => two(tcx).into(),
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
    raised_through(
        tcx,
        (instance, Wanted::Either),
        cell,
        0,
        &mut IndexSet::default(),
        &mut found,
        None,
    );
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
    oracle_sites(key, position).into_iter().next()
}

/// Each site the native oracle ran `key`'s `position` to, each once: two
/// where std's run chooses (`CLONED_AT_ORACLE`).
fn oracle_sites(key: &str, position: usize) -> Vec<String> {
    let mut sites: Vec<String> = Vec::new();
    for line in CLONED_AT_ORACLE.lines().filter(|line| !line.starts_with('#')) {
        // A key may hold spaces, `<T as ToOwned>::to_owned`'s: its
        // position and its site are the line's last two.
        let mut cells = line.rsplitn(3, ' ');
        let (Some(site), Some(at), Some(of)) = (cells.next(), cells.next(), cells.next()) else {
            continue;
        };
        if of == key && at == position.to_string() && !sites.iter().any(|kept| kept == site) {
            sites.push(site.to_string());
        }
    }
    sites
}

/// Whether `item`'s clone of a `RefCell` is placed by another than its
/// call's row: a `Clone` impl's `clone`, a std type's clone being
/// `cloned_at`'s, asked by its type; or one no program calls, a method of
/// a trait std keeps private, its specialization's, or a function it
/// keeps private, `Cycle::new`. A clone into a place, a `clone_from` or a
/// `clone_into`, is walked: what it keeps it writes by the cell's
/// `clone_from`, at its `clonedFromAt` row, and what it adds by its
/// `clone`, at its `clonedAt` row (#2627).
fn cloned_elsewhere(tcx: TyCtxt<'_>, item: DefId) -> bool {
    let Some(trait_) = tcx.impl_of_assoc(item).and_then(|imp| tcx.impl_opt_trait_id(imp)) else {
        return !tcx.visibility(item).is_public();
    };
    (Some(trait_) == tcx.lang_items().clone_trait() && tcx.item_name(item).as_str() == "clone")
        || !tcx.visibility(trait_).is_public()
}

/// Where `instance`'s run calls what tracks its caller with a value of
/// `needle`, a `RefCell`'s `clone`, each into `all`: through each call it
/// makes that holds `needle` in its arguments, resolved as rustc does,
/// specialized and through shims, at the frame nearest it that doesn't
/// track its caller, codegen's (`located_through`), a shim's where its
/// method is declared (#2509).
fn raised_through<'tcx>(
    tcx: TyCtxt<'tcx>,
    (instance, wanted): (ty::Instance<'tcx>, Wanted),
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
        // Of a callee its instance's guesses don't normalize, an `I:
        // IntoIterator` given a `()` for a closure it's never called with,
        // nothing to walk.
        let Ok(func_ty) = instance.try_instantiate_mir_and_normalize_erasing_regions(
            tcx,
            env,
            ty::EarlyBinder::bind(tcx, func.ty(body, tcx)),
        ) else {
            continue;
        };
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
        // A `RefCell`'s `clone` or `clone_from`, which tracks its caller:
        // its panic names where it's placed. Another frame that tracks it
        // places what it calls there too; a `RefCell`'s `borrow` of a sort's
        // comparison is no clone's.
        if let Some(method) = cell_clone(tcx, next.def_id()) {
            if wanted.takes(method) {
                all.extend(at);
            }
            continue;
        }
        raised_through(tcx, (next, wanted), needle, depth + 1, seen, all, at);
    }
}

/// A `RefCell`'s method whose panic the walk places: its `clone`, or its
/// `clone_from`, which a clone into a place calls of what it keeps (#2627).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum CellMethod {
    Clone,
    CloneFrom,
}

/// Which of a `RefCell`'s methods a walk places: one, a call's `clonedAt`
/// row its `clone`'s and its `clonedFromAt` row its `clone_from`'s, or
/// either, a std type's clone's and an iterator's steps'.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Wanted {
    Of(CellMethod),
    Either,
}

impl Wanted {
    fn takes(self, method: CellMethod) -> bool {
        self == Wanted::Either || self == Wanted::Of(method)
    }
}

/// Which of a `RefCell`'s `clone` and `clone_from` `id` is, the clone whose
/// panic the walk places (#2509); `None` of another.
fn cell_clone(tcx: TyCtxt<'_>, id: DefId) -> Option<CellMethod> {
    let imp = tcx.impl_of_assoc(id)?;
    let of_cell = tcx.impl_opt_trait_id(imp) == tcx.lang_items().clone_trait()
        && matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
            ty::Adt(adt, _) if crate::recognition::std_item(tcx, StdItem::RefCell) == adt.did());
    of_cell.then(|| match tcx.item_name(id).as_str() {
        "clone_from" => CellMethod::CloneFrom,
        _ => CellMethod::Clone,
    })
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

/// Of each std iterator a model of the catalog stands for whose `Iterator`
/// impl clones what it holds, `Repeat<A>`'s `A`, a `Cloned`'s `T`, where it
/// raises a borrowed `RefCell`'s panic, by the same walk: a row of each step
/// std's catalog names, by its path and the step's name, its item at 0,
/// which the lowering reads of the steps asked of what a call it lowers
/// makes (`call_clone_site`, #2509, #2577). A std function's own, what a
/// call of it clones of what it's given, is `cloned_calls`'.
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
                    let sites = method_sites(
                        tcx,
                        (method, Wanted::Either),
                        (vec![self_ty.into()], CellAt::Cloned),
                        cell,
                    )?
                    .unwrap_or_default();
                    let site = settled(&key, sites, &mut unsettled);
                    steps.push((key, site));
                }
                if steps.iter().any(|(_, site)| site.is_some()) {
                    kept.extend(steps.into_iter().map(|(key, site)| (key, vec![site])));
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

/// What a parameter of an iterator's impl, `param`, is made of a
/// `cell`, as its clauses take it: one `Clone` bounds the cell; one that's
/// an `Iterator` of references to a parameter `Clone` bounds a slice's
/// iterator of cells; another `()`. A lifetime is erased, a const `2`. A
/// function's impl's are `call_parent`'s, its allocator its default.
fn of_cells<'tcx>(
    tcx: TyCtxt<'tcx>,
    clauses: &[ty::Clause<'tcx>],
    param: &ty::GenericParamDef,
    cell: Ty<'tcx>,
) -> ty::GenericArg<'tcx> {
    match param.kind {
        ty::GenericParamDefKind::Lifetime => return tcx.lifetimes.re_erased.into(),
        ty::GenericParamDefKind::Const { .. } => return two(tcx).into(),
        ty::GenericParamDefKind::Type { .. } => {}
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
        .map(|at| of_cells(tcx, &clauses, generics.param_at(at, tcx), cell))
        .collect();
    Some(
        tcx.type_of(imp)
            .instantiate(tcx, tcx.mk_args(&args))
            .skip_normalization(),
    )
}

/// Which of a method's own type parameters the walk makes a cell: each
/// `Clone` bounds, of an iterator's step, a `cloned`'s `T`; or the one at a
/// position, of a call's (`cloned_calls`).
#[derive(Clone, Copy)]
enum CellAt {
    Cloned,
    At(usize),
}

/// Where `method`, given `parent`, its impl's arguments or a trait's
/// method's `Self`, over a `cell`, raises a borrowed cell's panic, by the
/// walk (`raised_through`): its own type parameters each a type its clauses
/// take, the cell as `cell_at` says, a closure's a function pointer of its
/// bound's signature, a `Try`'s an `Option` of its output, another `()`.
/// None where its clauses don't hold of them, an `I: Iterator` made a cell.
fn method_sites<'tcx>(
    tcx: TyCtxt<'tcx>,
    (method, wanted): (DefId, Wanted),
    (parent, cell_at): (Vec<ty::GenericArg<'tcx>>, CellAt),
    cell: Ty<'tcx>,
) -> Result<Option<Vec<String>>, String> {
    let generics = tcx.generics_of(method);
    let mut args = parent;
    let own = generics.parent_count..generics.count();
    for at in own.clone() {
        args.push(match generics.param_at(at, tcx).kind {
            ty::GenericParamDefKind::Lifetime => tcx.lifetimes.re_erased.into(),
            ty::GenericParamDefKind::Const { .. } => two(tcx).into(),
            ty::GenericParamDefKind::Type { .. } if matches!(cell_at, CellAt::At(cell_at) if cell_at == at) => {
                cell.into()
            }
            ty::GenericParamDefKind::Type { .. } => tcx.types.unit.into(),
        });
    }
    let clauses: Vec<ty::Clause<'tcx>> = (tcx.clauses_of(method).instantiate_identity(tcx).into_iter())
        .map(|(clause, _)| clause.skip_normalization())
        .collect();
    let fn_traits = [LangItem::Fn, LangItem::FnMut, LangItem::FnOnce].map(|item| tcx.lang_items().get(item));
    let clone = tcx.lang_items().clone_trait();
    let try_trait = tcx.lang_items().get(LangItem::Try);
    for at in own.clone() {
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
                if let CellAt::Cloned = cell_at {
                    args[at] = cell.into();
                }
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
    // Of args its clauses don't hold of, an `I: Iterator` made a cell, no
    // instance to walk: each clause of no own parameter but the cell, as
    // what stands for another, a closure's function pointer or a range's
    // `()`, is a guess its own bounds needn't hold of.
    let holds = {
        use rustc_trait_selection::infer::TyCtxtInferExt;
        use rustc_trait_selection::traits::query::evaluate_obligation::InferCtxtExt;
        let infcx = tcx.infer_ctxt().build(ty::TypingMode::non_body_analysis());
        let guessed =
            |index: u32| own.contains(&(index as usize)) && !matches!(cell_at, CellAt::At(at) if at == index as usize);
        (tcx.clauses_of(method).instantiate_identity(tcx).into_iter())
            .map(|(clause, _)| clause.skip_normalization())
            .filter(|clause| {
                let of_guessed = |arg: ty::GenericArg<'tcx>| {
                    arg.walk()
                        .any(|part| part.as_type().and_then(type_parameter_of).is_some_and(guessed))
                };
                !match clause.kind().skip_binder() {
                    ty::ClauseKind::Trait(tr) => tr.trait_ref.args.iter().any(of_guessed),
                    ty::ClauseKind::Projection(projection) => {
                        projection.projection_term.args.iter().any(of_guessed)
                            || projection.term.as_type().is_some_and(|term| of_guessed(term.into()))
                    }
                    _ => false,
                }
            })
            .all(|clause| {
                let clause = ty::EarlyBinder::bind(tcx, clause)
                    .instantiate(tcx, tcx.mk_args(&args))
                    .skip_normalization();
                let obligation = rustc_trait_selection::traits::Obligation::new(
                    tcx,
                    rustc_trait_selection::traits::ObligationCause::dummy(),
                    ty::ParamEnv::empty(),
                    clause,
                );
                infcx.predicate_must_hold_modulo_regions(&obligation)
            })
    };
    if !holds {
        return Ok(None);
    }
    let Ok(Some(instance)) = crate::traits::resolve(tcx, env, method, tcx.mk_args(&args)) else {
        return Ok(Some(Vec::new()));
    };
    let mut found = IndexSet::default();
    raised_through(
        tcx,
        (instance, wanted),
        cell,
        0,
        &mut IndexSet::default(),
        &mut found,
        None,
    );
    Ok(Some(
        found
            .into_iter()
            .map(|(file, line, column)| format!("{file}:{line}:{column}"))
            .collect(),
    ))
}

/// `2`, a `usize` constant: what the walk makes a const parameter, an
/// array's length.
fn two(tcx: TyCtxt<'_>) -> ty::Const<'_> {
    use rustc_middle::ty::consts::ConstExt as _;
    let two = ty::ScalarInt::try_from_target_usize(2u64, tcx).expect("2 is a usize");
    ty::Const::new_value(tcx, ty::ValTree::from_scalar_int(tcx, two), tcx.types.usize)
}

/// The key `clonedAt` names a std function by, the one a call is resolved
/// to: its path as Rust writes it, through no re-export, an impl's written
/// out, `alloc::vec::Vec::<T, A>::resize`, `<alloc::vec::Vec<T> as
/// core::convert::From<&[T]>>::from`. `bun run std` and the lowering each
/// ask it, so both name it alike (#2509).
pub(crate) fn call_key(tcx: TyCtxt<'_>, item: DefId) -> String {
    ty::print::with_no_trimmed_paths!(ty::print::with_no_visible_paths!(tcx.def_path_str(item)))
}

/// Each std function a program may call, by its path or a trait's method:
/// of std's crates, each public module's functions and its types'
/// inherent methods, each method of an impl of a slice, a `str` or an
/// array, and each method of a trait's impl, `ToOwned`'s for `[T]`.
fn callables(tcx: TyCtxt<'_>) -> IndexSet<DefId> {
    use rustc_middle::ty::fast_reject::SimplifiedType;
    let methods = |imp: DefId| {
        (tcx.associated_item_def_ids(imp).iter().copied()).filter(move |&item| tcx.def_kind(item) == DefKind::AssocFn)
    };
    let mut items: IndexSet<DefId> = IndexSet::default();
    let mut types: IndexSet<DefId> = IndexSet::default();
    for krate in [StdCrate::Core, StdCrate::Alloc, StdCrate::Std] {
        let Some(cnum) = crate::recognition::std_crate_num(tcx, krate) else {
            continue;
        };
        for &imp in tcx.trait_impls_in_crate(cnum) {
            items.extend(methods(imp));
        }
        for simplified in [SimplifiedType::Slice, SimplifiedType::Str, SimplifiedType::Array] {
            for &imp in tcx.incoherent_impls(simplified) {
                if imp.krate == cnum {
                    items.extend(methods(imp));
                }
            }
        }
        let mut modules = vec![cnum.as_def_id()];
        let mut seen: IndexSet<DefId> = IndexSet::default();
        while let Some(module) = modules.pop() {
            if !seen.insert(module) {
                continue;
            }
            for child in tcx.module_children(module) {
                let rustc_hir::def::Res::Def(kind, id) = child.res else {
                    continue;
                };
                if crate::recognition::std_crate(tcx, id.krate).is_none() {
                    continue;
                }
                match kind {
                    DefKind::Mod => modules.push(id),
                    DefKind::Fn => {
                        items.insert(id);
                    }
                    DefKind::Struct | DefKind::Enum | DefKind::Union => {
                        types.insert(id);
                    }
                    _ => {}
                }
            }
        }
    }
    for adt in types {
        for &imp in tcx.inherent_impls(adt) {
            items.extend(methods(imp));
        }
    }
    items
}

/// Of each std function a program may call (`callables`), at each of its
/// type parameters, its impl's first, that may be a `RefCell`, where std's
/// code of it raises that cell's clone's panic, mutably borrowed: the frame
/// nearest the cell's `clone` that doesn't track its caller, walked of
/// std's monomorphic MIR (`raised_through`), its other parameters its
/// self type's defaults, `Vec`'s `Global`, a closure a function pointer of
/// its bound, else a `u8`, where its clauses hold of them. Each
/// `clonedAt[call_key][position]`, `null` where none is raised; of several,
/// the native oracle's, else an error. The lowering reads the row of the
/// function rustc resolves a call to, at the position the value it clones
/// is given at (`call_clone_site`): `Rc::unwrap_or_clone`'s, a
/// `Result<&T, E>`'s `cloned`, a slice's `to_owned` (#2509). Not a
/// `Clone` impl's own: a std type's clone is `cloned_at`'s, asked by its
/// type, and its `clone_from` raises at a site its run chooses.
pub(super) fn cloned_calls<'tcx>(tcx: TyCtxt<'tcx>) -> Result<[Rows; 2], String> {
    let refcell = registry::by_path(tcx, StdCrate::Core, &["cell", "RefCell"]).ok_or("std has no `RefCell`")?;
    let cell = Ty::new_adt(tcx, tcx.adt_def(refcell), tcx.mk_args(&[tcx.types.u8.into()]));
    let mut kept: [Rows; 2] = Default::default();
    let mut unsettled: Vec<String> = Vec::new();
    for item in callables(tcx) {
        let generics = tcx.generics_of(item);
        // One a stable program can't call, behind a feature, isn't walked:
        // `VecDeque::prepend_from_within`, whose site its ring chooses.
        let unstable = tcx
            .lookup_stability(item)
            .is_some_and(|stability| stability.is_unstable());
        if !tcx.is_mir_available(item) || generics.count() == 0 || unstable || cloned_elsewhere(tcx, item) {
            continue;
        }
        let key = call_key(tcx, item);
        for (table, method) in kept.iter_mut().zip([CellMethod::Clone, CellMethod::CloneFrom]) {
            let mut sites: Vec<Option<String>> = vec![None; generics.count()];
            for (position, site) in sites.iter_mut().enumerate() {
                if !matches!(
                    generics.param_at(position, tcx).kind,
                    ty::GenericParamDefKind::Type { .. }
                ) {
                    continue;
                }
                // Its other parameters `u8`s, or where their clauses don't
                // hold of one, `Vec`s of the cell, `concat`'s `[V]` of a
                // `V: Borrow<[T]>` (#2627).
                let walked = |other: Ty<'tcx>| {
                    method_sites(
                        tcx,
                        (item, Wanted::Of(method)),
                        (call_parent(tcx, (item, position), (cell, other)), CellAt::At(position)),
                        cell,
                    )
                };
                let found = match walked(tcx.types.u8)? {
                    Some(found) => found,
                    None => walked(vec_of(tcx, cell)?)?.unwrap_or_default(),
                };
                // Of several, the oracle's; of one it found two of, by its
                // runs, none: a `Bound<&T>`'s `cloned`, by its variant.
                *site = match found.len() {
                    0 => None,
                    1 => found.into_iter().next(),
                    _ => match oracle_sites(&key, position)[..] {
                        [] => {
                            unsettled.push(format!("{key} {position} ({method:?}): {}", found.join(", ")));
                            None
                        }
                        [ref one] => Some(one.clone()),
                        _ => None,
                    },
                };
            }
            if sites.iter().any(Option::is_some) && table.insert(key.clone(), sites).is_some() {
                return Err(format!("two of std's functions are `{key}`"));
            }
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

/// Rows of `clonedAt` or `clonedFromAt`: each key's site at each position,
/// `null` where none is raised.
type Rows = IndexMap<String, Vec<Option<String>>>;

/// The arguments of `item`'s impl a walk of a call of it is given: the one
/// at `position` a `cell`; one the impl's self type gives a parameter of its
/// own with a default, that default, `Vec<T, A>`'s `A`'s `Global`; another
/// `other`, a `u8` or a `Vec` of the cell, a const `2`, a lifetime erased.
/// None of a free function's.
fn call_parent<'tcx>(
    tcx: TyCtxt<'tcx>,
    (item, position): (DefId, usize),
    (cell, other): (Ty<'tcx>, Ty<'tcx>),
) -> Vec<ty::GenericArg<'tcx>> {
    let Some(imp) = tcx.impl_of_assoc(item) else {
        return Vec::new();
    };
    let generics = tcx.generics_of(imp);
    let mut args: Vec<ty::GenericArg<'tcx>> = (0..generics.count())
        .map(|at| match generics.param_at(at, tcx).kind {
            _ if at == position => cell.into(),
            ty::GenericParamDefKind::Lifetime => tcx.lifetimes.re_erased.into(),
            ty::GenericParamDefKind::Const { .. } => two(tcx).into(),
            ty::GenericParamDefKind::Type { .. } => other.into(),
        })
        .collect();
    if let ty::Adt(adt, of) = tcx.type_of(imp).instantiate_identity().skip_normalization().kind() {
        let own = tcx.generics_of(adt.did());
        for (each, arg) in of.iter().enumerate() {
            if let Some(index) = arg.as_type().and_then(type_parameter_of)
                && index as usize != position
                && let ty::GenericParamDefKind::Type { has_default: true, .. } = own.param_at(each, tcx).kind
            {
                args[index as usize] = (tcx.type_of(own.param_at(each, tcx).def_id))
                    .instantiate_identity()
                    .skip_normalization()
                    .into();
            }
        }
    }
    args
}

/// A `Vec` of `cell`, `Global`'s: what the walk makes a parameter whose
/// clauses don't hold of a `u8`, a `[V]: Concat<T>`'s `V` (#2627).
fn vec_of<'tcx>(tcx: TyCtxt<'tcx>, cell: Ty<'tcx>) -> Result<Ty<'tcx>, String> {
    let vec = registry::by_path(tcx, StdCrate::Alloc, &["vec", "Vec"]).ok_or("std has no `Vec`")?;
    // Its allocator its parameter's default, `Global`.
    let allocator = tcx.generics_of(vec).param_at(1, tcx).def_id;
    let global = tcx.type_of(allocator).instantiate_identity().skip_normalization();
    Ok(Ty::new_adt(
        tcx,
        tcx.adt_def(vec),
        tcx.mk_args(&[cell.into(), global.into()]),
    ))
}
