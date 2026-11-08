//! Where a future's running as it's made would be seen (#965, ADR 0029,
//! amended). A rust-js future is a JS promise, which runs as it's made,
//! where Rust's runs only once it's polled. That's kept where nothing can
//! tell: a future whose own body is quiet (`unwinding::quiet`), or one
//! awaited before anything that may be seen runs. Each other future the
//! crate makes, an `async fn`'s, an `async` block's or an async closure's,
//! is refused where it's held past what may be seen and awaited after,
//! dropped unawaited, or given where its `.await` can't be followed. Given
//! to a binding, `js::spawn` or `js::promise` say, it's the JS boundary's,
//! a running promise by design; given back to its caller, its caller's.

use super::{awaits_quietly, destined_locally, quiet_but_their_parameters, quiet_call, writes_a_local};
use crate::collections::IndexSet;
use crate::lower::Body;
use crate::lower::recognition::Recognition;
use rustc_hir::CoroutineKind;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::mir::{self, BasicBlock, Operand, Place, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

/// What became of a future the crate made, followed from where it's made.
enum Fate {
    /// Awaited, after what may be seen ran there, its span.
    AwaitedAfter(Span),
    /// Dropped unawaited, there.
    Dropped(Span),
    /// Given where its `.await` can't be followed, there.
    Lost(Span),
}

/// Refuses each future of `bodies` whose running as it's made would be
/// seen: of a body that isn't `quiet`'s, where it's held past what may be
/// seen, dropped unawaited, or given where it can't be followed.
pub(super) fn refuse_where_eagerness_shows<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: &Recognition<'_, 'tcx>,
    bodies: &[&Body<'tcx>],
    quiet: &IndexSet<DefId>,
    runs: &dyn Fn(Ty<'tcx>, DefId) -> bool,
) {
    let quiet_but_theirs = &quiet_but_their_parameters(tcx, recognition, bodies, quiet, runs);
    for body in bodies {
        let Some(mir) = body.mir.as_ref().map(|m| &m.body) else {
            continue;
        };
        if !body.def_id.is_local() {
            continue;
        }
        let typing_env = ty::TypingEnv::post_analysis(tcx, body.def_id);
        let drops = |ty| runs(ty, body.def_id);
        let follow = Follow {
            tcx,
            recognition,
            mir,
            typing_env,
            quiet,
            quiet_but_theirs,
            drops: &drops,
            // A `Drop` of what's moved out by then drops nothing (#717): a
            // future moved into what holds it.
            dead: crate::lower::mir::dead_drops(tcx, mir, &drops),
        };
        for (made, at, span) in follow.made() {
            let Some(fate) = follow.fate(made, at) else {
                continue;
            };
            let (headline, at, label) = match fate {
                Fate::AwaitedAfter(seen) => (
                    "code that may be seen runs before it's awaited, where Rust's runs only then",
                    seen,
                    "this may be seen: await the future where it's made, or make it after this",
                ),
                Fate::Dropped(dropped) => (
                    "it's dropped unawaited, where Rust's never runs",
                    dropped,
                    "dropped here: await it, or don't make it",
                ),
                Fate::Lost(given) => (
                    "it's given where rust-js can't follow when it's awaited",
                    given,
                    "given here: await it where it's made, or give it to a binding such as `js::spawn` or `js::promise`, which takes a running promise",
                ),
            };
            tcx.dcx()
                .struct_span_err(
                    span,
                    format!("rust-js: this future runs as it's made, as a JS promise does, but {headline} (ADR 0029)"),
                )
                .with_span_label(at, label)
                .emit();
        }
    }
}

/// A body's futures followed through it.
struct Follow<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    recognition: &'a Recognition<'a, 'tcx>,
    mir: &'a mir::Body<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    quiet: &'a IndexSet<DefId>,
    quiet_but_theirs: &'a IndexSet<DefId>,
    drops: &'a dyn Fn(Ty<'tcx>) -> bool,
    dead: IndexSet<BasicBlock>,
}

