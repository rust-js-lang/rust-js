//! What an `extend` asks of a hint the crate's own may run, which std
//! asks as a `Vec`, a `VecDeque` or a `BinaryHeap` fills, as its capacity
//! says: whether the crate's own extends make it keep capacities
//! (`extends_as_it_fills`, ADR 0419, #849), and generic code's `extend` of
//! a collection its caller gives refused where none are kept
//! (`refuse_unknown_capacity_extends`, #849, #1180).

use super::*;
use crate::recognition::Recognition;
use crate::recognition::iterator_type_of;
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::mir::{Operand, TerminatorKind};
use rustc_middle::ty::TypeVisitableExt;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Whether any of `bodies` extends a `Vec`, a `VecDeque` or a
    /// `BinaryHeap` by an iterator whose hint may run the crate's own,
    /// which std asks as the collection fills, as its capacity says: then
    /// each one's capacity is native Rust's where it's asked (ADR 0419,
    /// #849).
    pub(crate) fn extends_as_it_fills(&self, bodies: &[&crate::Body<'tcx>]) -> bool {
        let extend = |func: &rustc_middle::mir::Operand<'tcx>,
                      args: &[rustc_span::Spanned<rustc_middle::mir::Operand<'tcx>>],
                      body: &rustc_middle::mir::Body<'tcx>| {
            let Some((def_id, generic_args)) = func.const_fn_def() else {
                return false;
            };
            let iterable = args.get(1).map(|arg| arg.node.ty(body, self.tcx));
            self.tcx
                .trait_of_assoc(def_id)
                .is_some_and(|t| crate::recognition::is_extend(self.tcx, t))
                && generic_args
                    .types()
                    .next()
                    .is_some_and(|ty| self.grows_as_it_extends(ty.peel_refs()))
                && iterable.is_some_and(|iterable| {
                    let iter_ty = crate::recognition::iterator_type_of(self.tcx, self.typing_env, iterable);
                    // A `dyn`'s too, of one Rust made, not a binding (#1377).
                    let binding = args
                        .get(1)
                        .is_some_and(|arg| self.made_by_a_binding_in(body, &arg.node));
                    self.hint_reaches_own_where(iter_ty, !binding)
                })
        };
        (bodies.iter())
            .filter_map(|body| body.mir.as_ref().map(|mir| &mir.body))
            .any(|body| {
                body.basic_blocks.iter().any(|data| {
                    matches!(&data.terminator().kind,
                    rustc_middle::mir::TerminatorKind::Call { func, args, .. } if extend(func, args, body))
                })
            })
    }

    /// Whether `operand` of `body` is what a binding's call made, JS's own
    /// iterator, as MIR shows it: a local a binding's call wrote, moved or
    /// copied on, or a `map`, `filter`, `take` or `skip` of one, whose
    /// hint is that one's (`iterators::binding_made`'s, of a value, #1377).
    fn made_by_a_binding_in(
        &self,
        body: &rustc_middle::mir::Body<'tcx>,
        operand: &rustc_middle::mir::Operand<'tcx>,
    ) -> bool {
        use rustc_middle::mir::{Operand, Rvalue, StatementKind, TerminatorKind};
        let mut operand = operand.clone();
        // Each step back is to an earlier write, so it ends, at most once
        // a local.
        for _ in 0..body.local_decls.len() {
            let (Operand::Move(place) | Operand::Copy(place)) = &operand else {
                return false;
            };
            if !place.projection.is_empty() {
                return false;
            }
            let local = place.local;
            let mut next = None;
            for data in body.basic_blocks.iter() {
                for statement in &data.statements {
                    if let StatementKind::Assign(assign) = &statement.kind
                        && assign.0.local == local
                        && assign.0.projection.is_empty()
                        && let Rvalue::Use(used, _) = &assign.1
                    {
                        next = Some(Err(used.clone()));
                    }
                }
                if let TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } = &data.terminator().kind
                    && destination.local == local
                    && destination.projection.is_empty()
                {
                    next = Some(Ok((func.clone(), args.first().map(|arg| arg.node.clone()))));
                }
            }
            match next {
                Some(Err(used)) => operand = used,
                Some(Ok((func, first))) => {
                    let Some((callee, _)) = func.const_fn_def() else {
                        return false;
                    };
                    if crate::bindings::is_binding(self.tcx, callee) {
                        return true;
                    }
                    let iterator = std_item(self.tcx, StdItem::Iterator);
                    let adapts = ["map", "filter", "take", "skip"]
                        .iter()
                        .any(|name| crate::recognition::trait_method(self.tcx, iterator, name) == callee);
                    match (adapts, first) {
                        (true, Some(first)) => operand = first,
                        _ => return false,
                    }
                }
                None => return false,
            }
        }
        false
    }
}

// Generic code's `extend` of a collection its caller gives, `c.extend(it)`
// of a `C: Extend<T>`, through its dictionary: where its caller gives a
// `Vec`, a `VecDeque` or a `BinaryHeap` and an iterator whose `size_hint`
// the crate's own may run, std asks that hint as the collection fills, as
// its capacity says, which a crate that reads no capacity doesn't keep
// (#849, the user's (b); #1180). Refused
// where those types are known, at the call that gives them; relayed
// where they're a caller's own type parameters, as #796 relays `TSize`, so
// the outermost caller that knows them decides. One of std's iterators, or
// the crate's that keeps std's default `size_hint`, compiles.
/// What a call refused gives, and what to write, a stopgap of the hint's
/// crossing (ADR 1201).
const REFUSED: &str = "a call giving generic code a `Vec`, a `VecDeque` or a `BinaryHeap` to `extend` by an iterator whose `size_hint` is the crate's own, which std asks as the collection fills, as its capacity says, which a crate that reads no capacity doesn't keep (#849, #1180)";
const INSTEAD: &str = "collect the iterator first and give that, or extend the collection here";

