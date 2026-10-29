//! What the crate does with its closures that JS can't see in a function:
//! whether one holding a value with a destructor is made a `dyn Fn` (ADR
//! 0521), and which are cloned apart (ADR 0522); and, as a `dyn Fn`'s, of
//! a `dyn Any`, whether a value with a destructor is made one (ADR 0691).

use crate::lower::passed_on::Taker;
use crate::lower::recognition::{PureStd, any_trait, given_back, is_fn_dyn, pure_std, rc_pointee};
use crate::lower::{Body, bindings};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def_id::DefId;
use rustc_middle::thir::{AdtExprBase, ExprId, ExprKind, Thir};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use std::collections::HashSet;

/// What the crate does with its closures.
#[derive(Default)]
pub struct ClosureFacts {
    /// Whether a function holding a value with a destructor is made a `dyn
    /// Fn`, which then drops it: the crate makes one so, a library a type
    /// parameter's, or a library hands it one whose own may, as a library's
    /// contract says (ADR 0626, amended).
    pub fn_dyns_drop: bool,
    /// Whether a value with a destructor is made a `dyn Any`, which then
    /// drops it: what unwound, a panic's message or what JS threw, drops
    /// nothing, nor a `dyn Any` of anything else (ADR 0691).
    pub any_dyns_drop: bool,
    /// The closures made a `dyn Fn`, each carrying its drop.
    made_fn_dyns: HashSet<DefId>,
    /// The closures made a `dyn Fn` only to hand it to JS, which Rust never
    /// drops, so none carries its drop (ADR 0873).
    handed_fn_dyns: HashSet<DefId>,
    /// The closures cloned, `Clone::clone`'s or a `T: Clone`'s.
    cloned: HashSet<DefId>,
    /// Of them, those a clone of which isn't the closure itself, each made
    /// by a function of what it holds, which its clone is made by again.
    pub(super) cloned_apart: HashSet<DefId>,
    /// The closures rustc infers `Fn` or `FnMut` made where they're given
    /// to a binding that calls what it's given once, `js::set_timeout(move
    /// || { let m = &n; .. }, ms)`'s, of a union bound (ADR 1020): each
    /// called as its `call_once` runs it, dropping what it took.
    called_once: HashSet<DefId>,
    /// Whether the crate is a library, whose callers' closures may be.
    library: bool,
}

impl ClosureFacts {
    /// What `all_bodies` make `dyn Fn`s of, and of the types the crate
    /// clones, `cloned`, the closures, whose drop is a library's where
    /// `library`.
    pub(super) fn new<'tcx>(
        tcx: TyCtxt<'tcx>,
        all_bodies: &[&Body<'tcx>],
        cloned: &HashSet<Ty<'tcx>>,
        library: bool,
    ) -> ClosureFacts {
        let closure_of = |ty: Ty<'tcx>| match ty.kind() {
            ty::Closure(id, _) => Some(*id),
            _ => None,
        };
        let (handed, made): (Vec<_>, Vec<_>) = (all_bodies.iter())
            .flat_map(|body| fn_dyn_sources(tcx, &body.thir))
            .partition(|&(_, handed)| handed);
        let made_fn_dyns: HashSet<DefId> = made.into_iter().filter_map(|(ty, _)| closure_of(ty)).collect();
        let called_once = (all_bodies.iter())
            .flat_map(|body| called_once_by_js(tcx, &body.thir))
            .collect();
        let handed_fn_dyns = (handed.into_iter())
            .filter_map(|(ty, _)| closure_of(ty))
            .filter(|id| !made_fn_dyns.contains(id))
            .collect();
        let cloned = (cloned.iter())
            .flat_map(|ty| {
                ty.walk()
                    .filter_map(|part| part.as_type())
                    .filter_map(closure_of)
                    .collect::<Vec<_>>()
            })
            .collect();
        ClosureFacts {
            fn_dyns_drop: false,
            any_dyns_drop: false,
            made_fn_dyns,
            handed_fn_dyns,
            cloned,
            cloned_apart: HashSet::new(),
            called_once,
            library,
        }
    }

    /// Of the closures cloned, those a clone of which `shares` says isn't
    /// the closure itself: `cloned_apart`'s, to set.
    pub(super) fn find_cloned_apart(&self, shares: impl Fn(DefId) -> bool) -> HashSet<DefId> {
        self.cloned.iter().copied().filter(|&id| !shares(id)).collect()
    }

    /// Whether the closure `closure` carries its own drop, `$dropping`'s:
    /// one its maker gives back, the type its `impl Fn` hides (ADR 0484),
    /// one made a `dyn Fn` (ADR 0521), or one cloned apart (ADR 0522),
    /// each dropped where its maker's variables it took aren't seen.
    pub(in crate::lower) fn carries_its_drop(&self, tcx: TyCtxt<'_>, closure: DefId) -> bool {
        self.made_fn_dyns.contains(&closure) || self.cloned_apart.contains(&closure) || given_back(tcx, closure)
    }

    /// Whether the closure `closure`, which rustc infers `Fn` or `FnMut`, is
    /// made where it's given to a binding that calls it once, which then
    /// drops what it took: its body is its `call_once`'s, `Mir::called_once`
    /// (ADR 1020).
    pub(in crate::lower) fn called_once(&self, closure: DefId) -> bool {
        self.called_once.contains(&closure)
    }

    /// Whether the closure `closure` is made a `dyn Fn` only to hand it to
    /// JS, which Rust then never drops (ADR 0873): it carries no drop, nor
    /// is given one where it's made a `dyn Fn`.
    pub(in crate::lower) fn handed_to_js(&self, closure: DefId) -> bool {
        self.handed_fn_dyns.contains(&closure)
    }

    /// Whether the closure `closure` is cloned apart: made by a function of
    /// what it holds, `$cloning(f, clone)`, which `$clone(f)` makes again of
    /// a clone of it (ADR 0522).
    pub(in crate::lower) fn clones_apart(&self, closure: DefId) -> bool {
        self.cloned_apart.contains(&closure)
    }

    /// Whether a function a type parameter is given may be cloned apart,
    /// so its clone is its `Clone` dictionary's, not the function itself
    /// (ADRs 0246, 0522): where the crate clones one apart, or its callers
    /// may.
    pub(in crate::lower) fn functions_clone_apart(&self) -> bool {
        self.library || !self.cloned_apart.is_empty()
    }
}

