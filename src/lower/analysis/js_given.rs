//! A function JS calls, whose type parameters JS gives any value, and the
//! copies of the crate's functions that its calls, and its crate's, need
//! where the two need a `Some` of one apart (ADR 0051, amended).
//!
//! A `Some` of a type parameter is the value where no call gives it what
//! may look like `None` (`unboxed_params`), and a box where one does. A
//! function JS calls is given any JS value, `undefined` too, and gives JS
//! the value its `Some` holds, as its declaration's `T | undefined` says:
//! its JS, the one its module exports, takes each of its type parameters
//! so. Where its crate's own Rust gives one of them what may look like
//! `None`, that call needs the box, and calls a copy of it that boxes,
//! compiled from the same MIR beside it. What either passes on a type
//! parameter to is copied so in turn, where what it's given needs its
//! `Some` otherwise than the function's own JS has it.

use std::collections::{BTreeSet, HashMap, HashSet};

use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

use crate::lower::Body;
use crate::lower::recognition::Recognition;

/// What a lowering of a function takes its type parameters as, by their
/// indices: those a `Some` of which is the value, and of those, those JS
/// gives, which may be `undefined` or `null` as they're given.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub(in crate::lower) struct Params {
    pub unboxed: BTreeSet<u32>,
    pub js: BTreeSet<u32>,
}

/// Each function's lowerings: the one its module names it by, its JS's,
/// and the copies its callers need besides, each named apart.
#[derive(Default)]
pub(in crate::lower) struct JsGiven {
    /// The generic functions only their callers call, whose calls each
    /// call the lowering that takes what they give as they give it.
    routed: HashSet<DefId>,
    /// Of those, the ones JS calls, whose own JS takes each type parameter
    /// as JS gives it.
    facing: HashSet<DefId>,
    /// The type parameters a `Some` of which is the value as Rust's calls
    /// give them (`unboxed_params`).
    unboxed: HashSet<(DefId, u32)>,
    /// Each function's copies, by the order they were found, and their names.
    copies: HashMap<DefId, Vec<(Params, String)>>,
}

/// What a call gives one of its callee's type parameters, as the lowering
/// it's in takes its own.
enum Given {
    /// What may look like `None`, whose `Some` is a box.
    Nullish,
    /// What JS gave, whose `Some` is the value, `undefined` too.
    Js,
    /// What's never `undefined` or `null`.
    Defined,
}

