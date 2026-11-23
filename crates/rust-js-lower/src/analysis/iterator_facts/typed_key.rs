//! Which item a type's question is asked in: a fact of a type that holds
//! wherever a value of it is made (back_stepped's `typed`, hint_asked's
//! `Hinted::typed`) holds crate-wide of a type of no type parameter, and of
//! one of type parameters only in the item that declares them. A type
//! parameter is its index and its name alone, so `Enumerate<Box<I>>` of two
//! functions that each spell their parameter `I` is one `Ty`: keyed by the
//! `Ty` alone, what one function asks of its own was taken for the other's
//! (#2133).

use rustc_hir::def_id::DefId;
use rustc_middle::ty::{Ty, TyCtxt};

/// A type's question, keyed where it holds (`typed_key`).
pub(crate) type TypedKey<'tcx> = (Option<DefId>, Ty<'tcx>);

/// `ty`, a type of `owner`'s body, keyed where its question holds: `None`
/// of a type of no type parameter, every value of it the crate's; else
/// `owner`'s typeck root, the function whose body it's of. Not the impl or
/// trait above it where `ty` names only theirs: no shape that compiles
/// shows an impl's methods sharing a fact of their `Filter<I, _>` (#2401,
/// whose fix brings that back, with its test). The one rule of which item
/// a type's parameters belong to, which each such set's writers and
/// readers ask.
pub(crate) fn typed_key<'tcx>(tcx: TyCtxt<'tcx>, owner: DefId, ty: Ty<'tcx>) -> TypedKey<'tcx> {
    // Of any type or const parameter, `[u32; N]`'s `N` too.
    let of_params = rustc_middle::ty::TypeVisitableExt::has_non_region_param(&ty);
    (of_params.then(|| tcx.typeck_root_def_id(owner)), ty)
}
