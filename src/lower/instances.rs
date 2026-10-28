//! The library functions a crate compiles itself (ADR 0914). A library's
//! generic function takes the copies and drops its own crate's calls give
//! it, as a program's does, so its JS is what it would be with no consumer,
//! the one JS calls. A consumer whose call gives it a value to copy or drop
//! that it takes nothing for compiles it from the library's MIR, as Rust's
//! codegen does, beside its first caller, and calls its own; so are the
//! library's private functions that one calls. One that reads the library's
//! private state, which no other crate can reach, is refused.

use std::collections::{HashMap, HashSet};

use rustc_hir::def::DefKind;
use rustc_middle::mir::{self, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::{DefId, LocalModId};

use super::Body;

/// The library functions the crate compiles itself, each placed in the
/// module of the crate's that first calls it (ADR 0914). Decided by the
/// pipeline once the crate's bodies are analyzed (`needed`), and given back
/// to it with their bodies, until none more is needed.
#[derive(Default)]
pub struct Instances {
    placed: HashMap<DefId, LocalModId>,
}

impl Instances {
    /// Whether `id`, a library's function or a closure of one, is compiled
    /// in this crate.
    pub(super) fn contains(&self, tcx: TyCtxt<'_>, id: DefId) -> bool {
        !id.is_local() && self.placed.contains_key(&tcx.typeck_root_def_id(id))
    }

    /// The module of the crate's an instance is placed in, its closures'
    /// too.
    pub(super) fn module(&self, tcx: TyCtxt<'_>, id: DefId) -> Option<LocalModId> {
        self.placed.get(&tcx.typeck_root_def_id(id)).copied()
    }

    /// Each instance `needed` found, with the others.
    pub fn extend(&mut self, more: Vec<(DefId, LocalModId)>) {
        self.placed.extend(more);
    }

    /// The trait impls of its libraries the crate compiles itself, which
    /// their JS doesn't export (ADR 0914, amended), in the order they're
    /// declared.
    pub(super) fn impls(&self, tcx: TyCtxt<'_>) -> Vec<DefId> {
        let mut impls: Vec<DefId> = (self.placed.keys().copied())
            .filter(|&id| matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true }))
            .collect();
        impls.sort_by_key(|&id| tcx.def_path_hash(id));
        impls
    }
}

/// What the crate's lowering asked for of its libraries that their JS
/// doesn't export (`Foreign::compiles`), each where it was first asked: a
/// trait impl, or its method, the impl with each of its methods, beside
/// it, whose dictionary names them (ADR 0914, amended).
pub(super) fn asked(tcx: TyCtxt<'_>, asked: Vec<(DefId, LocalModId)>) -> Vec<(DefId, LocalModId)> {
    let mut more: Vec<(DefId, LocalModId)> = Vec::new();
    for (id, module) in asked {
        let id = tcx.trait_impl_of_assoc(id).unwrap_or(id);
        let methods = match tcx.def_kind(id) {
            DefKind::Impl { of_trait: true } => (tcx.associated_item_def_ids(id).iter().copied())
                .filter(|&method| tcx.def_kind(method) == DefKind::AssocFn && tcx.is_mir_available(method))
                .collect(),
            _ => Vec::new(),
        };
        for id in std::iter::once(id).chain(methods) {
            if !more.iter().any(|&(placed, _)| placed == id) {
                more.push((id, module));
            }
        }
    }
    more
}

/// Which of a library's functions and impls may copy, or clone, a value
/// of what the crate changes in place, which its library's JS, copying
/// what its own crate changes, may then share: the crate compiles its own
/// of each it uses (ADR 0914, amended). Asked of each once.
pub(super) struct Copying<'a, 'tcx> {
    recognition: super::recognition::Recognition<'a, 'tcx>,
    changed: super::copies::Changed<'a, 'tcx>,
    known: std::cell::RefCell<HashMap<DefId, bool>>,
    /// Those being asked, outermost first.
    asking: std::cell::RefCell<Vec<DefId>>,
}

