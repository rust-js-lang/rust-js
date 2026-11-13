//! What a session keeps of the MIR intervals (ADR 1137): the proofs of
//! each body being lowered, what its functions' callers give them, and,
//! `--intervals-shadow`'s, each check compared with THIR's ranges.

use super::*;

/// What MIR proves of the session's bodies: what each function only calls
/// reach is given (`parameter_intervals`), and the proofs of each body
/// being lowered, the innermost last, with, where the session records the
/// shadow, of each span its operators are written at whether each is
/// proved.
#[derive(Default)]
pub(crate) struct Intervals {
    given: IndexMap<rustc_span::def_id::DefId, Vec<Option<Interval>>>,
    bodies: Vec<(Proofs, IndexMap<Span, bool>)>,
}

/// What `--intervals-shadow` records: each line.
#[derive(Default)]
pub(crate) struct Shadow {
    lines: IndexSet<String>,
}

/// The crate's caller-given parameters, of `bodies`, each a function's
/// MIR the emitter reads, by the call graph of them (#1672 M2b-2).
pub(in crate::lower) fn given_parameters<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &IndexMap<rustc_span::def_id::DefId, &Body<'tcx>>,
) {
    // Until MIR decides a check (#1672 M3), only the shadow asks it.
    if crate::lower::session::caches(tcx).intervals_shadow.is_none() {
        return;
    }
    let listed: Vec<(rustc_span::def_id::DefId, &Body<'tcx>)> = bodies.iter().map(|(&id, &body)| (id, body)).collect();
    let graph = crate::lower::call_graph::call_graph(tcx, &listed);
    let only_called = crate::lower::analysis::only_called::only_called(tcx);
    let given = parameter_intervals(tcx, &graph, bodies, &only_called);
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
    // Of a body with no bounds check and no `+`, `-` or `*`, nothing to
    // prove, its walk left out; of every body, until MIR decides a check
    // (#1672 M3), where the session records no shadow.
    let asks = caches.intervals_shadow.is_some()
        && (body.basic_blocks.iter()).any(|data| {
        matches!(&data.terminator().kind, TerminatorKind::Assert { msg, .. } if matches!(**msg, AssertKind::BoundsCheck { .. }))
            || (data.statements.iter()).any(|statement| {
                matches!(&statement.kind, StatementKind::Assign(assign)
                    if matches!(assign.1, Rvalue::BinaryOp(BinOp::Add | BinOp::Sub | BinOp::Mul, _)))
            })
    });
    let proofs = match asks {
        true => proofs(tcx, typing_env, body, given),
        false => Proofs::default(),
    };
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
    caches.intervals.borrow_mut().bodies.push((proofs, exact));
    ProofsScope { tcx }
}

/// Holds a body's proofs while it's lowered.
pub(in crate::lower) struct ProofsScope<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl Drop for ProofsScope<'_> {
    fn drop(&mut self) {
        let caches = crate::lower::session::caches(self.tcx);
        caches.intervals.borrow_mut().bodies.pop();
    }
}

/// What THIR's facts leave out: a bounds check, by the block its `assert`
/// ends, or a wrap.
pub(in crate::lower) enum LeftOut {
    Bounds(BasicBlock),
    Wrap,
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
