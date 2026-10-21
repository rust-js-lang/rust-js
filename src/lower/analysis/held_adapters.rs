//! The kinds of std's iterator adapters the crate's own types hold (ADR
//! 0542): `Enumerate` of itertools' `Positions { iter: Enumerate<I>, f }`.
//! A kind held is explicit wherever the crate makes one: it keeps its
//! source and its state, so what's asked of a held one later, its size
//! hint, its `Debug`, its back, is there.

use crate::lower::recognition::is_iter_adapter_def;
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;
use std::collections::HashSet;

/// Each adapter's definition, `Enumerate`'s, that a field of the crate's
/// structs, enums or unions holds, through whatever holds it.
pub(super) fn held_adapters(tcx: TyCtxt<'_>) -> HashSet<DefId> {
    let mut held = HashSet::new();
    for id in tcx.hir_crate_items(()).definitions() {
        if !matches!(tcx.def_kind(id), DefKind::Struct | DefKind::Enum | DefKind::Union) {
            continue;
        }
        for field in tcx.adt_def(id).all_fields() {
            let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
            for part in ty.walk().filter_map(|part| part.as_type()) {
                if let ty::Adt(adapter, _) = part.kind()
                    && is_iter_adapter_def(tcx, adapter.did())
                {
                    held.insert(adapter.did());
                }
            }
        }
    }
    held
}
