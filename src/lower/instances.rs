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
    pub(super) copied: &'a HashSet<(DefId, u32)>,
    pub(super) bodies: Vec<&'a Body<'tcx>>,
}

/// What the analyzed crate asks of its calls of a library's functions.
struct Calls<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    foreign: &'a super::library::Foreign<'a, 'tcx>,
    /// Whether a value of a type that's no type parameter runs a destructor
    /// where it's dropped, as the crate drops it.
    runs: &'a dyn Fn(Ty<'tcx>) -> bool,
    /// The type parameters of the crate's own functions given a drop.
    drop_params: &'a HashMap<DefId, Vec<u32>>,
    /// The type parameters of the crate's own functions given a copy.
    copied: &'a HashSet<(DefId, u32)>,
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
        copied,
        bodies,
    } = analyzed;
    let recognition = super::recognition::Recognition {
        tcx,
        typing_env: ty::TypingEnv::fully_monomorphized(),
        trait_impls,
        foreign,
    };
    let runs = |t: Ty<'tcx>| super::drops::drop_runs(recognition, (library, closure_facts), counted, t);
    let calls = Calls {
        tcx,
        foreign,
        runs: &runs,
        drop_params,
        copied,
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
        (drops && !takes(index, ParamFact::Drop) && body_may_drop(calls, (callee, index), may_drop))
            || (copies && copied.contains(&index) && !takes(index, ParamFact::Copy))
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
    for def in read.read {
        let state = matches!(tcx.def_kind(def), DefKind::Static { .. })
            || (matches!(tcx.def_kind(def), DefKind::Const { .. })
                && matches!(tcx.type_of(def).instantiate_identity().skip_normalization().kind(),
                    ty::Adt(key, _) if super::recognition::is_std_def(tcx, key.did(), super::recognition::StdItem::LocalKey)));
        if state && !def.is_local() && calls.foreign.item(def).is_none() {
            let root = tcx.typeck_root_def_id(id);
            tcx.dcx().err(format!(
                "rust-js: this crate gives `{}` a value to copy or drop that its library's JS takes nothing for, as its own crate's calls give it none, so it's compiled here; but it reads `{}`, private to its library, which only that library's code can reach (ADR 0914): make it `pub`, or give the function a value it needn't copy or drop",
                tcx.def_path_str(root),
                tcx.def_path_str(def),
            ));
        }
    }
    // A binding of its library's to a module by a path relative to that
    // library's file, `./stack.js`, which isn't this crate's.
    for (binding, _) in called(body) {
        let path = super::bindings::js_path(tcx, binding);
        if !binding.is_local()
            && super::bindings::is_binding(tcx, binding)
            && let Some(((from, _), _)) = path.as_deref().and_then(super::bindings::js_import)
            && crate::program::is_relative_specifier(&from)
        {
            let root = tcx.typeck_root_def_id(id);
            tcx.dcx().err(format!(
                "rust-js: this crate gives `{}` a value to copy or drop that its library's JS takes nothing for, as its own crate's calls give it none, so it's compiled here; but it calls `{}`, which its library binds to `{from}`, a module by a path relative to that library's own file, not this crate's (ADR 0914): give the function a value it needn't copy or drop",
                tcx.def_path_str(root),
                tcx.def_path_str(binding),
            ));
        }
    }
}
