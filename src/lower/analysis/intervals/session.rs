//! What a session keeps of the MIR intervals (ADR 1137): the proofs of
//! each body being lowered, what its functions' callers give them, and,
//! `--intervals-shadow`'s, each check compared with THIR's ranges.

use super::*;
use crate::lower::recognition::{MapKey, map_key, once_method};
use crate::lower::std_types::once::OnceOp;

/// What MIR proves of the session's bodies: what each function only calls
/// reach is given (`parameter_intervals`), and the proofs of each body
/// being lowered, the innermost last, with, where the session records the
/// shadow, of each span its operators are written at whether each is
/// proved.
#[derive(Default)]
pub(crate) struct Intervals {
    given: IndexMap<rustc_span::def_id::DefId, Given>,
    /// Of each closure whose result's `Some` knows a capture by shared
    /// reference below a length, the capture's field and the most it is
    /// (`closure_results`, #1701).
    results: IndexMap<rustc_span::def_id::DefId, (usize, i128)>,
    bodies: Vec<(Proofs, IndexMap<Span, bool>)>,
    /// Each body's address, of `bodies`' each: what's asked of a block is
    /// of the body proved, never another's.
    proved: Vec<usize>,
}

/// What `--intervals-shadow` records: each line.
#[derive(Default)]
pub(crate) struct Shadow {
    lines: IndexSet<String>,
}

/// The crate's caller-given parameters, of the crate's call graph, its
/// functions' MIR the emitter reads (#1672 M2b-2) and its constants', whose
/// calls are a function's callers too: a `const fn` only constants and
/// patterns call, `add(1, 2)` of `const SUM`, is given what they pass
/// (#1742; `call_graph::crate_graph`).
pub(in crate::lower) fn given_parameters<'tcx>(tcx: TyCtxt<'tcx>, calls: &crate::lower::call_graph::Crate<'_, 'tcx>) {
    let results = closure_results(tcx, &calls.bodies);
    crate::lower::session::caches(tcx).intervals.borrow_mut().results = results;
    let only_called = crate::lower::analysis::only_called::only_called(tcx);
    let mut given = parameter_intervals(tcx, &calls.graph, &calls.bodies, &only_called);
    capture_intervals(tcx, &calls.bodies, &mut given);
    crate::lower::session::caches(tcx).intervals.borrow_mut().given = given;
}

/// A body's lowering begun: its proofs, and where the session records the
/// shadow, of each span its operators are written at whether each is
/// proved, until the guard it gives drops.
pub(in crate::lower) fn body_proofs<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &Body<'tcx>,
) -> ProofsScope<'tcx> {
    let caches = crate::lower::session::caches(tcx);
    let given = (caches.intervals.borrow().given.get(&body.source.def_id()).cloned()).unwrap_or_default();
    // Of a body with no bounds check, no index of a map, no `+`, `-` or `*`
    // and no integer cast, nothing to prove, its walk left out.
    let asks = (body.basic_blocks.iter()).any(|data| {
        matches!(&data.terminator().kind, TerminatorKind::Assert { msg, .. } if matches!(**msg, AssertKind::BoundsCheck { .. }))
            || matches!(&data.terminator().kind, TerminatorKind::Call { func, .. }
                if func.const_fn_def().is_some_and(|(callee, args)| {
                    call_indexes(tcx, callee, args).is_some()
                        || range_index(tcx, callee, args).is_some()
                        || map_key(tcx, callee, args) == Some(MapKey::Index)
                }))
            || (data.statements.iter()).any(|statement| {
                matches!(&statement.kind, StatementKind::Assign(assign)
                    if matches!(assign.1, Rvalue::BinaryOp(BinOp::Add | BinOp::Sub | BinOp::Mul, _) | Rvalue::Cast(mir::CastKind::IntToInt, ..)))
            })
    });
    let mut proofs = match asks {
        true => proofs(tcx, typing_env, body, given),
        false => Proofs::default(),
    };
    // Of a body that sets a `OnceCell`, which cells are still empty there.
    let sets = (body.basic_blocks.iter()).any(|data| {
        matches!(&data.terminator().kind, TerminatorKind::Call { func, .. }
            if func.const_fn_def().is_some_and(|(def_id, _)| once_method(tcx, def_id) == Some(OnceOp::Set)))
    });
    if sets {
        proofs.empty_sets = crate::lower::analysis::empty_sets(tcx, typing_env, body);
    }
    let mut exact: IndexMap<Span, bool> = IndexMap::default();
    if caches.intervals_shadow.is_some() {
        for (block, data) in body.basic_blocks.iter_enumerated() {
            for (statement_index, statement) in data.statements.iter().enumerate() {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let Rvalue::BinaryOp(BinOp::Add | BinOp::Sub | BinOp::Mul, _) = &assign.1
                {
                    let proved = proofs.exact.contains(&Location { block, statement_index });
                    *exact.entry(statement.source_info.span).or_insert(true) &= proved;
                }
            }
        }
    }
    let mut intervals = caches.intervals.borrow_mut();
    intervals.bodies.push((proofs, exact));
    intervals.proved.push(std::ptr::from_ref(body).addr());
    ProofsScope { tcx }
}

/// Holds a body's proofs while it's lowered.
pub(in crate::lower) struct ProofsScope<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl Drop for ProofsScope<'_> {
    fn drop(&mut self) {
        let caches = crate::lower::session::caches(self.tcx);
        let mut intervals = caches.intervals.borrow_mut();
        intervals.bodies.pop();
        intervals.proved.pop();
    }
}

