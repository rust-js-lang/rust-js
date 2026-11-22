//! Whether two cells' types may be one cell (`may_be_cell`, ADRs 0397,
//! 0852, 0960), and where a struct holds an unsized value in place
//! (`unsized_in_place`). Split from cell_borrows.rs by concept.

use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt};

/// May `a` and `b`, two cells' types, be one cell? As ADR 0852's `may_be`
/// says of two types, a type parameter's any; and a slice's an array's of
/// its items, and a `dyn`'s any type where a value may be made one in
/// place, as one made the other, an `Rc<RefCell<Circle>>` an
/// `Rc<RefCell<dyn Shape>>`, shares its cell (ADR 0397): the cell's own
/// value, and a struct's last field's, `Framed<dyn Shape>`'s. Elsewhere,
/// in a `Vec`'s items or what an `Rc` points at, a `dyn` is only a `dyn`
/// of its trait: a `RefCell<Vec<Rc<dyn Fn()>>>` is no `RefCell<Vec<Rc<Node>>>`
/// (ADR 0960).
pub(crate) fn may_be_cell<'tcx>(tcx: TyCtxt<'tcx>, a: Ty<'tcx>, b: Ty<'tcx>) -> bool {
    may_be_cell_at(tcx, (a, b), true)
}

/// `may_be_cell` of `a` and `b`, where `unsized_here`, a `dyn` there may
/// be any type made one in place that may implement its trait, as rustc's
/// own selection says of a concrete type (`may_be_made`, #1006): not a
/// `Framed<Circle>` of a `dyn Named`; a `dyn` or a type parameter may be
/// either.
fn may_be_cell_at<'tcx>(tcx: TyCtxt<'tcx>, (a, b): (Ty<'tcx>, Ty<'tcx>), unsized_here: bool) -> bool {
    match (a.kind(), b.kind()) {
        (ty::Dynamic(..), _) | (_, ty::Dynamic(..)) if unsized_here => {
            let (dyn_ty, other) = if matches!(a.kind(), ty::Dynamic(..)) {
                (a, b)
            } else {
                (b, a)
            };
            matches!(other.kind(), ty::Dynamic(..)) || may_be_made(tcx, other, dyn_ty)
        }
        (ty::Slice(x) | ty::Array(x, _), ty::Slice(y) | ty::Array(y, _)) => may_be_cell_at(tcx, (*x, *y), false),
        _ => match (crate::analysis::head(tcx, a), crate::analysis::head(tcx, b)) {
            (Some((x, of_x)), Some((y, of_y))) => {
                let tail = unsized_in_place(tcx, a);
                x == y
                    && of_x.len() == of_y.len()
                    && (of_x.iter().zip(&of_y).enumerate())
                        .all(|(i, (&x, &y))| may_be_cell_at(tcx, (x, y), tail == Some(i)))
            }
            _ => true,
        },
    }
}

/// Whether a value of `ty` may be made `dyn_ty`, a `dyn` of a trait: not
/// where `ty` is concrete, no type parameter or alias in it, and rustc's
/// selection says no impl of the `dyn`'s trait may apply to it, though
/// generic. Ambiguous, or of a `dyn` of no trait but auto ones, it may
/// (#1006): what may be a cell over-approximates.
fn may_be_made<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, dyn_ty: Ty<'tcx>) -> bool {
    use rustc_trait_selection::infer::TyCtxtInferExt;
    use rustc_trait_selection::traits::query::evaluate_obligation::InferCtxtExt;
    let ty::Dynamic(predicates, ..) = dyn_ty.kind() else {
        return true;
    };
    let Some(principal) = predicates.principal() else {
        return true;
    };
    let ty = tcx.erase_and_anonymize_regions(ty);
    if crate::generic_facts::may_be_any_type(ty) || ty.has_aliases() || ty.has_infer() {
        return true;
    }
    let trait_ref = tcx.erase_and_anonymize_regions(principal.with_self_ty(tcx, ty).skip_binder());
    let infcx = tcx.infer_ctxt().build(ty::TypingMode::PostAnalysis);
    let obligation = rustc_trait_selection::traits::Obligation::new(
        tcx,
        rustc_trait_selection::traits::ObligationCause::dummy(),
        ty::ParamEnv::empty(),
        trait_ref,
    );
    infcx.predicate_may_hold(&obligation)
}

/// Which of `ty`'s type arguments, a struct's, counted among its types as
/// `head` lists them, a value of it may be made unsized by in place, as
/// Rust's `Unsize` of a struct is: the one its last field is, or is a
/// struct's tail of, `RefCell<T>`'s `T` through its `UnsafeCell<T>`. Not
/// a `Vec`'s or an `Rc`'s items, behind a pointer, which no value of it is
/// made another in place (ADR 0960).
pub(crate) fn unsized_in_place<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<usize> {
    let ty::Adt(adt, args) = *ty.kind() else {
        return None;
    };
    let at = unsized_tail(tcx, adt)?;
    Some(args.iter().take(at).filter(|arg| arg.as_type().is_some()).count())
}

/// The generic argument of `adt` its last field is the tail of, by its
/// index among all of them.
fn unsized_tail<'tcx>(tcx: TyCtxt<'tcx>, adt: ty::AdtDef<'tcx>) -> Option<usize> {
    if !adt.is_struct() {
        return None;
    }
    let last = adt.non_enum_variant().fields.iter().last()?;
    let mut ty = tcx.type_of(last.did).instantiate_identity().skip_normalization();
    loop {
        match *ty.kind() {
            ty::Param(param) => return Some(param.index as usize),
            ty::Adt(inner, args) => ty = args.get(unsized_tail(tcx, inner)?)?.as_type()?,
            _ => return None,
        }
    }
}
