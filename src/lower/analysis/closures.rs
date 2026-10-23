//! What the crate does with its closures that JS can't see in a function:
//! whether one holding a value with a destructor is made a `dyn Fn` (ADR
//! 0521), and which are cloned apart (ADR 0522); and, as a `dyn Fn`'s, of
//! a `dyn Any`, whether a value with a destructor is made one (ADR 0691).

use crate::lower::Body;
use crate::lower::recognition::{any_trait, given_back, is_fn_dyn, rc_pointee};
use rustc_hir::def_id::DefId;
use rustc_middle::thir::{ExprKind, Thir};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use std::collections::HashSet;

/// What the crate does with its closures.
#[derive(Default)]
pub struct ClosureFacts {
    /// Whether a function holding a value with a destructor is made a `dyn
    /// Fn`, which then drops it: a library's may be.
    pub fn_dyns_drop: bool,
    /// Whether a value with a destructor is made a `dyn Any`, which then
    /// drops it: what unwound, a panic's message or what JS threw, drops
    /// nothing, nor a `dyn Any` of anything else (ADR 0691).
    pub any_dyns_drop: bool,
    /// The closures made a `dyn Fn`, each carrying its drop.
    made_fn_dyns: HashSet<DefId>,
    /// The closures cloned, `Clone::clone`'s or a `T: Clone`'s.
    cloned: HashSet<DefId>,
    /// Of them, those a clone of which isn't the closure itself, each made
    /// by a function of what it holds, which its clone is made by again.
    pub(super) cloned_apart: HashSet<DefId>,
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
        let made_fn_dyns = (all_bodies.iter())
            .flat_map(|body| fn_dyn_sources(tcx, &body.thir))
            .filter_map(closure_of)
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
            fn_dyns_drop: library,
            any_dyns_drop: false,
            made_fn_dyns,
            cloned,
            cloned_apart: HashSet::new(),
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
                    drops(ty) || params_may_drop && ty.has_non_region_param()
                }
            })
    };
    (all_bodies.iter())
        .flat_map(|body| fn_dyn_sources(tcx, &body.thir))
        .any(|from| drops(from) || from.has_non_region_param() && closure_drops())
}

/// Whether any of `all_bodies` makes a value a `dyn Any` that `drops` says
/// has something to drop, or a type parameter's, where `params_may_drop`;
/// or, where `foreign_drops`, a library may have made one (ADR 0691).
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
            .any(|from| drops(from) || params_may_drop && from.has_non_region_param())
}

/// The functions `thir` makes `dyn Fn`s of, `Box<F>` to `Box<dyn Fn()>`'s
/// `F`.
fn fn_dyn_sources<'a, 'tcx>(tcx: TyCtxt<'tcx>, thir: &'a Thir<'tcx>) -> impl Iterator<Item = Ty<'tcx>> + 'a {
    dyn_sources(tcx, thir, move |predicates| is_fn_dyn(tcx, predicates))
}

/// The values `thir` makes `dyn`s of that `of` says are its, `Box<T>` to
/// `Box<dyn Trait>`'s `T`.
fn dyn_sources<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    thir: &'a Thir<'tcx>,
    of: impl Fn(&'tcx ty::List<ty::PolyExistentialPredicate<'tcx>>) -> bool + 'a,
) -> impl Iterator<Item = Ty<'tcx>> + 'a {
    thir.exprs.iter().filter_map(move |expr| {
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
        pointee(tcx, thir[source].ty)
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