/// Whether a clone of the closure of type `closure` can be the closure
/// itself, the one JS function: one nothing could tell from it, that never
/// writes what it took, nor moves it out, an `FnOnce`'s, for what it's
/// given to to change, holds no `Cell`, and holds nothing a clone of which
/// counts or drops, as `drops` says (ADRs 0522, 0525).
pub(in crate::lower) fn clone_shares<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    closure: Ty<'tcx>,
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> bool {
    let ty::Closure(id, args) = *closure.kind() else {
        return true;
    };
    let closure = args.as_closure();
    // One that writes what it took, by value or through a `&mut`: a
    // `FnMut` may be one only by the bound it's given to, `map(|x| x * 2)`.
    let writes = match id.as_local() {
        Some(local) => tcx
            .closure_captures(local)
            .iter()
            .any(|captured| match captured.info.capture_kind {
                ty::UpvarCapture::ByRef(kind) => kind != ty::BorrowKind::Immutable,
                ty::UpvarCapture::ByValue | ty::UpvarCapture::ByUse => captured.mutability == ty::Mutability::Mut,
            }),
        None => closure.kind() != ty::ClosureKind::Fn,
    } || closure.kind() == ty::ClosureKind::FnOnce;
    !writes && (closure.upvar_tys().iter()).all(|t| t.is_freeze(tcx, typing_env) && !drops(t))
}

/// Whether any of `all_bodies` makes a function a `dyn Fn` that `drops`
/// says has something to drop, or one of a type parameter's, given a
/// closure: where a closure the crate makes has, or, where
/// `foreign_drops`, one a library makes may.
pub(super) fn fn_dyns_drop<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    (params_may_drop, foreign_drops): (bool, bool),
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> bool {
    // A type parameter's: in Rust a function is a closure, a `fn` or a
    // `dyn Fn`, whose own coercion answers for it.
    let closure_drops = || {
        foreign_drops
            || all_bodies.iter().any(|body| {
                let id = body.def_id;
                tcx.is_closure_like(id) && {
                    let ty = tcx.type_of(id).instantiate_identity().skip_normalization();
                    drops(ty) || params_may_drop && holds_a_param(ty)
                }
            })
    };
    (all_bodies.iter())
        .flat_map(|body| fn_dyn_sources(tcx, &body.thir))
        .any(|(from, handed)| !handed && (drops(from) || holds_a_param(from) && closure_drops()))
}

/// Whether a function of type `ty` may be one of a type parameter's,
/// whose drop only its callers know: a type parameter's own, `F`, or a
/// closure that captures one. Not a closure of a generic function's that
/// holds none, `move |resolve| listeners.borrow_mut().push(resolve)` in
/// `chain_thenable_value<T>`, whose type names `T` among its parent's
/// generic arguments but holds what it captures, which `drops` answers
/// for; nor a `dyn Fn` made another, `&dyn Fn(S, A) -> S`, a function
/// already, whose own coercion answers (ADR 0521, amended).
fn holds_a_param(ty: Ty<'_>) -> bool {
    match ty.kind() {
        ty::Closure(_, args) => (args.as_closure().upvar_tys().iter()).any(|t| t.has_non_region_param()),
        ty::Dynamic(..) => false,
        _ => ty.has_non_region_param(),
    }
}