impl<'tcx> Follow<'_, 'tcx> {
    /// The async body a value of `ty` runs, where it's a future the crate
    /// made: an `async fn`'s, an `async` block's or an async closure's.
    fn future_body(&self, ty: Ty<'tcx>) -> Option<DefId> {
        match *self.normalized(ty).kind() {
            ty::Coroutine(def_id, _)
                if def_id.is_local()
                    && matches!(
                        self.tcx.coroutine_kind(def_id),
                        Some(CoroutineKind::Desugared(rustc_hir::CoroutineDesugaring::Async, _))
                    ) =>
            {
                Some(def_id)
            }
            _ => None,
        }
    }

    /// Each future this body makes whose own body isn't quiet: the local
    /// it's made in, or whose field it's made in, `s.f = greet()`, where
    /// it's made, and its span.
    fn made(&self) -> Vec<(mir::Local, (BasicBlock, usize), Span)> {
        let mut out = Vec::new();
        let noisy = |ty: Ty<'tcx>| self.future_body(ty).is_some() && !self.quiet_instance(ty, 0);
        for (block, data) in self.mir.basic_blocks.iter_enumerated() {
            if data.is_cleanup {
                continue;
            }
            for (i, statement) in data.statements.iter().enumerate() {
                if let StatementKind::Assign(assign) = &statement.kind
                    && !assign.0.is_indirect()
                    && let Rvalue::Aggregate(kind, _) = &assign.1
                    && matches!(**kind, mir::AggregateKind::Coroutine(..))
                    && noisy(assign.0.ty(self.mir, self.tcx).ty)
                {
                    out.push((assign.0.local, (block, i + 1), statement.source_info.span));
                }
            }
            if let TerminatorKind::Call {
                destination,
                target: Some(target),
                fn_span,
                ..
            } = &data.terminator().kind
                && !destination.is_indirect()
                && noisy(destination.ty(self.mir, self.tcx).ty)
            {
                out.push((destination.local, (*target, 0), *fn_span));
            }
        }
        // Not what's given back as it's made, an `async fn`'s own body's
        // future, its caller's.
        out.retain(|&(local, ..)| local != mir::RETURN_PLACE);
        out
    }

    /// What becomes of the future made in `made` at `at`, of what isn't
    /// kept: none where it's awaited before anything that may be seen,
    /// given to a binding, or given back.
    fn fate(&self, made: mir::Local, at: (BasicBlock, usize)) -> Option<Fate> {
        // The future's own type, which what holds it holds.
        let future = self.normalized(self.mir.local_decls[made].ty);
        let mut holding: IndexSet<mir::Local> = IndexSet::from_iter([made]);
        let mut seen: Option<Span> = None;
        let mut queue = vec![at];
        let mut visited: IndexSet<BasicBlock> = IndexSet::default();
        let holds = |holding: &IndexSet<mir::Local>, operand: &Operand<'tcx>| {
            operand.place().is_some_and(|p| holding.contains(&p.local))
        };
        while let Some((block, from)) = queue.pop() {
            if from == 0 && !visited.insert(block) {
                continue;
            }
            let data = &self.mir.basic_blocks[block];
            for statement in &data.statements[from..] {
                let StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                let (to, value) = (&assign.0, &assign.1);
                let reads = match value {
                    Rvalue::Use(operand, _) | Rvalue::Cast(_, operand, _) => holds(&holding, operand),
                    Rvalue::Ref(_, _, place) | Rvalue::RawPtr(_, place) => holding.contains(&place.local),
                    Rvalue::Aggregate(_, operands) => operands.iter().any(|o| holds(&holding, o)),
                    _ => false,
                };
                if reads {
                    // Given back to its caller.
                    if to.local == mir::RETURN_PLACE {
                        return None;
                    }
                    // Moved, borrowed, pinned or boxed into another local,
                    // or a field of one, `pin!`'s helper's, which holds it
                    // then; not through a reference, into what the body
                    // doesn't own.
                    if !to.is_indirect() {
                        holding.insert(to.local);
                        continue;
                    }
                    return Some(Fate::Lost(statement.source_info.span));
                }
                if seen.is_none() && !writes_a_local(statement) {
                    seen = Some(statement.source_info.span);
                }
            }
            let terminator = data.terminator();
            let span = terminator.source_info.span;
            let next = |queue: &mut Vec<(BasicBlock, usize)>| {
                for successor in terminator.successors() {
                    if !self.mir.basic_blocks[successor].is_cleanup {
                        queue.push((successor, 0));
                    }
                }
            };
            match &terminator.kind {
                TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    target,
                    ..
                } => {
                    let callee = func.const_fn_def();
                    let given = args.iter().any(|arg| holds(&holding, &arg.node));
                    let awaits =
                        callee.is_some_and(|(def, _)| self.tcx.is_lang_item(def, LangItem::IntoFutureIntoFuture));
                    if given && awaits {
                        // Awaited: after what may be seen, or not.
                        if let Some(seen) = seen {
                            return Some(Fate::AwaitedAfter(seen));
                        }
                        continue;
                    }
                    if given {
                        let Some((def, _)) = callee else {
                            return Some(Fate::Lost(span));
                        };
                        // `drop(f)`: dropped unawaited.
                        if crate::lower::recognition::is_mem_drop(self.tcx, def) {
                            return Some(Fate::Dropped(span));
                        }
                        // The JS boundary's: a binding takes a running
                        // promise.
                        if crate::lower::bindings::is_binding(self.tcx, def) {
                            return None;
                        }
                        // What holds it, `Box::new(f)`, `Box::pin(f)`, the pin
                        // `pin!` makes of its helper: std's, which runs
                        // nothing of it, giving a value whose type holds the
                        // future's. Not a function of the crate's or another
                        // crate's, an `async fn later(f: F)` that may run
                        // what's seen before it awaits `f`.
                        let held = self.normalized(destination.ty(self.mir, self.tcx).ty);
                        let carried = crate::lower::recognition::is_std_crate(self.tcx, def)
                            && held.walk().any(|part| part.as_type() == Some(future));
                        if !carried || !destination.projection.is_empty() {
                            return Some(Fate::Lost(span));
                        }
                        holding.insert(destination.local);
                        if let Some(target) = target {
                            queue.push((*target, 0));
                        }
                        continue;
                    }
                    // Another future awaited, its `into_future` and each
                    // `poll` of the loop that awaits it: seen unless what's
                    // awaited does nothing.
                    if let Some((def, generic_args)) = callee
                        && let Some(quietly) = awaits_quietly(
                            (self.tcx, self.recognition, self.typing_env),
                            self.quiet,
                            (def, generic_args),
                        )
                    {
                        if seen.is_none() && !quietly {
                            seen = Some(span);
                        }
                        next(&mut queue);
                        continue;
                    }
                    if seen.is_none() {
                        let quiet = destined_locally(terminator)
                            && callee.is_some_and(|(def, generic_args)| {
                                quiet_call(
                                    (self.tcx, self.recognition, self.typing_env),
                                    self.quiet,
                                    (def, generic_args, args),
                                    self.drops,
                                )
                            });
                        if !quiet {
                            seen = Some(span);
                        }
                    }
                    next(&mut queue);
                }
                TerminatorKind::Drop { .. } if self.dead.contains(&block) => next(&mut queue),
                TerminatorKind::Drop { place, .. } => {
                    if self.owns(&holding, place) {
                        return Some(Fate::Dropped(span));
                    }
                    if seen.is_none() && (self.drops)(place.ty(self.mir, self.tcx).ty) {
                        seen = Some(span);
                    }
                    next(&mut queue);
                }
                TerminatorKind::Goto { .. }
                | TerminatorKind::SwitchInt { .. }
                | TerminatorKind::FalseEdge { .. }
                | TerminatorKind::FalseUnwind { .. } => next(&mut queue),
                // Its end, or where its own future is dropped before it's
                // done, which isn't this body's doing.
                TerminatorKind::Return | TerminatorKind::Unreachable | TerminatorKind::CoroutineDrop => {}
                // The body waiting on what it awaits, which runs nothing of
                // its own: on, to its `resume`; its `drop` edge is where its
                // own future is dropped before it's done.
                TerminatorKind::Yield { resume, .. } => queue.push((*resume, 0)),
                _ => {
                    if seen.is_none() {
                        seen = Some(span);
                    }
                    next(&mut queue);
                }
            }
        }
        None
    }

    /// Whether `ty`, a future the crate made, runs nothing a program can
    /// see: its body's quiet, or quiet but for awaiting and dropping what's
    /// of its type parameters, each future among those its caller gives
    /// quiet and the rest dropping nothing.
    fn quiet_instance(&self, ty: Ty<'tcx>, depth: usize) -> bool {
        let ty = self.normalized(ty);
        let ty::Coroutine(body, args) = *ty.kind() else {
            return false;
        };
        if self.quiet.contains(&body) {
            return true;
        }
        depth < 8
            && self.quiet_but_theirs.contains(&body)
            && args
                .as_coroutine()
                .parent_args()
                .iter()
                .filter_map(|arg| arg.as_type())
                .all(|given| match self.future_body(given) {
                    Some(_) => self.quiet_instance(given, depth + 1),
                    None => !(self.drops)(given),
                })
    }

    /// `ty`, its opaque types revealed, as the body's typing environment
    /// sees it.
    fn normalized(&self, ty: Ty<'tcx>) -> Ty<'tcx> {
        self.tcx
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
            .unwrap_or(ty)
    }

    /// Whether `place` holds the future itself, not a reference to it.
    fn owns(&self, holding: &IndexSet<mir::Local>, place: &Place<'tcx>) -> bool {
        holding.contains(&place.local) && !place.ty(self.mir, self.tcx).ty.is_ref()
    }
}