impl<'a, 'tcx> Copying<'a, 'tcx> {
    /// Of the crate's impls and libraries, `trait_impls` and `foreign`, and
    /// what it changes in place.
    pub(super) fn new(
        (trait_impls, foreign): (&'a [DefId], &'a super::library::Foreign<'a, 'tcx>),
        changed: super::copies::Changed<'a, 'tcx>,
    ) -> Self {
        let recognition = super::recognition::Recognition {
            tcx: foreign.tcx(),
            typing_env: ty::TypingEnv::fully_monomorphized(),
            trait_impls,
            foreign,
        };
        Copying {
            recognition,
            changed,
            known: Default::default(),
            asking: Default::default(),
        }
    }

    /// Whether `id`, a library's function or trait impl the crate uses,
    /// may copy or clone a value of what the crate changes in place: an
    /// impl's of its methods; a function's, of its MIR: a `copy` of a place
    /// of one, a call of std's, or of what else its library doesn't compile,
    /// that makes one of what it's given a reference to, a call of its
    /// library's that may, and a closure it makes that may.
    pub(super) fn copies(&self, id: DefId) -> bool {
        self.copies_below(id).0
    }

    /// `copies`, and the lowest of the functions still being asked that
    /// the answer read, where it read one: a call it makes of one that
    /// calls it. Of the least answer that holds, kept once it's settled: a
    /// `true`, or one that read none asked below it.
    fn copies_below(&self, id: DefId) -> (bool, usize) {
        let tcx = self.recognition.tcx;
        if id.is_local() || !self.recognition.foreign.is_library(id.krate) {
            return (false, usize::MAX);
        }
        if let Some(&known) = self.known.borrow().get(&id) {
            return (known, usize::MAX);
        }
        if let Some(at) = self.asking.borrow().iter().position(|&asked| asked == id) {
            return (false, at);
        }
        let depth = self.asking.borrow().len();
        self.asking.borrow_mut().push(id);
        let (copies, read) = match tcx.def_kind(id) {
            DefKind::Impl { of_trait: true } => (tcx.associated_item_def_ids(id).iter())
                .map(|&method| self.copies_below(method))
                .fold((false, usize::MAX), |(a, at), (b, bt)| (a || b, at.min(bt))),
            DefKind::Fn | DefKind::AssocFn | DefKind::Closure if tcx.is_mir_available(id) => {
                self.body_copies(tcx.optimized_mir(id))
            }
            _ => (false, usize::MAX),
        };
        self.asking.borrow_mut().pop();
        if copies || read >= depth {
            self.known.borrow_mut().insert(id, copies);
            return (copies, usize::MAX);
        }
        (copies, read)
    }

