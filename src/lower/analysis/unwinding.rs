//! Which of the crate's functions and closures never unwind (ADR 0902):
//! each whose MIR has no way to panic, read once from it, so a call of one
//! throws only where what it's given does (`Known::NeverThrows`), a drop
//! after it needs no `finally`, and an iterator whose closure stage it is
//! steps without a panic (`steps_may_panic`, #701).

use crate::collections::{IndexMap, IndexSet};
use crate::lower::Body;
use crate::lower::bindings::is_binding;
use crate::lower::recognition::{Recognition, is_std_crate};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::mir::{self, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use rustc_span::def_id::DefId;

mod eager_futures;

/// What `found` finds of a crate's functions and closures.
pub(super) struct Found {
    /// `never_unwinding`'s.
    pub(super) never_unwinding: IndexSet<DefId>,
    /// `quiet`'s.
    pub(super) quiet: IndexSet<DefId>,
    /// Those that never unwind but through what their formatting shows
    /// (`Unwinding::ButThroughWhatsShown`, #1496, #1530).
    pub(super) never_but_through_whats_shown: IndexSet<DefId>,
}

/// `never_unwinding`'s, of both kinds, and `quiet`'s; and, of what's quiet,
/// each future whose running as it's made would be seen, refused (#965).
pub(super) fn found<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: &Recognition<'_, 'tcx>,
    bodies: &[&Body<'tcx>],
    runs: &dyn Fn(Ty<'tcx>, DefId) -> bool,
) -> Found {
    let quiet = quiet(tcx, recognition, bodies, runs);
    eager_futures::refuse_where_eagerness_shows(tcx, recognition, bodies, &quiet, runs);
    Found {
        never_unwinding: never_unwinding(tcx, recognition, bodies, runs, Unwinding::AtAll),
        quiet,
        never_but_through_whats_shown: never_unwinding(tcx, recognition, bodies, runs, Unwinding::ButThroughWhatsShown),
    }
}

/// Which unwinding `never_unwinding` asks after.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Unwinding {
    /// Any: what never unwinds, whatever it's given.
    AtAll,
    /// Any but through what its formatting shows (#1496, #1530): std's
    /// formatting, a `Formatter`'s writes and builders, an `Arguments` of
    /// any value, a `Debug` or `Display` call of one, is taken to go on, as
    /// it unwinds only through what it runs of the crate's, a shown value's
    /// writer, an iterator a builder's `entries` steps, a crate's
    /// `fmt::Write`, which `prints` follows by what may run and by the
    /// shown value's type, or through the `Formatter`'s destination, which
    /// is stdout's line where it asks.
    ButThroughWhatsShown,
}

/// The crate's functions and closures that never unwind, as `unwinding`
/// asks: of no type parameters, no binding nor trait's method a dictionary
/// may call another impl of, each whose every way on can't panic
/// (`steps_on`), each call of another of these or of std's that can't.
/// Found from the most, each that may taken out until none is.
fn never_unwinding<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: &Recognition<'_, 'tcx>,
    bodies: &[&Body<'tcx>],
    runs: &dyn Fn(Ty<'tcx>, DefId) -> bool,
    unwinding: Unwinding,
) -> IndexSet<DefId> {
    // Each one's `Drop`s that drop nothing, of what JS drops nothing of or
    // of what's moved out by then (`mir::dead_drops`, #717).
    let candidates: Vec<(DefId, &mir::Body<'tcx>, IndexSet<mir::BasicBlock>)> = (bodies.iter())
        .filter(|body| candidate(tcx, body.def_id))
        .filter_map(|body| {
            let mir = &body.mir.as_ref()?.body;
            let mut dead = crate::lower::mir::dead_drops(tcx, mir, &|ty| runs(ty, body.def_id));
            // A closure of a generic function's: a drop of what's of its
            // type parameters may run anything, so none is dead.
            dead.retain(|&block| {
                !matches!(&mir.basic_blocks[block].terminator().kind,
                    TerminatorKind::Drop { place, .. } if place.ty(mir, tcx).ty.has_param())
            });
            Some((body.def_id, mir, dead))
        })
        .collect();
    let mut never: IndexSet<DefId> = candidates.iter().map(|&(id, ..)| id).collect();
    loop {
        let unwinding: Vec<DefId> = (candidates.iter())
            .filter(|(id, mir, dead)| {
                // Of formatting's, an exhaustive `match`'s impossible branch,
                // a derived `Debug`'s of an enum, never runs.
                let unreachable = unwinding == Unwinding::ButThroughWhatsShown;
                never.contains(id)
                    && !steps_on((mir, dead, unreachable), &mut |callee, args, _| {
                        never.contains(&callee)
                            || std_never_unwinds((tcx, recognition), (callee, args), &|ty| runs(ty, *id))
                            || (unwinding == Unwinding::ButThroughWhatsShown && formats(tcx, callee))
                    })
            })
            .map(|&(id, ..)| id)
            .collect();
        if unwinding.is_empty() {
            return never;
        }
        for id in unwinding {
            never.swap_remove(&id);
        }
    }
}

/// The crate's functions and closures whose calls can't be seen (#925): of
/// those `never_unwinding` may hold, each whose every way on can't panic
/// (`steps_on`), that writes nothing but its own locals, no place through a
/// reference, a captured `&mut`'s or a `static mut`'s, and whose every call
/// is of another of these or of std's that `std_quiet` says can't be seen:
/// no print, no `Cell`'s set. Found from the most, each that isn't taken out
/// until none is. So a comparator that only compares, `|a, b|
/// a.name.cmp(&b.name)`, is one, and a sort by it is JS's own (ADR 0036).
pub(super) fn quiet<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: &Recognition<'_, 'tcx>,
    bodies: &[&Body<'tcx>],
    runs: &dyn Fn(Ty<'tcx>, DefId) -> bool,
) -> IndexSet<DefId> {
    // An async body too, a coroutine's, whose future may run as it's made
    // without that being seen where it's quiet (#965): one that awaits
    // isn't, as its `yield` doesn't step on.
    let candidates: Vec<(DefId, &mir::Body<'tcx>, IndexSet<mir::BasicBlock>)> = (bodies.iter())
        .filter(|body| candidate(tcx, body.def_id) || (body.def_id.is_local() && tcx.is_coroutine(body.def_id)))
        .filter_map(|body| {
            let mir = &body.mir.as_ref()?.body;
            let mut dead = crate::lower::mir::dead_drops(tcx, mir, &|ty| runs(ty, body.def_id));
            dead.retain(|&block| {
                !matches!(&mir.basic_blocks[block].terminator().kind,
                    TerminatorKind::Drop { place, .. } if place.ty(mir, tcx).ty.has_param())
            });
            Some((body.def_id, mir, dead))
        })
        .filter(|(_, mir, _)| writes_only_locals(mir))
        .collect();
    let mut quiet: IndexSet<DefId> = candidates.iter().map(|&(id, ..)| id).collect();
    loop {
        let seen: Vec<DefId> = (candidates.iter())
            .filter(|(id, mir, dead)| {
                let typing_env = ty::TypingEnv::post_analysis(tcx, *id);
                // And a drop of what drops only by `quiet`'s `Drop`s,
                // `impl Drop for Big { fn drop(&mut self) {} }`'s, or of an
                // async body's own state that drops nothing (#965).
                let mut dead = dead.clone();
                dead.extend(quiet_drops(tcx, mir, &quiet, &|ty| runs(ty, *id)));
                quiet.contains(id)
                    && !steps_on((mir, &dead, true), &mut |callee, args, operands| {
                        awaits_quietly((tcx, recognition, typing_env), &quiet, (callee, args)).unwrap_or_else(|| {
                            quiet_call(
                                (tcx, recognition, typing_env),
                                &quiet,
                                (callee, args, operands),
                                &|ty| runs(ty, *id),
                            )
                        })
                    })
            })
            .map(|&(id, ..)| id)
            .collect();
        if seen.is_empty() {
            return quiet;
        }
        for id in seen {
            quiet.swap_remove(&id);
        }
    }
}

/// Each `Drop` of `mir` of what drops only by `Drop`s `quiet` holds, its
/// fields' too, `drops` saying where a type runs code as it's dropped.
fn quiet_drops<'tcx>(
    tcx: TyCtxt<'tcx>,
    mir: &mir::Body<'tcx>,
    quiet: &IndexSet<DefId>,
    drops: &dyn Fn(Ty<'tcx>) -> bool,
) -> Vec<mir::BasicBlock> {
    (mir.basic_blocks.iter_enumerated())
        .filter(|(_, block)| {
            matches!(&block.terminator().kind, TerminatorKind::Drop { place, .. }
                if drops_quietly(tcx, place.ty(mir, tcx).ty, quiet, drops, 0))
        })
        .map(|(at, _)| at)
        .collect()
}

/// Whether a drop of `ty` runs only what `quiet` holds: nothing, or its
/// `Drop`'s, one `quiet` holds, then its fields', each likewise.
fn drops_quietly<'tcx>(
    tcx: TyCtxt<'tcx>,
    ty: Ty<'tcx>,
    quiet: &IndexSet<DefId>,
    drops: &dyn Fn(Ty<'tcx>) -> bool,
    depth: usize,
) -> bool {
    if !drops(ty) {
        return true;
    }
    if depth > 8 || ty.has_param() {
        return false;
    }
    let quietly = |t: Ty<'tcx>| drops_quietly(tcx, t, quiet, drops, depth + 1);
    match ty.kind() {
        ty::Adt(adt, args) if adt.is_box() => quietly(args.type_at(0)),
        ty::Adt(adt, args) => {
            adt.did().is_local()
                && tcx.adt_destructor(adt.did()).is_none_or(|d| quiet.contains(&d.did))
                && adt.all_fields().all(|f| quietly(f.ty(tcx, args).skip_normalization()))
        }
        ty::Tuple(items) => items.iter().all(quietly),
        ty::Array(item, _) => quietly(*item),
        _ => false,
    }
}

/// Whether `mir` writes no place through a reference or a pointer: a
/// captured `&mut`'s, a `static mut`'s, a `&mut` parameter's. Its own locals
/// only, which no caller sees.
fn writes_only_locals(mir: &mir::Body<'_>) -> bool {
    mir.basic_blocks.iter().filter(|block| !block.is_cleanup).all(|block| {
        block.statements.iter().all(writes_a_local) && block.terminator.as_ref().is_none_or(destined_locally)
    })
}

/// Whether `statement` writes only its body's own locals, no place through
/// a reference or a pointer.
pub(super) fn writes_a_local(statement: &mir::Statement<'_>) -> bool {
    match &statement.kind {
        mir::StatementKind::Assign(assign) => !assign.0.is_indirect(),
        mir::StatementKind::SetDiscriminant { place, .. } => !place.is_indirect(),
        _ => true,
    }
}

/// Whether `terminator`, a call's, gives what it gives to its body's own
/// locals.
pub(super) fn destined_locally(terminator: &mir::Terminator<'_>) -> bool {
    match &terminator.kind {
        TerminatorKind::Call { destination, .. } => !destination.is_indirect(),
        _ => true,
    }
}

/// Of a call that awaits, an `.await`'s `into_future`, its loop's `poll` or
/// its `get_context`, whether what it awaits runs nothing a program can
/// see: `future::ready`'s future, or an async body `quiet` holds, pinned,
/// boxed or lent (#965); of another call, none.
pub(super) fn awaits_quietly<'tcx>(
    (tcx, recognition, typing_env): (TyCtxt<'tcx>, &Recognition<'_, 'tcx>, ty::TypingEnv<'tcx>),
    quiet: &IndexSet<DefId>,
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
) -> Option<bool> {
    let awaited = awaited_future((tcx, typing_env), (callee, args))?;
    Some(awaited.is_none_or(|awaited| quiet_future(recognition, quiet, awaited)))
}

/// Whether `future`, a future's type, runs nothing a program can see:
/// `future::ready`'s, or an async body `quiet` holds.
fn quiet_future<'tcx>(recognition: &Recognition<'_, 'tcx>, quiet: &IndexSet<DefId>, future: Ty<'tcx>) -> bool {
    match future.kind() {
        ty::Coroutine(body, _) => quiet.contains(body),
        _ => recognition.is_std_type(future, crate::lower::recognition::StdItem::FutureReady),
    }
}

/// Of a call of an `.await`'s, what it awaits, revealed and taken out of
/// its pin, box or reference: `Some(Some(ty))` of an `into_future` or a
/// `poll`, `Some(None)` of a `get_context`, which reads the context its body
/// was resumed with and awaits nothing; of another call, none.
fn awaited_future<'tcx>(
    (tcx, typing_env): (TyCtxt<'tcx>, ty::TypingEnv<'tcx>),
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
) -> Option<Option<Ty<'tcx>>> {
    if tcx.is_lang_item(callee, LangItem::GetContext) {
        return Some(None);
    }
    if !(tcx.is_lang_item(callee, LangItem::IntoFutureIntoFuture) || tcx.is_lang_item(callee, LangItem::FuturePoll)) {
        return None;
    }
    let awaited = args.types().next()?;
    let mut awaited = tcx
        .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(awaited))
        .unwrap_or(awaited);
    loop {
        match awaited.kind() {
            ty::Ref(_, inner, _) => awaited = *inner,
            ty::Adt(adt, inner) if adt.is_box() || tcx.is_lang_item(adt.did(), LangItem::Pin) => {
                awaited = inner.type_at(0)
            }
            _ => return Some(Some(awaited)),
        }
    }
}

