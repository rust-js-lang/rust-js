//! Which of its libraries' functions a crate compiles itself (ADR 0914):
//! one its call gives a value to copy or drop that its export takes nothing
//! for, or what may fail that it writes or formats through where its export
//! never fails (`failing.rs`, #1392), and the library's private functions an
//! instance calls; and the refusal of an instance's read of its library's
//! private state.

use super::*;

/// What the crate's analysis found that asks what its calls give a
/// library's functions: whose impls' dictionaries there are, what drops,
/// the drops and copies its own functions' type parameters are given, and
/// its bodies, its own and the instances it has.
pub(in crate::lower) struct Analyzed<'a, 'tcx> {
    pub(in crate::lower) trait_impls: &'a [DefId],
    pub(in crate::lower) drops: (
        bool,
        &'a crate::lower::analysis::ClosureFacts<'tcx>,
        &'a crate::lower::analysis::Counted<'tcx>,
    ),
    pub(in crate::lower) drop_params: &'a IndexMap<DefId, Vec<u32>>,
    /// The type parameters of its own functions given a copy, and a
    /// `Clone` dictionary (ADR 0914, amended).
    pub(in crate::lower) given: crate::lower::traits::Given<'a>,
    /// Its functions that may give back a write's error, and whether any
    /// of its writes may (ADR 0187).
    pub(in crate::lower) failing: crate::lower::analysis::Failing<'a>,
    pub(in crate::lower) bodies: Vec<&'a Body<'tcx>>,
}

/// What the analyzed crate asks of its calls of a library's functions.
struct Calls<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    recognition: crate::lower::recognition::Recognition<'a, 'tcx>,
    foreign: &'a crate::lower::library::Foreign<'a, 'tcx>,
    /// Whether a value of a type that's no type parameter runs a destructor
    /// where it's dropped, as the crate drops it.
    runs: &'a dyn Fn(Ty<'tcx>) -> bool,
    /// The type parameters of the crate's own functions given a drop.
    drop_params: &'a IndexMap<DefId, Vec<u32>>,
    /// The type parameters of the crate's own functions given a copy.
    copied: &'a IndexSet<(DefId, u32)>,
    /// And given a `Clone` dictionary (ADR 0914, amended).
    clones: &'a IndexSet<(DefId, u32)>,
    /// And given nothing that may look like `None` (ADR 0051, amended).
    unboxed: &'a IndexSet<(DefId, u32)>,
    /// Whether a value of a type is its own clone, as the crate counts its
    /// `Rc`s.
    clones_itself: &'a dyn Fn(Ty<'tcx>) -> bool,
    /// What may fail of the crate's writes (ADR 0187).
    failing: crate::lower::analysis::Failing<'a>,
    /// The `Rc`s the crate counts (ADR 0852).
    counted: &'a crate::lower::analysis::Counted<'tcx>,
}

