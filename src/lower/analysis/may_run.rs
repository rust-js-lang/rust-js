//! What a body of the crate's may run of the crate's, the one rule of it
//! (ADR 1006, amended; ADR 0317, amended): the bodies it calls, of the
//! arguments each call gives, a closure or a function it makes or gives std
//! or a binding, the impls of a type of the crate's std is given, each impl
//! of a trait a call through it may be of, the destructors a drop runs, and
//! what each of those runs in turn; each thread-local, `static` and `const`
//! they read by its name, and what the initializer of a thread-local read
//! runs, which may make it there. `None` where it may run what rust-js
//! can't follow. A thread-local's initializer asks what it runs as its
//! module loads (`lazy_locals`); a function whose return alone owns a
//! callback, what it runs before it returns (`owned_by_return`).

use std::collections::{HashMap, HashSet};

use rustc_hir::def::DefKind;
use rustc_middle::mir::{self, CastKind, Local, Operand, PlaceElem, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::{DefId, LocalDefId};

use super::fn_pointers::FnPointers;

use crate::lower::Body;

/// What the crate's bodies may run, asked of each where it starts
/// (`may_run`): the bodies by their ids, each thread-local's initializer by
/// its key, the crate's impls, and the owners' answers it asks of what a fn
/// pointer may be, what std runs, and whether a drop runs a destructor.
pub(in crate::lower) struct Runs<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    bodies: HashMap<DefId, &'a Body<'tcx>>,
    made_of: HashMap<DefId, DefId>,
    impls: Impls,
    /// Each closure the crate makes a `dyn Fn` of only to hand it to JS:
    /// what a drop of one may drop beside what `drop_runs` says.
    fn_dyns: Vec<ty::Ty<'tcx>>,
    pointers: &'a FnPointers<'tcx>,
    std_runs: &'a StdRuns<'a, 'tcx>,
    drop_runs: &'a DropRuns<'a, 'tcx>,
}

