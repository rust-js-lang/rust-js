//! What std's signatures can't say of what a call writes, named here and
//! nowhere else (#2112 B, ADR 1174): the reads, the pointers given, a
//! `RefCell`'s `borrow_mut()`.

use super::*;

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
    reads_only(tcx, callee, args) || keeps_vec_length(tcx, callee) || refcell_borrow(tcx, callee) == Some(true)
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