/// The library functions the crate's bodies, its own and the instances it
/// has, call where they're given more than they take, and the private ones
/// an instance calls, which aren't yet instances, each with the module of
/// the crate's it's placed in, its first caller's (ADR 0914); refused,
/// where one reads its library's private state.
pub(in crate::lower) fn needed<'tcx>(
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    instances: &Instances,
    analyzed: Analyzed<'_, 'tcx>,
) -> Vec<(DefId, LocalModId)> {
    let tcx = foreign.tcx();
    let Analyzed {
        trait_impls,
        drops: (library, closure_facts, counted),
        drop_params,
        given,
        failing,
        bodies,
    } = analyzed;
    let recognition = crate::lower::recognition::Recognition {
        tcx,
        typing_env: ty::TypingEnv::fully_monomorphized(),
        trait_impls,
        foreign,
    };
    let runs = |t: Ty<'tcx>| crate::lower::drops::drop_runs(recognition, (library, closure_facts), counted, t);
    let clones_itself = |t: Ty<'tcx>| crate::lower::analysis::clones_itself(tcx, counted, t);
    let calls = Calls {
        tcx,
        recognition,
        foreign,
        runs: &runs,
        drop_params,
        copied: given.copied,
        clones: given.clones,
        unboxed: given.unboxed,
        clones_itself: &clones_itself,
        failing,
        counted,
    };
    let mut more: Vec<(DefId, LocalModId)> = Vec::new();
    let mut may_drop: IndexMap<(DefId, u32), bool> = IndexMap::default();
    for body in bodies {
        let Some(mir) = &body.mir else { continue };
        let caller = tcx.typeck_root_def_id(body.def_id);
        let instance = instances.contains(tcx, caller);
        // A library's namespace's method called through a value: the
        // value's function, its library's JS, none compiled here, which
        // would be no instance JS gave; refused where the call gives it what
        // its export takes nothing for, but where its result only
        // `.flatten()` reads, of what it only makes and returns (rd311, ADRs
        // 0914, 1037).
        let (by_name, through) = called_through(tcx, &mir.body);
        for (callee, args, span, destination) in through {
            if !compiled_here(foreign, tcx, callee) {
                continue;
            }
            let flattened =
                (crate::lower::analysis::flattened_alike(&calls.recognition, (callee, args))).filter(|&p| {
                    (foreign.item(callee).and_then(|item| item.calling.param(p)))
                        .is_some_and(|param| param.has(crate::library::ParamFact::Unboxed))
                        && crate::lower::analysis::read_by_flatten(&calls.recognition, &mir.body, destination)
                });
            let given = (caller, callee, args);
            if gives_more(&calls, given, &|index| Some(index) != flattened, &mut may_drop) {
                let other = gives_more(&calls, given, &|_| false, &mut may_drop);
                crate::lower::classes::refuse_through(tcx, callee, span, !other);
            }
        }
        for &(callee, args) in &by_name {
            if !compiled_here(foreign, tcx, callee)
                || instances.contains(tcx, callee)
                || more.iter().any(|&(id, _)| id == callee)
            {
                continue;
            }
            // A namespace's method by its name, `Reconciler.flush_sync(f)
            // .flatten()`, is its library's as through a value (rd324).
            let flattened = crate::lower::analysis::flattened_by_name(&calls.recognition, &mir.body, (callee, args));
            // An instance's call of what its library doesn't export, its own
            // too; else one given more than its export takes.
            let private = instance && foreign.item(callee).is_none();
            if private
                || gives_more(
                    &calls,
                    (caller, callee, args),
                    &|index| Some(index) != flattened,
                    &mut may_drop,
                )
            {
                more.push((callee, crate::lower::body_module(tcx, instances, body.def_id)));
            }
        }
        if instance {
            refuse_private_state(&calls, body.def_id, &mir.body);
        }
    }
    more
}

/// A call of a namespace's method through a value: the method, its generic
/// arguments, where it's called, and the local its result is.
type ThroughCall<'tcx> = (DefId, ty::GenericArgsRef<'tcx>, rustc_span::Span, mir::Local);

