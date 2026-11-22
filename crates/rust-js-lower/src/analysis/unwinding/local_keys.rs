//! A LocalKey's with/try_with callback, where its key is already made.
//! Initialization timing belongs to lazy_locals; callback unwinding belongs
//! to unwinding. This only joins those facts at the actual MIR call.

use super::*;
use crate::body_queries::assigned_once;
use crate::recognition::Std;
use rustc_middle::mir::{Local, Operand, Rvalue};
use rustc_span::def_id::LocalDefId;
use std::cell::OnceCell;

pub(super) struct Calls<'a, 'tcx> {
    mir: &'a crate::analysis::Mir<'tcx>,
    // Built only for a body that calls with, once across the fixed point.
    assigned: OnceCell<IndexMap<Local, Option<&'a Rvalue<'tcx>>>>,
}

impl<'a, 'tcx> Calls<'a, 'tcx> {
    pub(super) fn new(mir: &'a crate::analysis::Mir<'tcx>) -> Self {
        Self {
            mir,
            assigned: OnceCell::new(),
        }
    }

    /// The callback whose call is all this access runs. Unknown keys,
    /// lazy initializers and indirect function pointers remain unproved.
    pub(super) fn callback(
        &self,
        recognition: &Recognition<'_, 'tcx>,
        (callee, types): (DefId, ty::GenericArgsRef<'tcx>),
        operands: &[rustc_span::Spanned<Operand<'tcx>>],
        eager: &IndexSet<LocalDefId>,
    ) -> Option<(DefId, ty::GenericArgsRef<'tcx>)> {
        if !matches!(recognition.classify(callee, types), Some(Std::LocalWith { .. })) {
            return None;
        }
        let [key, callback] = operands else { return None };
        let assigned = self.assigned.get_or_init(|| assigned_once(&self.mir.body));
        let key = crate::analysis::mir_reads::referred::referred_key(
            &self.mir.promoted,
            |local| assigned.get(&local).copied().flatten(),
            &key.node,
        )?;
        if !key.as_local().is_some_and(|key| eager.contains(&key)) {
            return None;
        }
        let callback = callback
            .node
            .ty(&self.mir.body.local_decls, recognition.tcx)
            .peel_refs();
        match callback.kind() {
            ty::Closure(id, args) => Some((*id, *args)),
            _ => crate::fn_def(callback),
        }
    }
}
