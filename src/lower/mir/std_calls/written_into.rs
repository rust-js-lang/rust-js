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
    /// And one a writer that writes through a sink gives its own
    /// `Formatter`, which a `write!` into a writer may arm with its
    /// writer's (`$writingInto`), so each piece is a call of that writer's.
    pub(super) fn arguments_written(&self, state: &State<'_, 'tcx>) -> bool {
        use super::super::arguments_use::{ArgumentsUse, arguments_use, written_into_formatter};
        (state.called_into.and_then(|place| place.as_local())).is_some_and(|local| {
            let recognition = self.recognition();
            arguments_use(&recognition, state.body, local) == ArgumentsUse::Written
                || (crate::lower::recognition::writes_piecewise()
                    && self.writes_through_a_sink(state)
                    && (state.writer.as_ref())
                        .is_some_and(|(aliases, _)| written_into_formatter(&recognition, state.body, local, aliases)))
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
        // Of what may fail of its own, a writer of the crate's own that
        // returns its `Err`, its `fmt::Result` is a value (ADR 0187).
        let fails = items.iter().any(|item| match *item {
            Value::Fmt(Std::FmtDisplay, ty, _) => self.fmt_may_fail(self.display_trait(), ty),
            Value::Fmt(Std::FmtDebug, ty, _) => self.fmt_may_fail(self.debug_trait(), ty),
            _ => false,
        });
        Ok(Value::Writes(writes, fails))
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
        (writes, shown_fails): (Vec<SinkWrite>, bool),
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
        let fails = shown_fails
            || self.call_may_fail(write_str, generic_args)
            || (own_char && self.call_may_fail(write_char, generic_args));
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

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call that writes to the `Formatter` of a writer that writes through
    /// a sink, `operation` of `def_id`, given `values`, its arguments but
    /// the `Formatter`: each piece written into the sink as std makes it,
    /// which a `write!` into a writer may arm with its writer's, so each is
    /// a call of that writer's (#1652, ADR 1135). Its own `write!`, each
    /// write `sink_writes` says; `write_str`, one; `write_char`, one of the
    /// writer's `write_char`; `pad` and `pad_integral`, std's, given its
    /// `Formatter`'s options; and a `fmt` given its `Formatter`, a std
    /// value's by those options, a writer's of the crate's own with the
    /// sink armed for it. What isn't followed yet is refused. Its result,
    /// kept as a value, is what `$writeTry` caught, as the sink's calls may
    /// throw the writer's `Err`.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::lower::mir) fn written_through_the_sink(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (DefId, ty::GenericArgsRef<'tcx>),
        operation: crate::lower::recognition::WriteCall,
        values: Vec<Value<'tcx>>,
        destination: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        use crate::lower::recognition::WriteCall;
        let sink = Expr::var(&state.writer.as_ref().expect("a writer").1);
        let options = self.given_to_writer(None).unwrap_or_else(Expr::undefined);
        let mut values = values.into_iter();
        let writes = match operation {
            WriteCall::Text => match values.next() {
                Some(Value::Writes(writes, _)) => writes,
                Some(value) => {
                    let value = self.shown_in_place(value, span)?;
                    let literal = matches!(value.kind, js::ExprKind::Str(_));
                    match (self.recognition().formatter_write_fmt(def_id, generic_args), literal) {
                        (true, false) => {
                            return Err(
                                self.refused_in_a_writer("a `write_fmt` of a `fmt::Arguments` made elsewhere", span)
                            );
                        }
                        _ if self.recognition().is_write_char(def_id) => vec![SinkWrite::Char(value)],
                        _ => vec![SinkWrite::Str(value)],
                    }
                }
                None => return Err(self.unsupported(span, "this write's arguments, from its MIR")),
            },
            WriteCall::Pad | WriteCall::PadIntegral | WriteCall::Display | WriteCall::Debug => {
                let mut given = Vec::new();
                for value in values {
                    given.push(self.shown_in_place(value, span)?);
                }
                match operation {
                    WriteCall::Pad => {
                        given.push(options);
                        vec![SinkWrite::Helper("$writePadded", given)]
                    }
                    WriteCall::PadIntegral => {
                        given.push(options);
                        vec![SinkWrite::Helper("$writeIntegral", given)]
                    }
                    _ => {
                        let kind = match operation {
                            WriteCall::Display => Std::FmtDisplay,
                            _ => Std::FmtDebug,
                        };
                        let ty = generic_args.type_at(0).peel_refs();
                        let value = given.into_iter().next().expect("a `fmt`'s value");
                        match self.sink_writes_given(value, (kind, ty), options, span)? {
                            Some(writes) => writes,
                            None => {
                                let what = format!("a `fmt` of a `{ty}` given its `Formatter`");
                                return Err(self.refused_in_a_writer(&what, span));
                            }
                        }
                    }
                }
            }
            // Another writer of the crate's own given this one's `Formatter`,
            // `helper(f, n)`, which writes through a sink: called with this
            // writer's sink armed for it.
            WriteCall::Function | WriteCall::Trait => {
                let framed = match self.resolve_self_instance(def_id, generic_args) {
                    Ok(Some(instance))
                        if instance.def_id().is_local() && self.writes_through_sink(instance.def_id()) =>
                    {
                        Some(instance)
                    }
                    _ => None,
                };
                let mut given = Vec::new();
                for value in values {
                    given.push(self.shown_in_place(value, span)?);
                }
                let armed = match framed {
                    Some(instance) => {
                        let given = self.writer_function_given(instance.def_id(), instance.args, given, span)?;
                        self.writer_armed(instance.def_id(), given)
                    }
                    None => None,
                };
                match armed {
                    Some(write) => vec![write],
                    None => {
                        return Err(self.refused_in_a_writer("a function given the writer's `Formatter`", span));
                    }
                }
            }
            WriteCall::OtherFmt => {
                return Err(self.refused_in_a_writer("a `{:x}`, `{:e}` or the like given the `Formatter`", span));
            }
            _ => return Err(self.refused_in_a_writer("a derived `Debug`'s fields", span)),
        };
        self.flush(state, out)?;
        let js_span = self.js_span(span);
        let written: Vec<Stmt> = (writes.into_iter())
            .map(|write| write.written_into(&sink, js_span))
            .collect();
        use super::super::result_use::{ResultUse, result_use};
        if result_use(&self.recognition(), state.body, destination) == ResultUse::Inspected {
            return Ok(Value::Expr(self.fmt_try(written)));
        }
        out.extend(written);
        Ok(Value::Expr(Expr::undefined()))
    }

    /// The refusal of `what`, written by a writer that writes through a
    /// sink a `write!` into a writer may arm, where its split isn't
    /// followed yet (#1652).
    pub(in crate::lower) fn refused_in_a_writer(&self, what: &str, span: Span) -> rustc_span::ErrorGuaranteed {
        self.unsupported(
            span,
            &format!(
                "{what} in a writer shown into a writer by `write!`: std writes it a piece at a time, each a call of the writer's, which isn't followed for it yet (#1652)"
            ),
        )
    }
}