impl JsGiven {
    /// Of the crate's functions `items`, those its JS exports whose type
    /// parameters a call gives otherwise than JS does, each copy each of
    /// their calls needs, named apart in its module by `name`.
    pub(in crate::lower) fn of_crate<'tcx>(
        recognition: &Recognition<'_, 'tcx>,
        all_bodies: &[&Body<'tcx>],
        (items, unboxed): (&HashSet<DefId>, &HashSet<(DefId, u32)>),
        copyable: &dyn Fn(DefId) -> bool,
        name: &mut dyn FnMut(DefId, &str) -> String,
    ) -> JsGiven {
        let tcx = recognition.tcx;
        let routed: HashSet<DefId> = (items.iter().copied())
            .filter(|&id| {
                tcx.typeck_root_def_id(id) == id
                    && super::mutation::copies_by_callers(tcx, recognition.foreign, id)
                    && !super::mutation::type_params(tcx, id).is_empty()
                    && copyable(id)
            })
            .collect();
        let mut given = JsGiven {
            facing: (routed.iter().copied())
                .filter(|&id| js_facing(tcx, recognition.foreign, id))
                .collect(),
            routed,
            unboxed: unboxed.clone(),
            copies: HashMap::new(),
        };
        // What each function's bodies, its closures' too, call.
        let mut calls: HashMap<DefId, Vec<(DefId, ty::GenericArgsRef<'tcx>)>> = HashMap::new();
        for body in all_bodies {
            let caller = tcx.typeck_root_def_id(body.def_id);
            let called = crate::lower::instances::calls_of(body).into_iter();
            let routed = called.filter(|&(callee, ..)| given.routed.contains(&callee));
            (calls.entry(caller).or_default()).extend(routed.map(|(callee, args, _)| (callee, args)));
        }
        // From each function's own JS, a generic one's or not.
        let mut work: Vec<(DefId, Params)> = (calls.keys().copied()).map(|id| (id, given.main(tcx, id))).collect();
        while let Some((caller, params)) = work.pop() {
            for &(callee, args) in calls.get(&caller).into_iter().flatten() {
                let Some(needed) = given.needed(recognition, &params, (callee, args)) else {
                    continue;
                };
                if given.copy_of(callee, &needed).is_some() {
                    continue;
                }
                // Named for what it does apart from its own JS: of one JS
                // calls, the copy Rust's calls box in; of another, the one
                // JS's values reach.
                let named = name(
                    callee,
                    if given.facing.contains(&callee) {
                        "Boxed"
                    } else {
                        "Unboxed"
                    },
                );
                given.copies.entry(callee).or_default().push((needed.clone(), named));
                work.push((callee, needed));
            }
        }
        given
    }

    /// What `id`'s own JS, the one its module names it by, takes its type
    /// parameters as: of one JS calls, each as JS gives it; of any other,
    /// as its crate's calls give them.
    pub(in crate::lower) fn main(&self, tcx: TyCtxt<'_>, id: DefId) -> Params {
        let all: BTreeSet<u32> = super::mutation::type_params(tcx, id).into_iter().collect();
        match self.facing.contains(&id) {
            true => Params {
                unboxed: all.clone(),
                js: all,
            },
            false => Params {
                unboxed: all
                    .into_iter()
                    .filter(|&index| self.unboxed.contains(&(id, index)))
                    .collect(),
                js: BTreeSet::new(),
            },
        }
    }

    /// What lowering `copy` of `id` takes its type parameters as: its own
    /// JS's (`main`) where it's none.
    pub(in crate::lower) fn params(&self, tcx: TyCtxt<'_>, id: DefId, copy: Option<usize>) -> Params {
        match copy {
            Some(at) => self.copies[&id][at].0.clone(),
            None => self.main(tcx, id),
        }
    }

    /// Each copy of `id`'s, by its index, and its name.
    pub(in crate::lower) fn copies(&self, id: DefId) -> impl Iterator<Item = (usize, &str)> {
        (self.copies.get(&id).into_iter().flatten().enumerate()).map(|(at, (_, name))| (at, name.as_str()))
    }

    /// The name of `id`'s copy `at`.
    pub(in crate::lower) fn name(&self, id: DefId, at: usize) -> &str {
        &self.copies[&id][at].1
    }

    /// The copy of `id` that takes its type parameters as `params`.
    fn copy_of(&self, id: DefId, params: &Params) -> Option<usize> {
        (self.copies.get(&id)?.iter()).position(|(made, _)| made == params)
    }

    /// Which lowering of `callee` a call in one that takes its own type
    /// parameters as `params` calls, given `args`: its own JS where that
    /// takes what the call gives as it gives it, `None`; else the copy
    /// that does, by its index, or `Err` where none was made.
    pub(in crate::lower) fn called<'tcx>(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        params: &Params,
        (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
    ) -> Result<Option<usize>, ()> {
        if !self.routed.contains(&callee) {
            return Ok(None);
        }
        match self.needed(recognition, params, (callee, args)) {
            None => Ok(None),
            Some(needed) => self.copy_of(callee, &needed).map(Some).ok_or(()),
        }
    }

    /// What a call needs `callee`, one `routed`, to take its type parameters as, where its
    /// own JS doesn't: a `Some` of what may look like `None` a box, and of
    /// what JS gave the value.
    fn needed<'tcx>(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        params: &Params,
        (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
    ) -> Option<Params> {
        let tcx = recognition.tcx;
        let (mut nullish, mut js) = (BTreeSet::new(), BTreeSet::new());
        for index in super::mutation::type_params(tcx, callee) {
            let Some(given) = args.get(index as usize).and_then(|arg| arg.as_type()) else {
                continue;
            };
            match given_as(recognition, params, given) {
                Given::Nullish => _ = nullish.insert(index),
                Given::Js => _ = js.insert(index),
                Given::Defined => {}
            }
        }
        let main = self.main(tcx, callee);
        if main.unboxed.is_disjoint(&nullish) && js.is_subset(&main.js) {
            return None;
        }
        let unboxed = (super::mutation::type_params(tcx, callee).into_iter())
            .filter(|&index| self.unboxed.contains(&(callee, index)) || js.contains(&index))
            .collect();
        Some(Params { unboxed, js })
    }
}

/// What `given`, which a call gives a type parameter, is, as the lowering
/// the call is in takes its own type parameters (`params`).
fn given_as<'tcx>(recognition: &Recognition<'_, 'tcx>, params: &Params, given: Ty<'tcx>) -> Given {
    match crate::lower::representation::payload_through(recognition, given).kind() {
        ty::Param(param) if params.js.contains(&param.index) => Given::Js,
        ty::Param(param) if params.unboxed.contains(&param.index) => Given::Defined,
        _ if crate::lower::representation::may_look_like_none(recognition, given) => Given::Nullish,
        _ => Given::Defined,
    }
}

