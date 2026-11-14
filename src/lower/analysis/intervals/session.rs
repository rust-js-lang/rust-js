//! What a session keeps of the MIR intervals (ADR 1137): the proofs of
//! each body being lowered, what its functions' callers give them, and,
//! `--intervals-shadow`'s, each check compared with THIR's ranges.

use super::*;
use crate::lower::recognition::once_method;
use crate::lower::std_types::once::OnceOp;
use rustc_hir::def::DefKind;

/// What MIR proves of the session's bodies: what each function only calls
/// reach is given (`parameter_intervals`), and the proofs of each body
/// being lowered, the innermost last, with, where the session records the
/// shadow, of each span its operators are written at whether each is
/// proved.
#[derive(Default)]
pub(crate) struct Intervals {
    given: IndexMap<rustc_span::def_id::DefId, Given>,
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

/// The crate's caller-given parameters, of `bodies`, each a function's
/// MIR the emitter reads, by the call graph of them (#1672 M2b-2); and of
/// each constant's, a `const`'s, a `static`'s or an inline `const { .. }`'s,
/// the MIR rustc evaluates it by, its calls a function's callers too: a
/// `const fn` only constants and patterns call, `add(1, 2)` of `const SUM`,
/// is given what they pass (#1742).
pub(in crate::lower) fn given_parameters<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &IndexMap<rustc_span::def_id::DefId, &Body<'tcx>>,
) {
    let mut callers: IndexMap<rustc_span::def_id::DefId, &Body<'tcx>> = bodies.clone();
    for owner in tcx.hir_body_owners() {
        let owner = owner.to_def_id();
        if !callers.contains_key(&owner)
            && matches!(
                tcx.def_kind(owner),
                DefKind::Const | DefKind::AssocConst | DefKind::AnonConst | DefKind::Static { .. }
            )
        {
            callers.insert(owner, tcx.mir_for_ctfe(owner));
        }
    }
    let listed: Vec<(rustc_span::def_id::DefId, &Body<'tcx>)> = callers.iter().map(|(&id, &body)| (id, body)).collect();
    let graph = crate::lower::call_graph::call_graph(tcx, &listed);
    let only_called = crate::lower::analysis::only_called::only_called(tcx);
    let given = parameter_intervals(tcx, &graph, &callers, &only_called);
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
    // Of a body with no bounds check, no `+`, `-` or `*` and no integer
    // cast, nothing to prove, its walk left out.
    let asks = (body.basic_blocks.iter()).any(|data| {
        matches!(&data.terminator().kind, TerminatorKind::Assert { msg, .. } if matches!(**msg, AssertKind::BoundsCheck { .. }))
            || (data.statements.iter()).any(|statement| {
                matches!(&statement.kind, StatementKind::Assign(assign)
                    if matches!(assign.1, Rvalue::BinaryOp(BinOp::Add | BinOp::Sub | BinOp::Mul, _) | Rvalue::Cast(mir::CastKind::IntToInt, ..)))
            })
    });
    let mut proofs = match asks {
        true => proofs(tcx, typing_env, body, given),
        false => Proofs::default(),
    };
    // A body that makes a raw pointer, or lends an integer local `&mut`:
    // THIR's ranges see neither's write after, a read through the owner
    // leaving the pointer able to (#1782).
    proofs.thir_untrusted = (body.basic_blocks.iter()).flat_map(|data| &data.statements).any(|statement| {
        matches!(&statement.kind, StatementKind::Assign(assign) if match &assign.1 {
            Rvalue::RawPtr(..) => true,
            Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of) => of.as_local().is_some_and(|l| body.local_decls[l].ty.is_integral()),
            _ => false,
        })
    });
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

/// A wrap the bridge leaves out (ADR 1137): one THIR's ranges know never
/// wraps that MIR's intervals don't prove yet, a `bridged` line of the
/// shadow, test/intervals-bridged.txt's list.
pub(in crate::lower) fn shadow_bridged(tcx: TyCtxt<'_>, owner: rustc_span::def_id::DefId, span: Span) {
    let caches = crate::lower::session::caches(tcx);
    let Some(shadow) = &caches.intervals_shadow else {
        return;
    };
    let at = tcx.sess.source_map().span_to_diagnostic_string(span);
    let line = format!("bridged\t{}\t{at}\n", tcx.def_path_str(owner));
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

/// Whether THIR's ranges may be taken for a wrap MIR doesn't prove, of the
/// body being lowered (the bridge, ADR 1137): not of one that makes a raw
/// pointer or lends an integer local `&mut` (#1782).
pub(in crate::lower) fn thir_trusted(tcx: TyCtxt<'_>) -> bool {
    let caches = crate::lower::session::caches(tcx);
    let intervals = caches.intervals.borrow();
    (intervals.bodies.last()).is_none_or(|(proofs, _)| !proofs.thir_untrusted)
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
