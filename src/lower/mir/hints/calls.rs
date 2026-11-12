//! The temporaries calls the source writes by their names make, and the
//! names `temporaries_names` gives them (ADR 0569), which a value read in
//! place knows where it's spilled (#769).

use super::*;
use crate::collections::IndexMap;

impl<'tcx> FnCx<'_, 'tcx> {
    /// Each temporary a call the source writes by its name makes, `_5` of
    /// `a.trailing_zeros()`, and what it calls: not one a macro writes,
    /// `vec![..]`'s, nor an operator's, `a[i]`'s, nor a closure's, `f(x)`'s
    /// `call_mut`. The one answer `temporaries_names` and the lowering, whose
    /// value read in place knows it's held by such a temporary (#769), ask.
    pub(in crate::lower::mir) fn calls_named_in_source(
        &self,
        mir_body: &mir::Body<'tcx>,
    ) -> IndexMap<Local, rustc_span::def_id::DefId> {
        let source = self.tcx.sess.source_map();
        let mut called = IndexMap::default();
        for data in mir_body.basic_blocks.iter() {
            if let TerminatorKind::Call {
                func,
                destination,
                fn_span,
                ..
            } = &data.terminator().kind
                && destination.projection.is_empty()
                && let Some((def_id, _)) = func.const_fn_def()
                && !fn_span.from_expansion()
                && let Ok(written) = source.span_to_snippet(*fn_span)
                && written.starts_with(self.tcx.item_name(def_id).as_str())
            {
                called.insert(destination.local, def_id);
            }
        }
        called
    }

    /// Those of `calls_named_in_source` whose call names them, by that name as
    /// `temporaries_names` gives it first, `next` of `r.next()`; not
    /// `unwrap` of `x.unwrap()`, which names nothing. Nor an `Option`'s or
    /// a `Result`'s own method, `as_deref`, `filter`, `map_or_else`, which
    /// gives back the option it had: named by a closure's parameter, or
    /// `option`, as before #769. A binding read as a property, `get G`'s
    /// `g()`, `ReactSharedInternals.G` in JS, which calls nothing, is
    /// `read`: it names it only where no closure's parameter does (#784).
    pub(in crate::lower::mir) fn named_by_their_call(&self, mir_body: &mir::Body<'tcx>) -> IndexMap<Local, js::Held> {
        let tcx = self.tcx;
        let optional = |def_id| crate::lower::recognition::is_option_or_result_method(tcx, def_id);
        let read = |def_id| {
            crate::lower::bindings::is_binding(tcx, def_id)
                && matches!(
                    crate::lower::bindings::js_form(tcx, def_id),
                    crate::lower::bindings::JsForm::Get(_) | crate::lower::bindings::JsForm::Prop(_)
                )
        };
        (self.calls_named_in_source(mir_body).into_iter())
            .filter(|&(_, def_id)| !optional(def_id))
            .filter_map(|(local, def_id)| {
                let name = crate::lower::temporary_names::call_suggestion(tcx, def_id)?;
                Some((
                    local,
                    js::Held {
                        name,
                        read: read(def_id),
                    },
                ))
            })
            .collect()
    }
}
