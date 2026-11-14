//! Whether a chain of iterator calls is transparent (review #1671's change
//! 2, #1673): every link a std function, on an array, a `Vec`, a slice, a
//! string, a range, or a binding's boxed iterator, JS's own (#1681's step
//! ii); no `Iterator` impl of the crate's reachable; no item
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
/// destructor; an iterator method no table proves (`PROVED_LINKS`); an
/// adapter's argument evaluated as it's given, whose order
/// matters; a `Vec` collected where its capacity is read; or admitted.
pub(in crate::lower) fn count_chains<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    (thir, owner): (&Thir<'tcx>, DefId),
    keeps_capacity: bool,
) {
    let tcx = recognition.tcx;
    let recognition = Recognition {
        typing_env: ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(owner)),
        ..*recognition
    };
    let iterator = std_item(tcx, StdItem::Iterator);
    let is_iterator = |ty: Ty<'tcx>| recognition.iterator_item(ty.peel_refs()).is_some();
    // A method of `Iterator`, or of a trait over it, `DoubleEndedIterator`'s;
    // `IntoIterator::into_iter` makes an iterator, as a source's maker does.
    let iterator_method = |callee: DefId| {
        (tcx.trait_of_assoc(callee))
            .is_some_and(|tr| rustc_middle::ty::elaborate::supertrait_def_ids(tcx, tr).any(|t| t == iterator))
    };
    // A link: an iterator method's call, or a std function's that makes an
    // iterator of what isn't one, a source's, `v.iter()`; not one that
    // makes a range, the source itself, nor what's given an iterator to
    // read, `{:?}`'s.
    let link = |e: ExprId| -> Option<(DefId, ty::GenericArgsRef<'tcx>, Option<ExprId>)> {
        let ExprKind::Call { fun, ref args, .. } = thir[e].kind else {
            return None;
        };
        let ty::FnDef(callee, callee_args) = *thir[fun].ty.kind() else {
            return None;
        };
        let first = args.first().map(|&arg| given(thir, arg));
        let makes = is_iterator(thir[e].ty) && !is_plain_source(&recognition, thir[e].ty.peel_refs());
        (recognition.is_std(callee) && (iterator_method(callee) || makes)).then_some((
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
    // What a chain may start on: a plain source, or a binding's boxed
    // iterator, JS's own, `keys()`'s, which runs no Rust as it steps. Not
    // one Rust makes, `Box::new(..)`, whose `dyn` hides what it boxes,
    // which may be an `Iterator` of the crate's, `Box::new(Counter(0))`.
    let plain = |s: ExprId| is_plain_source(&recognition, thir[s].ty.peel_refs()) || binding_boxed(tcx, thir, s);
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
        } else if source.is_some_and(|s| is_iterator(thir[s].ty) && !plain(s)) {
            // It starts on an iterator held apart, a variable's, a field's
            // or a parameter's, std's struct; not a range, which is one.
            "held"
        } else if !source.is_none_or(plain) {
            "source"
        } else if is_iterator(thir[root].ty) && !thir[root].span.is_desugaring(DesugaringKind::ForLoop) {
            "escapes"
        } else if chain.iter().any(|&e| drops_in(tcx, &recognition, thir[e].ty, link(e))) {
            "drop-glue"
        } else if chain
            .iter()
            .filter_map(|&e| link(e))
            .any(|(callee, ..)| iterator_method(callee) && !crate::lower::recognition::is_proved_link(tcx, callee))
        {
            // An iterator method whose hand-written form no table proves.
            "unproved"
        } else if chain.iter().any(|&e| !arguments_inert(thir, e)) {
            // An adapter's argument run as it's given, `chain(make())`,
            // whose order a hand-written form may change (#1015).
            "order"
        } else if keeps_capacity && recognition.is_std_type(thir[root].ty, StdItem::Vec) {
            // A `Vec` collected in a crate that reads capacities, which
            // std's hint makes (#1635).
            "capacity"
        } else {
            "admitted"
        };
        crate::decisions::record("transparent-chain", answer);
    }
}

/// Whether each argument of the call `e` but its first is inert: a
/// closure, a function, a constant or a place, which evaluating runs
/// nothing.
fn arguments_inert(thir: &Thir<'_>, e: ExprId) -> bool {
    let ExprKind::Call { ref args, .. } = thir[e].kind else {
        return true;
    };
    args.iter().skip(1).all(|&arg| inert(thir, arg))
}

/// Whether evaluating `e` runs nothing: a closure, a function item, a
/// literal or a constant, a variable, a place of one, or a value made of
/// them, a range's `..`.
fn inert(thir: &Thir<'_>, e: ExprId) -> bool {
    match thir[strip(thir, e)].kind {
        ExprKind::Closure(_)
        | ExprKind::ZstLiteral { .. }
        | ExprKind::Literal { .. }
        | ExprKind::NonHirLiteral { .. }
        | ExprKind::NamedConst { .. }
        | ExprKind::ConstParam { .. }
        | ExprKind::VarRef { .. }
        | ExprKind::UpvarRef { .. } => true,
        ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } | ExprKind::Field { lhs: arg, .. } => inert(thir, arg),
        // A value made of inert ones, a range's `..` or `a..b`.
        ExprKind::Adt(ref adt) => {
            adt.fields.iter().all(|field| inert(thir, field.expr))
                && matches!(adt.base, rustc_middle::thir::AdtExprBase::None)
        }
        _ => false,
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

/// Whether `e` is a binding's call that gives a boxed `dyn Iterator`, JS's
/// own iterator, `entries()`'s (#1681's step ii).
fn binding_boxed<'tcx>(tcx: TyCtxt<'tcx>, thir: &Thir<'tcx>, e: ExprId) -> bool {
    let e = strip(thir, e);
    let ExprKind::Call { fun, .. } = thir[e].kind else {
        return false;
    };
    let ty::FnDef(callee, _) = *thir[fun].ty.kind() else {
        return false;
    };
    let boxed_dyn = thir[e].ty.boxed_ty().is_some_and(|inner| inner.is_trait());
    boxed_dyn && crate::lower::bindings::is_binding(tcx, callee)
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