/// Each function `body` calls or names as a value, and its generic
/// arguments there, but a namespace's method called through a value; and
/// each of those.
fn called_through<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
) -> (Vec<(DefId, ty::GenericArgsRef<'tcx>)>, Vec<ThroughCall<'tcx>>) {
    let through: Vec<_> = (body.basic_blocks.iter())
        .filter_map(|data| match &data.terminator().kind {
            TerminatorKind::Call {
                func,
                args,
                destination,
                fn_span,
                ..
            } => {
                let (callee, generic) = func.const_fn_def()?;
                let through = crate::lower::classes::member(tcx, callee)
                    == Some(crate::lower::classes::Member::Namespace)
                    && args
                        .first()
                        .is_some_and(|receiver| !crate::lower::classes::names_namespace(tcx, body, &receiver.node));
                through.then_some((callee, generic, *fn_span, destination.local))
            }
            _ => None,
        })
        .collect();
    let mut by_name = called(body);
    for &(callee, generic, ..) in &through {
        if let Some(at) = by_name.iter().position(|&named| named == (callee, generic)) {
            by_name.remove(at);
        }
    }
    (by_name, through)
}

/// Whether `id` is a function of a rust-js library a consumer can compile:
/// one with MIR, not a trait's or an impl's, which a dictionary calls, nor
/// a binding, nor `@rust-js/std`'s.
pub(in crate::lower::instances) fn compiled_here(
    foreign: &crate::lower::library::Foreign<'_, '_>,
    tcx: TyCtxt<'_>,
    id: DefId,
) -> bool {
    !id.is_local() && foreign.is_library(id.krate) && super::consumer_compiles(tcx, id) && tcx.is_mir_available(id)
}

/// Whether `caller`'s call of `callee`, of `args`, gives it a value to copy
/// or drop that its export takes nothing for: of a type parameter its body
/// may drop, a value whose drop runs a destructor; of one its `Copy` bound
/// copies, a value whose copy isn't itself (ADRs 0098, 0289, 0914).
fn gives_more<'tcx>(
    calls: &Calls<'_, 'tcx>,
    (caller, callee, args): (DefId, DefId, ty::GenericArgsRef<'tcx>),
    counted: &dyn Fn(u32) -> bool,
    may_drop: &mut IndexMap<(DefId, u32), bool>,
) -> bool {
    use crate::library::ParamFact;
    let tcx = calls.tcx;
    let item = calls.foreign.item(callee);
    let takes = |index: u32, fact: ParamFact| {
        (item.and_then(|item| item.calling.param(index))).is_some_and(|param| param.has(fact))
    };
    let copied = crate::lower::analysis::copy_bounded(tcx, callee);
    let cloned = crate::lower::analysis::clone_bounded(tcx, callee);
    (0..tcx.generics_of(callee).count() as u32).any(|index| {
        let Some(given) = args.get(index as usize).and_then(|arg| arg.as_type()) else {
            return false;
        };
        let caller_drops = |index: &u32| {
            calls
                .drop_params
                .get(&caller)
                .is_some_and(|given| given.contains(index))
        };
        let drops = match given.kind() {
            ty::Param(param) => caller_drops(&param.index),
            // One that owns the caller's, a closure holding a `Vec<K>`, as
            // its caller is given a drop for `K` (#1031).
            _ if given.has_param() => crate::lower::drops::owned_params(tcx, given).iter().any(caller_drops),
            _ => (calls.runs)(given),
        };
        let copies = match given.kind() {
            ty::Param(param) => calls.copied.contains(&(caller, param.index)),
            _ => !crate::lower::analysis::copies_itself(tcx, given),
        };
        // A clone of it that isn't the value, which an export that takes no
        // `Clone` dictionary can't make (ADR 0914, amended).
        let clones = match given.kind() {
            ty::Param(param) => calls.clones.contains(&(caller, param.index)),
            _ => !(calls.clones_itself)(given),
        };
        // A value that may look like `None`, whose `Some` an export that
        // boxes none gives as it is (ADR 0051, amended).
        let nullish = match given.kind() {
            ty::Param(param) => !calls.unboxed.contains(&(caller, param.index)),
            _ => crate::lower::representation::may_look_like_none(calls.recognition.tcx, given),
        };
        (drops && !takes(index, ParamFact::Drop) && body_may_drop(calls, (callee, index), may_drop))
            || (nullish
                && counted(index)
                && takes(index, ParamFact::Unboxed)
                && body_may_box(calls, (callee, index), &mut IndexSet::default()))
            || (copies && copied.contains(&index) && !takes(index, ParamFact::Copy))
            || (clones && cloned.contains(&index) && !takes(index, ParamFact::Clone))
    }) || items_given_drops(calls, (caller, callee, args), &takes)
        || counts_otherwise(calls, (caller, callee, args), &takes)
        || super::failing::fails_given(calls.tcx, calls.foreign, calls.failing, (caller, callee, args))
}

/// Whether a call gives `callee`, a library's function, a type an `Rc` of
/// which the crate counts otherwise than its export's JS does
/// (`ParamFact::Counted`, #1234): of each type parameter its `Rc`s hold,
/// the crate counts one of them where the export counts none, or none where
/// it counts one; or the export counts one of two or more, which one fact
/// can't say each of.
fn counts_otherwise<'tcx>(
    calls: &Calls<'_, 'tcx>,
    (caller, callee, args): (DefId, DefId, ty::GenericArgsRef<'tcx>),
    takes: &dyn Fn(u32, crate::library::ParamFact) -> bool,
) -> bool {
    let tcx = calls.tcx;
    let env = ty::TypingEnv::post_analysis(tcx, caller);
    let counts = |pointee| calls.counted.counts(tcx, pointee);
    (crate::lower::analysis::params_counted(tcx, callee, Some((args, env)), &counts).into_iter()).any(
        |(index, (counted, pointees))| {
            let exported = takes(index, crate::library::ParamFact::Counted);
            counted != exported || (exported && pointees > 1)
        },
    )
}

/// Whether a call gives `callee`, a library's function, a std associated
/// type of a type parameter whose values drop, `<Gen as Iterator>::Item`
/// a `Loud`, where its export takes no drop for that slot (ADR 0489): an
/// iterator with no destructor may give items with one, which its library's
/// own calls never gave it (#1031). Of the caller's own slot, as its
/// analysis gives it one.
fn items_given_drops<'tcx>(
    calls: &Calls<'_, 'tcx>,
    (caller, callee, args): (DefId, DefId, ty::GenericArgsRef<'tcx>),
    takes: &dyn Fn(u32, crate::library::ParamFact) -> bool,
) -> bool {
    let tcx = calls.tcx;
    let typing_env = ty::TypingEnv::post_analysis(tcx, caller);
    let given = |slot: u32| {
        calls
            .drop_params
            .get(&caller)
            .is_some_and(|given| given.contains(&slot))
    };
    let callers = std::cell::OnceCell::new();
    let items = crate::lower::traits::drop_items(tcx, calls.foreign, callee);
    items.into_iter().enumerate().any(|(at, item)| {
        if takes(
            crate::lower::traits::item_slot(tcx, callee, at),
            crate::library::ParamFact::Drop,
        ) {
            return false;
        }
        let item = ty::EarlyBinder::bind(tcx, item)
            .instantiate(tcx, args)
            .skip_normalization();
        let item = (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(item))).unwrap_or(item);
        match item.kind() {
            ty::Param(param) => given(param.index),
            ty::Alias(..) => (callers.get_or_init(|| crate::lower::traits::drop_items(tcx, calls.foreign, caller)))
                .iter()
                .position(|&own| own == item)
                .is_some_and(|own| given(crate::lower::traits::item_slot(tcx, caller, own))),
            _ => !item.has_param() && (calls.runs)(item),
        }
    })
}