    fn body_copies(&self, body: &mir::Body<'tcx>) -> (bool, usize) {
        let tcx = self.recognition.tcx;
        let holds = |ty: Ty<'tcx>| {
            let ty = tcx.erase_and_anonymize_regions(ty);
            super::copies::may_hold_changed(&self.recognition, self.changed, ty)
        };
        struct Found<'b, 'a, 'tcx> {
            copying: &'b Copying<'a, 'tcx>,
            body: &'b mir::Body<'tcx>,
            holds: &'b dyn Fn(Ty<'tcx>) -> bool,
            copies: bool,
            read: usize,
        }
        impl Found<'_, '_, '_> {
            /// What `id`, a function it calls or a closure it makes, copies.
            fn ask(&mut self, id: DefId) {
                let (copies, read) = self.copying.copies_below(id);
                self.copies |= copies;
                self.read = self.read.min(read);
            }
        }
        impl<'tcx> mir::visit::Visitor<'tcx> for Found<'_, '_, 'tcx> {
            fn visit_operand(&mut self, operand: &mir::Operand<'tcx>, location: mir::Location) {
                let tcx = self.copying.recognition.tcx;
                let copy = |ty: Ty<'tcx>| {
                    let typing_env = ty::TypingEnv::post_analysis(tcx, self.body.source.def_id());
                    tcx.type_is_copy_modulo_regions(typing_env, ty)
                };
                match operand {
                    // A `Copy` value's, not what optimized MIR copies of a
                    // `Box` it reads through.
                    mir::Operand::Copy(place) if copy(place.ty(&self.body.local_decls, tcx).ty) => {
                        self.copies |= (self.holds)(place.ty(&self.body.local_decls, tcx).ty)
                    }
                    // A constant's, each use of which is a value of its own
                    // (ADR 0031).
                    mir::Operand::Constant(constant) => self.copies |= (self.holds)(constant.ty()),
                    _ => {}
                }
                self.super_operand(operand, location);
            }
            fn visit_rvalue(&mut self, rvalue: &mir::Rvalue<'tcx>, location: mir::Location) {
                if let mir::Rvalue::Aggregate(kind, _) = rvalue
                    && let mir::AggregateKind::Closure(closure, _) = **kind
                {
                    self.ask(closure);
                }
                self.super_rvalue(rvalue, location);
            }
            fn visit_terminator(&mut self, terminator: &mir::Terminator<'tcx>, location: mir::Location) {
                let tcx = self.copying.recognition.tcx;
                if let TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } = &terminator.kind
                {
                    // A trait's method, the impl's it resolves to, where it does.
                    let resolved = func.const_fn_def().map(|(callee, args)| {
                        let typing_env = ty::TypingEnv::post_analysis(tcx, self.body.source.def_id());
                        match tcx.trait_of_assoc(callee) {
                            Some(_) => super::items::resolve_instance_in(tcx, typing_env, callee, args)
                                .ok()
                                .flatten()
                                .map_or(callee, |instance| instance.def_id()),
                            None => callee,
                        }
                    });
                    self.copies |= match resolved {
                        // Its library's, which may copy one itself.
                        Some(callee) if self.copying.recognition.foreign.is_library(callee.krate) => {
                            self.ask(callee);
                            false
                        }
                        // A binding's, whose result is JS's own, the same
                        // however its caller is compiled, `js::object::keys`'s
                        // `Vec<String>`.
                        Some(callee) if super::bindings::is_binding(tcx, callee) => false,
                        // A slice's `concat()`, an array of its own made of
                        // its parts' items, `[...a, ...b]`: what it may share
                        // of what it's given is its items, `addValueToProperties`'
                        // `Option<&Unknown>`s (ADR 0914, amended).
                        Some(_)
                            if let Some((callee, generic)) = func.const_fn_def()
                                && self.copying.recognition.classify(callee, generic)
                                    == Some(super::Std::Comb(super::combinators::Comb::Concat)) =>
                        {
                            match destination.ty(&self.body.local_decls, tcx).ty.kind() {
                                ty::Adt(_, made) => made.types().next().is_some_and(|item| (self.holds)(item)),
                                _ => true,
                            }
                        }
                        // Another's, std's: one that makes what it's given a
                        // reference to, `Cell::get`, `clone`, `to_vec`.
                        Some(_) => {
                            (self.holds)(destination.ty(&self.body.local_decls, tcx).ty)
                                && args.iter().any(|arg| {
                                    (arg.node.ty(&self.body.local_decls, tcx).walk())
                                        .filter_map(|part| part.as_type())
                                        .any(|part| part.is_ref() || super::pointers::raw_pointee(tcx, part).is_some())
                                })
                        }
                        // A fn pointer's, a `dyn`'s: code of its own.
                        None => false,
                    };
                }
                self.super_terminator(terminator, location);
            }
        }
        let mut found = Found {
            copying: self,
            body,
            holds: &holds,
            copies: false,
            read: usize::MAX,
        };
        mir::visit::Visitor::visit_body(&mut found, body);
        (found.copies, found.read)
    }
}

/// The bodies of each of `more`, a library's functions, from the MIR its
/// metadata keeps, and of each closure they make (ADR 0914).
pub fn instance_bodies<'tcx>(tcx: TyCtxt<'tcx>, more: &[(DefId, LocalModId)]) -> Vec<Body<'tcx>> {
    super::analysis::foreign_bodies(tcx, more.iter().map(|&(id, _)| id).collect())
}

/// What the crate's analysis found that asks what its calls give a
/// library's functions: whose impls' dictionaries there are, what drops,
/// the drops and copies its own functions' type parameters are given, and
/// its bodies, its own and the instances it has.
pub(super) struct Analyzed<'a, 'tcx> {
    pub(super) trait_impls: &'a [DefId],
    pub(super) drops: (
        bool,
        &'a super::analysis::ClosureFacts,
        &'a super::analysis::Counted<'tcx>,
    ),
    pub(super) drop_params: &'a HashMap<DefId, Vec<u32>>,
    /// The type parameters of its own functions given a copy, and a
    /// `Clone` dictionary (ADR 0914, amended).
    pub(super) given: super::traits::Given<'a>,
    pub(super) bodies: Vec<&'a Body<'tcx>>,
}

