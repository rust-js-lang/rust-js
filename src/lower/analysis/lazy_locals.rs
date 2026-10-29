//! What each thread-local's initializer may run and read as its module
//! loads, where it's made, as JS's `const x = f();` and `export const`
//! are, whatever it runs (the user's decisions, ADR 1006, amended): React
//! DOM's `const isScrollEndEventSupported = isEventSupported("scrollend")`.
//! The pipeline asks of the modules' imports where its module's load can't
//! make it first (`load_cycles::out_of_order`): one that runs what rust-js
//! can't follow, reads itself, or runs a module in an import cycle with its
//! own. A private one of those is made where it's first used, as Rust makes
//! each (ADR 0037, amended), the crate lowered again so (`made_where_used`);
//! a `pub` one is refused.

use std::collections::{HashMap, HashSet};

use rustc_hir::def::DefKind;
use rustc_middle::mir::TerminatorKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::{DefId, LocalDefId};

use super::fn_pointers::FnPointers;

use crate::lower::Body;

/// The thread-locals made where they're first used, and each other one,
/// made as its module loads, by its key.
pub(in crate::lower) struct LazyLocals {
    pub(in crate::lower) lazy: HashSet<LocalDefId>,
    /// Each made as its module loads, as JS's `const x = f();` is, and what
    /// its initializer may run and read there, all `loaded_by` finds, the
    /// one rule; `None` where it may run what rust-js can't follow. The pipeline asks, of the
    /// modules' imports, where its module's load can't make it
    /// (`load_cycles::out_of_order`, ADR 1006, amended).
    pub(in crate::lower) at_load: HashMap<LocalDefId, Option<Vec<DefId>>>,
}

/// The thread-locals, by their keys, of `inits`, each initializer's key:
/// those made where they're first used already, `made_where_used`, which a
/// lowering of the crate found their module's load can't make; and each
/// other, made as its module loads whatever its initializer runs (the
/// user's decision, ADR 1006, amended), and what it may run and read there.
/// A fn pointer's call runs what `pointers` says it may be (ADR 0907).
pub(super) fn lazy_locals<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[&Body<'tcx>],
    (inits, made_where_used): (&HashMap<LocalDefId, LocalDefId>, &HashSet<LocalDefId>),
    pointers: &FnPointers<'tcx>,
    std_runs: &StdRuns<'_, 'tcx>,
) -> LazyLocals {
    let by_id: HashMap<DefId, &Body<'tcx>> = bodies.iter().map(|&body| (body.def_id, body)).collect();
    let mut found = LazyLocals {
        lazy: made_where_used.clone(),
        at_load: HashMap::new(),
    };
    let made_of: HashMap<DefId, DefId> = (inits.iter())
        .map(|(&init, &key)| (key.to_def_id(), init.to_def_id()))
        .collect();
    let impls = Impls::of_crate(tcx);
    // What each initializer may run and read as its module loads, all
    // `loaded_by` finds, which follows each thread-local it reads to its
    // own initializer: the one rule (the user's decision, rd120).
    for body in bodies {
        let Some(init) = body.def_id.as_local() else { continue };
        let Some(&key) = inits.get(&init) else { continue };
        if made_where_used.contains(&key) {
            continue;
        }
        let reached = loaded_by(tcx, (&by_id, &made_of, &impls), (pointers, std_runs), body.def_id);
        found.at_load.insert(key, reached);
    }
    found
}

/// What std's `id`, called of `args`, may run of the crate's bodies
/// (`CellBorrows::std_runs`), each of the args its type says, where it
/// does: `None` of what isn't std's; `Some(None)` where it may run what
/// rust-js can't see.
pub(in crate::lower) type StdRuns<'a, 'tcx> =
    dyn Fn(DefId, ty::GenericArgsRef<'tcx>) -> Option<Option<Vec<(DefId, Option<ty::GenericArgsRef<'tcx>>)>>> + 'a;

/// The crate's trait impls, by the trait and by the type they're of: what
/// a call through a trait may run of the crate's, where which impl isn't
/// known, and what std's code bounding a type of the crate's by a trait
/// may.
struct Impls {
    of_trait: HashMap<DefId, Vec<DefId>>,
    of_type: HashMap<DefId, Vec<(DefId, DefId)>>,
}

impl Impls {
    fn of_crate(tcx: TyCtxt<'_>) -> Self {
        let mut found = Impls {
            of_trait: HashMap::new(),
            of_type: HashMap::new(),
        };
        for (&trait_id, impls) in tcx.all_local_trait_impls(()) {
            for &imp in impls {
                found.of_trait.entry(trait_id).or_default().push(imp.to_def_id());
                let self_ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
                if let Some(adt) = self_ty.ty_adt_def() {
                    found
                        .of_type
                        .entry(adt.did())
                        .or_default()
                        .push((trait_id, imp.to_def_id()));
                }
            }
        }
        found
    }
}

