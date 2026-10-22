//! The thread-locals that are their module's `let` or `const` (ADR 0270),
//! and the cells that are their function's `let` (ADR 0287).

use super::super::Body;
use super::super::body_queries::strip;
use super::super::fn_def;
use super::super::recognition::{is_local_borrow, local_key_access};
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::LocalDefId;
use std::collections::HashSet;

/// The thread-locals only read and set, by `get`, `set`, `with_borrow` and
/// `with_borrow_mut`, and not public: nothing shares their cell, so each
/// is its module's variable, without a `{ value }`. A `with_borrow`'s
/// closure must be `quiet` of its key, borrowing nothing that may be its
/// cell, as nothing counts its borrows (ADRs 0328, 0580).
pub(super) fn plain_thread_locals<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[&Body<'tcx>],
    keys: impl Iterator<Item = LocalDefId>,
    quiet: &dyn Fn(LocalDefId, LocalDefId) -> bool,
) -> HashSet<LocalDefId> {
    let mut plain: HashSet<LocalDefId> = keys.filter(|&key| !tcx.visibility(key).is_public()).collect();
    for body in bodies {
        let thir = &body.thir;
        // Each thread-local read or set where it's read, `KEY` of `KEY.get()`.
        let mut accessed = HashSet::new();
        for expr in thir.exprs.iter() {
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && let Some((id, _)) = fn_def(thir[fun].ty)
                && local_key_access(tcx, id).is_some()
                && let Some(&first) = args.first()
                && let ExprKind::Borrow { arg, .. } = thir[strip(thir, first)].kind
                && (!is_local_borrow(tcx, id)
                    || matches!((&thir[strip(thir, args[1])].kind, &thir[strip(thir, arg)].kind),
                        (ExprKind::Closure(c), ExprKind::NamedConst { def_id: key, .. })
                            if key.as_local().is_some_and(|key| quiet(c.closure_id, key))))
            {
                accessed.insert(strip(thir, arg));
            }
        }
        for (id, expr) in thir.exprs.iter_enumerated() {
            if let ExprKind::NamedConst { def_id, .. } = expr.kind
                && let Some(key) = def_id.as_local()
                && !accessed.contains(&id)
            {
                plain.remove(&key);
            }
        }
    }
    plain
}