/// What the analyzed crate asks of its calls of a library's functions.
struct Calls<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    recognition: super::recognition::Recognition<'a, 'tcx>,
    foreign: &'a super::library::Foreign<'a, 'tcx>,
    /// Whether a value of a type that's no type parameter runs a destructor
    /// where it's dropped, as the crate drops it.
    runs: &'a dyn Fn(Ty<'tcx>) -> bool,
    /// The type parameters of the crate's own functions given a drop.
    drop_params: &'a HashMap<DefId, Vec<u32>>,
    /// The type parameters of the crate's own functions given a copy.
    copied: &'a HashSet<(DefId, u32)>,
    /// And given a `Clone` dictionary (ADR 0914, amended).
    clones: &'a HashSet<(DefId, u32)>,
    /// And given nothing that may look like `None` (ADR 0051, amended).
    unboxed: &'a HashSet<(DefId, u32)>,
    /// Whether a value of a type is its own clone, as the crate counts its
    /// `Rc`s.
    clones_itself: &'a dyn Fn(Ty<'tcx>) -> bool,
}

/// The library functions the crate's bodies, its own and the instances it
/// has, call where they're given more than they take, and the private ones
/// an instance calls, which aren't yet instances, each with the module of
/// the crate's it's placed in, its first caller's (ADR 0914); refused,
/// where one reads its library's private state.
pub(super) fn needed<'tcx>(
    foreign: &super::library::Foreign<'_, 'tcx>,
    instances: &Instances,
    analyzed: Analyzed<'_, 'tcx>,
) -> Vec<(DefId, LocalModId)> {
    let tcx = foreign.tcx();
    let Analyzed {
        trait_impls,
        drops: (library, closure_facts, counted),
        drop_params,
        given,
        bodies,
    } = analyzed;
    let recognition = super::recognition::Recognition {
        tcx,
        typing_env: ty::TypingEnv::fully_monomorphized(),
        trait_impls,
        foreign,
    };
    let runs = |t: Ty<'tcx>| super::drops::drop_runs(recognition, (library, closure_facts), counted, t);
    let clones_itself = |t: Ty<'tcx>| super::analysis::clones_itself(tcx, counted, t);
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
    };
    let mut more: Vec<(DefId, LocalModId)> = Vec::new();
    let mut may_drop: HashMap<(DefId, u32), bool> = HashMap::new();
    for body in bodies {
        let Some(mir) = &body.mir else { continue };
        let caller = tcx.typeck_root_def_id(body.def_id);
        let instance = instances.contains(tcx, caller);
        for (callee, args) in called(&mir.body) {
            if !compiled_here(foreign, tcx, callee)
                || instances.contains(tcx, callee)
                || more.iter().any(|&(id, _)| id == callee)
            {
                continue;
            }
            // An instance's call of what its library doesn't export, its own
            // too; else one given more than its export takes.
            let private = instance && foreign.item(callee).is_none();
            if private || gives_more(&calls, (caller, callee, args), &mut may_drop) {
                more.push((callee, super::body_module(tcx, instances, body.def_id)));
            }
        }
        if instance {
            refuse_private_state(&calls, body.def_id, &mir.body);
        }
    }
    more
}

/// Each function a body of MIR alone names, which its THIR would: one of a
/// library's the crate compiles (ADR 0914).
pub(super) fn named(body: &Body<'_>) -> Vec<DefId> {
    let Some(mir) = &body.mir else { return Vec::new() };
    called(&mir.body).into_iter().map(|(id, _)| id).collect()
}