/// Whether JS calls `id`, the crate's function its module exports: a
/// function, or an inherent impl's method, that's `pub`, and no library's
/// function the crate compiles itself (ADR 0914), which no other crate or
/// JS calls.
pub(in crate::lower) fn js_facing(
    tcx: TyCtxt<'_>,
    foreign: &crate::lower::library::Foreign<'_, '_>,
    id: DefId,
) -> bool {
    matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn)
        && tcx.visibility(id).is_public()
        && !foreign.is_instance(id)
        && (tcx.def_kind(id) != DefKind::AssocFn || tcx.inherent_impl_of_assoc(id).is_some())
}

/// Of a call of `callee` given `args`, the type parameter `P` whose `Some`
/// its JS that takes `P` as JS gives it leaves out where the call's result,
/// an `Option<P>` of what may look like `None`, `.flatten()` alone reads:
/// `callee` makes that `Option` of what it's given, `None` or a `Some` of a
/// `P`, and returns it, reading it nowhere (`made_and_returned`). Its JS
/// gives `p` for `Some(p)` and `undefined` for `None`, which is the result
/// flattened, as `Some(None)` and `None` both flatten to `None`: so React's
/// noop renderer's `noop_renderer.flush_sync_from_reconciler(f).flatten()`
/// calls the reconciler's function through the value it's given (rd311, the
/// user's decision; ADRs 0051, 1037).
pub(in crate::lower) fn flattened_alike<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
) -> Option<u32> {
    let tcx = recognition.tcx;
    let output = tcx.fn_sig(callee).instantiate_identity().skip_binder().output();
    let p = option_param(tcx, output)?;
    let given = args.get(p as usize)?.as_type()?;
    let ty::Adt(adt, _) = given.kind() else { return None };
    (tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::Option)
        && made_and_returned(recognition, (callee, p), true, &mut HashSet::new()))
    .then_some(p)
}

/// Whether `local` of `body` is read once, by a `.flatten()` it's moved to.
pub(in crate::lower) fn read_by_flatten<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    local: rustc_middle::mir::Local,
) -> bool {
    use rustc_middle::mir::{self, visit::PlaceContext, visit::Visitor};
    struct Reads(rustc_middle::mir::Local, usize);
    impl<'tcx> Visitor<'tcx> for Reads {
        fn visit_local(&mut self, local: mir::Local, context: PlaceContext, _: mir::Location) {
            if local == self.0 && context.is_use() && !context.is_place_assignment() && !context.is_drop() {
                self.1 += 1;
            }
        }
    }
    let mut reads = Reads(local, 0);
    reads.visit_body(body);
    reads.1 == 1
        && body.basic_blocks.iter().any(|data| {
            matches!(&data.terminator().kind, mir::TerminatorKind::Call { func, args, .. }
                if matches!(&args[..], [arg] if matches!(&arg.node, mir::Operand::Move(read) if *read == mir::Place::from(local)))
                    && func.const_fn_def().is_some_and(|(id, generic)| {
                        recognition.classify(id, generic) == Some(crate::lower::Std::OptionFlatten)
                    }))
        })
}

