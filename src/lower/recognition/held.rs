//! What a wrapper of an iterator holds: the iterator a reference or a box
//! of it steps; split from iterators.rs at its cap.

use rustc_middle::ty::Ty;

/// The iterator a `ty` steps: itself, or what a reference to it or a `Box`
/// of it holds, each of which steps it, a `Box` by what std's impl passes
/// on to it (`Wrapper::passes`, #1601). The one answer of which iterator's
/// impl a type's steps reach.
pub(in crate::lower) fn iterator_held(ty: Ty<'_>) -> Ty<'_> {
    let mut ty = ty.peel_refs();
    while let Some(inner) = ty.boxed_ty() {
        ty = inner.peel_refs();
    }
    ty
}

/// Whether a `ty` is a `Box` of the iterator it steps, or a reference to
/// one (`iterator_held`).
pub(super) fn behind_a_box(ty: Ty<'_>) -> bool {
    ty.peel_refs().boxed_ty().is_some()
}
