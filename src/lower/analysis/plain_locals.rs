//! The thread-locals that are their module's `let` or `const` (ADR 0270).

use super::super::Body;
use super::super::body_queries::strip;
use super::super::fn_def;
use super::super::recognition::local_key_access;
use rustc_middle::thir::ExprKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::LocalDefId;
use std::collections::{HashMap, HashSet};

/// The thread-locals only read and set, by `get`, `set`, `with_borrow` and
/// `with_borrow_mut`, in their own module, and not public: nothing shares
/// their cell, so each is its module's variable, without a `{ value }`. Each
/// says whether it's `set`, a `let`, or not, a `const`.
pub(super) fn plain_thread_locals<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &[&Body<'tcx>],
    keys: impl Iterator<Item = LocalDefId>,
) -> HashMap<LocalDefId, bool> {
    let mut plain: HashMap<LocalDefId, bool> = keys
        .filter(|&key| !tcx.visibility(key).is_public())
        .map(|key| (key, false))
        .collect();
    for body in bodies {
        let thir = &body.thir;
        let module = tcx.parent_module_from_def_id(body.def_id);
        // Each thread-local read or set where it's read, `KEY` of `KEY.get()`,
        // and those set.
        let mut accessed = HashSet::new();
        let mut set = HashSet::new();
        for expr in thir.exprs.iter() {
            if let ExprKind::Call { fun, ref args, .. } = expr.kind
                && let Some(sets) = fn_def(thir[fun].ty).and_then(|(id, _)| local_key_access(tcx, id))
                && let Some(&first) = args.first()
                && let ExprKind::Borrow { arg, .. } = thir[strip(thir, first)].kind
            {
                accessed.insert(strip(thir, arg));
                if sets {
                    set.insert(strip(thir, arg));
                }
            }
        }
        for (id, expr) in thir.exprs.iter_enumerated() {
            if let ExprKind::NamedConst { def_id, .. } = expr.kind
                && let Some(key) = def_id.as_local()
            {
                if !accessed.contains(&id) || tcx.parent_module_from_def_id(key) != module {
                    plain.remove(&key);
                } else if set.contains(&id)
                    && let Some(sets) = plain.get_mut(&key)
                {
                    *sets = true;
                }
            }
        }
    }
    plain
}