/// Whether any of `all_bodies` makes a value a `dyn Any` that `drops` says
/// has something to drop, or a type parameter's, where `params_may_drop`;
/// or, where `foreign_drops`, a library made one, as its contract says
/// (ADR 0691, amended).
/// Only the crate makes one: std's of its own is a panic's message.
pub(super) fn any_dyns_drop<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    (params_may_drop, foreign_drops): (bool, bool),
    drops: impl Fn(Ty<'tcx>) -> bool,
) -> bool {
    let Some(any) = any_trait(tcx) else {
        return false;
    };
    let of_any = |predicates: &ty::List<ty::PolyExistentialPredicate<'tcx>>| predicates.principal_def_id() == Some(any);
    foreign_drops
        || (all_bodies.iter())
            .flat_map(|body| dyn_sources(tcx, &body.thir, of_any))
            .map(|(_, from)| from)
            .any(|from| drops(from) || params_may_drop && from.has_non_region_param())
}

/// The functions `thir` makes `dyn Fn`s of, `Box<F>` to `Box<dyn Fn()>`'s
/// `F`, each with whether it's a closure made one only to hand it to JS.
fn fn_dyn_sources<'a, 'tcx>(tcx: TyCtxt<'tcx>, thir: &'a Thir<'tcx>) -> impl Iterator<Item = (Ty<'tcx>, bool)> + 'a {
    let handed = handed_to_js(tcx, thir);
    dyn_sources(tcx, thir, move |predicates| is_fn_dyn(tcx, predicates))
        .map(move |(at, from)| (from, matches!(from.kind(), ty::Closure(..)) && handed.contains(&at)))
}

/// The coercions to a `dyn Fn` in `thir` handed straight to JS (ADR 0873):
/// of a closure boxed where it's made, a binding's argument, or the `Some`
/// of one, that nothing evaluated after it, before the call, can unwind
/// past, of a binding that gives back no function, which might be the one
/// it's given, as `widen`'s `this` does. JS keeps what it's given, and has
/// no drop: Rust never drops it again.
fn handed_to_js(tcx: TyCtxt<'_>, thir: &Thir<'_>) -> HashSet<ExprId> {
    let through = |mut e: ExprId| {
        while let ExprKind::Scope { value: inner, .. } | ExprKind::Use { source: inner } = thir[e].kind {
            e = inner;
        }
        e
    };
    let coercion = |arg: ExprId| {
        let mut e = through(arg);
        if let ExprKind::Adt(adt) = &thir[e].kind
            && tcx.is_lang_item(adt.adt_def.did(), LangItem::Option)
            && let [field] = &adt.fields[..]
            && matches!(adt.base, AdtExprBase::None)
        {
            e = through(field.expr);
        }
        // Of a closure boxed where it's made, `Box::new(move || ..)`: nothing
        // else holds it before.
        let ExprKind::PointerCoercion {
            cast: PointerCoercion::Unsize,
            source,
            ..
        } = thir[e].kind
        else {
            return None;
        };
        let ExprKind::Call { fun, ref args, .. } = thir[through(source)].kind else {
            return None;
        };
        let boxed = matches!(*thir[fun].ty.kind(), ty::FnDef(id, _) if pure_std(tcx, id) == Some(PureStd::BoxNew));
        (boxed && matches!(&args[..], [made] if matches!(thir[through(*made)].kind, ExprKind::Closure(_)))).then_some(e)
    };
    let mut handed = HashSet::new();
    for expr in thir.exprs.iter() {
        let ExprKind::Call { fun, ref args, .. } = expr.kind else {
            continue;
        };
        let ty::FnDef(id, _) = *thir[fun].ty.kind() else {
            continue;
        };
        if !bindings::is_binding(tcx, id) || gives_back_a_function(tcx, id) {
            continue;
        }
        for (i, &arg) in args.iter().enumerate() {
            if let Some(e) = coercion(arg)
                && args[i + 1..].iter().all(|&after| cannot_unwind(thir, after))
            {
                handed.insert(e);
            }
        }
    }
    handed
}

