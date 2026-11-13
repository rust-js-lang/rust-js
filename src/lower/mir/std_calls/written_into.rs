//! A `write!` into a writer of the crate's own, or a `W: fmt::Write`'s, a
//! call at a time, as std's `write_fmt` writes it (#1652, ADR 1135): its
//! `format_args!` made each write `display::sink_writes` says, and the
//! `write_fmt` each written into the writer's sink, `$writerSink(w,
//! writeStr, writeChar)`, whose calls throw the writer's `Err` as `?` does.

use super::*;
use crate::lower::display::SinkWrite;
use rustc_span::def_id::DefId;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether the `format_args!` being lowered, which stores to
    /// `destination`, is written into such a writer (`ArgumentsUse::Written`).
    pub(super) fn arguments_written(&self, state: &State<'_, 'tcx>) -> bool {
        (state.called_into.and_then(|place| place.as_local())).is_some_and(|local| {
            super::super::arguments_use::arguments_use(&self.recognition(), state.body, local)
                == super::super::arguments_use::ArgumentsUse::Written
        })
    }

    /// The writes of a `format_args!` of `items` by `template`: each
    /// literal piece a `write_str`, each placeholder what std's `fmt` of its
    /// value writes. Each argument is evaluated first, as Rust's are, before
    /// any write runs the writer's code. A value whose split isn't followed
    /// yet is refused (#1652).
    pub(super) fn arguments_writes(
        &mut self,
        state: &mut State<'_, 'tcx>,
        mut items: Vec<Value<'tcx>>,
        template: &[Piece],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        self.flush(state, out)?;
        for item in items.iter_mut() {
            if let Value::Fmt(_, _, value) = item
                && value.has_effects()
            {
                *value = self.spill(Base::Temporary, std::mem::replace(value, Expr::undefined()), out);
            }
        }
        let item = |i: usize| match items.get(i) {
            Some(Value::Fmt(kind, ty, value)) => Some((*kind, *ty, value.clone())),
            _ => None,
        };
        let bad = |cx: &Self| cx.unsupported(span, "this format argument, from its MIR");
        let mut writes = Vec::new();
        for piece in template {
            match *piece {
                Piece::Text(ref text) => writes.push(SinkWrite::Str(Expr::str(text))),
                Piece::Argument(i, spec) => {
                    let (kind, ty, value) = item(i).ok_or_else(|| bad(self))?;
                    let width = match spec.width_from {
                        Some(from) => Some(item(from).ok_or_else(|| bad(self))?.2),
                        None => spec.width.map(|w| Expr::int(w.into())),
                    };
                    let precision = match spec.precision_from {
                        Some(from) => Some(item(from).ok_or_else(|| bad(self))?.2),
                        None => spec.precision.map(|p| Expr::int(p.into())),
                    };
                    let Some(written) = self.sink_writes(value, (kind, ty), spec, (width, precision), span)? else {
                        let shown = match kind {
                            Std::FmtDisplay => "Display",
                            Std::FmtDebug => "Debug",
                            Std::FmtExp(_) => "LowerExp",
                            Std::FmtPointer => "Pointer",
                            _ => "a radix",
                        };
                        let what = format!(
                            "`write!` of a `{ty}` by its `{shown}` into a writer of the crate's own or a generic one: std writes it a piece at a time, each a call of the writer's, which isn't followed for it yet (#1652)"
                        );
                        return Err(self.unsupported(span, &what));
                    };
                    writes.extend(written);
                }
            }
        }
        Ok(Value::Writes(writes))
    }

    /// `w.write_fmt(args)` of `writes`, `args`' (`arguments_writes`), into
    /// `writer`, a writer `fmt::Write`'s `def_id` of `generic_args` is
    /// called on: each written into its sink, `$writerSink(w, writeStr,
    /// writeChar)`, its `write_char` the writer's own or std's, a
    /// `write_str` of the `char`. What it gives is its `fmt::Result`: of a
    /// writer that may fail, what `$writeTry` caught; else nothing, the
    /// writes made here.
    pub(super) fn written_into_writer(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (DefId, ty::GenericArgsRef<'tcx>),
        writer: Expr,
        writes: Vec<SinkWrite>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let tcx = self.tcx;
        let trait_id = tcx.trait_of_assoc(def_id).expect("`fmt::Write`'s method");
        let write_str = trait_method(tcx, trait_id, "write_str");
        let write_char = trait_method(tcx, trait_id, "write_char");
        let own_char = |cx: &Self| match cx.resolve_self_instance(write_char, generic_args) {
            Ok(Some(instance)) => cx.krate.fns.contains_key(&instance.def_id()),
            // A generic writer's dictionary has its own, or std's.
            _ => true,
        };
        let own_char = own_char(self);
        let write_str_entry = self.writer_entry(write_str, generic_args, span)?;
        let write_char_entry = match own_char {
            true => self.writer_entry(write_char, generic_args, span)?,
            false => write_str_entry.clone(),
        };
        let fails =
            self.call_may_fail(write_str, generic_args) || (own_char && self.call_may_fail(write_char, generic_args));
        let js_span = self.js_span(span);
        let sink = self.fresh(Role::F);
        let mut body = vec![
            StmtKind::Const(
                sink.clone(),
                Expr::call_helper("$writerSink", vec![writer, write_str_entry, write_char_entry]),
            )
            .at(js_span),
        ];
        body.extend(
            writes
                .into_iter()
                .map(|write| write.written_into(&Expr::var(&sink), js_span)),
        );
        self.flush(state, out)?;
        Ok(Value::Expr(match fails {
            true => self.fmt_try(body),
            false => {
                out.extend(body);
                Expr::undefined()
            }
        }))
    }

    /// Of a `write_fmt` into such a writer given an `Arguments` not made
    /// here, `values`' second, a parameter's or one given on: refused, as
    /// std writes it a piece at a time, which its text alone doesn't say
    /// (#1652). A literal's is its text, one `write_str`, as std's
    /// `as_statically_known_str` writes it.
    pub(super) fn arguments_not_written(&self, def_id: DefId, values: &[Value<'tcx>], span: Span) -> R<()> {
        let literal = matches!(values.get(1), Some(Value::Expr(text)) if matches!(text.kind, js::ExprKind::Str(_)));
        if !crate::lower::recognition::writes_piecewise()
            || self.tcx.item_name(def_id).as_str() != "write_fmt"
            || literal
        {
            return Ok(());
        }
        Err(self.unsupported(
            span,
            "`write_fmt` of a `fmt::Arguments` made elsewhere into a writer of the crate's own or a generic one: std writes it a piece at a time, each a call of the writer's, which isn't followed for it yet (#1652)",
        ))
    }

    /// A writer's `write_str` or `write_char`, `method` of `generic_args`,
    /// as a function of the writer and what it writes: the crate's own,
    /// `logWrite_writeStr`, or its dictionary's, `W.writeStr`.
    fn writer_entry(&mut self, method: DefId, generic_args: ty::GenericArgsRef<'tcx>, span: Span) -> R<Expr> {
        let (writer, written) = (self.fresh(Role::Arg), self.fresh(Role::S));
        let mut body = Vec::new();
        let call = self
            .trait_call(
                method,
                generic_args,
                vec![Expr::var(&writer), Expr::var(&written)],
                span,
                &mut body,
            )?
            .ok_or_else(|| self.unsupported(span, "this writer's `write_str`"))?;
        // Its `Err` thrown, as the sink's calls throw it: not the value
        // `$writeTry` makes of it where a call's result is a value.
        let thrown = match &call.kind {
            js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$writeTry") => match &args[..] {
                [made] => match &made.kind {
                    js::ExprKind::Arrow(params, made) if params.is_empty() => match &made[..] {
                        [only] => match &only.kind {
                            StmtKind::Return(Some(called)) => Some(called.clone()),
                            _ => None,
                        },
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
        let call = thrown.unwrap_or(call);
        body.push(StmtKind::Return(Some(call)).at(js::Span::NONE));
        let arrow = Expr::arrow(vec![js::Pattern::Name(writer), js::Pattern::Name(written)], body);
        Ok(
            crate::lower::passed_on::passed_on(&arrow, crate::lower::passed_on::Taker::Made, self.forwarding())
                .unwrap_or(arrow),
        )
    }
}
