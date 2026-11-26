//! Whether a chain of iterator calls is transparent (review #1671's change
//! 2, #1673): every link a std function, on an array, a `Vec`, a slice, a
//! string, a range, or a binding's boxed iterator, JS's own (#1681's step
//! ii); no `Iterator` impl of the crate's reachable; no item
//! nor closure capture with a destructor; and nothing escapes, the chain
//! consumed where it's made, or a `for` loop's. Its answers are counted
//! (`transparent-chain`, `RUST_JS_DECISIONS`), and `in_place::decide`
//! reads them: a `held` chain's adapters are std's compiled structs
//! (#1681's step iii).

use crate::body_queries::strip;
use crate::recognition::{Recognition, StdItem, std_item};
use rustc_middle::thir::{ExprId, ExprKind, Thir};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::DesugaringKind;
use rustc_span::def_id::DefId;

/// What keeps a chain from being transparent, the first of these, or
/// nothing, `Admitted`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Transparency {
    /// An `Iterator` written in Rust, the crate's or a library's
    /// (`Recognition::is_user_iterator`), among its links' generic
    /// arguments.
    UserIterator,
    /// It starts on an iterator held apart, a variable's, a field's, a
    /// parameter's, or a boxed one Rust made.
    Held,
    /// It starts on another source than an array, a `Vec`, a slice, a
    /// string, a range or a binding's boxed iterator.
    Source,
    /// Its iterator escapes where it's made.
    Escapes,
    /// An iterator method no table proves (`PROVED_LINKS`).
    Unproved,
    /// An adapter's argument evaluated as it's given, whose order matters.
    Order,
    /// A `Vec` collected in a crate that reads capacities.
    Capacity,
    Admitted,
}

impl Transparency {
    /// Its answer's name in `transparent-chain`'s counts.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Transparency::UserIterator => "user-iterator",
            Transparency::Held => "held",
            Transparency::Source => "source",
            Transparency::Escapes => "escapes",
            Transparency::Unproved => "unproved",
            Transparency::Order => "order",
            Transparency::Capacity => "capacity",
            Transparency::Admitted => "admitted",
        }
    }
}

/// A chain of iterator calls: each link's call, its outermost first, and
/// what keeps it from being transparent.
pub(crate) struct Chain {
    pub(crate) links: Vec<ExprId>,
    /// What it starts on, its first link's first argument.
    pub(crate) source: Option<ExprId>,
    pub(crate) answer: Transparency,
}

/// Counts each chain of `thir`, the body `owner`'s, by its answer
/// (`chains`), where decisions are asked.
pub(crate) fn count_chains<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    body: (&Thir<'tcx>, DefId),
    keeps_capacity: bool,
) {
    for chain in chains(recognition, body, keeps_capacity) {
        rust_js_program::decisions::record("transparent-chain", chain.answer.name());
    }
}

