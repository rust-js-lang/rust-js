//! What each `impl Trait` of a body hides, read once, where its MIR is
//! taken (#1142): the body's types the types its values are, as rustc's
//! codegen reads them after analysis. An `impl Iterator` a function gives
//! back is the adapter or the crate's own iterator its body made, so every
//! rule that asks what a value is, its steps, its drop, its back, asks of
//! that, and the value its maker gives and the one its caller holds are
//! one representation.

use rustc_middle::ty::{self, Ty, TyCtxt, TypeFoldable, TypeFolder, TypeSuperFoldable, TypeVisitableExt};

/// `value`, a body or a type of one, each opaque type in it the type it
/// hides. One that holds itself, an `async fn` that awaits itself through a
/// box, stays as it is past where it does.
pub(in crate::lower) fn revealed<'tcx, T: TypeFoldable<TyCtxt<'tcx>>>(tcx: TyCtxt<'tcx>, value: T) -> T {
    value.fold_with(&mut Revealer { tcx })
}

struct Revealer<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl<'tcx> TypeFolder<TyCtxt<'tcx>> for Revealer<'tcx> {
    fn cx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }

    fn fold_ty(&mut self, ty: Ty<'tcx>) -> Ty<'tcx> {
        if !ty.has_opaque_types() {
            return ty;
        }
        match *ty.kind() {
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::Opaque { def_id },
                    args,
                    ..
                },
            ) => match self.tcx.try_expand_impl_trait_type(def_id, args) {
                Ok(hidden) => hidden,
                Err(_) => ty.super_fold_with(self),
            },
            _ => ty.super_fold_with(self),
        }
    }
}