/// Whether a library's `id` may drop a value of its type parameter `index`
/// it's given: its MIR drops a place that holds one, where it runs or
/// unwinds to from what may throw, not a binding that can't (ADR 0301), or
/// gives one to what may, a function of its library's that may drop it
/// there, or any other.
fn body_may_drop<'tcx>(
    calls: &Calls<'_, 'tcx>,
    (id, index): (DefId, u32),
    seen: &mut IndexMap<(DefId, u32), bool>,
) -> bool {
    if let Some(&known) = seen.get(&(id, index)) {
        return known;
    }
    // Assumed not, while it's asked of a call it makes of itself.
    seen.insert((id, index), false);
    let tcx = calls.tcx;
    let body = tcx.optimized_mir(id);
    let holds = |ty: Ty<'tcx>| crate::lower::drops::owned_params(tcx, ty).contains(&index);
    // The cleanup it may unwind to: from what may throw where it runs.
    let mut unwound: IndexSet<mir::BasicBlock> = IndexSet::default();
    let mut work: Vec<mir::BasicBlock> = (body.basic_blocks.iter())
        .filter(|data| !data.is_cleanup)
        .filter_map(|data| {
            let terminator = data.terminator();
            let mir::UnwindAction::Cleanup(cleanup) = terminator.unwind()? else {
                return None;
            };
            let cannot_throw = matches!(&terminator.kind, TerminatorKind::Call { func, .. }
                if func.const_fn_def().is_some_and(|(callee, _)| crate::lower::bindings::cannot_throw(tcx, callee)));
            (!cannot_throw).then_some(*cleanup)
        })
        .collect();
    while let Some(block) = work.pop() {
        if unwound.insert(block) {
            work.extend(body.basic_blocks[block].terminator().successors());
        }
    }
    // A closure it makes of one, which carries its drop where it's given
    // back or made a `dyn Fn` (ADRs 0484, 0521), `joined`'s `move |acc, x|
    // g(acc, f(x))`: a drop it's given is that closure's (#1051).
    let closes_over = (body.basic_blocks.iter())
        .flat_map(|data| data.statements.iter())
        .any(|statement| match &statement.kind {
            mir::StatementKind::Assign(assign) => match &assign.1 {
                mir::Rvalue::Aggregate(kind, operands) if matches!(**kind, mir::AggregateKind::Closure(..)) => {
                    operands.iter().any(|operand| match operand {
                        mir::Operand::Move(place) | mir::Operand::Copy(place) => {
                            holds(place.ty(&body.local_decls, tcx).ty)
                        }
                        _ => false,
                    })
                }
                _ => false,
            },
            _ => false,
        });
    let drops = body
        .basic_blocks
        .iter_enumerated()
        .any(|(block, data)| match &data.terminator().kind {
            _ if data.is_cleanup && !unwound.contains(&block) => false,
            TerminatorKind::Drop { place, .. } => holds(place.ty(&body.local_decls, tcx).ty),
            TerminatorKind::Call { func, args, .. } => {
                // `ptr::drop_in_place::<T>(p)`, which drops what its pointer
                // points to, a `*mut T` owning none (#1031).
                if let Some((callee, callee_args)) = func.const_fn_def()
                    && calls.recognition.classify(callee, callee_args)
                        == Some(crate::lower::Std::Pointer(
                            crate::lower::pointers::PointerOp::DropInPlace,
                        ))
                {
                    return callee_args.types().any(holds);
                }
                // A function of its library's given what makes one as a
                // std associated type, `third(made)` of an `M: Iterator<Item
                // = T>`, whose items it may drop there (ADR 0489; #1031).
                if let Some((callee, callee_args)) = func.const_fn_def()
                    && compiled_here(calls.foreign, tcx, callee)
                    && crate::lower::traits::drop_items(tcx, calls.foreign, callee)
                        .into_iter()
                        .any(|item| {
                            let item = ty::EarlyBinder::bind(tcx, item)
                                .instantiate(tcx, callee_args)
                                .skip_normalization();
                            let typing_env = ty::TypingEnv::post_analysis(tcx, id);
                            holds(
                                (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(item)))
                                    .unwrap_or(item),
                            )
                        })
                {
                    return true;
                }
                // Given by its place, moved or, of what optimized MIR copies
                // where it's last used, copied; or a `&mut` to what holds one,
                // through which it may write one again, dropping the one
                // there, `slot.renew()` (#1032).
                let lent = |ty: Ty<'tcx>| {
                    holds(ty) || matches!(*ty.kind(), ty::Ref(_, inner, ty::Mutability::Mut) if holds(inner))
                };
                let gives_one = args.iter().any(|arg| match &arg.node {
                    mir::Operand::Move(place) | mir::Operand::Copy(place) => lent(place.ty(&body.local_decls, tcx).ty),
                    _ => false,
                });
                if !gives_one {
                    return false;
                }
                match func.const_fn_def() {
                    // A function of its library's given it as its own type
                    // parameter, or what holds one, a `Slot<T>` whose `renew`
                    // writes its `T` again (#1032), which may drop it there.
                    Some((callee, callee_args)) if compiled_here(calls.foreign, tcx, callee) => {
                        (0..callee_args.len() as u32).any(|at| {
                            (callee_args.get(at as usize).and_then(|arg| arg.as_type())).is_some_and(holds)
                                && body_may_drop(calls, (callee, at), seen)
                        })
                    }
                    // A closure's, a `dyn Fn`'s or a fn pointer's, which drops
                    // what it's given by its own code, of the caller's that made
                    // it, which gives no drop.
                    Some((callee, _))
                        if tcx
                            .trait_of_assoc(callee)
                            .is_some_and(|tr| tcx.fn_trait_kind_from_def_id(tr).is_some()) =>
                    {
                        false
                    }
                    None => false,
                    // Any other function, std's, which may drop it by the drop
                    // it's given.
                    Some(_) => true,
                }
            }
            _ => false,
        });
    let drops = closes_over || drops;
    seen.insert((id, index), drops);
    drops
}