/// The crate's async bodies, `quiet` holds none of, that are quiet but for
/// awaiting a future of a type parameter's and dropping what's of one:
/// `async fn use_future(fut: impl Future) { fut.await }`'s. Its future is
/// quiet where each of those its caller gives is (`eager_futures`, #965).
fn quiet_but_their_parameters<'tcx>(
    tcx: TyCtxt<'tcx>,
    recognition: &Recognition<'_, 'tcx>,
    bodies: &[&Body<'tcx>],
    quiet: &IndexSet<DefId>,
    runs: &dyn Fn(Ty<'tcx>, DefId) -> bool,
) -> IndexSet<DefId> {
    (bodies.iter())
        .filter(|body| body.def_id.is_local() && tcx.is_coroutine(body.def_id) && !quiet.contains(&body.def_id))
        .filter(|body| {
            let Some(mir) = body.mir.as_ref().map(|m| &m.body) else {
                return false;
            };
            let drops = |ty: Ty<'tcx>| !ty.has_param() && runs(ty, body.def_id);
            let mut dead = crate::lower::mir::dead_drops(tcx, mir, &drops);
            // What's of a type parameter, its caller's to say.
            dead.extend((mir.basic_blocks.iter_enumerated()).filter(|(_, block)| {
                matches!(&block.terminator().kind, TerminatorKind::Drop { place, .. } if place.ty(mir, tcx).ty.has_param())
            }).map(|(at, _)| at));
            let typing_env = ty::TypingEnv::post_analysis(tcx, body.def_id);
            writes_only_locals(mir)
                && steps_on((mir, &dead, true), &mut |callee, args, operands| {
                    match awaited_future((tcx, typing_env), (callee, args)) {
                        Some(None) => true,
                        Some(Some(awaited)) => awaited.has_param() || quiet_future(recognition, quiet, awaited),
                        None => quiet_call((tcx, recognition, typing_env), quiet, (callee, args, operands), &|ty| runs(ty, body.def_id)),
                    }
                })
        })
        .map(|body| body.def_id)
        .collect()
}

/// Whether a call of `callee`, given `args` and `operands`, in `typing_env`,
/// does nothing a program can see, `quiet`'s question of each call: one of
/// `quiet`'s functions, or the crate's one it resolves to, or std's that
/// `std_quiet` says can't be seen, `drops` saying what a drop of a type
/// runs. Asked too of what runs before a future is awaited (#965).
pub(super) fn quiet_call<'tcx>(
    (tcx, recognition, typing_env): (TyCtxt<'tcx>, &Recognition<'_, 'tcx>, ty::TypingEnv<'tcx>),
    quiet: &IndexSet<DefId>,
    (callee, args, operands): (
        DefId,
        ty::GenericArgsRef<'tcx>,
        &[rustc_span::Spanned<mir::Operand<'tcx>>],
    ),
    drops: &dyn Fn(Ty<'tcx>) -> bool,
) -> bool {
    if quiet.contains(&callee) {
        return true;
    }
    let resolved = crate::lower::items::resolve_instance_in(tcx, typing_env, callee, args);
    if let Ok(Some(ty::Instance {
        def: ty::InstanceKind::Item(def),
        ..
    })) = resolved
        && def.is_local()
    {
        return quiet.contains(&def);
    }
    std_quiet(
        (tcx, recognition),
        (callee, args, nonzero_divisor(operands)),
        &|imp| !crate::lower::ordering::impl_quiet(tcx, quiet, imp),
        &|f| quiet.contains(&f),
        drops,
    )
}

/// Whether a call's divisor, its second operand, is a constant neither 0 nor
/// -1, so a division or a remainder by it can't panic.
fn nonzero_divisor(operands: &[rustc_span::Spanned<mir::Operand<'_>>]) -> bool {
    let Some(mir::Operand::Constant(divisor)) = operands.get(1).map(|o| &o.node) else {
        return false;
    };
    divisor.const_.try_to_scalar_int().is_some_and(|n| {
        let bits = n.to_bits_unchecked();
        bits != 0 && bits != u128::MAX >> (128 - n.size().bits())
    })
}

/// Whether std's `callee`, given `args`, does nothing a program can see
/// (#925): a comparison, `cmp`, `partial_cmp`, `lt` and the rest, of what
/// `order_may` with `impl_seen` says isn't seen, and its `max`, `min` and
/// `clamp` of what `drops` says drops nothing; `Ordering`'s `then`,
/// `reverse`, `is_lt` and the rest, and its `then_with` of a function
/// `quiet` holds; a number's `total_cmp`, `abs`, `min` and `max`; text's and
/// a collection's `len`, `is_empty` and views, `as_str`, `deref`; an
/// `Option`'s `is_some` and `as_ref`; a clone of std's text; an enum's
/// discriminant, which a derived order reads; a pin or a box made of what
/// it's given, and a pin's views; and JS's `localeCompare`.
/// Nothing else, none that prints, writes a cell or may panic.
pub(in crate::lower) fn std_quiet<'tcx>(
    (tcx, recognition): (TyCtxt<'tcx>, &Recognition<'_, 'tcx>),
    (callee, args, divisor_known): (DefId, ty::GenericArgsRef<'tcx>, bool),
    impl_seen: &dyn Fn(DefId) -> bool,
    quiet: &dyn Fn(DefId) -> bool,
    drops: &dyn Fn(Ty<'tcx>) -> bool,
) -> bool {
    if matches!(tcx.def_kind(callee), DefKind::Ctor(..)) || crate::lower::bindings::is_locale_compare(tcx, callee) {
        return true;
    }
    if !is_std_crate(tcx, callee) {
        return false;
    }
    let name = tcx.item_name(callee);
    let name = name.as_str();
    let self_ty = || args.types().next().map(|ty| ty.peel_refs());
    let of_trait = tcx.trait_of_assoc(callee);
    let is = |tr: Option<DefId>, item: LangItem| tr.is_some_and(|tr| tcx.is_lang_item(tr, item));
    let ord = crate::lower::recognition::std_item(tcx, crate::lower::recognition::StdItem::Ord);
    // A comparison by `Ord` or `PartialOrd`, of its `Self`; `max`, `min`
    // and `clamp`, which drop the ones they don't give back; `std::cmp`'s
    // `max` and `min`, of their `T`.
    let ordering = is(of_trait, LangItem::PartialOrd) || of_trait == Some(ord);
    // `==` and `!=`, of what `eq_may` with `impl_seen` says isn't seen.
    if is(of_trait, LangItem::PartialEq) {
        return self_ty().is_some_and(|ty| !crate::lower::ordering::eq_may(recognition, ty, impl_seen));
    }
    let cmp_fn = crate::lower::recognition::is_cmp_max_or_min(tcx, callee);
    if ordering || cmp_fn {
        let Some(ty) = self_ty() else {
            return false;
        };
        let gives_one = matches!(name, "max" | "min" | "clamp");
        return !crate::lower::ordering::order_may(recognition, ty, impl_seen) && !(gives_one && drops(ty));
    }
    // A function it calls, a closure's or a function's type: one `quiet`
    // holds, or std's that's quiet itself.
    let fn_quiet = |f: Ty<'tcx>| match *f.kind() {
        ty::Closure(id, _) => quiet(id),
        ty::FnDef(id, f_args) => {
            quiet(id)
                || std_quiet(
                    (tcx, recognition),
                    (id, f_args.skip_binder(), false),
                    impl_seen,
                    quiet,
                    drops,
                )
        }
        _ => false,
    };
    // Arithmetic of numbers, through references too, which wraps, as
    // overflow checks are off (ADR 0011); a division or a remainder only
    // by a divisor known to be neither 0 nor -1.
    let numbers = args.types().all(|ty| ty.peel_refs().is_numeric());
    let arithmetic = [
        LangItem::Add,
        LangItem::Sub,
        LangItem::Mul,
        LangItem::Neg,
        LangItem::Not,
        LangItem::BitAnd,
    ]
    .into_iter()
    .chain([LangItem::BitOr, LangItem::BitXor, LangItem::Shl, LangItem::Shr])
    .any(|item| is(of_trait, item));
    let divides = is(of_trait, LangItem::Div) || is(of_trait, LangItem::Rem);
    if numbers && (arithmetic || divides) {
        return !divides || divisor_known;
    }
    let is_iterator = |tr: Option<DefId>| tr.is_some_and(|tr| crate::lower::recognition::is_iterator_trait(tcx, tr));
    // A part of a std iterator's type that runs nothing of the crate's:
    // std's, a number, text, a reference; no closure, no parameter.
    let std_part = |part: Ty<'tcx>| match part.kind() {
        ty::Adt(adt, _) => is_std_crate(tcx, adt.did()),
        ty::Closure(..) | ty::FnDef(..) | ty::Param(_) | ty::Dynamic(..) | ty::Alias(..) => false,
        _ => true,
    };
    let own =
        (tcx.inherent_impl_of_assoc(callee)).map(|imp| tcx.type_of(imp).instantiate_identity().skip_normalization());
    let own_is = |item: LangItem| own.is_some_and(|own| recognition.is_lang_adt(own, item));
    match () {
        _ if own_is(LangItem::OrderingEnum) => match name {
            "then" | "reverse" | "is_eq" | "is_ne" | "is_lt" | "is_gt" | "is_le" | "is_ge" => true,
            "then_with" => args.types().any(fn_quiet),
            _ => false,
        },
        _ if own.is_some_and(|own| own.is_numeric()) => matches!(name, "total_cmp" | "abs" | "min" | "max"),
        // An `Option`'s tests and views; its `map` of a quiet function, and
        // its `unwrap_or` of what drops nothing, which drops what it's
        // given where it holds one.
        _ if own_is(LangItem::Option) => match name {
            "is_some" | "is_none" | "as_ref" | "as_deref" | "copied" => true,
            "map" => args.types().last().is_some_and(fn_quiet),
            "unwrap_or" => self_ty().is_some_and(|ty| !drops(ty)),
            _ => false,
        },
        _ if name == "discriminant_value" => true,
        // A pin, or a box, of what it's given, and a pin's views: a pointer
        // to what's held, as `pin!` and `Box::pin` make one of a future
        // (#965), which runs nothing.
        _ if own_is(LangItem::Pin) => matches!(
            name,
            "new" | "new_unchecked" | "as_mut" | "as_ref" | "get_mut" | "get_ref" | "into_ref"
        ),
        _ if own_is(LangItem::OwnedBox) => matches!(name, "new" | "pin"),
        // `future::ready(v)`, a future already done, which runs nothing.
        _ if matches!(
            recognition.classify(callee, args),
            Some(crate::lower::recognition::Std::Future(_))
        ) =>
        {
            true
        }
        // `format!` of what can't fail to show, which only allocates.
        // `format!` of what can't fail to show, whose `Arguments` say so, and
        // which only allocates; its `must_use`, which gives what it's given.
        _ if matches!(name, "format" | "format_inner" | "must_use") && own.is_none() && of_trait.is_none() => true,
        _ if matches!(
            name,
            "new" | "new_const" | "new_v1" | "from_str" | "new_display" | "new_debug"
        ) =>
        {
            crate::lower::recognition::print_of(tcx, callee).is_none()
                && std_never_unwinds((tcx, recognition), (callee, args), drops)
        }
        // A std iterator of std's items, of text's `chars()`, stepped,
        // counted or compared.
        _ if is_iterator(of_trait) => {
            matches!(
                name,
                "next" | "count" | "last" | "rev" | "cmp" | "partial_cmp" | "eq" | "ne" | "lt" | "le"
            ) && args
                .types()
                .all(|ty| ty.walk().filter_map(|part| part.as_type()).all(std_part))
        }
        // Text's and a collection's length and views of what it holds, and
        // text's characters and cases.
        _ if own.is_some() || is(of_trait, LangItem::Deref) => matches!(
            name,
            "len"
                | "is_empty"
                | "as_str"
                | "as_slice"
                | "as_bytes"
                | "deref"
                | "chars"
                | "bytes"
                | "char_indices"
                | "to_lowercase"
                | "to_uppercase"
                | "to_ascii_lowercase"
                | "to_ascii_uppercase"
                | "trim"
        ),
        // A clone of std's text, or text made of text, a `char` or a
        // number, which only allocate.
        _ if is(of_trait, LangItem::Clone) || matches!(name, "to_string" | "to_owned") => self_ty().is_some_and(|ty| {
            ty.is_str() || recognition.is_lang_adt(ty, LangItem::String) || (name == "to_string" && ty.is_primitive())
        }),
        _ => false,
    }
}

/// Whether the crate's generic function `id`, given `args`, never unwinds in
/// `typing_env`, its caller's (#861): its MIR, `args` filled in, steps
/// on as one of no type parameters must, each call of a trait's method
/// what it resolves to, a closure's `call_mut` the closure, which then
/// never unwinds where `never` holds it; each `Drop` one that drops
/// nothing, or one `may_panic` says can't panic. So the guide's `impl<F:
/// FnMut()> Drop for Defer<F>`, given a closure that can't panic, drops in
/// a `finally`, where each use of it asks. Not of a binding, nor a
/// coroutine; one that reaches itself again may unwind, as nothing is
/// known of it yet.
pub(in crate::lower) fn never_unwinds_given<'tcx>(
    (recognition, never, bodies): (&Recognition<'_, 'tcx>, &IndexSet<DefId>, &IndexMap<DefId, &Body<'tcx>>),
    (id, args, typing_env): (DefId, ty::GenericArgsRef<'tcx>, ty::TypingEnv<'tcx>),
    (drops, may_panic): (&dyn Fn(Ty<'tcx>) -> bool, &dyn Fn(Ty<'tcx>) -> bool),
) -> bool {
    let judged = Judged {
        recognition,
        never,
        bodies,
        typing_env,
        drops,
        may_panic,
    };
    judged.never_unwinds(id, args, &mut Vec::new())
}

/// What `never_unwinds_given` asks of each function it reaches.
struct Judged<'a, 'b, 'tcx> {
    recognition: &'a Recognition<'b, 'tcx>,
    never: &'a IndexSet<DefId>,
    bodies: &'a IndexMap<DefId, &'a Body<'tcx>>,
    typing_env: ty::TypingEnv<'tcx>,
    drops: &'a dyn Fn(Ty<'tcx>) -> bool,
    may_panic: &'a dyn Fn(Ty<'tcx>) -> bool,
}