/// Each function `body` names, called or as a value, of its generic
/// arguments there, and where: as its THIR names it, or, of a body of MIR
/// alone, a library's the crate compiles (ADR 0914), its MIR.
pub(super) fn calls_of<'tcx>(body: &Body<'tcx>) -> Vec<(DefId, ty::GenericArgsRef<'tcx>, rustc_span::Span)> {
    use rustc_middle::thir::ExprKind;
    if body.root().is_some() {
        return (body.thir.exprs.iter())
            .filter_map(|expr| match (&expr.kind, super::fn_def(expr.ty)) {
                (ExprKind::ZstLiteral { .. }, Some((callee, args))) => Some((callee, args, expr.span)),
                _ => None,
            })
            .collect();
    }
    struct Called<'tcx>(Vec<(DefId, ty::GenericArgsRef<'tcx>, rustc_span::Span)>);
    impl<'tcx> mir::visit::Visitor<'tcx> for Called<'tcx> {
        fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, _: mir::Location) {
            if let Some((callee, args)) = super::fn_def(constant.ty()) {
                self.0.push((callee, args, constant.span));
            }
        }
    }
    let mut called = Called(Vec::new());
    if let Some(mir) = &body.mir {
        mir::visit::Visitor::visit_body(&mut called, &mir.body);
    }
    called.0
}

/// Each function `body` calls or names as a value, and its generic
/// arguments there.
fn called<'tcx>(body: &mir::Body<'tcx>) -> Vec<(DefId, ty::GenericArgsRef<'tcx>)> {
    struct Called<'tcx>(Vec<(DefId, ty::GenericArgsRef<'tcx>)>);
    impl<'tcx> mir::visit::Visitor<'tcx> for Called<'tcx> {
        fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, _: mir::Location) {
            if let Some(called) = super::fn_def(constant.ty()) {
                self.0.push(called);
            }
        }
    }
    let mut called = Called(Vec::new());
    mir::visit::Visitor::visit_body(&mut called, body);
    called.0
}

/// Whether `id` is a function of a rust-js library a consumer can compile:
/// one with MIR, not a trait's or an impl's, which a dictionary calls, nor
/// a binding, nor `@rust-js/std`'s.
fn compiled_here(foreign: &super::library::Foreign<'_, '_>, tcx: TyCtxt<'_>, id: DefId) -> bool {
    !id.is_local()
        && foreign.is_library(id.krate)
        && matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn)
        && tcx.trait_of_assoc(id).is_none()
        && tcx.trait_impl_of_assoc(id).is_none()
        && !super::bindings::is_binding(tcx, id)
        && tcx.is_mir_available(id)
}