/// Whether a library's `id` may make a `Some` of its type parameter
/// `index`, whose box an export that boxes none leaves out (ADR 0051,
/// amended): an `Option` of what holds one is the type of a place its MIR
/// has, or of a closure's it makes, or it gives one to a function of its
/// library's that may, or to std's, which may. Not given to a binding,
/// JS's, which makes no `Option` of it, nor to a closure, a `dyn Fn` or a
/// fn pointer, which takes it as its own maker's code gives it. So React's
/// `runWithFiberInDEV(fiber, () => { .. })` of React DOM's, given `()`s,
/// is the reconciler's export (gap rd75b).
fn body_may_box<'tcx>(calls: &Calls<'_, 'tcx>, (id, index): (DefId, u32), seen: &mut IndexSet<DefId>) -> bool {
    if !seen.insert(id) {
        return false;
    }
    let tcx = calls.tcx;
    let holds = |ty: Ty<'tcx>| holds_param(ty, index);
    let boxes = |ty: Ty<'tcx>| {
        ty.walk().filter_map(|arg| arg.as_type()).any(|ty| match ty.kind() {
            ty::Adt(adt, args) => {
                tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::Option) && args.types().any(holds)
            }
            _ => false,
        })
    };
    if !tcx.is_mir_available(id) {
        return true;
    }
    let body = tcx.optimized_mir(id);
    if body.local_decls.iter().any(|decl| boxes(decl.ty)) {
        return true;
    }
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            if let mir::StatementKind::Assign(assign) = &statement.kind
                && let mir::Rvalue::Aggregate(kind, _) = &assign.1
                && let mir::AggregateKind::Closure(closure, _) = **kind
                && body_may_box(calls, (closure, index), seen)
            {
                return true;
            }
        }
        let TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
            continue;
        };
        let given = args.iter().any(|arg| holds(arg.node.ty(&body.local_decls, tcx)));
        let Some((callee, callee_args)) = func.const_fn_def() else {
            continue;
        };
        let generic = callee_args.types().any(holds);
        if !given && !generic {
            continue;
        }
        let asks = if crate::lower::bindings::is_binding(tcx, callee)
            || tcx
                .trait_of_assoc(callee)
                .is_some_and(|tr| tcx.fn_trait_kind_from_def_id(tr).is_some())
        {
            false
        } else if compiled_here(calls.foreign, tcx, callee) {
            (0..callee_args.len() as u32).any(|at| {
                (callee_args.get(at as usize).and_then(|arg| arg.as_type())).is_some_and(holds)
                    && body_may_box(calls, (callee, at), seen)
            })
        } else {
            true
        };
        if asks {
            return true;
        }
    }
    false
}

