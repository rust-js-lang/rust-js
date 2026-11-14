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
    /// `skip(n)` of a range, a slice's iterator or its `enumerate()`: the
    /// rest of its values, each below its end (#1814).
    Skip,
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
        ty::Adt(adt, inner) if core_adapter(tcx, adt.did(), "Skip") => {
            inner.types().next().is_some_and(|inner| skippable(tcx, inner))
        }
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

/// Whether `ty` is what a `skip(n)` of keeps its values below its end: a
/// range of integers, a slice's iterator or its `enumerate()`.
fn skippable<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    plain_range_of_integers(tcx, ty) || enumerated_slice(tcx, ty) || slice_iter(tcx, ty)
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
            "skip" => RangeStep::Skip,
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
        RangeStep::Skip => skippable(tcx, given),
        _ => range_of_integers(tcx, given),
    };
    of_integers.then_some(step)
}

/// Whether a call is std's `From<bool>` of an integer, `usize::from(apple)`:
/// 0 or 1 (#1863).
pub(in crate::lower) fn integer_from_bool<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    (tcx.trait_of_assoc(def_id)).is_some_and(|t| is_std_def(tcx, t, StdItem::From))
        && args.types().next().is_some_and(|of| of.is_integral())
        && args.types().nth(1).is_some_and(|from| from.is_bool())
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

/// Whether a call is a `RefCell`'s `borrow()` or `borrow_mut()`, std's:
/// `Some(true)` of `borrow_mut` (ADR 1137, #1741).
pub(in crate::lower) fn refcell_borrow(tcx: TyCtxt<'_>, def_id: DefId) -> Option<bool> {
    let mutable = match tcx.item_name(def_id).as_str() {
        "borrow" => false,
        "borrow_mut" => true,
        _ => return None,
    };
    (tcx.inherent_impl_of_assoc(def_id))
        .filter(|impl_| {
            matches!(tcx.type_of(*impl_).instantiate_identity().skip_normalization().kind(),
                ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::RefCell))
        })
        .map(|_| mutable)
}

/// The indexes a call checks against its receiver's length, its first
/// argument's, by argument, each below it or, of `insert`, at most it:
/// `v[i]`'s `Index::index` and `index_mut` of a `Vec`, a `VecDeque`, a
/// slice or an array; a slice's `swap(i, j)`; and a `Vec`'s `swap_remove(i)`,
/// `remove(i)` and `insert(i, x)`, std's (#1672 M3, step 5).
pub(in crate::lower) fn call_indexes<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<Vec<(usize, bool)>> {
    let vec = |ty: Ty<'tcx>, items: &[StdItem]| matches!(ty.kind(), ty::Adt(adt, _) if items.iter().any(|&item| is_std_def(tcx, adt.did(), item)));
    if (tcx.trait_of_assoc(def_id))
        .is_some_and(|t| tcx.is_lang_item(t, LangItem::Index) || tcx.is_lang_item(t, LangItem::IndexMut))
    {
        let (holder, index) = (args.type_at(0), args.type_at(1));
        let held =
            matches!(holder.kind(), ty::Slice(_) | ty::Array(..)) || vec(holder, &[StdItem::Vec, StdItem::VecDeque]);
        return (held && index.is_usize()).then(|| vec![(1, true)]);
    }
    let impl_ = tcx.inherent_impl_of_assoc(def_id)?;
    let of = tcx.type_of(impl_).instantiate_identity().skip_normalization();
    match tcx.item_name(def_id).as_str() {
        "swap" if of.is_slice() => Some(vec![(1, true), (2, true)]),
        "swap_remove" | "remove" if vec(of, &[StdItem::Vec]) => Some(vec![(1, true)]),
        "insert" if vec(of, &[StdItem::Vec]) => Some(vec![(1, false)]),
        _ => None,
    }
}

