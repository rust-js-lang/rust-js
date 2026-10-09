//! The `Rc`s whose counts the crate reads (ADR 0320): only those are
//! `{ value, strong, weak }`; every other is the value it points at (ADR 0023).

use crate::lower::recognition::{rc_pointee, reads_rc_count, weak_pointee};
use crate::lower::{Body, fn_def};
use rustc_hir::def::DefKind;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::{Ty, TyCtxt, TypeVisitableExt};
use std::collections::HashSet;

/// What an `Rc` points at, of those counted: each type, or every one.
#[derive(Default)]
pub struct Counted<'tcx> {
    /// Every `Rc` is: one counted is of a type a generic function knows only
    /// as its parameter, or a generic `Rc` might be one counted.
    pub all: bool,
    pub types: HashSet<Ty<'tcx>>,
}

impl<'tcx> Counted<'tcx> {
    /// Does the crate count any `Rc`? Then dropping one runs (ADR 0098).
    pub fn any(&self) -> bool {
        self.all || !self.types.is_empty()
    }

    /// Is an `Rc` of `pointee` counted?
    pub fn counts(&self, tcx: TyCtxt<'tcx>, pointee: Ty<'tcx>) -> bool {
        self.all || self.types.contains(&tcx.erase_and_anonymize_regions(pointee))
    }
}

/// The types whose `Rc`s the crate reads a count of, `strong_count`,
/// `get_mut`, `try_unwrap` and the rest, or makes a `Weak` of: one of
/// those is counted as Rust counts it, where it's made, cloned and dropped.
pub(super) fn counted_rcs<'tcx>(tcx: TyCtxt<'tcx>, all_bodies: &[&Body<'tcx>]) -> Counted<'tcx> {
    let mut types: HashSet<Ty<'tcx>> = HashSet::new();
    let mut generic_rc = false;
    let found = |ty: Ty<'tcx>, types: &mut HashSet<Ty<'tcx>>, generic_rc: &mut bool| {
        for part in ty.walk().filter_map(|part| part.as_type()) {
            if let Some(pointee) = weak_pointee(tcx, part) {
                types.insert(tcx.erase_and_anonymize_regions(pointee));
            }
            if rc_pointee(tcx, part).is_some_and(|pointee| pointee.has_non_region_param()) {
                *generic_rc = true;
            }
        }
    };
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            found(expr.ty, &mut types, &mut generic_rc);
            if let ExprKind::Call { fun, .. } = expr.kind
                && let Some((def_id, args)) = fn_def(body.thir[fun].ty)
                && reads_rc_count(tcx, def_id)
            {
                types.insert(tcx.erase_and_anonymize_regions(args.type_at(0)));
            }
        }
    }
    // A field's `Weak`, which may be made where no expression names it.
    for id in tcx.hir_crate_items(()).definitions() {
        if matches!(tcx.def_kind(id), DefKind::Struct | DefKind::Enum | DefKind::Union) {
            for field in tcx.adt_def(id).all_fields() {
                found(
                    tcx.type_of(field.did).instantiate_identity().skip_normalization(),
                    &mut types,
                    &mut generic_rc,
                );
            }
        }
    }
    let all = types.iter().any(|t| t.has_non_region_param()) || !types.is_empty() && generic_rc;
    Counted { all, types }
}