/// Why the crate compiles `root`, a library's function, itself, as each
/// refusal of what it then can't reach says: a writer given more than its
/// library's JS takes names what (#1026); another, why any is.
fn compiled_because(foreign: &crate::lower::library::Foreign<'_, '_>, root: DefId) -> String {
    match foreign.writes_more(root) {
        Some(more) => format!("a writer given {more}, which its library's JS doesn't take"),
        None if foreign.merges_identities(root) => "one whose JS may give one function where Rust makes two, which this crate compares".to_string(),
        None => "one given a value to copy or drop that it takes nothing for, a writer or a value whose writes may fail where its library's never do, an impl its library's code never asks for, or one that copies what this crate changes in place".to_string(),
    }
}

/// What a copy of a library's function, compiled in another crate, can't
/// reach of what `body`, its MIR, reads or calls (ADR 0914): a static or a
/// thread-local its library doesn't export, as its library's own code has
/// the one there is; a thread-local its library makes its module's
/// variable, `export let last` (ADR 0270, amended), which it sets; and a
/// binding its library names by a path relative to its own file.
pub(in crate::lower) enum OutOfReach {
    /// Its library's private state, read.
    Private(DefId),
    /// Its library's module variable, set.
    ModuleVariable(DefId),
    /// A binding to a module by a path relative to its library's file,
    /// read, of a static, or called.
    RelativeBinding(DefId, String),
}

