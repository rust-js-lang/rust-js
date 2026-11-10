//! A std call that adds to a `Vec`, a `VecDeque` or a `BinaryHeap`, of a
//! crate that keeps capacities: what it adds grown as std's growth does
//! (ADR 0419, amended, #1180).

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A std function rust-js knows, given what MIR made of its arguments,
    /// `mir_std_call`'s; of one that adds to a `Vec`, a `VecDeque` or a `BinaryHeap` of a
    /// crate that keeps capacities, `grows_vec`'s, and an `extend` that
    /// adds as many as it's given at once (`extend_fills`), what it adds
    /// grown as std's `reserve` of as many does, `$grew(v, size, v.length,
    /// call)`, its length read before the call (#1180).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::lower::mir) fn grown_std_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (known, def_id): (Std, rustc_span::def_id::DefId),
        generic_args: ty::GenericArgsRef<'tcx>,
        (arg_tys, given): (&[Ty<'tcx>], crate::lower::calls::Given<'_>),
        mut values: Vec<Value<'tcx>>,
        output: Ty<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let extends = || {
            self.recognition().extend_grows(def_id, generic_args)
                && arg_tys.len() == 2
                && !self.extend_fills(arg_tys[0].peel_refs(), arg_tys[1])
        };
        let grows = (self.krate.keeps_capacity)
            .then(|| crate::lower::recognition::grows_vec(self.tcx, def_id).or_else(|| extends().then_some(0)))
            .flatten()
            .filter(|&at| at < values.len());
        let item = grows.and_then(|_| self.recognition().grown_vec_item(def_id, generic_args));
        let size = item.and_then(|item| self.capacity_item_size(item));
        let (Some(at), Some(size)) = (grows, size) else {
            return self.mir_std_call(
                state,
                (known, def_id),
                generic_args,
                (arg_tys, given),
                values,
                output,
                span,
                out,
            );
        };
        let vector = self.value_expr(values[at].clone(), span)?;
        self.flush(state, out)?;
        let vector = match vector.reads_same() {
            true => vector,
            false => self.spill("items", vector, out),
        };
        values[at] = Value::Expr(vector.clone());
        let mut length = Expr::member(vector.clone(), "length");
        // Its length, held where the call runs statements first, an index
        // loop's `extend` (ADR 1042).
        let mut made_out = Vec::new();
        let made = self.mir_std_call(
            state,
            (known, def_id),
            generic_args,
            (arg_tys, given),
            values,
            output,
            span,
            &mut made_out,
        )?;
        let made = self.value_expr(made, span)?;
        if !made_out.is_empty() {
            length = self.spill("length", length, out);
        }
        out.extend(made_out);
        Ok(Value::Expr(Expr::call_helper(
            "$grew",
            vec![vector, size, length, made],
        )))
    }
}
