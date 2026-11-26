//! What a function's call by value, `<F as FnOnce<Args>>::call_once`,
//! drops of it, which its type alone decides (#2600):
//!
//! - a `Fn` or `FnMut` closure is dropped whole, `_1`, where the call
//!   returns and where it unwinds, by rustc's `ClosureOnce` shim
//!   (`closure_once_shim`), and called by `&` or `&mut`, by what holds it;
//! - an `FnOnce` closure drops what its own body drops, where its MIR does;
//! - `Box<F>`'s moves out what it holds, calls it, then frees the box
//!   (`boxed_call_once_drop`);
//! - `&F` and a function pointer drop nothing.
//!
//! How the JS gets that drop run is each consumer's: a closure's body
//! written as its `call_once`, a wrap where a call by value takes a `Fn`,
//! `$droppingOnce`, or a refusal. Who calls by value is `analysis/closures`'.

use rustc_middle::mir;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

/// What a call by value of a function drops of it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OnceDrop {
    /// It's dropped whole: of a `Fn` or `FnMut`, called by `&` or `&mut`,
    /// by what holds it, and by its `call_once` as that call returns or
    /// unwinds (`closure_once_shim`, ADR 0470).
    Whole,
    /// Its body drops what it took, as it ends: of one called by `FnOnce`
    /// alone (ADR 0470).
    Body,
}

impl OnceDrop {
    /// What `call_once` of a closure of `args` drops of it, by the kind
    /// rustc infers it.
    pub fn of_closure(args: ty::GenericArgsRef<'_>) -> OnceDrop {
        OnceDrop::of(args.as_closure().kind())
    }

    fn of(kind: ty::ClosureKind) -> OnceDrop {
        match kind {
            ty::ClosureKind::Fn | ty::ClosureKind::FnMut => OnceDrop::Whole,
            ty::ClosureKind::FnOnce => OnceDrop::Body,
        }
    }

    /// Of two bounds of one type parameter, what its call drops, the
    /// strongest bound's: `F: FnMut + FnOnce` is called by `&mut`.
    fn strongest(a: OnceDrop, b: OnceDrop) -> OnceDrop {
        match (a, b) {
            (OnceDrop::Body, OnceDrop::Body) => OnceDrop::Body,
            _ => OnceDrop::Whole,
        }
    }
}

/// What `call_once` of a function of type `ty`, in `owner`, drops of it: a
/// closure's by its kind, and a type parameter's or an associated type's by
/// the strongest `Fn` bound its `owner` gives it, `F: FnMut`'s, its caller's
/// to give. An `AssertUnwindSafe`'s is what it holds': its `call_once` calls
/// `(self.0)()` and drops it as that returns (ADR 0660). A bound of a
/// binding's union whose `Function` member is any function of a kind,
/// `H: js::IntoTimerHandler`'s, is that kind's (`member_kind`). `None` of
/// what isn't one of these: a `dyn`'s is `dyn_call_once_drop`'s, a box's
/// `boxed_call_once_drop`'s.
pub fn call_once_drop<'tcx>(tcx: TyCtxt<'tcx>, owner: DefId, ty: Ty<'tcx>) -> Option<OnceDrop> {
    match ty.kind() {
        ty::Closure(_, args) => Some(OnceDrop::of_closure(args)),
        ty::Adt(adt, args) if crate::recognition::is_assert_unwind_safe(tcx, adt.did()) => {
            call_once_drop(tcx, owner, args.type_at(0))
        }
        ty::Param(_) | ty::Alias(..) => (tcx.param_env(owner).caller_bounds())
            .filter_map(|clause| clause.as_trait_clause())
            .filter(|bound| bound.self_ty().skip_binder() == ty)
            .filter_map(|bound| {
                (tcx.fn_trait_kind_from_def_id(bound.def_id())).or_else(|| member_kind(tcx, bound.def_id()))
            })
            .map(OnceDrop::of)
            .reduce(OnceDrop::strongest),
        _ => None,
    }
}

/// What `call_once` of a `dyn` of `predicates` drops of what it's of: a
/// `dyn FnOnce`'s body drops what it took (#409), a `dyn Fn`'s or a `dyn
/// FnMut`'s is dropped whole. `None` of a `dyn` of another trait.
pub fn dyn_call_once_drop(
    tcx: TyCtxt<'_>,
    predicates: &ty::List<ty::PolyExistentialPredicate<'_>>,
) -> Option<OnceDrop> {
    tcx.fn_trait_kind_from_def_id(predicates.principal_def_id()?)
        .map(OnceDrop::of)
}

/// What `<Box<F> as FnOnce>::call_once` drops of the `F` it moves out of a
/// box of type `ty` and calls, before it frees the box, which `F`'s type
/// doesn't say where it's a `dyn`'s, `Box<dyn FnMut(..)>`: a box of a
/// closure by the closure's, of a `dyn` by its trait's, through an
/// `AssertUnwindSafe` as what it holds (#1220). `None` of what isn't one.
pub fn boxed_call_once_drop<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<OnceDrop> {
    if let ty::Adt(adt, args) = ty.kind()
        && crate::recognition::is_assert_unwind_safe(tcx, adt.did())
    {
        return boxed_call_once_drop(tcx, args.type_at(0));
    }
    match ty.boxed_ty()?.kind() {
        ty::Closure(_, args) => Some(OnceDrop::of_closure(args)),
        ty::Dynamic(predicates, ..) => dyn_call_once_drop(tcx, predicates),
        _ => None,
    }
}

/// rustc's own `call_once` of a `Fn` or `FnMut` closure, `closure` of
/// `args`, its `ClosureOnce` shim (`Instance::fn_once_adapter_instance`):
/// it calls the closure's body by `&` or `&mut`, then drops `_1`, the
/// closure, whole, where that returns and where it unwinds, which is
/// `OnceDrop::Whole`. The drops shadow holds rust-js's drops of a body
/// written as its `call_once` to it (`mir/elaborated.rs`).
pub fn closure_once_shim<'tcx>(
    tcx: TyCtxt<'tcx>,
    closure: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> &'tcx mir::Body<'tcx> {
    let shim = ty::Instance::fn_once_adapter_instance(tcx, closure, args);
    tcx.instance_mir(shim.def)
}

/// How a binding's union trait, one `#[rust_js::types]` types, calls its
/// `Function` member where that's any function of a kind, the kind its
/// blanket impl bounds it by: `js::IntoTimerHandler`'s `impl<F: FnOnce() +
/// 'static>`, so `setTimeout` calls what it's given once (ADR 1020). `None`
/// of another trait.
fn member_kind(tcx: TyCtxt<'_>, trait_id: DefId) -> Option<ty::ClosureKind> {
    let union = [Symbol::intern("rust_js"), Symbol::intern("types")];
    tcx.get_attrs_by_path(trait_id, &union).next()?;
    tcx.all_impls(trait_id).find_map(|imp| {
        let member = tcx.type_of(imp).instantiate_identity().skip_normalization();
        if !matches!(member.kind(), ty::Param(_)) {
            return None;
        }
        (tcx.param_env(imp).caller_bounds())
            .filter_map(|clause| clause.as_trait_clause())
            .filter(|bound| bound.self_ty().skip_binder() == member)
            .find_map(|bound| tcx.fn_trait_kind_from_def_id(bound.def_id()))
    })
}