/// `OutOfReach`'s, of `body`, a function's MIR of `foreign`'s libraries.
pub(in crate::lower) fn out_of_reach<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    body: &mir::Body<'tcx>,
) -> Vec<OutOfReach> {
    let mut out = Vec::new();
    struct Read<'tcx> {
        tcx: TyCtxt<'tcx>,
        read: Vec<DefId>,
    }
    impl<'tcx> mir::visit::Visitor<'tcx> for Read<'tcx> {
        fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, _: mir::Location) {
            if let mir::Const::Unevaluated(unevaluated, _) = constant.const_ {
                self.read.push(unevaluated.def);
            }
            // A static's, the pointer to it its MIR has.
            self.read.extend(constant.check_static_ptr(self.tcx));
        }
        fn visit_rvalue(&mut self, rvalue: &mir::Rvalue<'tcx>, location: mir::Location) {
            if let mir::Rvalue::ThreadLocalRef(def) = rvalue {
                self.read.push(*def);
            }
            self.super_rvalue(rvalue, location);
        }
    }
    let mut read = Read { tcx, read: Vec::new() };
    mir::visit::Visitor::visit_body(&mut read, body);
    for &def in &read.read {
        let state = matches!(tcx.def_kind(def), DefKind::Static { .. })
            || (matches!(tcx.def_kind(def), DefKind::Const)
                && matches!(tcx.type_of(def).instantiate_identity().skip_normalization().kind(),
                    ty::Adt(key, _) if crate::lower::recognition::is_std_def(tcx, key.did(), crate::lower::recognition::StdItem::LocalKey)));
        // A binding's, an `extern` block's static, is JS's, which the
        // crate's JS reads as its library's does, but of a module its
        // library names by a path relative to its own file, which is below.
        let binding = tcx.is_foreign_item(def);
        if state && !binding && !def.is_local() && foreign.item(def).is_none() {
            out.push(OutOfReach::Private(def));
        }
    }
    // A thread-local its library makes its module's variable, which only
    // that module's JS can set: an import can't be assigned. Set, by `set`,
    // `take`, `replace`, `with_borrow_mut` or its cell lent by `with`, of
    // the type it holds.
    let variable =
        |def: DefId| (foreign.item(def)).is_some_and(|item| item.calling.has(crate::library::ItemFact::Variable));
    let keys: Vec<DefId> = (read.read.iter().copied())
        .filter(|&def| !def.is_local() && variable(def))
        .collect();
    let held = |key: DefId| match tcx.type_of(key).instantiate_identity().skip_normalization().kind() {
        ty::Adt(_, args) => args.types().next(),
        _ => None,
    };
    for (callee, args) in called(body) {
        let sets = crate::lower::recognition::local_key_sets(tcx, callee);
        // What the key it's called of holds, of its impl's `LocalKey<Cell<T>>`.
        let of = (tcx.inherent_impl_of_assoc(callee)).and_then(|imp| {
            match tcx.type_of(imp).instantiate(tcx, args).skip_normalization().kind() {
                ty::Adt(_, key_args) => key_args.types().next(),
                _ => None,
            }
        });
        if sets && let Some(&key) = keys.iter().find(|&&key| held(key).is_some() && held(key) == of) {
            out.push(OutOfReach::ModuleVariable(key));
        }
    }
    // A binding of its library's to a module by a path relative to that
    // library's file, `./stack.js`, which isn't this crate's.
    let statics = (read.read.iter().copied())
        .filter(|&def| tcx.is_foreign_item(def) && matches!(tcx.def_kind(def), DefKind::Static { .. }));
    let bindings: Vec<DefId> = called(body)
        .into_iter()
        .map(|(binding, _)| binding)
        .chain(statics)
        .collect();
    for binding in bindings {
        let path = crate::lower::bindings::js_path(tcx, binding);
        if !binding.is_local()
            && crate::lower::bindings::is_binding(tcx, binding)
            && let Some(((from, _), _)) = path.as_deref().and_then(crate::lower::bindings::js_import)
            && crate::program::is_relative_specifier(&from)
        {
            out.push(OutOfReach::RelativeBinding(binding, from));
        }
    }
    out
}