/// The closures in `thir` rustc infers `Fn` or `FnMut`, made where they're
/// given to a binding's parameter whose callback JS calls once
/// (`Taker::parameter`), a type parameter of a union whose `Function`
/// member is called by `FnOnce`, `js::set_timeout`'s `H: IntoTimerHandler`
/// (ADR 1020): JS calls it once, or never, and keeps it, as Rust's
/// `call_once` takes it, so what it took is dropped as its call ends.
fn called_once_by_js(tcx: TyCtxt<'_>, thir: &Thir<'_>) -> Vec<DefId> {
    let mut once = Vec::new();
    for expr in thir.exprs.iter() {
        let ExprKind::Call { fun, ref args, .. } = expr.kind else {
            continue;
        };
        let ty::FnDef(id, _) = *thir[fun].ty.kind() else {
            continue;
        };
        if !bindings::is_binding(tcx, id) {
            continue;
        }
        for (i, &arg) in args.iter().enumerate() {
            let mut e = arg;
            while let ExprKind::Scope { value: inner, .. } | ExprKind::Use { source: inner } = thir[e].kind {
                e = inner;
            }
            if let ExprKind::Closure(_) = thir[e].kind
                && let ty::Closure(closure, closure_args) = *thir[e].ty.kind()
                && closure_args.as_closure().kind() != ty::ClosureKind::FnOnce
                && Taker::parameter(tcx, id, i) == Some(Taker::OnceCallback)
            {
                once.push(closure);
            }
        }
    }
    once
}

/// Whether the binding `id` may give a function back: what it returns, or
/// writes through a `&mut` it's given, may hold one, a `dyn Fn`, a closure,
/// an `impl Fn` or a type parameter's, by any pointer.
fn gives_back_a_function(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let holds = |given: Ty<'_>| {
        given
            .walk()
            .filter_map(|part| part.as_type())
            .any(|ty| match ty.kind() {
                ty::Dynamic(predicates, ..) => is_fn_dyn(tcx, predicates),
                ty::Closure(..) | ty::FnPtr(..) | ty::Param(_) => true,
                ty::Alias(_, alias) => matches!(alias.kind, ty::Opaque { .. }),
                _ => false,
            })
    };
    let sig = tcx.fn_sig(id).skip_binder().skip_binder();
    holds(sig.output())
        || (sig.inputs().iter()).any(|input| {
            input
                .walk()
                .filter_map(|part| part.as_type())
                .any(|ty| matches!(ty.kind(), ty::Ref(_, written, ty::Mutability::Mut) if holds(*written)))
        })
}

/// Whether evaluating `e` can't unwind: a literal, a constant, a variable
/// or a place of one, borrowed, and what's made of those alone.
fn cannot_unwind(thir: &Thir<'_>, e: ExprId) -> bool {
    match &thir[e].kind {
        ExprKind::Scope { value: inner, .. }
        | ExprKind::Use { source: inner }
        | ExprKind::Field { lhs: inner, .. }
        | ExprKind::Borrow { arg: inner, .. }
        | ExprKind::PointerCoercion { source: inner, .. } => cannot_unwind(thir, *inner),
        ExprKind::Literal { .. }
        | ExprKind::NonHirLiteral { .. }
        | ExprKind::ZstLiteral { .. }
        | ExprKind::NamedConst { .. }
        | ExprKind::ConstParam { .. }
        | ExprKind::StaticRef { .. }
        | ExprKind::VarRef { .. }
        | ExprKind::UpvarRef { .. } => true,
        ExprKind::Adt(adt) => {
            matches!(adt.base, AdtExprBase::None) && adt.fields.iter().all(|field| cannot_unwind(thir, field.expr))
        }
        ExprKind::Tuple { fields } | ExprKind::Array { fields } => {
            fields.iter().all(|&field| cannot_unwind(thir, field))
        }
        _ => false,
    }
}

/// The values `thir` makes `dyn`s of that `of` says are its, `Box<T>` to
/// `Box<dyn Trait>`'s `T`.
fn dyn_sources<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &'a Thir<'tcx>,
    of: impl Fn(&'tcx ty::List<ty::PolyExistentialPredicate<'tcx>>) -> bool + 'a,
) -> impl Iterator<Item = (ExprId, Ty<'tcx>)> + 'a {
    thir.exprs.iter_enumerated().filter_map(move |(at, expr)| {
        let ExprKind::PointerCoercion {
            cast: PointerCoercion::Unsize,
            source,
            ..
        } = expr.kind
        else {
            return None;
        };
        let to = pointee(tcx, expr.ty)?;
        let ty::Dynamic(predicates, ..) = to.kind() else {
            return None;
        };
        of(predicates).then_some(())?;
        Some((at, pointee(tcx, thir[source].ty)?))
    })
}

/// What a `&`, a `Box` or an `Rc` points at.
fn pointee<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Ref(_, inner, _) => Some(*inner),
        _ if ty.is_box() => Some(ty.boxed_ty()?),
        _ => rc_pointee(tcx, ty),
    }
}
