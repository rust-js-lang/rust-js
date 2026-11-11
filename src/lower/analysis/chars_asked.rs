//! Whether the crate asks a string's `Chars` more than its steps from its
//! front, or its back, which the lowering asks of each value (#1186): its
//! rest, `as_str()`, its hint, its `Debug`, a `dyn` of it, whose methods
//! no JS shows, or what code it doesn't see asks. A `Chars` is JS's own
//! string iterator where only its front is stepped, and the runtime's
//! `$chars(s)`, which knows its back, its rest and its hint, where more is
//! asked; a crate that asks this of any makes every one so. A body it
//! compiles from another crate's MIR, which has no THIR, is asked of its
//! MIR (#1325).

use crate::lower::Body;
use crate::lower::recognition::{Recognition, reaches_str_chars};
use rustc_middle::mir::{self, CastKind, Rvalue, TerminatorKind};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{self, TyCtxt};

/// Whether `all_bodies` ask a `Chars` more than its front's steps: a call
/// that does (`Recognition::asks_chars_whole`), or a `dyn` made of what
/// holds one; of a crate that keeps capacities, a `Vec` grown by what holds
/// one, whose hint std grows it by (`vec_grown_by`, #1316). Of a `library`, too, where a function another crate may call
/// gives or takes what holds one, which that crate may ask anything.
pub(super) fn asks_chars_whole<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: Recognition<'_, 'tcx>,
    all_bodies: &[&Body<'tcx>],
    (library, keeps_capacity): (bool, bool),
) -> bool {
    all_bodies.iter().any(|body| {
        let thir = &body.thir;
        let crosses = library && {
            let id = body.def_id;
            tcx.typeck_root_def_id(id) == id
                && matches!(
                    tcx.def_kind(id),
                    rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
                )
                && crate::lower::library::reachable(tcx, id)
                && {
                    let sig = tcx.instantiate_bound_regions_with_erased(
                        tcx.fn_sig(id).instantiate_identity().skip_normalization(),
                    );
                    let mut seen = Vec::new();
                    sig.inputs_and_output
                        .iter()
                        .any(|ty| reaches_str_chars(recognition, ty, &mut seen))
                }
        };
        let mir_asks = !body.def_id.is_local() && body.mir.as_ref().is_some_and(|mir| mir_asks(recognition, &mir.body));
        crosses
            || mir_asks
            || thir.exprs.iter().any(|expr| match expr.kind {
                ExprKind::PointerCoercion {
                    cast: ty::adjustment::PointerCoercion::Unsize,
                    source,
                    ..
                } => recognition.holds_str_chars(thir[source].ty),
                ExprKind::ZstLiteral { .. } => crate::lower::fn_def(expr.ty).is_some_and(|(callee, args)| {
                    recognition.asks_chars_whole(callee, args)
                        || keeps_capacity
                            && (recognition.vec_grown_by(callee, args, ty::TypingEnv::post_analysis(tcx, body.def_id)))
                                .is_some_and(|grown| recognition.holds_str_chars(grown))
                }),
                _ => false,
            })
    })
}

/// Whether `body`, another crate's MIR the crate compiles, asks a `Chars`
/// more than its front's steps, as `asks_chars_whole` asks a THIR: a call
/// that does, or a `dyn` made of what holds one.
fn mir_asks<'tcx>(recognition: Recognition<'_, 'tcx>, body: &mir::Body<'tcx>) -> bool {
    body.basic_blocks.iter().any(|data| {
        let calls = matches!(&data.terminator().kind, TerminatorKind::Call { func, .. }
            if func.const_fn_def().is_some_and(|(callee, args)| recognition.asks_chars_whole(callee, args)));
        calls
            || data.statements.iter().any(|statement| {
                matches!(statement.kind.as_assign(), Some((_, Rvalue::Cast(
                    CastKind::PointerCoercion(ty::adjustment::PointerCoercion::Unsize, _),
                    operand,
                    _,
                ))) if recognition.holds_str_chars(operand.ty(body, recognition.tcx)))
            })
    })
}