/// The index of `P` where `ty` is an `Option<P>` of a type parameter.
fn option_param(tcx: TyCtxt<'_>, ty: Ty<'_>) -> Option<u32> {
    match ty.kind() {
        ty::Adt(adt, args) if tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::Option) => {
            match args.type_at(0).kind() {
                ty::Param(param) => Some(param.index),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether `id`, the crate's function or a library's, given values of its
/// type parameter `p`, reads no `Option` of one: where `returns`, it makes
/// its result, an `Option<P>`, of `None` or a `Some` of a `P` and returns
/// it, and has no other; else it has none. A `P` it gives only to what
/// takes it as it's given, a closure, a `dyn Fn`'s or a fn pointer's, the
/// caller's code, a binding, JS's, or a function of its crate or library
/// that reads none either; never std's, whose JS may read one apart, an
/// iterator's `undefined` its end. Its `Option<P>` it gives to nothing but
/// its return, and std's `Option::map` makes one of a closure's `P` as it
/// would (`flattened_alike`, rd311).
fn made_and_returned<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    (id, p): (DefId, u32),
    returns: bool,
    seen: &mut HashSet<(DefId, u32, bool)>,
) -> bool {
    use rustc_middle::mir::{self, visit::PlaceContext, visit::Visitor};
    let tcx = recognition.tcx;
    if !seen.insert((id, p, returns)) {
        return true;
    }
    let ours = id.is_local() || recognition.foreign.is_library(id.krate);
    if !ours || !tcx.is_mir_available(id) {
        return false;
    }
    let body = tcx.optimized_mir(id);
    let holds = |ty: Ty<'tcx>| crate::lower::instances::holds_param(ty, p);
    let boxes = |ty: Ty<'tcx>| {
        ty.walk().filter_map(|arg| arg.as_type()).any(|ty| match ty.kind() {
            ty::Adt(adt, args) => {
                tcx.is_lang_item(adt.did(), rustc_hir::attrs::lang_items::LangItem::Option) && args.types().any(holds)
            }
            _ => false,
        })
    };
    let returned = |ty: Ty<'tcx>| option_param(tcx, ty) == Some(p);
    if returns && !returned(body.local_decls[mir::RETURN_PLACE].ty) {
        return false;
    }
    // Its `Option<P>`s, its result and what's moved to it, none given it.
    let slots: HashSet<mir::Local> = (body.local_decls.iter_enumerated())
        .filter(|&(local, decl)| {
            returns && (local == mir::RETURN_PLACE || (local.as_usize() > body.arg_count && returned(decl.ty)))
        })
        .map(|(local, _)| local)
        .collect();
    if (body.local_decls.iter_enumerated()).any(|(local, decl)| !slots.contains(&local) && boxes(decl.ty)) {
        return false;
    }
    // A closure, of what it's given or gives back, that reads none.
    let closure_reads_none = |ty: Ty<'tcx>, seen: &mut HashSet<(DefId, u32, bool)>| match ty.kind() {
        ty::Closure(closure, _) => !holds(ty) || made_and_returned(recognition, (*closure, p), false, seen),
        _ => !holds(ty) || matches!(ty.kind(), ty::Param(_)),
    };
    struct Slots<'s> {
        slots: &'s HashSet<mir::Local>,
        read: bool,
    }
    impl<'tcx> Visitor<'tcx> for Slots<'_> {
        fn visit_local(&mut self, local: mir::Local, _: PlaceContext, _: mir::Location) {
            self.read |= self.slots.contains(&local);
        }
    }
    let reads_slot = |visit: &dyn Fn(&mut Slots<'_>)| {
        let mut found = Slots {
            slots: &slots,
            read: false,
        };
        visit(&mut found);
        found.read
    };
    let location = mir::Location::START;
    let option_variant = |place: &mir::Place<'tcx>| {
        matches!(
            place.projection[..],
            [mir::PlaceElem::Downcast(..), mir::PlaceElem::Field(..)]
        )
    };
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            match &statement.kind {
                mir::StatementKind::Assign(assign) => {
                    let (place, rvalue) = &**assign;
                    let gives = reads_slot(&|found| found.visit_rvalue(rvalue, location));
                    if slots.contains(&place.local) {
                        let made = match rvalue {
                            // `Some(p)` or `None`.
                            mir::Rvalue::Aggregate(kind, _) => {
                                matches!(**kind, mir::AggregateKind::Adt(adt, ..)
                                if tcx.is_lang_item(adt, rustc_hir::attrs::lang_items::LangItem::Option))
                                    && !gives
                            }
                            // Another of its `Option<P>`s, moved.
                            mir::Rvalue::Use(mir::Operand::Move(from), _) => {
                                from.projection.is_empty() && slots.contains(&from.local)
                            }
                            mir::Rvalue::Use(_, _) => option_variant(place) && !gives,
                            _ => false,
                        };
                        if !made || !(place.projection.is_empty() || option_variant(place)) {
                            return false;
                        }
                    } else if gives {
                        return false;
                    }
                    if let mir::Rvalue::Aggregate(kind, _) = rvalue
                        && let mir::AggregateKind::Closure(closure, args) = **kind
                        && !closure_reads_none(ty::Ty::new_closure(tcx, closure, args), seen)
                    {
                        return false;
                    }
                }
                mir::StatementKind::SetDiscriminant { place, .. } if slots.contains(&place.local) => {}
                mir::StatementKind::StorageLive(_) | mir::StatementKind::StorageDead(_) => {}
                _ => {
                    if reads_slot(&|found| found.visit_statement(statement, location)) {
                        return false;
                    }
                }
            }
        }
        match &data.terminator().kind {
            mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => {
                if args
                    .iter()
                    .any(|arg| reads_slot(&|found| found.visit_operand(&arg.node, location)))
                {
                    return false;
                }
                let into_slot = slots.contains(&destination.local);
                if into_slot && !destination.projection.is_empty() {
                    return false;
                }
                let given_p = args.iter().any(|arg| holds(arg.node.ty(&body.local_decls, tcx)));
                let called = func.const_fn_def();
                let generic_p = called.is_some_and(|(_, generic)| generic.types().any(holds));
                if !into_slot && !given_p && !generic_p {
                    continue;
                }
                let ok = match called {
                    // `option.map(f)` of what holds no `P`, to a `P` that `f`
                    // gives: `None` or `Some` of it.
                    Some((callee, generic))
                        if into_slot && recognition.classify(callee, generic) == Some(crate::lower::Std::OptionMap) =>
                    {
                        matches!(&args[..], [option, f]
                            if !holds(option.node.ty(&body.local_decls, tcx))
                                && closure_reads_none(f.node.ty(&body.local_decls, tcx), seen))
                    }
                    // A closure's, a `dyn Fn`'s or a fn pointer's, a binding's.
                    None if !into_slot => true,
                    Some((callee, _))
                        if !into_slot
                            && (crate::lower::bindings::is_binding(tcx, callee)
                                || tcx
                                    .trait_of_assoc(callee)
                                    .is_some_and(|tr| tcx.fn_trait_kind_from_def_id(tr).is_some())) =>
                    {
                        true
                    }
                    // Its crate's or library's, given `P` as its own `Q`.
                    Some((callee, generic))
                        if (callee.is_local() || recognition.foreign.is_library(callee.krate))
                            && tcx.trait_of_assoc(callee).is_none()
                            && matches!(tcx.def_kind(callee), DefKind::Fn | DefKind::AssocFn) =>
                    {
                        let mut given = (0..generic.len() as u32).filter(|&at| {
                            generic
                                .get(at as usize)
                                .and_then(|arg| arg.as_type())
                                .is_some_and(holds)
                        });
                        let first = given.next();
                        given.next().is_none()
                            && match first {
                                Some(q) => {
                                    matches!(generic.type_at(q as usize).kind(), ty::Param(param) if param.index == p)
                                        && made_and_returned(recognition, (callee, q), into_slot, seen)
                                }
                                None => !into_slot,
                            }
                    }
                    _ => false,
                };
                if !ok {
                    return false;
                }
            }
            mir::TerminatorKind::Drop { .. } | mir::TerminatorKind::Return => {}
            _ => {
                let mut found = Slots {
                    slots: &slots,
                    read: false,
                };
                found.visit_terminator(data.terminator(), location);
                if found.read {
                    return false;
                }
            }
        }
    }
    true
}
