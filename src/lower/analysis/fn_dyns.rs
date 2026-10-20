//! Whether a function that holds a value with a destructor is made a `dyn
//! Fn` (ADR 0521): where one is, a `dyn Fn` drops what it holds, the drop
//! its function carries, `$dispose(f)`; where none is, nothing.

use crate::lower::Body;
use crate::lower::recognition::{given_back, is_fn_dyn, rc_pointee};
use rustc_hir::def_id::DefId;
use rustc_middle::thir::{ExprKind, Thir};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};
use std::collections::HashSet;

/// What the crate makes `dyn Fn`s of.
#[derive(Default)]
pub struct FnDyns {
    /// Whether a function holding a value with a destructor is made one,
    /// which then drops it: a library's may be.
    pub drops: bool,
    /// The closures made one, each carrying its drop.
    closures: HashSet<DefId>,
}

impl FnDyns {
    /// What `all_bodies` make `dyn Fn`s of, whose drop is a library's
    /// where `library`.
    pub(super) fn new<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>], library: bool) -> FnDyns {
        let closures = (all_bodies.iter())
            .flat_map(|body| fn_dyn_sources(tcx, &body.thir))
            .filter_map(|from| match from.kind() {
                ty::Closure(id, _) => Some(*id),
                _ => None,
            })
            .collect();
        FnDyns {
            drops: library,
            closures,
        }
    }

    /// Whether the closure `closure` carries its own drop, `$dropping`'s:
    /// one its maker gives back, the type its `impl Fn` hides (ADR 0484),
    /// or one made a `dyn Fn` (ADR 0521), each dropped where its maker's
    /// variables it took aren't seen.
    pub(in crate::lower) fn carries_its_drop(&self, tcx: TyCtxt<'_>, closure: DefId) -> bool {
        self.closures.contains(&closure) || given_back(tcx, closure)
    }
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
                let id = body.def_id.to_def_id();
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

/// The functions `thir` makes `dyn Fn`s of, `Box<F>` to `Box<dyn Fn()>`'s
/// `F`.
fn fn_dyn_sources<'a, 'tcx>(tcx: TyCtxt<'tcx>, thir: &'a Thir<'tcx>) -> impl Iterator<Item = Ty<'tcx>> + 'a {
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
        is_fn_dyn(tcx, predicates).then_some(())?;
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