/// What `init`, a thread-local's initializer, made as its module loads, may
/// run and read of the crate's there (ADR 1006, amended), the one rule of
/// it: a binding's call runs what it's given but where it says it runs
/// nothing, `#[rust_js::runs_nothing]`, as React's `memo` and an event
/// handler's setter do, and a JS object's `Deref`, which is the object
/// (`cell_borrows::call_runs_nothing`). The
/// crate's bodies it calls, of the arguments each call gives, a closure
/// or a function it makes or gives std or a binding, the impls of a type of
/// the crate's std is given or a drop runs, each impl of a trait a call
/// through it may be of, and what each of those runs in turn; each
/// thread-local, `static` and `const` they read by its name, and what the
/// initializer of a thread-local read runs, which may make it there. `None`
/// where it may run what rust-js can't follow: a `dyn Fn`'s or a type
/// parameter's call of what it's given, a fn pointer that may be anything
/// (ADR 0907), or a body whose MIR it can't read.
fn loaded_by<'tcx>(
    tcx: TyCtxt<'tcx>,
    (bodies, made_of, impls): (&HashMap<DefId, &Body<'tcx>>, &HashMap<DefId, DefId>, &Impls),
    (pointers, std_runs): (&FnPointers<'tcx>, &StdRuns<'_, 'tcx>),
    init: DefId,
) -> Option<Vec<DefId>> {
    let typing_env = ty::TypingEnv::fully_monomorphized();
    let identity = |id: DefId| ty::GenericArgs::identity_for_item(tcx, id);
    let mut reached: Vec<DefId> = Vec::new();
    let mut seen: HashSet<(DefId, ty::GenericArgsRef<'tcx>)> = HashSet::new();
    let mut work = vec![(init, identity(init))];
    while let Some((id, args)) = work.pop() {
        if !seen.insert((id, args)) {
            continue;
        }
        if !reached.contains(&id) {
            reached.push(id);
        }
        let Some(&body) = bodies.get(&id) else { continue };
        for read in named_reads(body) {
            // A `const` the initializer reads itself, not a thread-local's
            // key, is read as its value where its module may load after
            // (`load_cycles::load_cycles`, ADR 0031, amended); one a body it
            // calls reads, or a closure it makes, by its name.
            if id == init
                && !made_of.contains_key(&read)
                && matches!(tcx.def_kind(read), DefKind::Const { .. } | DefKind::AssocConst { .. })
            {
                continue;
            }
            if !reached.contains(&read) {
                reached.push(read);
            }
            if let Some(&made) = made_of.get(&read) {
                work.push((made, identity(made)));
            }
        }
        let Some(mir) = &body.mir else { return None };
        let mir = &mir.body;
        let of = |ty: ty::Ty<'tcx>| {
            tcx.erase_and_anonymize_regions(
                ty::EarlyBinder::bind(tcx, ty)
                    .instantiate(tcx, args)
                    .skip_normalization(),
            )
        };
        // Each impl's method of `trait_method`'s trait, of the crate's; of
        // an `Fn` trait's, what rust-js can't follow.
        let implementors = |trait_method: DefId, work: &mut Vec<_>| {
            let Some(trait_id) = tcx.trait_of_assoc(trait_method) else {
                return false;
            };
            if tcx.fn_trait_kind_from_def_id(trait_id).is_some() {
                return false;
            }
            for &imp in impls.of_trait.get(&trait_id).into_iter().flatten() {
                if let Some(&method) = tcx.impl_item_implementor_ids(imp).get(&trait_method) {
                    work.push((method, identity(method)));
                }
            }
            true
        };
        // A destructor of the crate's a value of `ty` holds, which its drop runs.
        let dropped = |ty: ty::Ty<'tcx>, work: &mut Vec<_>| {
            for part in ty.walk().filter_map(|part| part.as_type()) {
                if let Some(adt) = part.ty_adt_def()
                    && let Some(destructor) = tcx.adt_destructor(adt.did())
                    && destructor.did.is_local()
                {
                    work.push((destructor.did, identity(destructor.did)));
                }
            }
        };
        // What a call of a library's or a binding's, or a shim of std's, of
        // `args` and taking `inputs`, may run of what it's given of the
        // crate's: a closure or a function among them, a destructor of a type
        // among them, which it may drop, and what a fn pointer or a `dyn`
        // among them may be: of a `dyn Fn`, what rust-js can't follow
        // (`false`). A library's own code runs only what it's given, as
        // its modules load before the crate's.
        let given = |tys: &[ty::Ty<'tcx>], work: &mut Vec<_>| {
            for &ty in tys {
                for part in ty.walk().filter_map(|part| part.as_type()) {
                    match *part.kind() {
                        ty::Closure(f, f_args) if bodies.contains_key(&f) => work.push((f, f_args)),
                        ty::FnDef(..) => {
                            if let Some((f, f_args)) = crate::lower::fn_def(part)
                                && bodies.contains_key(&f)
                            {
                                work.push((f, f_args));
                            }
                        }
                        ty::FnPtr(..) => match pointers.may_be(tcx, part) {
                            Some(may_be) => work.extend(may_be.iter().copied()),
                            None => return false,
                        },
                        ty::Dynamic(predicates, ..) => {
                            if let Some(trait_id) = predicates.principal_def_id() {
                                for &method in tcx.associated_item_def_ids(trait_id) {
                                    if tcx.def_kind(method) == DefKind::AssocFn && !implementors(method, work) {
                                        return false;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                dropped(ty, work);
            }
            true
        };
        // What a call of `callee`, of `callee_args`, that isn't the crate's
        // body runs of the crate's: std's, by the bounds it's called under;
        // a binding's that runs nothing, none; another's, what it's given.
        let runs_given = |callee: DefId, callee_args: ty::GenericArgsRef<'tcx>, work: &mut Vec<_>| {
            if super::cell_borrows::call_runs_nothing(tcx, callee) {
                return true;
            }
            match std_runs(callee, callee_args) {
                Some(Some(runs)) => {
                    work.extend(
                        runs.into_iter()
                            .map(|(f, f_args)| (f, f_args.unwrap_or_else(|| identity(f)))),
                    );
                    callee_args.types().for_each(|ty| dropped(ty, work));
                    return true;
                }
                // Of what `std_runs` can't see, a `dyn`'s, a library's type's
                // impl: what it's given, and the crate's impls of its bounds'
                // traits of a type among them, which std calls through them.
                Some(None) => {
                    let bounds: HashSet<DefId> = (tcx.clauses_of(callee).instantiate_identity(tcx).into_iter())
                        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
                        .flat_map(|bound| {
                            rustc_middle::ty::elaborate::supertrait_def_ids(tcx, bound.skip_binder().def_id())
                        })
                        .collect();
                    for adt in (callee_args.types())
                        .flat_map(|ty| ty.walk())
                        .filter_map(|part| part.as_type()?.ty_adt_def())
                    {
                        for &(trait_id, imp) in impls.of_type.get(&adt.did()).into_iter().flatten() {
                            if bounds.contains(&trait_id) {
                                let methods = tcx.associated_item_def_ids(imp).iter().copied();
                                work.extend(
                                    methods
                                        .filter(|&m| tcx.def_kind(m) == DefKind::AssocFn)
                                        .map(|m| (m, identity(m))),
                                );
                            }
                        }
                    }
                    return given(&callee_args.types().collect::<Vec<_>>(), work);
                }
                None => {}
            }
            let sig = tcx.fn_sig(callee).instantiate(tcx, callee_args).skip_normalization();
            let tys: Vec<ty::Ty<'tcx>> = callee_args
                .types()
                .chain(sig.skip_binder().inputs().iter().copied())
                .collect();
            given(&tys, work)
        };
        for data in mir.basic_blocks.iter() {
            match &data.terminator().kind {
                TerminatorKind::Call { func, .. } => {
                    let ty = of(func.ty(&mir.local_decls, tcx));
                    let called = match crate::lower::fn_def(ty) {
                        Some((callee, callee_args)) => {
                            match crate::lower::items::resolve_instance_in(tcx, typing_env, callee, callee_args) {
                                Ok(Some(instance)) => match instance.def {
                                    ty::InstanceKind::Item(f) if bodies.contains_key(&f) => {
                                        work.push((f, instance.args));
                                        true
                                    }
                                    ty::InstanceKind::Virtual(method, _) => implementors(method, &mut work),
                                    ty::InstanceKind::Item(f) => runs_given(f, instance.args, &mut work),
                                    // A shim of std's, a closure's, a clone's or
                                    // a drop's, runs what it's of.
                                    _ => given(&instance.args.types().collect::<Vec<_>>(), &mut work),
                                },
                                // A type parameter's call, which any impl of
                                // its trait may be.
                                _ => implementors(callee, &mut work),
                            }
                        }
                        None if ty.is_fn_ptr() => match pointers.may_be(tcx, ty) {
                            Some(may_be) => {
                                work.extend(may_be.iter().copied());
                                true
                            }
                            None => false,
                        },
                        None => false,
                    };
                    if !called {
                        return None;
                    }
                }
                TerminatorKind::Drop { place, .. } => dropped(of(place.ty(&mir.local_decls, tcx).ty), &mut work),
                _ => {}
            }
        }
    }
    reached.retain(|&id| id != init);
    Some(reached)
}

/// What `body` reads by its name of the crate's: each thread-local, by its
/// key, `static` and `const`. Its closures are bodies of their own.
fn named_reads(body: &Body<'_>) -> Vec<DefId> {
    (body.thir.exprs.iter())
        .filter_map(|expr| match expr.kind {
            ExprKind::NamedConst { def_id, .. } | ExprKind::StaticRef { def_id, .. } if def_id.is_local() => {
                Some(def_id)
            }
            _ => None,
        })
        .collect()
}
