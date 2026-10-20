//! The `Rc`s whose counts the crate reads, or that point at a value with a
//! destructor, which the last of them drops (ADR 0320): only those are
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
/// `get_mut`, `try_unwrap` and the rest, or makes a `Weak` of, and those
/// whose drop runs a destructor, `runs`, which the last of them drops: one of
/// those is counted as Rust counts it, where it's made, cloned and dropped.
/// What a type drops depends on which `Rc`s are counted, so the second to a
/// fixed point.
/// And an `Rc` of a value of a type parameter's, `Rc<RefCell<Buffer<I>>>`,
/// where `params_may_drop`, a type may have a destructor: what it's made of
/// may be one whose last clone drops it, so every `Rc` is (ADR 0494).
/// And every `Rc` is where `shares_rcs`, a library or a crate using one,
/// whose `Rc`s another crate shares, as one representation (ADR 0523).
pub(super) fn counted_rcs<'tcx>(
    tcx: TyCtxt<'tcx>,
    all_bodies: &[&Body<'tcx>],
    (params_may_drop, shares_rcs): (bool, bool),
    runs: impl Fn(&Counted<'tcx>, Ty<'tcx>) -> bool,
) -> Counted<'tcx> {
    let mut types: HashSet<Ty<'tcx>> = HashSet::new();
    let mut generic_rc = false;
    // Whether an `Rc` holds a type parameter's value itself, not one it
    // points at.
    let mut holds_param = false;
    // What each `Rc` of a known type points at.
    let mut pointees: HashSet<Ty<'tcx>> = HashSet::new();
    let mut found = |ty: Ty<'tcx>, types: &mut HashSet<Ty<'tcx>>, generic_rc: &mut bool| {
        for part in ty.walk().filter_map(|part| part.as_type()) {
            if let Some(pointee) = weak_pointee(tcx, part) {
                types.insert(tcx.erase_and_anonymize_regions(pointee));
            }
            match rc_pointee(tcx, part) {
                Some(pointee) if pointee.has_non_region_param() => {
                    *generic_rc = true;
                    holds_param |= owns_param(pointee);
                }
                Some(pointee) => {
                    pointees.insert(tcx.erase_and_anonymize_regions(pointee));
                }
                None => {}
            }
        }
    };
    // Each `Rc` made another's, `Rc<RefCell<Logger>>` an
    // `Rc<RefCell<dyn Listener>>`: the two share their counts.
    let mut made_others: HashSet<(Ty<'tcx>, Ty<'tcx>)> = HashSet::new();
    for body in all_bodies {
        for expr in body.thir.exprs.iter() {
            found(expr.ty, &mut types, &mut generic_rc);
            if let ExprKind::PointerCoercion { source, .. } = expr.kind
                && let (Some(from), Some(to)) = (rc_pointee(tcx, body.thir[source].ty), rc_pointee(tcx, expr.ty))
            {
                made_others.insert((
                    tcx.erase_and_anonymize_regions(from),
                    tcx.erase_and_anonymize_regions(to),
                ));
            }
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
    let mut counted = Counted { all: false, types };
    loop {
        let mut dropping: Vec<Ty<'tcx>> = (pointees.iter())
            .filter(|&&pointee| !counted.types.contains(&pointee) && runs(&counted, pointee))
            .copied()
            .collect();
        // An `Rc` made a counted one is counted: its counts are the other's.
        dropping.extend(
            (made_others.iter())
                .filter(|(from, to)| counted.types.contains(to) && !counted.types.contains(from))
                .map(|&(from, _)| from),
        );
        if dropping.is_empty() {
            break;
        }
        counted.types.extend(dropping);
    }
    counted.all = shares_rcs
        || counted.types.iter().any(|t| t.has_non_region_param())
        || !counted.types.is_empty() && generic_rc
        || params_may_drop && holds_param;
    counted
}

/// Whether `ty` holds a value of a type parameter, or of an associated type
/// of one, itself: not behind a reference, which drops nothing.
fn owns_param(ty: Ty<'_>) -> bool {
    let mut walk = ty.walk();
    while let Some(part) = walk.next() {
        match part.as_type().map(|t| *t.kind()) {
            Some(rustc_middle::ty::Ref(..)) => walk.skip_current_subtree(),
            Some(rustc_middle::ty::Param(_) | rustc_middle::ty::Alias(..)) => return true,
            _ => {}
        }
    }
    false
}