/// How many times what each function extends is relayed to its callers,
/// and how much of it each holds, past which what's left isn't: a
/// recursive generic function's types may grow without end.
const ROUNDS: usize = 16;
const HELD: usize = 64;

/// Refuses each call of `bodies` that gives generic code's `extend` a
/// collection that grows as it's extended and an iterator whose hint may
/// run the crate's own, both known there, of a crate that doesn't keep
/// capacities, `keeps_capacity`.
pub(crate) fn refuse_unknown_capacity_extends<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    bodies: &[&crate::Body<'tcx>],
    keeps_capacity: bool,
) {
    // Of a crate that keeps capacities, each `extend` grows as std's does
    // where it's made, a dictionary's too (`dictionary_grown`, #1180).
    if keeps_capacity {
        return;
    }
    let tcx = recognition.tcx;
    let mirs: Vec<(DefId, &rustc_middle::mir::Body<'tcx>)> = (bodies.iter())
        .filter_map(|body| Some((body.def_id, &body.mir.as_ref()?.body)))
        .collect();
    // What each function extends of what it doesn't know: the collection
    // and what it's extended by, in its own type parameters' terms.
    let mut extends: IndexMap<DefId, IndexSet<(Ty<'tcx>, Ty<'tcx>)>> = IndexMap::default();
    for &(def_id, body) in &mirs {
        for data in body.basic_blocks.iter() {
            let TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
                continue;
            };
            let Some((callee, generic_args)) = func.const_fn_def() else {
                continue;
            };
            let extend = tcx
                .trait_of_assoc(callee)
                .is_some_and(|t| crate::recognition::is_extend(tcx, t));
            let Some(collection) = generic_args.types().next().map(|ty| ty.peel_refs()) else {
                continue;
            };
            let Some(iterable) = args.get(1).map(|arg| arg.node.ty(body, tcx)) else {
                continue;
            };
            // Of a collection it doesn't know, whose dictionary's `extend`
            // can't ask its hint as std's does where capacities aren't
            // kept. One that grows as it's extended by an iterator its
            // caller gives keeps capacities (`extends_as_it_fills`).
            let given = extend && recognition.is_unknown(collection);
            if given {
                extends.entry(def_id).or_default().insert((collection, iterable));
            }
        }
    }
    // Each call of a function of the crate's, resolved where it's a
    // trait's, with what it's given in its caller's terms.
    let mut calls: Vec<(DefId, DefId, ty::GenericArgsRef<'tcx>, Span)> = Vec::new();
    for &(def_id, body) in &mirs {
        let typing_env = ty::TypingEnv::post_analysis(tcx, def_id);
        for data in body.basic_blocks.iter() {
            let TerminatorKind::Call { func, fn_span, .. } = &data.terminator().kind else {
                continue;
            };
            let Some((callee, generic_args)) = Operand::const_fn_def(func) else {
                continue;
            };
            let (callee, generic_args) = match crate::items::resolve_instance_in(tcx, typing_env, callee, generic_args)
            {
                Ok(Some(instance)) => (instance.def_id(), instance.args),
                _ => (callee, generic_args),
            };
            if callee.is_local() {
                calls.push((def_id, callee, generic_args, *fn_span));
            }
        }
    }
    let mut refused: IndexSet<Span> = IndexSet::default();
    for _ in 0..ROUNDS {
        // A closure's, the function's it's written in, whose type
        // parameters it has: what std's code that calls it isn't seen to.
        let mut changed = false;
        let closures: Vec<(DefId, IndexSet<(Ty<'tcx>, Ty<'tcx>)>)> = (extends.iter())
            .filter(|(def_id, _)| tcx.is_closure_like(**def_id))
            .map(|(&def_id, held)| (tcx.typeck_root_def_id(def_id), held.clone()))
            .collect();
        for (root, held) in closures {
            let theirs = extends.entry(root).or_default();
            for each in held {
                if theirs.len() < HELD {
                    changed |= theirs.insert(each);
                }
            }
        }
        for &(caller, callee, generic_args, span) in &calls {
            let Some(theirs) = extends.get(&callee).cloned() else {
                continue;
            };
            let typing_env = ty::TypingEnv::post_analysis(tcx, caller);
            for (collection, iterable) in theirs {
                let given = |ty: Ty<'tcx>| {
                    let ty = ty::EarlyBinder::bind(tcx, ty)
                        .instantiate(tcx, generic_args)
                        .skip_normalization();
                    tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))
                        .unwrap_or(ty)
                };
                let (collection, iterable) = (given(collection).peel_refs(), given(iterable));
                let grows = recognition.grows_as_it_extends(collection);
                if !grows && !recognition.is_unknown(collection) {
                    continue;
                }
                if grows && !iterable.has_param() && !iterable.has_aliases() {
                    let iter_ty = iterator_type_of(tcx, typing_env, iterable);
                    if recognition.hint_reaches_own(iter_ty) && refused.insert(span) {
                        crate::refusal::refuse(
                            tcx,
                            span,
                            crate::refusal::Refused::Unconverted(
                                crate::refusal::Crossing::HintArgument,
                                &crate::refusal::Refused::Instead(REFUSED, INSTEAD),
                            ),
                        );
                    }
                    continue;
                }
                let held = extends.entry(caller).or_default();
                if held.len() < HELD {
                    changed |= held.insert((collection, iterable));
                }
            }
        }
        if !changed {
            break;
        }
    }
}