/// Of `v[a..b]`'s `Index::index` or `index_mut` of a `Vec`, a slice or an
/// array, by a range of `usize`, `..`, `a..`, `..b`, `..=b` or `a..b`, and
/// of a `Vec`'s `drain(a..b)` (#1815): the range's argument, its bounds
/// checked against the receiver's length (#1672 M3, step 5). Not text's,
/// whose bounds are on its characters too.
pub(in crate::lower) fn range_index<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<usize> {
    let vec = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Vec));
    let ranged = |index: Ty<'tcx>| {
        matches!(index.kind(), ty::Adt(adt, args) if range_index_bounds(tcx, adt.did()).is_some()
            && args.types().all(|t| t.is_usize()))
    };
    let indexing = (tcx.trait_of_assoc(def_id))
        .is_some_and(|t| tcx.is_lang_item(t, LangItem::Index) || tcx.is_lang_item(t, LangItem::IndexMut));
    if !indexing {
        let drain = tcx.item_name(def_id).as_str() == "drain"
            && (tcx.inherent_impl_of_assoc(def_id))
                .is_some_and(|impl_| vec(tcx.type_of(impl_).instantiate_identity().skip_normalization()));
        return (drain && args.types().last().is_some_and(ranged)).then_some(1);
    }
    let (holder, index) = (args.type_at(0), args.type_at(1));
    let held = matches!(holder.kind(), ty::Slice(_) | ty::Array(..)) || vec(holder);
    (held && ranged(index)).then_some(1)
}

/// A range's bounds, by their fields: its start's, and its end's with
/// whether the end is in it (`range_index_bounds`).
pub(in crate::lower) type RangeIndexBounds = (Option<usize>, Option<(usize, bool)>);

/// Which bounds a range of `..`, `a..`, `..b`, `..=b` or `a..b` has, by its
/// struct: its start's field, its end's and whether the end is in it.
pub(in crate::lower) fn range_index_bounds(tcx: TyCtxt<'_>, adt: DefId) -> Option<RangeIndexBounds> {
    let lang = |item| tcx.is_lang_item(adt, item);
    if lang(LangItem::RangeFull) {
        Some((None, None))
    } else if lang(LangItem::RangeFrom) {
        Some((Some(0), None))
    } else if lang(LangItem::RangeTo) {
        Some((None, Some((0, false))))
    } else if lang(LangItem::RangeToInclusive) {
        Some((None, Some((0, true))))
    } else if lang(LangItem::Range) {
        Some((Some(0), Some((1, false))))
    } else {
        None
    }
}

/// Whether a call keeps the length of the `Vec` it's given `&mut`:
/// `IndexMut::index_mut`, `v[i] = x`, and `DerefMut::deref_mut`, a slice
/// of its items (#1672 M3, step 5).
pub(in crate::lower) fn keeps_vec_length(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    (tcx.trait_of_assoc(def_id)).is_some_and(|t| tcx.is_lang_item(t, LangItem::IndexMut))
        || tcx.is_diagnostic_item(Symbol::intern("deref_mut_method"), def_id)
}

/// Whether a call is `DerefMut::deref_mut`.
pub(in crate::lower) fn deref_mut_call(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.is_diagnostic_item(Symbol::intern("deref_mut_method"), def_id)
}

/// Whether a call is `Deref::deref` or `DerefMut::deref_mut`.
pub(in crate::lower) fn deref_call(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.is_diagnostic_item(Symbol::intern("deref_method"), def_id)
        || tcx.is_diagnostic_item(Symbol::intern("deref_mut_method"), def_id)
}

/// Whether a call is a thread-local's `with`, `with_borrow` or
/// `with_borrow_mut`, std's, which gives what its closure gives (#1701).
pub(in crate::lower) fn local_key_with(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    matches!(
        tcx.item_name(def_id).as_str(),
        "with" | "with_borrow" | "with_borrow_mut"
    ) && (tcx.inherent_impl_of_assoc(def_id)).is_some_and(|impl_| {
        matches!(tcx.type_of(impl_).instantiate_identity().skip_normalization().kind(),
                ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::LocalKey))
    })
}

/// Whether a call is a `Ref`'s `Deref`, std's, which lends what its
/// `RefCell` holds and runs nothing (ADR 1137, #1741).
pub(in crate::lower) fn ref_deref<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    tcx.is_diagnostic_item(Symbol::intern("deref_method"), def_id)
        && args
            .types()
            .next()
            .is_some_and(|of| matches!(of.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::RefCellRef)))
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
