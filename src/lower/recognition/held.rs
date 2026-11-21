//! What a wrapper of an iterator holds: the iterator a reference or a box
//! of it steps, and what a sized box is, for every question of the
//! iterator, `held_unboxed`'s (#2219); split from iterators.rs at its cap.

use rustc_middle::ty::{self, Ty};

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

/// The iterator a sized `Box` holds, each such box peeled: `I` of a
/// `Box<I>` or a `Box<Box<I>>`, `ty` itself of another. In JS a box is the
/// value it holds, so every question of an iterator, which steps it, what
/// closing it drops, whether its hint may run the crate's code, which
/// drop generic code is given for it, asks it of what the box holds: the
/// one answer of what a box of an iterator is (#2219, #2221). Not through
/// a reference, which lends what it's to, nor of a `Box<dyn Iterator>`,
/// which carries its own steps and drop (ADR 0521), nor of what's
/// unsized, a slice's or a `str`'s.
pub(in crate::lower) fn held_unboxed(ty: Ty<'_>) -> Ty<'_> {
    let mut ty = ty;
    while let Some(inner) = ty
        .boxed_ty()
        .filter(|inner| !matches!(inner.kind(), ty::Dynamic(..) | ty::Slice(_) | ty::Str))
    {
        ty = inner;
    }
    ty
}

/// Whether a `ty` is a `Box` of the iterator it steps, or a reference to
/// one (`iterator_held`).
pub(super) fn behind_a_box(ty: Ty<'_>) -> bool {
    ty.peel_refs().boxed_ty().is_some()
}
