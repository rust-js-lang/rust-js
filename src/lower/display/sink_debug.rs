//! A std value's `Debug` written into a writer's sink a piece at a time,
//! as core's builders write it (#1652, step 4 of ADR 1135): a leaf by
//! `sink_writes_given`, a sequence by `debug_list`'s `entries`, an `Option`
//! and a `Result` by their derived tuples, a tuple by `debug_tuple("")`,
//! each part written by a writing function into the builder's sink, std's
//! `PadAdapter` for `{:#?}` (`$padAdapter`). The builders are the runtime's
//! one model of core's (`debug_builders.js`, #1533).

use super::*;
use crate::lower::temporary_names::Role;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What `value`'s `Debug`, of `ty`, writes into a writer's sink, given
    /// the `Formatter`'s `options` and whether it's `{:#?}`, `pretty`, each
    /// known as it runs: a leaf's writes, or a builder's of each part.
    /// `None` of a value whose split isn't followed yet (#1652): a map's, a
    /// set's, a generic or `dyn` value's, and another std type's.
    pub(in crate::lower) fn debug_sink_writes(
        &mut self,
        value: Expr,
        ty: Ty<'tcx>,
        (options, pretty): (&Expr, &Expr),
        span: Span,
    ) -> R<Option<Vec<SinkWrite>>> {
        let mut ty = ty.peel_refs();
        while let Some(inner) = ty.boxed_ty() {
            ty = inner.peel_refs();
        }
        if let Some(written) = self.sink_writes_given(value.clone(), (Std::FmtDebug, ty), options.clone(), span)? {
            return Ok(Some(written));
        }
        let helper = |name: &'static str, args: Vec<Expr>| Ok(Some(vec![SinkWrite::Helper(name, args)]));
        // `()`'s, `f.pad("()")`.
        if ty.is_unit() {
            return helper("$writePadded", vec![Expr::str("()"), options.clone()]);
        }
        // `Some(x)`'s derived tuple, `None` one write.
        if let Some(inner) = self.option_of(ty) {
            let boxed = self.boxed_payload(inner);
            let Some(write) = self.writing_fn(|cx, option| {
                let payload = match boxed {
                    true => cx.some_value(option),
                    false => option,
                };
                cx.debug_sink_writes(payload, inner, (options, pretty), span)
            })?
            else {
                return Ok(None);
            };
            return helper("$writeDebugOption", vec![value, write, pretty.clone()]);
        }
        // `Ok(x)`'s or `Err(e)`'s derived tuple.
        if self.recognition().is_result(ty)
            && !self.is_write_result(ty)
            && let ty::Adt(_, args) = ty.kind()
        {
            let (ok, err) = (args.type_at(0), args.type_at(1));
            let mut writes = Vec::new();
            for part in [ok, err] {
                let Some(write) = self.writing_fn(|cx, result| {
                    cx.debug_sink_writes(crate::js::result::held(result), part, (options, pretty), span)
                })?
                else {
                    return Ok(None);
                };
                writes.push(write);
            }
            let mut args = vec![value];
            args.extend(writes);
            args.push(pretty.clone());
            return helper("$writeDebugResult", args);
        }
        // A tuple's, `debug_tuple("")`: each field one step.
        if let ty::Tuple(tys) = ty.kind() {
            let mut fields = Vec::new();
            for (i, part) in tys.iter().enumerate() {
                let Some(write) = self.writing_fn(|cx, tuple| {
                    cx.debug_sink_writes(Expr::index(tuple, Expr::int(i as i128)), part, (options, pretty), span)
                })?
                else {
                    return Ok(None);
                };
                fields.push(write);
            }
            return helper(
                "$writeDebugFields",
                vec![value, Expr::str(""), Expr::array(fields), pretty.clone()],
            );
        }
        // A sequence's, `debug_list().entries(..)`.
        let item = match ty.kind() {
            ty::Array(item, _) | ty::Slice(item) => Some(*item),
            ty::Adt(_, args) if self.is_vec_like(ty) => Some(args.type_at(0)),
            _ => None,
        };
        if let Some(item) = item {
            let Some(write) =
                self.writing_fn(|cx, entry| cx.debug_sink_writes(entry, item, (options, pretty), span))?
            else {
                return Ok(None);
            };
            return helper("$writeDebugList", vec![value, write, pretty.clone()]);
        }
        Ok(None)
    }

    /// A writing function of a value, `(g, value) => { .. }`, its writes
    /// `write` makes of the value, each into `g`, the sink a builder's step
    /// gives it. `None` where `write` has none.
    pub(in crate::lower) fn writing_fn(
        &mut self,
        write: impl FnOnce(&mut Self, Expr) -> R<Option<Vec<SinkWrite>>>,
    ) -> R<Option<Expr>> {
        let (sink, value) = (self.fresh(Role::F), self.fresh(Role::Value));
        let Some(writes) = write(self, Expr::var(&value))? else {
            return Ok(None);
        };
        let body = (writes.into_iter())
            .map(|w| w.written_into(&Expr::var(&sink), js::Span::NONE))
            .collect();
        Ok(Some(Expr::arrow(
            vec![js::Pattern::Name(sink), js::Pattern::Name(value)],
            body,
        )))
    }
}
