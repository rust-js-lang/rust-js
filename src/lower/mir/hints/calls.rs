//! The temporaries calls the source writes by their names make, and the
//! names `temporaries_names` gives them (ADR 0569), which a value read in
//! place knows where it's spilled (#769).

use super::*;

impl<'tcx> FnCx<'_, 'tcx> {
    /// Each temporary a call the source writes by its name makes, `_5` of
    /// `a.trailing_zeros()`, and what it calls: not one a macro writes,
    /// `vec![..]`'s, nor an operator's, `a[i]`'s, nor a closure's, `f(x)`'s
    /// `call_mut`. The one answer `temporaries_names` and the lowering, whose
    /// value read in place knows it's held by such a temporary (#769), ask.
    pub(in crate::lower::mir) fn calls_named_in_source(
        &self,
        mir_body: &mir::Body<'tcx>,
    ) -> std::collections::HashMap<Local, rustc_span::def_id::DefId> {
        let source = self.tcx.sess.source_map();
        let mut called = std::collections::HashMap::new();
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
    /// `unwrap` of `x.unwrap()`, which names nothing.
    pub(in crate::lower::mir) fn named_by_their_call(
        &self,
        mir_body: &mir::Body<'tcx>,
    ) -> std::collections::HashMap<Local, String> {
        (self.calls_named_in_source(mir_body).into_iter())
            .filter_map(|(local, def_id)| {
                let name = variable_name(&crate::lower::camel_case(&self.call_name(def_id)?));
                (crate::names::js_ident(&name) == name).then_some((local, name))
            })
            .collect()
    }
}
