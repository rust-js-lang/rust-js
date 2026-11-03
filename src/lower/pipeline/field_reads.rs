//! An untagged enum's method that reads the same field of each variant's
//! value, lowered before its callers, so each call is that read of the
//! value it's called on where its JS is the read alone (rd317,
//! `untagged::field_reads`).

use super::super::Body;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;
use std::collections::HashSet;

/// The crate's methods of untagged enums that read the same field of
/// each variant's value (`untagged::field_reads::reads_each_variants_field`).
pub(super) fn found<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> HashSet<DefId> {
    (bodies.iter())
        .filter(|body| super::super::untagged::field_reads::reads_each_variants_field(tcx, body))
        .map(|body| body.def_id)
        .collect()
}

/// `work` with each of `methods` first, so its JS is known where its
/// callers are lowered; the rest in their order, which is no order of
/// what the modules write.
pub(super) fn first<T>(work: &mut [(DefId, T, Option<usize>)], methods: &HashSet<DefId>) {
    work.sort_by_key(|&(id, _, copy)| !(methods.contains(&id) && copy.is_none()));
}
