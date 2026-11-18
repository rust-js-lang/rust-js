//! What std's signatures can't say of what a call writes, named here and
//! nowhere else (#2112 B, ADR 1174): the reads, the pointers given, a
//! `RefCell`'s `borrow_mut()`.

use super::*;
use crate::lower::recognition::{StdItem, deref_call, is_std_def};

/// Whether a call of std's `callee` writes nothing through what it's given,
/// what its signature can't say: a `RefCell`'s `borrow()`, which marks the
/// cell borrowed and changes nothing it holds; a `Ref`'s or an `Rc`'s
/// `Deref`, which gives what it points to; a step of a range of integers or
/// a slice's iterator the walk knows (`range_step`), whose start only grows,
/// so what's known of its least and its end holds (ADR 1168, #1948); and
/// `IndexMut::index_mut` and `DerefMut::deref_mut`, which give a pointer
/// into what they're given and write nothing (`keeps_vec_length`); and a
/// `RefCell`'s `borrow_mut()`, which marks it borrowed, its writes through
/// the guard it gives, which are asked of where they are. Where it
/// runs no code but std's: a call that does is asked of its code.
pub(super) fn writes_nothing<'tcx>(tcx: TyCtxt<'tcx>, callee: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    reads_only(tcx, callee, args)
        || keeps_vec_length(tcx, callee)
        || refcell_borrow(tcx, callee) == Some(true)
        || guard_deref(tcx, callee, args)
        || args.types().any(|t| is_a_rest(tcx, t))
}

/// Of a projection std's named, how many pointers down from its first
/// argument what it gives points into: an index of a `Vec`'s or a
/// `VecDeque`'s, a `Vec`'s `Deref` to its slice, a `RefCell`'s
/// `borrow()` or `borrow_mut()`, one, into what it's given points to; a
/// guard's or an `Rc`'s `Deref`, two, into what what it's given points
/// to points to. It stores nothing into what it's given.
pub(super) fn projection_depth<'tcx>(
    tcx: TyCtxt<'tcx>,
    callee: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<usize> {
    if slice_length(tcx, callee, args) == Some(SliceLength::Index)
        || vec_deref(tcx, callee, args)
        || refcell_borrow(tcx, callee).is_some()
    {
        Some(1)
    } else if guard_deref(tcx, callee, args) || rc_deref(tcx, callee, args) {
        Some(2)
    } else {
        None
    }
}

/// Whether a call is a `Vec`'s `Deref` or `DerefMut`, to its slice.
fn vec_deref<'tcx>(tcx: TyCtxt<'tcx>, callee: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    deref_call(tcx, callee)
        && args
            .types()
            .next()
            .is_some_and(|ty| matches!(ty.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::Vec)))
}

/// Whether a call is the `Deref` or the `DerefMut` of a `RefCell`'s
/// guard, a `Ref` or a `RefMut`, std's: what the cell holds, given.
fn guard_deref<'tcx>(tcx: TyCtxt<'tcx>, callee: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    deref_call(tcx, callee)
        && args.types().next().is_some_and(|ty| {
            matches!(ty.kind(), ty::Adt(adt, _)
                if is_std_def(tcx, adt.did(), StdItem::RefCellRef) || is_std_def(tcx, adt.did(), StdItem::RefCellRefMut))
        })
}

/// Whether a call of std's `callee` runs none of the code of what the
/// `Vec`, the slice, the cell or the `Rc` it's given holds, whatever
/// that is: a read `reads_only` names (a step of a slice's iterator, an
/// `Rc`'s `Deref`), an index, a length or a push of a `Vec`'s
/// (`slice_length`), a `RefCell`'s `borrow()` or `borrow_mut()`, the
/// `Deref` or `DerefMut` of the guard it gives, and a `Vec`'s to its
/// slice.
pub(super) fn runs_none_of_what_it_holds<'tcx>(
    tcx: TyCtxt<'tcx>,
    callee: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> bool {
    reads_only(tcx, callee, args)
        || slice_length(tcx, callee, args).is_some()
        || refcell_borrow(tcx, callee).is_some()
        || guard_deref(tcx, callee, args)
        || vec_deref(tcx, callee, args)
}

/// Whether `ty` is rust-js's rest arguments, `dyn js::Rest<T>`
/// (`bindings::is_rest_value`): JS's arguments array, which std's code
/// given it, its `Deref` to the slice it is, reads and runs nothing of.
pub(super) fn is_a_rest<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    let shared = |of| Ty::new_imm_ref(tcx, tcx.lifetimes.re_erased, of);
    match ty.kind() {
        ty::Dynamic(..) => crate::lower::bindings::is_rest_value(tcx, shared(ty)),
        _ => crate::lower::bindings::is_rest_value(tcx, ty),
    }
}

/// Whether a call of std's `callee` only reads what it's given: a
/// `RefCell`'s `borrow()`, a `Ref`'s or an `Rc`'s `Deref`, a length, an
/// `is_empty()`, an index's read, or a step of a range or a slice's
/// iterator the walk knows (`range_step`): `writes_nothing` but the
/// pointers `keeps_vec_length` gives (the MIR intervals' `may_write_cells`
/// asks it too).
pub(in crate::lower) fn reads_only<'tcx>(tcx: TyCtxt<'tcx>, callee: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    refcell_borrow(tcx, callee) == Some(false)
        || ref_deref(tcx, callee, args)
        || rc_deref(tcx, callee, args)
        || range_step(tcx, callee, args).is_some()
        || matches!(
            slice_length(tcx, callee, args),
            Some(SliceLength::Len | SliceLength::IsEmpty | SliceLength::Index)
        )
}

/// Whether a call of `def_id` keeps the length of the `Vec` it's given:
/// `IndexMut::index_mut` and `DerefMut::deref_mut`, which give a pointer
/// into it and change nothing (the MIR intervals' `called_on`, `guards`
/// and `tables` ask it too).
pub(in crate::lower) fn keeps_vec_length(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    (tcx.trait_of_assoc(def_id)).is_some_and(|t| tcx.is_lang_item(t, LangItem::IndexMut)) || deref_mut_call(tcx, def_id)
}