/// What THIR's facts leave out: a bounds check, by the block its `assert`
/// ends, or a wrap.
pub(in crate::lower) enum LeftOut {
    Bounds(BasicBlock),
    Wrap,
    /// An integer cast, by its statement's location.
    Cast(Location),
    /// A `OnceCell`'s `set` of a cell known empty, by the block its call ends.
    EmptySet(BasicBlock),
    /// A call's indexes known in bounds, by the block it ends.
    CallBounds(BasicBlock),
}

/// A check THIR's facts leave out or keep, `left_out`, against MIR's
/// proof of it: a line where THIR leaves out what MIR doesn't prove, a
/// wrong answer's candidate or a gap of MIR's precision, and a `mir-only`
/// line where MIR proves what THIR keeps, which M3 leaves out, so what it
/// proves there must be right.
pub(in crate::lower) fn shadow_check(
    tcx: TyCtxt<'_>,
    owner: rustc_span::def_id::DefId,
    span: Span,
    left_out: bool,
    check: impl FnOnce() -> Option<LeftOut>,
) {
    let caches = crate::lower::session::caches(tcx);
    let Some(shadow) = &caches.intervals_shadow else {
        return;
    };
    let Some(check) = check() else {
        return;
    };
    let intervals = caches.intervals.borrow();
    let Some((proofs, exact)) = intervals.bodies.last() else {
        return;
    };
    let (kind, proved) = match check {
        LeftOut::Bounds(block) => ("bounds", proofs.bounds.contains(&block)),
        LeftOut::Wrap => ("wrap", exact.get(&span).copied().unwrap_or(false)),
        LeftOut::Cast(location) => ("cast", proofs.casts.contains(&location)),
        LeftOut::EmptySet(block) => ("empty-set", proofs.empty_sets.contains(&block)),
        LeftOut::CallBounds(block) => ("call-bounds", proofs.call_bounds.contains(&block)),
    };
    let kind = match (left_out, proved) {
        (true, false) => kind,
        (false, true) => &format!("mir-only-{kind}"),
        _ => return,
    };
    let at = tcx.sess.source_map().span_to_diagnostic_string(span);
    let line = format!("{kind}\t{}\t{at}\n", tcx.def_path_str(owner));
    shadow.borrow_mut().lines.insert(line);
}

/// Each check THIR's facts leave out that MIR doesn't prove, a line each,
/// `kind<TAB>function<TAB>where`: what `--intervals-shadow` writes.
pub(crate) fn shadow_report(tcx: TyCtxt<'_>) -> String {
    let caches = crate::lower::session::caches(tcx);
    caches
        .intervals_shadow
        .as_ref()
        .map(|s| s.borrow().lines.iter().map(String::as_str).collect())
        .unwrap_or_default()
}

/// What `closure`'s result's `Some` knows: a capture by shared reference,
/// by field, at most the most it gives (`closure_results`, #1701).
pub(super) fn closure_result(tcx: TyCtxt<'_>, closure: rustc_span::def_id::DefId) -> Option<(usize, i128)> {
    crate::lower::session::caches(tcx)
        .intervals
        .borrow()
        .results
        .get(&closure)
        .copied()
}

/// Whether MIR proves the bounds check `block`'s `assert` makes, of
/// `body`, never fails (#1672 M3): none of a body not the one being
/// lowered, whose proofs aren't these.
pub(in crate::lower) fn bounds_proved(tcx: TyCtxt<'_>, body: &Body<'_>, block: BasicBlock) -> bool {
    let caches = crate::lower::session::caches(tcx);
    let intervals = caches.intervals.borrow();
    intervals.proved.last() == Some(&std::ptr::from_ref(body).addr())
        && (intervals.bodies.last()).is_some_and(|(proofs, _)| proofs.bounds.contains(&block))
}

/// Whether MIR proves the `OnceCell` the `set` ending `block` of `body` sets
/// is still empty there (#1672 M3): none of a body not the one being lowered.
pub(in crate::lower) fn empty_set_proved(tcx: TyCtxt<'_>, body: &Body<'_>, block: BasicBlock) -> bool {
    let caches = crate::lower::session::caches(tcx);
    let intervals = caches.intervals.borrow();
    intervals.proved.last() == Some(&std::ptr::from_ref(body).addr())
        && (intervals.bodies.last()).is_some_and(|(proofs, _)| proofs.empty_sets.contains(&block))
}

/// Whether MIR proves the integer cast at `location` of `body` is of a
/// value in its target's range, the same number (#1672 M3): none of a body
/// not the one being lowered.
pub(in crate::lower) fn cast_proved(tcx: TyCtxt<'_>, body: &Body<'_>, location: Location) -> bool {
    let caches = crate::lower::session::caches(tcx);
    let intervals = caches.intervals.borrow();
    intervals.proved.last() == Some(&std::ptr::from_ref(body).addr())
        && (intervals.bodies.last()).is_some_and(|(proofs, _)| proofs.casts.contains(&location))
}

/// Whether MIR proves the `+`, `-` or `*` at `location` of `body` stays in
/// its type's range, JS's own operator its value (#1672 M3): none of a body
/// not the one being lowered.
pub(in crate::lower) fn exact_proved(tcx: TyCtxt<'_>, body: &Body<'_>, location: Location) -> bool {
    let caches = crate::lower::session::caches(tcx);
    let intervals = caches.intervals.borrow();
    intervals.proved.last() == Some(&std::ptr::from_ref(body).addr())
        && (intervals.bodies.last()).is_some_and(|(proofs, _)| proofs.exact.contains(&location))
}
