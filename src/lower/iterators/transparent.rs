//! Whether a chain of iterator calls is transparent (review #1671's change
//! 2, #1673): every link a std function, on an array, a `Vec`, a slice, a
//! string or a range; no `Iterator` impl of the crate's reachable; no item
//! nor closure capture with a destructor; and nothing escapes, the chain
//! consumed where it's made, or a `for` loop's. A read-only query: its
//! answers are counted (`transparent-chain`, `RUST_JS_DECISIONS`), and
//! nothing reads them yet.

use crate::lower::body_queries::strip;
use crate::lower::recognition::{Recognition, StdItem, std_item};
use rustc_middle::thir::{ExprId, ExprKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::DesugaringKind;
use rustc_span::def_id::DefId;

/// Counts each chain of `thir`, the body `owner`'s, by what keeps it from
/// being transparent, the first of: an `Iterator` of the crate's; an
/// iterator held apart it starts on; another source than an array, a
/// `Vec`, a slice, a string or a range; an iterator that escapes; a
/// destructor; or admitted.
pub(in crate::lower) fn count_chains<'tcx>(recognition: &Recognition<'_, 'tcx>, thir: &Thir<'tcx>, owner: DefId) {
    let tcx = recognition.tcx;
    let recognition = Recognition {
        typing_env: ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(owner)),
        ..*recognition
    };
    let iterator = std_item(tcx, StdItem::Iterator);
    let is_iterator = |ty: Ty<'tcx>| recognition.iterator_item(ty.peel_refs()).is_some();
    // A link: a std function's call that makes an iterator or steps one.
    let link = |e: ExprId| -> Option<(DefId, ty::GenericArgsRef<'tcx>, Option<ExprId>)> {
        let ExprKind::Call { fun, ref args, .. } = thir[e].kind else {
            return None;
        };
        let ty::FnDef(callee, callee_args) = *thir[fun].ty.kind() else {
            return None;
        };
        let first = args.first().map(|&arg| given(thir, arg));
        let steps = first.is_some_and(|arg| is_iterator(thir[arg].ty));
        (recognition.is_std(callee) && (is_iterator(thir[e].ty) || steps)).then_some((
            callee,
            callee_args.skip_binder(),
            first,
        ))
    };
    let links: Vec<ExprId> = thir.exprs.indices().filter(|&e| link(e).is_some()).collect();
    let inner: Vec<ExprId> = links
        .iter()
        .filter_map(|&e| link(e)?.2)
        .filter(|&arg| link(arg).is_some())
        .collect();
    // Not a `for` loop's own `next`, which steps what its `into_iter`
    // made, that chain's.
    let loops_next = |e: ExprId| thir[e].span.is_desugaring(DesugaringKind::ForLoop) && !is_iterator(thir[e].ty);
    for &root in links.iter().filter(|&&e| !inner.contains(&e) && !loops_next(e)) {
        let mut chain = vec![root];
        let mut source = None;
        let mut at = root;
        while let Some((_, _, first)) = link(at) {
            match first {
                Some(arg) if link(arg).is_some() => {
                    chain.push(arg);
                    at = arg;
                }
                arg => {
                    source = arg;
                    break;
                }
            }
        }
        // Of its links' generic arguments, what it starts on its first's.
        let crate_iterator = |ty: Ty<'tcx>| crate_iterator_in(&recognition, iterator, ty);
        let answer = if chain
            .iter()
            .any(|&e| link(e).is_some_and(|(_, args, _)| args.types().any(crate_iterator)))
        {
            "user-iterator"
        } else if source
            .is_some_and(|s| is_iterator(thir[s].ty) && !is_plain_source(&recognition, thir[s].ty.peel_refs()))
        {
            // It starts on an iterator held apart, a variable's, a field's
            // or a parameter's, std's struct; not a range, which is one.
            "held"
        } else if !source.is_none_or(|s| is_plain_source(&recognition, thir[s].ty.peel_refs())) {
            "source"
        } else if is_iterator(thir[root].ty) && !thir[root].span.is_desugaring(DesugaringKind::ForLoop) {
            "escapes"
        } else if chain.iter().any(|&e| drops_in(tcx, &recognition, thir[e].ty, link(e))) {
            "drop-glue"
        } else {
            "admitted"
        };
        crate::decisions::record("transparent-chain", answer);
    }
}

/// What `arg` gives, through a borrow for it, `rev().nth(n)`'s `&mut`.
fn given<'tcx>(thir: &Thir<'tcx>, arg: ExprId) -> ExprId {
    match thir[strip(thir, arg)].kind {
        ExprKind::Borrow { arg, .. } => strip(thir, arg),
        _ => strip(thir, arg),
    }
}

/// An array, a `Vec`, a slice, a string or a range: what JS holds as an
/// array, a string or a range of its own.
fn is_plain_source<'tcx>(recognition: &Recognition<'_, 'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Array(..) | ty::Slice(_))
        || recognition.is_std_type(ty, StdItem::Vec)
        || recognition.is_string_like(ty)
        || recognition.range_kind(ty).is_some()
}

/// Whether `ty` holds a type of the crate's that's an `Iterator`, whose
/// `next` the chain may run.
fn crate_iterator_in<'tcx>(recognition: &Recognition<'_, 'tcx>, iterator: DefId, ty: Ty<'tcx>) -> bool {
    ty.walk().filter_map(|part| part.as_type()).any(|part| {
        matches!(part.kind(), ty::Adt(adt, _) if adt.did().is_local()) && recognition.implements_trait(part, iterator)
    })
}

/// Whether a link's item, or a capture of a closure among its generic
/// arguments, has a destructor.
fn drops_in<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: &Recognition<'_, 'tcx>,
    made: Ty<'tcx>,
    link: Option<(DefId, ty::GenericArgsRef<'tcx>, Option<ExprId>)>,
) -> bool {
    let needs_drop = |ty: Ty<'tcx>| ty.needs_drop(tcx, recognition.typing_env);
    let item = recognition.iterator_item(made.peel_refs()).is_some_and(needs_drop);
    let captures = link.is_some_and(|(_, args, _)| {
        args.types()
            .flat_map(|ty| ty.walk().filter_map(|part| part.as_type()))
            .any(|part| match part.kind() {
                ty::Closure(_, closure_args) => closure_args.as_closure().upvar_tys().iter().any(needs_drop),
                _ => false,
            })
    });
    item || captures
}