/// Each chain of `thir`, the body `owner`'s, and what keeps it from being
/// transparent, the first of: an `Iterator` written in Rust, the crate's
/// or a library's; an iterator held apart it starts on; another source
/// than an array, a `Vec`, a slice, a string, a range or a binding's boxed
/// iterator; an iterator that escapes; an iterator method no table proves (`PROVED_LINKS`); an adapter's
/// argument evaluated as it's given, whose order matters; a `Vec`
/// collected, in a crate that reads capacities, `keeps_capacity`, by an
/// iterator whose hand-written hint isn't std's lower bound, the capacity
/// owner's answer; or admitted.
pub(crate) fn chains<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    (thir, owner): (&Thir<'tcx>, DefId),
    keeps_capacity: bool,
) -> Vec<Chain> {
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
    // What a std collection's `extend` is given, consumed there by std's own
    // consumer, as known as `collect()`: a `Vec`'s, a `VecDeque`'s, a
    // `String`'s or a `HashMap`'s, each `$extend`, whose hand-written form
    // the tables' extend rows prove (#1681's change 2). Not escaping; a
    // `Vec`'s grown by the hint, as a collect's.
    let extended: Vec<(ExprId, bool)> = (thir.exprs.indices())
        .filter_map(|e| {
            let ExprKind::Call { fun, ref args, .. } = thir[e].kind else {
                return None;
            };
            let ty::FnDef(callee, callee_args) = *thir[fun].ty.kind() else {
                return None;
            };
            let extend = tcx
                .trait_of_assoc(callee)
                .is_some_and(|tr| crate::recognition::is_extend(tcx, tr));
            if !extend || !recognition.is_std(callee) {
                return None;
            }
            // `Extend::extend`'s `Self`, the collection extended.
            let target = callee_args.skip_binder().type_at(0).peel_refs();
            let vec = recognition.is_std_type(target, StdItem::Vec);
            let proved = vec
                || recognition.is_std_type(target, StdItem::VecDeque)
                || recognition.is_std_type(target, StdItem::HashMap)
                || recognition.is_lang_adt(target, rustc_hir::attrs::lang_items::LangItem::String);
            proved
                .then(|| args.get(1).map(|&given_| (given(thir, given_), vec)))
                .flatten()
        })
        .collect();
    let extended_by = |root: ExprId| extended.iter().find(|(given, _)| *given == root).map(|&(_, vec)| vec);
    let mut found = Vec::new();
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
        let crate_iterator = |ty: Ty<'tcx>| user_iterator_in(&recognition, ty);
        let answer = if chain
            .iter()
            .any(|&e| link(e).is_some_and(|(_, args, _)| args.types().any(crate_iterator)))
        {
            Transparency::UserIterator
        } else if source.is_some_and(|s| is_iterator(thir[s].ty) && !plain(s)) {
            // It starts on an iterator held apart, a variable's, a field's
            // or a parameter's, std's struct; not a range, which is one.
            Transparency::Held
        } else if !source.is_none_or(plain) {
            Transparency::Source
        } else if is_iterator(thir[root].ty)
            && !thir[root].span.is_desugaring(DesugaringKind::ForLoop)
            && extended_by(root).is_none()
        {
            Transparency::Escapes
        } else if chain
            .iter()
            .filter_map(|&e| link(e))
            .any(|(callee, ..)| iterator_method(callee) && !crate::recognition::is_proved_link(tcx, callee))
        {
            // An iterator method whose hand-written form no table proves.
            Transparency::Unproved
        } else if chain.iter().any(|&e| !arguments_inert(thir, e)) {
            // An adapter's argument run as it's given, `chain(make())`,
            // whose order a hand-written form may change (#1015).
            Transparency::Order
        } else if keeps_capacity
            && (recognition.is_std_type(thir[root].ty, StdItem::Vec) || extended_by(root) == Some(true))
            && !(match extended_by(root) {
                // A `Vec` extended by the chain, grown by its hint as a
                // collect is.
                Some(true) => Some(root),
                _ => link(root).and_then(|(.., collected)| collected),
            })
            .is_some_and(|collected| {
                let collected = thir[collected].ty;
                recognition.grows_by_exact_len(collected)
                    || recognition.lower_bound_is_stds_where(collected, false)
                    || recognition.lower_bound_kept_by_parts_where(collected, false)
            })
        {
            // A `Vec` collected in a crate that reads capacities, which
            // std's hint makes (#1635), by an iterator whose hand-written
            // form's hint isn't std's lower bound, as the capacity owner
            // finds it (`lower_bound_is_stds`, #1180): not one collected
            // knowing its exact length (`grows_by_exact_len`, #1200).
            Transparency::Capacity
        } else {
            Transparency::Admitted
        };
        found.push(Chain {
            links: chain,
            source,
            answer,
        });
    }
    found
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
        || recognition.helper_stepped(ty).is_some()
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
    boxed_dyn && crate::bindings::is_binding(tcx, callee)
}

/// Whether `ty` holds an `Iterator` written in Rust, whose `next` the
/// chain may run: the crate's, or a library's, whose own steps std's
/// adapters reach as they reach the crate's (#2067), as its owner,
/// `Recognition::is_user_iterator`, answers.
fn user_iterator_in<'tcx>(recognition: &Recognition<'_, 'tcx>, ty: Ty<'tcx>) -> bool {
    (ty.walk().filter_map(|part| part.as_type())).any(|part| recognition.is_user_iterator(part))
}