impl<'tcx> Judged<'_, '_, 'tcx> {
    fn never_unwinds(
        &self,
        id: DefId,
        args: ty::GenericArgsRef<'tcx>,
        seen: &mut Vec<(DefId, ty::GenericArgsRef<'tcx>)>,
    ) -> bool {
        let tcx = self.recognition.tcx;
        if self.never.contains(&id) {
            return true;
        }
        if !id.is_local()
            || is_binding(tcx, id)
            || !matches!(tcx.def_kind(id), DefKind::Fn | DefKind::AssocFn)
            || seen.contains(&(id, args))
        {
            return false;
        }
        let Some(mir) = self.bodies.get(&id).and_then(|body| body.mir.as_ref()) else {
            return false;
        };
        let mir = mir.instantiated(tcx, args).body;
        let mut dead = crate::lower::mir::dead_drops(tcx, &mir, &self.drops);
        dead.extend(
            (mir.basic_blocks.iter_enumerated()).filter_map(|(at, block)| match &block.terminator().kind {
                TerminatorKind::Drop { place, .. } => {
                    let ty = place.ty(&mir, tcx).ty;
                    let ty =
                        (tcx.try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))).ok()?;
                    (!(self.may_panic)(ty)).then_some(at)
                }
                _ => None,
            }),
        );
        seen.push((id, args));
        let steps = steps_on((&mir, &dead, false), &mut |callee, callee_args, _| {
            if self.never.contains(&callee)
                || std_never_unwinds((tcx, self.recognition), (callee, callee_args), self.drops)
            {
                return true;
            }
            let resolved = crate::lower::items::resolve_instance_in(tcx, self.typing_env, callee, callee_args);
            let Ok(Some(instance)) = resolved else {
                return false;
            };
            match instance.def {
                ty::InstanceKind::Item(def) => self.never_unwinds(def, instance.args, seen),
                _ => false,
            }
        });
        seen.pop();
        steps
    }
}

