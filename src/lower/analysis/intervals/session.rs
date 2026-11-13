//! What a session keeps of the MIR intervals (ADR 1137): the proofs of
//! each body being lowered, what its functions' callers give them, and,
//! `--intervals-shadow`'s, each check compared with THIR's ranges.

use super::*;

/// What `--intervals-shadow` records: each line, and the proofs of each
/// body being lowered, the innermost last.
#[derive(Default)]
pub(crate) struct Shadow {
    lines: IndexSet<String>,
    bodies: Vec<(Proofs, IndexMap<Span, bool>)>,
    /// What each function only calls reach is given (`parameter_intervals`).
    given: IndexMap<rustc_span::def_id::DefId, Vec<Option<Interval>>>,
}

/// The crate's caller-given parameters, worked out where the session
/// records the shadow, of `bodies`, each a function's MIR the emitter
/// reads, by the call graph of them (#1672 M2b-2).
pub(in crate::lower) fn shadow_parameters<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &IndexMap<rustc_span::def_id::DefId, &Body<'tcx>>,
) {
    let caches = crate::lower::session::caches(tcx);
    let Some(shadow) = &caches.intervals_shadow else {
        return;
    };
    let listed: Vec<(rustc_span::def_id::DefId, &Body<'tcx>)> = bodies.iter().map(|(&id, &body)| (id, body)).collect();
    let graph = crate::lower::call_graph::call_graph(tcx, &listed);
    let only_called = crate::lower::analysis::only_called::only_called(tcx);
    shadow.borrow_mut().given = parameter_intervals(tcx, &graph, bodies, &only_called);
}

/// A body's lowering begun, of a session that records the shadow: its
/// proofs, and of each span its operators are written at whether each is
/// proved, until the guard it gives drops.
pub(in crate::lower) fn shadow_body<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &Body<'tcx>,
) -> Option<ShadowScope<'tcx>> {
    let caches = crate::lower::session::caches(tcx);
    let shadow = caches.intervals_shadow.as_ref()?;
    let given = shadow
        .borrow()
        .given
        .get(&body.source.def_id())
        .cloned()
        .unwrap_or_default();
    let proofs = proofs(tcx, typing_env, body, given);
    let mut exact: IndexMap<Span, bool> = IndexMap::default();
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
    shadow.borrow_mut().bodies.push((proofs, exact));
    Some(ShadowScope { tcx })
}

/// Holds a body's proofs in the shadow while it's lowered.
pub(in crate::lower) struct ShadowScope<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl Drop for ShadowScope<'_> {
    fn drop(&mut self) {
        if let Some(shadow) = &crate::lower::session::caches(self.tcx).intervals_shadow {
            shadow.borrow_mut().bodies.pop();
        }
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
    let mut shadow = shadow.borrow_mut();
    let Some((proofs, exact)) = shadow.bodies.last() else {
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
    shadow.lines.insert(line);
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
