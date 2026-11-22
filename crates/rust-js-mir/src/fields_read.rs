//! An untagged enum's method that reads the same field of each variant's
//! value, read where it's called as its JS reads it (rd317). Its MIR half,
//! the reads of a field through a trait of classes' method, is the base's
//! (`analysis/fields_read.rs`, #2299).

use super::*;

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// A call of `callee`, given `values`, of a method of an untagged
    /// enum that reads the same field of each variant's value
    /// (`untagged::field_reads`), whose JS is that read of its parameter,
    /// lowered before its callers: that read of the value it's given,
    /// `parentInstance.children` (rd317). `None` of another call.
    pub(super) fn variant_field_read(
        &mut self,
        callee: rustc_span::def_id::DefId,
        values: &[Value<'tcx>],
        span: Span,
    ) -> R<Option<Expr>> {
        let [value] = values else { return Ok(None) };
        let Some((param, read)) = self.settled.field_reads.get(&callee).cloned() else {
            return Ok(None);
        };
        let value = self.value_expr(value.clone(), span)?;
        Ok(read.substitute_names(&|v| (*v == param).then(|| value.clone())))
    }
}