/// Whether `id` may be one: the crate's own function or closure, no
/// binding's, a function of no type parameters (#701). A closure's captures
/// are read in its MIR as its fields, so a call, a drop or an assert of what
/// it captured is a block `steps_on` reads; one written in a generic
/// function may be one too, as each call and drop of what's of its type
/// parameters is no way on `steps_on` takes (#847): a guard's closure,
/// `Defer(|| DEPTH.set(previous))` in a function given `impl FnOnce`. Not a
/// coroutine, an `async` block's, which no stage is.
fn candidate(tcx: TyCtxt<'_>, id: DefId) -> bool {
    id.is_local()
        && match tcx.def_kind(id) {
            DefKind::Fn | DefKind::AssocFn => !tcx.generics_of(id).requires_monomorphization(tcx),
            DefKind::Closure => !tcx.is_coroutine(id),
            _ => false,
        }
        && !is_binding(tcx, id)
}

/// What `steps_on` asks of each call: of its callee, given its generic
/// arguments and its operands, whether it goes on.
type Called<'a, 'tcx> =
    dyn FnMut(DefId, ty::GenericArgsRef<'tcx>, &[rustc_span::Spanned<mir::Operand<'tcx>>]) -> bool + 'a;

/// Whether each block of `mir` but its cleanup goes on without a panic:
/// a jump, a test, a return, a `Drop` of `dead`'s, which drops nothing
/// (#717), or a call `called` says never unwinds: one of the crate's that
/// never does, or std's formatting and printing of what can't fail to show.
/// Not an assert, a bound's or a divisor's, nor another drop, nor what's
/// unreachable, but where `unreachable` says, of `quiet`'s, which asks only
/// whether a call can be seen: an exhaustive `match`'s impossible branch
/// never runs.
fn steps_on<'tcx>(
    (mir, dead, unreachable): (&mir::Body<'tcx>, &IndexSet<mir::BasicBlock>, bool),
    called: &mut Called<'_, 'tcx>,
) -> bool {
    mir.basic_blocks
        .iter_enumerated()
        .filter(|(_, block)| !block.is_cleanup)
        .all(|(at, block)| {
            let Some(terminator) = &block.terminator else {
                return true;
            };
            match &terminator.kind {
                TerminatorKind::Drop { .. } => dead.contains(&at),
                // And an async body's end where its future is dropped
                // before it's done, which runs nothing more, and its
                // waiting on what it awaits, which runs nothing itself:
                // what's awaited is its `poll`'s, a call (#965).
                TerminatorKind::Yield { .. } => true,
                TerminatorKind::Goto { .. }
                | TerminatorKind::SwitchInt { .. }
                | TerminatorKind::Return
                | TerminatorKind::CoroutineDrop
                | TerminatorKind::FalseEdge { .. }
                | TerminatorKind::FalseUnwind { .. } => true,
                TerminatorKind::Unreachable => unreachable,
                TerminatorKind::Call {
                    func, args: operands, ..
                } => func
                    .const_fn_def()
                    .is_some_and(|(callee, args)| called(callee, args, operands)),
                _ => false,
            }
        })
}

