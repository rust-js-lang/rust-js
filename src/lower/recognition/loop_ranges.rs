//! What a loop over a range of integers or a slice's items calls of it, and
//! an integer's `min` and `max`, for the MIR intervals (ADR 1137, #1672 M3).

use super::*;

/// What a `for` loop over a range of integers calls of it, `for i in a..b`'s
/// desugaring: its `into_iter()`, the range itself, and each `next()`,
/// which gives its start and steps it, never past its end; and `rev()`,
/// whose `next()` gives its end's predecessor, never below its start. By
/// the lang items the desugaring calls, of a `Range` of an integer type,
/// or `core`'s `Rev` of one (ADR 1137).
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum RangeStep {
    IntoIter,
    Next,
    Rev,
    /// `step_by(n)` of a range, whose values are its own, each below its end.
    StepBy,
    /// A slice's `iter()` or `iter_mut()`, and its `enumerate()`, each
    /// index below the slice's length.
    SliceIter,
    Enumerate,
}

/// Whether `ty` is a `Range` or a `RangeInclusive` of an integer type, a
/// slice's `Iter` or `IterMut`, `core`'s `Enumerate` of one, `StepBy` of a range, or `Rev` of a
/// range or an `Enumerate`, `(0..n).rev()`.
pub(in crate::lower) fn range_of_integers<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Adt(adt, inner) if core_adapter(tcx, adt.did(), "Rev") => inner
            .types()
            .next()
            .is_some_and(|inner| plain_range_of_integers(tcx, inner) || enumerated_slice(tcx, inner)),
        ty::Adt(adt, inner) if core_adapter(tcx, adt.did(), "StepBy") => inner
            .types()
            .next()
            .is_some_and(|inner| plain_range_of_integers(tcx, inner)),
        _ => plain_range_of_integers(tcx, ty) || enumerated_slice(tcx, ty) || slice_iter(tcx, ty),
    }
}

/// Whether `ty` is a `Range` or a `RangeInclusive` of an integer type.
fn plain_range_of_integers<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, item)
        if (tcx.is_lang_item(adt.did(), LangItem::Range) || tcx.is_lang_item(adt.did(), LangItem::RangeInclusiveStruct))
            && item.types().next().is_some_and(|item| item.is_integral()))
}

/// Whether `id` is `core`'s struct `name`, an iterator adapter's.
fn core_adapter(tcx: TyCtxt<'_>, id: DefId, name: &str) -> bool {
    tcx.item_name(id).as_str() == name && of_std_crate(tcx, id, StdCrate::Core)
}

/// Whether `ty` is a slice's iterator, `core::slice`'s `Iter` or `IterMut`.
fn slice_iter<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _)
        if (core_adapter(tcx, adt.did(), "Iter") || core_adapter(tcx, adt.did(), "IterMut"))
            && tcx.def_path_str(adt.did()).contains("slice"))
}

/// Whether `ty` is `Enumerate` of a slice's iterator.
fn enumerated_slice<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, inner) if core_adapter(tcx, adt.did(), "Enumerate")
        && inner.types().next().is_some_and(|inner| slice_iter(tcx, inner)))
}

pub(in crate::lower) fn range_step<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<RangeStep> {
    let step = if tcx.is_lang_item(def_id, LangItem::IntoIterIntoIter) {
        RangeStep::IntoIter
    } else if tcx.is_lang_item(def_id, LangItem::IteratorNext) {
        RangeStep::Next
    } else if (tcx.trait_of_assoc(def_id)).is_some_and(|t| tcx.is_diagnostic_item(sym::Iterator, t)) {
        match tcx.item_name(def_id).as_str() {
            "rev" => RangeStep::Rev,
            "step_by" => RangeStep::StepBy,
            "enumerate" => RangeStep::Enumerate,
            _ => return None,
        }
    } else if (tcx.inherent_impl_of_assoc(def_id)).is_some_and(|impl_| {
        tcx.type_of(impl_)
            .instantiate_identity()
            .skip_normalization()
            .is_slice()
    }) && matches!(tcx.item_name(def_id).as_str(), "iter" | "iter_mut")
    {
        return Some(RangeStep::SliceIter);
    } else {
        return None;
    };
    let given = args.types().next()?;
    // `rev()` of a range or an enumerated slice, not of a `Rev`.
    let of_integers = match step {
        RangeStep::Rev => plain_range_of_integers(tcx, given) || enumerated_slice(tcx, given),
        RangeStep::Enumerate => slice_iter(tcx, given),
        RangeStep::StepBy => plain_range_of_integers(tcx, given),
        _ => range_of_integers(tcx, given),
    };
    of_integers.then_some(step)
}

/// Whether a call is an integer's `min` or `max`, `Ord`'s or `cmp`'s, by
/// their diagnostic items: of `max`, true (ADR 1137).
pub(in crate::lower) fn integer_min_max<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<bool> {
    let max = match tcx.get_diagnostic_name(def_id)?.as_str() {
        "cmp_ord_min" | "cmp_min" => false,
        "cmp_ord_max" | "cmp_max" => true,
        _ => return None,
    };
    args.types().next().is_some_and(|ty| ty.is_integral()).then_some(max)
}

/// Whether a call is text's `len()`, `str`'s or a `String`'s own: its
/// bytes, at most what an allocation holds (ADR 1137).
pub(in crate::lower) fn text_length(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.item_name(def_id).as_str() == "len"
        && tcx.inherent_impl_of_assoc(def_id).is_some_and(|impl_| {
            let owner = tcx.type_of(impl_).instantiate_identity().skip_normalization();
            owner.is_str() || matches!(owner.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::String))
        })
}

/// Whether a call is a slice's `get(index)` of a `usize`, a `Vec`'s too
/// through its `Deref`: its `Some` of an index below the slice's length
/// (ADR 1137).
pub(in crate::lower) fn slice_get<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    tcx.item_name(def_id).as_str() == "get"
        && (tcx.inherent_impl_of_assoc(def_id)).is_some_and(|impl_| {
            tcx.type_of(impl_)
                .instantiate_identity()
                .skip_normalization()
                .is_slice()
        })
        && args.types().nth(1).is_some_and(|index| index.is_usize())
}

/// Whether a call is an `Option`'s `cloned()` or `copied()`: `Some` where
/// what it's given is (ADR 1137).
pub(in crate::lower) fn option_copied(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(tcx.item_name(def_id).as_str(), "cloned" | "copied")
        && (tcx.inherent_impl_of_assoc(def_id)).is_some_and(|impl_| {
            matches!(tcx.type_of(impl_).instantiate_identity().skip_normalization().kind(),
                ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option))
        })
}