/// Whether `caller`'s call of `callee`, of `args`, gives it a value to copy
/// or drop that its export takes nothing for: of a type parameter its body
/// may drop, a value whose drop runs a destructor; of one its `Copy` bound
/// copies, a value whose copy isn't itself (ADRs 0098, 0289, 0914).
fn gives_more<'tcx>(
    calls: &Calls<'_, 'tcx>,
    (caller, callee, args): (DefId, DefId, ty::GenericArgsRef<'tcx>),
    may_drop: &mut HashMap<(DefId, u32), bool>,
) -> bool {
    use crate::library::ParamFact;
    let tcx = calls.tcx;
    let item = calls.foreign.item(callee);
    let takes = |index: u32, fact: ParamFact| {
        (item.and_then(|item| item.calling.param(index))).is_some_and(|param| param.has(fact))
    };
    let copied = super::analysis::copy_bounded(tcx, callee);
    let cloned = super::analysis::clone_bounded(tcx, callee);
    (0..tcx.generics_of(callee).count() as u32).any(|index| {
        let Some(given) = args.get(index as usize).and_then(|arg| arg.as_type()) else {
            return false;
        };
        let drops = match given.kind() {
            ty::Param(param) => calls
                .drop_params
                .get(&caller)
                .is_some_and(|given| given.contains(&param.index)),
            _ => !given.has_param() && (calls.runs)(given),
        };
        let copies = match given.kind() {
            ty::Param(param) => calls.copied.contains(&(caller, param.index)),
            _ => !super::analysis::copies_itself(tcx, given),
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
            _ => super::representation::may_look_like_none(&calls.recognition, given),
        };
        (drops && !takes(index, ParamFact::Drop) && body_may_drop(calls, (callee, index), may_drop))
            || (nullish
                && takes(index, ParamFact::Unboxed)
                && body_may_box(calls, (callee, index), &mut HashSet::new()))
            || (copies && copied.contains(&index) && !takes(index, ParamFact::Copy))
            || (clones && cloned.contains(&index) && !takes(index, ParamFact::Clone))
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
    seen: &mut HashMap<(DefId, u32), bool>,
) -> bool {
    if let Some(&known) = seen.get(&(id, index)) {
        return known;
    }
    // Assumed not, while it's asked of a call it makes of itself.
    seen.insert((id, index), false);
    let tcx = calls.tcx;
    let body = tcx.optimized_mir(id);
    let holds = |ty: Ty<'tcx>| super::drops::owned_params(tcx, ty).contains(&index);
    // The cleanup it may unwind to: from what may throw where it runs.
    let mut unwound: HashSet<mir::BasicBlock> = HashSet::new();
    let mut work: Vec<mir::BasicBlock> = (body.basic_blocks.iter())
        .filter(|data| !data.is_cleanup)
        .filter_map(|data| {
            let terminator = data.terminator();
            let mir::UnwindAction::Cleanup(cleanup) = terminator.unwind()? else {
                return None;
            };
            let cannot_throw = matches!(&terminator.kind, TerminatorKind::Call { func, .. }
                if func.const_fn_def().is_some_and(|(callee, _)| super::bindings::cannot_throw(tcx, callee)));
            (!cannot_throw).then_some(*cleanup)
        })
        .collect();
    while let Some(block) = work.pop() {
        if unwound.insert(block) {
            work.extend(body.basic_blocks[block].terminator().successors());
        }
    }
    let drops = body
        .basic_blocks
        .iter_enumerated()
        .any(|(block, data)| match &data.terminator().kind {
            _ if data.is_cleanup && !unwound.contains(&block) => false,
            TerminatorKind::Drop { place, .. } => holds(place.ty(&body.local_decls, tcx).ty),
            TerminatorKind::Call { func, args, .. } => {
                // Given by its place, moved or, of what optimized MIR copies
                // where it's last used, copied.
                let gives_one = args.iter().any(|arg| match &arg.node {
                    mir::Operand::Move(place) | mir::Operand::Copy(place) => holds(place.ty(&body.local_decls, tcx).ty),
                    _ => false,
                });
                if !gives_one {
                    return false;
                }
                match func.const_fn_def() {
                    // A function of its library's given it as its own type
                    // parameter, which may drop it there.
                    Some((callee, callee_args)) if compiled_here(calls.foreign, tcx, callee) => {
                        (0..callee_args.len() as u32).any(|at| {
                            (callee_args.get(at as usize).and_then(|arg| arg.as_type()))
                                .is_some_and(|ty| matches!(ty.kind(), ty::Param(p) if p.index == index))
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
fn body_may_box<'tcx>(calls: &Calls<'_, 'tcx>, (id, index): (DefId, u32), seen: &mut HashSet<DefId>) -> bool {
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
        let asks = if super::bindings::is_binding(tcx, callee)
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

/// Whether a value of `ty` may hold one of type parameter `index`: `ty` is
/// of it, but a closure only by what it captures and what it's given or
/// gives back. A closure's type is of each of its maker's type parameters,
/// whose values it holds none of unless it captures one, so React's
/// `runWithFiberInDEV`'s `CURRENT.with_borrow(|current| current.clone())`
/// gives std no `()` it's given (gap rd85).
fn holds_param(ty: Ty<'_>, index: u32) -> bool {
    let mut walk = ty.walk();
    while let Some(arg) = walk.next() {
        let Some(ty) = arg.as_type() else { continue };
        match ty.kind() {
            ty::Param(param) if param.index == index => return true,
            ty::Closure(_, args) => {
                walk.skip_current_subtree();
                let closure = args.as_closure();
                let sig = closure.sig().skip_binder();
                if holds_param(closure.tupled_upvars_ty(), index)
                    || sig.inputs_and_output.iter().any(|ty| holds_param(ty, index))
                {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

/// Refuses an instance's read of its library's private state: a static or
/// a thread-local its library doesn't export, which a function compiled in
/// another crate can't reach, as its library's own code has the one there
/// is; and its call of a binding its library names by a path relative to
/// its own file (ADR 0914).
fn refuse_private_state<'tcx>(calls: &Calls<'_, 'tcx>, id: DefId, body: &mir::Body<'tcx>) {
    let tcx = calls.tcx;
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
            || (matches!(tcx.def_kind(def), DefKind::Const { .. })
                && matches!(tcx.type_of(def).instantiate_identity().skip_normalization().kind(),
                    ty::Adt(key, _) if super::recognition::is_std_def(tcx, key.did(), super::recognition::StdItem::LocalKey)));
        // A binding's, an `extern` block's static, is JS's, which the
        // crate's JS reads as its library's does, but of a module its
        // library names by a path relative to its own file, which is below.
        let binding = tcx.is_foreign_item(def);
        if state && !binding && !def.is_local() && calls.foreign.item(def).is_none() {
            let root = tcx.typeck_root_def_id(id);
            tcx.dcx().err(format!(
                "rust-js: this crate compiles `{}` itself, as its library's JS has none it can call: one given a value to copy or drop that it takes nothing for, an impl its library's code never asks for, or one that copies what this crate changes in place; but it reads `{}`, private to its library, which only that library's code can reach (ADR 0914): make it `pub`",
                tcx.def_path_str(root),
                tcx.def_path_str(def),
            ));
        }
    }
    // A thread-local its library makes its module's variable, `export let
    // last` (ADR 0270, amended), which only that module's JS can set: an
    // import can't be assigned. Set, by `set`, `take`, `replace`,
    // `with_borrow_mut` or its cell lent by `with`, of the type it holds.
    let variable =
        |def: DefId| (calls.foreign.item(def)).is_some_and(|item| item.calling.has(crate::library::ItemFact::Variable));
    let keys: Vec<DefId> = (read.read.iter().copied())
        .filter(|&def| !def.is_local() && variable(def))
        .collect();
    let held = |key: DefId| match tcx.type_of(key).instantiate_identity().skip_normalization().kind() {
        ty::Adt(_, args) => args.types().next(),
        _ => None,
    };
    for (callee, args) in called(body) {
        let sets = super::recognition::local_key_sets(tcx, callee);
        // What the key it's called of holds, of its impl's `LocalKey<Cell<T>>`.
        let of = (tcx.inherent_impl_of_assoc(callee)).and_then(|imp| {
            match tcx.type_of(imp).instantiate(tcx, args).skip_normalization().kind() {
                ty::Adt(_, key_args) => key_args.types().next(),
                _ => None,
            }
        });
        if sets && let Some(&key) = keys.iter().find(|&&key| held(key).is_some() && held(key) == of) {
            let root = tcx.typeck_root_def_id(id);
            tcx.dcx().err(format!(
                "rust-js: this crate compiles `{}` itself, as its library's JS has none it can call: one given a value to copy or drop that it takes nothing for, an impl its library's code never asks for, or one that copies what this crate changes in place; but it sets `{}`, its library's module variable, which JS lets only that module set (ADR 0270, amended): set it in a `pub fn` of that library that takes nothing to copy or drop, and call that",
                tcx.def_path_str(root),
                tcx.def_path_str(key),
            ));
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
        let path = super::bindings::js_path(tcx, binding);
        if !binding.is_local()
            && super::bindings::is_binding(tcx, binding)
            && let Some(((from, _), _)) = path.as_deref().and_then(super::bindings::js_import)
            && crate::program::is_relative_specifier(&from)
        {
            let root = tcx.typeck_root_def_id(id);
            tcx.dcx().err(format!(
                "rust-js: this crate compiles `{}` itself, as its library's JS has none it can call: one given a value to copy or drop that it takes nothing for, an impl its library's code never asks for, or one that copies what this crate changes in place; but it {} `{}`, which its library binds to `{from}`, a module by a path relative to that library's own file, not this crate's (ADR 0914): have its library bind it by a package's path, which this crate can import too",
                tcx.def_path_str(root),
                if tcx.is_foreign_item(binding) && matches!(tcx.def_kind(binding), DefKind::Static { .. }) { "reads" } else { "calls" },
                tcx.def_path_str(binding),
            ));
        }
    }
}