/// Whether a copy of `id`, a library's function, and of each of its
/// library's functions it calls in turn, which are copied with it, can
/// reach all they read and call (`out_of_reach`): one that can't stays its
/// library's (#1180's (5)).
pub(in crate::lower) fn copy_reaches<'tcx>(
    tcx: TyCtxt<'tcx>,
    foreign: &crate::lower::library::Foreign<'_, 'tcx>,
    id: DefId,
) -> bool {
    let mut seen: IndexSet<DefId> = IndexSet::default();
    let mut todo = vec![id];
    while let Some(id) = todo.pop() {
        if !seen.insert(id) || id.is_local() || id.krate != seen[0].krate || !tcx.is_mir_available(id) {
            continue;
        }
        let body = tcx.optimized_mir(id);
        if !out_of_reach(tcx, foreign, body).is_empty() {
            return false;
        }
        todo.extend(called(body).into_iter().map(|(callee, _)| callee));
    }
    true
}

/// Refuses an instance's read of its library's private state, its set of
/// its library's module variable and its call of a binding its library
/// names by a path relative to its own file (`out_of_reach`, ADR 0914).
fn refuse_private_state<'tcx>(calls: &Calls<'_, 'tcx>, id: DefId, body: &mir::Body<'tcx>) {
    let tcx = calls.tcx;
    let root = tcx.typeck_root_def_id(id);
    for reach in out_of_reach(tcx, calls.foreign, body) {
        let message = match reach {
            OutOfReach::Private(def) => format!(
                "rust-js: this crate compiles `{}` itself, as its library's JS has none it can call: {}; but it reads `{}`, private to its library, which only that library's code can reach (ADR 0914): make it `pub`",
                tcx.def_path_str(root),
                compiled_because(calls.foreign, root),
                tcx.def_path_str(def),
            ),
            OutOfReach::ModuleVariable(key) => format!(
                "rust-js: this crate compiles `{}` itself, as its library's JS has none it can call: {}; but it sets `{}`, its library's module variable, which JS lets only that module set (ADR 0270, amended): set it in a `pub fn` of that library that takes nothing to copy or drop, and call that",
                tcx.def_path_str(root),
                compiled_because(calls.foreign, root),
                tcx.def_path_str(key),
            ),
            OutOfReach::RelativeBinding(binding, from) => format!(
                "rust-js: this crate compiles `{}` itself, as its library's JS has none it can call: {}; but it {} `{}`, which its library binds to `{from}`, a module by a path relative to that library's own file, not this crate's (ADR 0914): have its library bind it by a package's path, which this crate can import too",
                tcx.def_path_str(root),
                compiled_because(calls.foreign, root),
                if tcx.is_foreign_item(binding) && matches!(tcx.def_kind(binding), DefKind::Static { .. }) {
                    "reads"
                } else {
                    "calls"
                },
                tcx.def_path_str(binding),
            ),
        };
        tcx.dcx().err(message);
    }
}