/// Whether std's `callee`, given `args`, never panics as rust-js runs it:
/// a print, whose stream never fails (ADR 0132), the `Arguments` it's
/// given, and each argument's `Display` or `Debug` of a number, a `bool`,
/// a `char` or text, which shows any (ADR 0901); a cell's set of what JS
/// drops nothing of, `drops` false of its old value.
fn std_never_unwinds<'tcx>(
    (tcx, recognition): (TyCtxt<'tcx>, &Recognition<'_, 'tcx>),
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
    drops: &dyn Fn(Ty<'tcx>) -> bool,
) -> bool {
    if !is_std_crate(tcx, callee) {
        return false;
    }
    let path = tcx.def_path_str(callee);
    let name = tcx.item_name(callee);
    let name = name.as_str();
    match () {
        _ if crate::lower::recognition::print_of(tcx, callee).is_some() => true,
        _ if path.contains("fmt::Arguments") => matches!(name, "new" | "new_const" | "new_v1" | "from_str"),
        _ if path.contains("fmt::rt::Argument") => {
            matches!(name, "new_display" | "new_debug") && args.types().all(|ty| shown_whole(recognition, ty))
        }
        // A thread-local's or a `Cell`'s set of a value whose old one JS
        // drops nothing of, `drops` false, an `Option` of a JS object or of
        // an `Rc` no count is kept of (#861), and its replace, which gives
        // the old one back: a write of a module's `let` or of the cell's
        // place, as rust-js runs it, where native Rust's thread-local may
        // panic once destroyed, which a JS module never is. So a guard that
        // restores one, `DEPTH.set(self.0)`, drops in a `finally` (#847).
        // Not its `get` yet: a scope whose calls then never throw is MIR's
        // blocks, `bb18: { .. }`, where tidy makes no `finally` of its early
        // returns (`drop_guard_early_returns`), #861's to make.
        _ if name == "set" && of_a_cell(tcx, callee) => args.types().all(|ty| !drops(ty)),
        _ if name == "replace" && of_a_cell(tcx, callee) => true,
        // An `Option`'s take, the value it held, `None` left (#861).
        _ if name == "take" && of_an_option(tcx, recognition, callee) => true,
        _ => false,
    }
}

/// Whether `callee` is std's formatting, which unwinds only through what
/// it runs of the crate's or where it writes
/// (`Unwinding::ButThroughWhatsShown`).
fn formats(tcx: TyCtxt<'_>, callee: DefId) -> bool {
    crate::lower::recognition::is_std_formatting(tcx, callee)
}

/// Whether `method` is a `Cell`'s own, or a thread-local `Cell`'s,
/// `LocalKey<Cell<T>>`'s; not a `RefCell`'s, whose `set` borrows.
fn of_a_cell(tcx: TyCtxt<'_>, method: DefId) -> bool {
    let Some(imp) = tcx.inherent_impl_of_assoc(method) else {
        return false;
    };
    let is = |ty: Ty<'_>, name: &str| matches!(ty.kind(), ty::Adt(adt, _) if is_std_crate(tcx, adt.did()) && tcx.item_name(adt.did()).as_str() == name);
    let own = tcx.type_of(imp).instantiate_identity().skip_normalization();
    is(own, "Cell")
        || matches!(own.kind(), ty::Adt(_, local) if is(own, "LocalKey")
            && local.types().next().is_some_and(|cell| is(cell, "Cell")))
}

/// Whether `method` is an `Option`'s own.
fn of_an_option<'tcx>(tcx: TyCtxt<'tcx>, recognition: &Recognition<'_, 'tcx>, method: DefId) -> bool {
    tcx.inherent_impl_of_assoc(method).is_some_and(|imp| {
        let own = tcx.type_of(imp).instantiate_identity().skip_normalization();
        recognition.is_lang_adt(own, LangItem::Option)
    })
}

/// Whether showing a value of `ty` can't fail: a number's, a `bool`'s, a
/// `char`'s or text's, through references.
fn shown_whole<'tcx>(recognition: &Recognition<'_, 'tcx>, ty: Ty<'tcx>) -> bool {
    let ty = ty.peel_refs();
    ty.is_primitive() || ty.is_str() || recognition.is_lang_adt(ty, LangItem::String)
}