impl<'a, 'tcx> Runs<'a, 'tcx> {
    /// What `bodies` may run, `inits` each thread-local's initializer, to
    /// its key, and `fn_dyns` each closure they make a `dyn Fn` of only to
    /// hand it to JS (`closures::handed_fn_dyns`).
    pub(in crate::lower) fn new(
        tcx: TyCtxt<'tcx>,
        bodies: &[&'a Body<'tcx>],
        (inits, fn_dyns): (&HashMap<LocalDefId, LocalDefId>, Vec<ty::Ty<'tcx>>),
        (pointers, std_runs, drop_runs): (&'a FnPointers<'tcx>, &'a StdRuns<'a, 'tcx>, &'a DropRuns<'a, 'tcx>),
    ) -> Self {
        Runs {
            tcx,
            bodies: bodies.iter().map(|&body| (body.def_id, body)).collect(),
            made_of: (inits.iter())
                .map(|(&init, &key)| (key.to_def_id(), init.to_def_id()))
                .collect(),
            impls: Impls::of_crate(tcx),
            fn_dyns,
            pointers,
            std_runs,
            drop_runs,
        }
    }

    /// The body of `id`, the crate's, where it has one.
    pub(in crate::lower) fn body(&self, id: DefId) -> Option<&'a Body<'tcx>> {
        self.bodies.get(&id).copied()
    }

    /// What running `start` may run and read of the crate's, all `may_run`
    /// finds; `None` where it may run what rust-js can't follow.
    pub(in crate::lower) fn of(&self, start: DefId) -> Option<Vec<DefId>> {
        may_run(
            self.tcx,
            (&self.bodies, &self.made_of, &self.impls, &self.fn_dyns),
            (self.pointers, self.std_runs, self.drop_runs),
            start,
        )
    }
}

/// What std's `id`, called of `args`, may run of the crate's bodies
/// (`CellBorrows::std_runs`), each of the args its type says, where it
/// does: `None` of what isn't std's; `Some(None)` where it may run what
/// rust-js can't see.
pub(in crate::lower) type StdRuns<'a, 'tcx> =
    dyn Fn(DefId, ty::GenericArgsRef<'tcx>) -> Option<Option<Vec<(DefId, Option<ty::GenericArgsRef<'tcx>>)>>> + 'a;

/// Whether dropping a value of a type, asked where a body has it, runs a
/// destructor (`drops::drop_runs`, the one rule of it): where it doesn't,
/// what it holds runs nothing as it drops.
pub(in crate::lower) type DropRuns<'a, 'tcx> = dyn Fn(ty::Ty<'tcx>, DefId) -> bool + 'a;

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

/// What `start`, a body of the crate's, may run and read of the crate's,
/// the one rule of it: a binding's call runs what it's given but where it
/// says it runs nothing, `#[rust_js::runs_nothing]`, as React's `memo` and
/// an event handler's setter do, and a JS object's `Deref`, which is the
/// object (`cell_borrows::call_runs_nothing`). The crate's bodies it calls,
/// of the arguments each call gives, a closure or a function it makes or
/// gives std or a binding, the impls of a type of the crate's std is given,
/// each impl of a trait a call through it may be of, the destructors of the
/// crate's a value a drop drops holds, and what each of those runs in turn;
/// each thread-local, `static` and `const` they read by its name, and what
/// the initializer of a thread-local read runs, which may make it there.
/// `None` where it may run what rust-js can't follow: a `dyn Fn`'s call but
/// of what its call was given (`given_dyn`, rd542), a type parameter's call
/// of what it's given, a fn pointer that may be anything (ADR 0907), a drop
/// of a `dyn` whose drop may run a destructor (`drop_runs`), or a body
/// whose MIR it can't read. A `dyn Fn` whose drop runs none may still hold
/// a closure the crate makes one of only to hand it to JS, `fn_dyns`,
/// whose captures' destructors its drop runs.
fn may_run<'tcx>(
    tcx: TyCtxt<'tcx>,
    (bodies, made_of, impls, fn_dyns): (
        &HashMap<DefId, &Body<'tcx>>,
        &HashMap<DefId, DefId>,
        &Impls,
        &[ty::Ty<'tcx>],
    ),
    (pointers, std_runs, drop_runs): (&FnPointers<'tcx>, &StdRuns<'_, 'tcx>, &DropRuns<'_, 'tcx>),
    start: DefId,
) -> Option<Vec<DefId>> {
    let typing_env = ty::TypingEnv::fully_monomorphized();
    let identity = |id: DefId| ty::GenericArgs::identity_for_item(tcx, id);
    let mut reached: Vec<DefId> = Vec::new();
    let mut seen: HashSet<(DefId, ty::GenericArgsRef<'tcx>, Given<'tcx>)> = HashSet::new();
    // Each body to follow, of its arguments, and what its call gave each
    // `dyn Fn` parameter, where it said (`given_dyn`); one reached otherwise
    // was given what may be anything.
    let mut called: Vec<(DefId, ty::GenericArgsRef<'tcx>, Given<'tcx>)> = Vec::new();
    let mut work = vec![(start, identity(start))];
    loop {
        let next = (work.pop().map(|(id, args)| (id, args, Vec::new()))).or_else(|| called.pop());
        let Some((id, args, given_dyn)) = next else { break };
        if !seen.insert((id, args, given_dyn.clone())) {
            continue;
        }
        if !reached.contains(&id) {
            reached.push(id);
        }
        let Some(&body) = bodies.get(&id) else { continue };
        for read in named_reads(body) {
            // A `const` the body it starts at reads itself, not a
            // thread-local's key, is read as its value, where an
            // initializer's module may load after (`load_cycles`, ADR 0031,
            // amended); one a body it calls reads, or a closure it makes,
            // by its name.
            if id == start
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
        // What dropping a value of `ty` runs of the crate's: nothing where
        // its drop runs no destructor (`drop_runs`) and it holds no `dyn
        // Fn`; else each destructor of the crate's a part of it has, through
        // the fields of the crate's types, what std's and a library's hold,
        // and each function a `dyn Fn` may be, and what a library's
        // destructor may call of what it holds, a closure or a function.
        // What rust-js can't follow (`false`) of a `dyn` whose drop may run
        // one, whatever it holds, or a library's destructor holding a `dyn`
        // or a fn pointer (ADRs 0521, 0317, amended).
        let dropped = |ty: ty::Ty<'tcx>, work: &mut Vec<_>| {
            // A part of a type under a binder, a fn pointer's of a fn
            // pointer's arguments: one drops nothing, another isn't known.
            if ty.has_escaping_bound_vars() {
                return ty.is_fn_ptr();
            }
            let fn_dyn = |part: ty::Ty<'tcx>| {
                matches!(part.kind(), ty::Dynamic(predicates, ..)
                    if crate::lower::recognition::is_fn_dyn(tcx, predicates))
            };
            if !ty.walk().filter_map(|part| part.as_type()).any(fn_dyn) && !drop_runs(ty, id) {
                return true;
            }
            let mut parts = vec![ty];
            let mut seen = HashSet::new();
            while let Some(part) = parts.pop() {
                if !seen.insert(part) {
                    continue;
                }
                match *part.kind() {
                    ty::Adt(adt, adt_args) => {
                        let destructor = tcx.adt_destructor(adt.did()).map(|d| d.did);
                        if let Some(destructor) = destructor.filter(|d| d.is_local()) {
                            work.push((destructor, identity(destructor)));
                        }
                        let library = !adt.did().is_local() && !crate::lower::recognition::is_std_crate(tcx, adt.did());
                        if library && destructor.is_some() {
                            for held in adt_args.types().flat_map(values_in) {
                                match *held.kind() {
                                    ty::Closure(f, f_args) if bodies.contains_key(&f) => work.push((f, f_args)),
                                    ty::FnDef(f, f_args) if bodies.contains_key(&f) => {
                                        work.push((f, f_args.skip_binder()))
                                    }
                                    ty::Closure(..) | ty::FnDef(..) | ty::FnPtr(..) | ty::Dynamic(..) => return false,
                                    _ => {}
                                }
                            }
                        }
                        if adt.did().is_local() {
                            parts.extend(
                                adt.all_fields()
                                    .map(|field| field.ty(tcx, adt_args).skip_normalization()),
                            );
                        }
                        parts.extend(adt_args.types());
                    }
                    ty::Closure(_, closure_args) => parts.extend(closure_args.as_closure().upvar_tys()),
                    ty::Tuple(items) => parts.extend(items.iter()),
                    ty::Array(item, _) | ty::Slice(item) => parts.push(item),
                    ty::Dynamic(..) if part.has_param() || drop_runs(part, id) => return false,
                    ty::Dynamic(..) if fn_dyn(part) => parts.extend(fn_dyns.iter().copied()),
                    _ => {}
                }
            }
            true
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
                for part in values_in(ty) {
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
                if !dropped(ty, work) {
                    return false;
                }
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
                    return callee_args.types().all(|ty| dropped(ty, work));
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
            // Of its type arguments, `js::class::<WeakMap<dyn Fn(), String>>()`'s,
            // what they name, of no value given: a `dyn` among them nothing
            // it may call (rd281).
            let sig = tcx.erase_and_anonymize_regions(tcx.instantiate_bound_regions_with_erased(
                tcx.fn_sig(callee).instantiate(tcx, callee_args).skip_normalization(),
            ));
            let named: Vec<ty::Ty<'tcx>> = (callee_args.types())
                .flat_map(values_in)
                .filter(|part| matches!(part.kind(), ty::Closure(..) | ty::FnDef(..) | ty::FnPtr(..)))
                .collect();
            callee_args.types().all(|ty| dropped(ty, work)) && given(&named, work) && given(sig.inputs(), work)
        };
        for data in mir.basic_blocks.iter() {
            match &data.terminator().kind {
                TerminatorKind::Call {
                    func, args: call_args, ..
                } => {
                    let ty = of(func.ty(&mir.local_decls, tcx));
                    let called = match crate::lower::fn_def(ty) {
                        Some((callee, callee_args)) => {
                            match crate::lower::items::resolve_instance_in(tcx, typing_env, callee, callee_args) {
                                Ok(Some(instance)) => match instance.def {
                                    ty::InstanceKind::Item(f) if bodies.contains_key(&f) => {
                                        let gives = (call_args.iter())
                                            .map(|arg| dyn_given(tcx, mir, &arg.node, &given_dyn, &of))
                                            .collect();
                                        called.push((f, instance.args, gives));
                                        true
                                    }
                                    // A `dyn Fn`'s, what its call was given.
                                    ty::InstanceKind::Virtual(method, _)
                                        if tcx
                                            .trait_of_assoc(method)
                                            .is_some_and(|t| tcx.fn_trait_kind_from_def_id(t).is_some()) =>
                                    {
                                        match (call_args.first())
                                            .and_then(|receiver| dyn_given(tcx, mir, &receiver.node, &given_dyn, &of))
                                        {
                                            Some(functions) => {
                                                functions.into_iter().all(|(f, f_args)| match bodies.contains_key(&f) {
                                                    true => {
                                                        work.push((f, f_args));
                                                        true
                                                    }
                                                    false => runs_given(f, f_args, &mut work),
                                                })
                                            }
                                            None => false,
                                        }
                                    }
                                    ty::InstanceKind::Virtual(method, _) => implementors(method, &mut work),
                                    ty::InstanceKind::Item(f) => runs_given(f, instance.args, &mut work),
                                    // A clone of a closure, or of what holds
                                    // one, runs no function it holds: only the
                                    // clones of what it holds (rd542).
                                    ty::InstanceKind::Shim(ty::ShimKind::Clone(_, of)) => {
                                        cloned(tcx, of, impls, &mut work);
                                        true
                                    }
                                    // Another shim of std's, a closure's or
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
                TerminatorKind::Drop { place, .. } if !dropped(of(place.ty(&mir.local_decls, tcx).ty), &mut work) => {
                    return None;
                }
                _ => {}
            }
        }
    }
    reached.retain(|&id| id != start);
    Some(reached)
}

/// What std's clone of a value of `ty` runs of the crate's, `work`: a
/// closure's is the clone of each value it captured, a function's or a
/// pointer's none, and of each type of the crate's among them, its own
/// `Clone`'s methods, a derived one's too, which a clone of what holds it
/// may call (rd542). A function a closure holds isn't called by its clone.
fn cloned<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>, impls: &Impls, work: &mut Vec<(DefId, ty::GenericArgsRef<'tcx>)>) {
    let Some(clone) = tcx.lang_items().clone_trait() else {
        return;
    };
    // A closure's type holds what it captured, as its arguments' types do.
    for part in ty.walk().filter_map(|part| part.as_type()) {
        let ty::Adt(adt, _) = part.kind() else { continue };
        for &(trait_id, imp) in impls.of_type.get(&adt.did()).into_iter().flatten() {
            if trait_id == clone {
                let methods = tcx.associated_item_def_ids(imp).iter().copied();
                work.extend(
                    methods
                        .filter(|&m| tcx.def_kind(m) == DefKind::AssocFn)
                        .map(|m| (m, ty::GenericArgs::identity_for_item(tcx, m))),
                );
            }
        }
    }
}

/// What a body's call gave each of its `dyn Fn` parameters, by position, of
/// the functions and closures it may be; `None` of one it didn't say, and
/// none at all of a body reached by no call that said (rd542).
type Given<'tcx> = Vec<Option<Vec<(DefId, ty::GenericArgsRef<'tcx>)>>>;

/// What `operand`, a pointer to a `dyn Fn` of `body`, may be, where the body
/// says: a function or a closure coerced to it there, of its type, or what
/// its call gave the parameter it is (`given`), through copies and
/// reborrows of variables written once and lent nowhere. `None` where it
/// doesn't, a field's, a collection's or a capture's (rd542). `of` gives a
/// type of the body's in its arguments' terms.
fn dyn_given<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    operand: &Operand<'tcx>,
    given: &Given<'tcx>,
    of: &dyn Fn(ty::Ty<'tcx>) -> ty::Ty<'tcx>,
) -> Option<Vec<(DefId, ty::GenericArgsRef<'tcx>)>> {
    let pointee = of(operand.ty(&body.local_decls, tcx)).builtin_deref(true)?;
    let ty::Dynamic(predicates, ..) = pointee.kind() else {
        return None;
    };
    tcx.fn_trait_kind_from_def_id(predicates.principal_def_id()?)?;
    let place = operand.place()?;
    if !place.projection.is_empty() {
        return None;
    }
    let mut local = place.local;
    for _ in 0..body.local_decls.len() {
        let writes = writes_of(body, local)?;
        // A parameter, which only its call gives.
        if (1..=body.arg_count).contains(&local.as_usize()) {
            return match writes.as_slice() {
                [] => given.get(local.as_usize() - 1).cloned().flatten(),
                _ => None,
            };
        }
        let [written] = writes.as_slice() else { return None };
        match written {
            Rvalue::Cast(CastKind::PointerCoercion(PointerCoercion::Unsize, _), made, _) => {
                return match *of(made.ty(&body.local_decls, tcx)).builtin_deref(true)?.kind() {
                    ty::FnDef(f, f_args) => Some(vec![(f, f_args.skip_binder())]),
                    ty::Closure(f, f_args) => Some(vec![(f, f_args)]),
                    _ => None,
                };
            }
            Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _) if from.projection.is_empty() => {
                local = from.local
            }
            Rvalue::Ref(_, _, from) | Rvalue::CopyForDeref(from) if from.projection[..] == [PlaceElem::Deref] => {
                local = from.local
            }
            _ => return None,
        }
    }
    None
}

/// What each write of `local` in `body` writes, its statements' whole:
/// `None` where a call's result is written to it, a part of it is, or it's
/// lent mutably or through a pointer.
fn writes_of<'a, 'tcx>(body: &'a mir::Body<'tcx>, local: Local) -> Option<Vec<&'a Rvalue<'tcx>>> {
    let mut writes = Vec::new();
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            if let Rvalue::RawPtr(_, lent) | Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, lent) = &assign.1
                && lent.local == local
                && !lent.projection.iter().any(|step| matches!(step, PlaceElem::Deref))
            {
                return None;
            }
            if assign.0.local == local {
                match assign.0.projection.is_empty() {
                    true => writes.push(&assign.1),
                    false => return None,
                }
            }
        }
        match &data.terminator().kind {
            TerminatorKind::Call { destination, .. } if destination.local == local => return None,
            TerminatorKind::Yield { resume_arg, .. } if resume_arg.local == local => return None,
            _ => {}
        }
    }
    Some(writes)
}

/// The types of the values a value of `ty` may hold or be, itself among
/// them: not a function item's type arguments, which name types,
/// `js::class::<WeakMap<dyn Fn(), String>>`'s, of no value (rd281); the
/// crate's function is followed of them where it's called.
fn values_in<'tcx>(ty: ty::Ty<'tcx>) -> Vec<ty::Ty<'tcx>> {
    let mut found = Vec::new();
    let mut walker = ty.walk();
    while let Some(part) = walker.next() {
        if let Some(part) = part.as_type() {
            found.push(part);
            if matches!(part.kind(), ty::FnDef(..)) {
                walker.skip_current_subtree();
            }
        }
    }
    found
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
