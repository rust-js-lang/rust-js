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
    /// And one a writer that writes through a sink gives a `Formatter`, its
    /// own or one a struct holds, which a `write!` into a writer may arm
    /// with its writer's (`$writingInto`), so each piece is a call of that
    /// writer's.
    pub(super) fn arguments_written(&self, state: &State<'_, 'tcx>) -> bool {
        use super::super::arguments_use::{ArgumentsUse, arguments_use, written_into_a_formatter};
        (state.called_into.and_then(|place| place.as_local())).is_some_and(|local| {
            let recognition = self.recognition();
            arguments_use(&recognition, state.body, local) == ArgumentsUse::Written
                || (self.writes_into_writers(state) && written_into_a_formatter(&recognition, state.body, local))
                || (arguments_use(&recognition, state.body, local) == ArgumentsUse::GivenOn
                    && self.given_to_a_writer(state.body, local))
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
                            "`write!` of a `{ty}` by its `{shown}` into a writer of the crate's own or a generic one"
                        );
                        return Err(self.refused_piecewise(span, &what));
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

    /// A writer of the crate's own, or a `W: fmt::Write`, `Std::UserWrite`'s
    /// call of `def_id` of `generic_args`, given `values`: a `write!`'s
    /// writes, a piece at a time (#1652), into its sink; else what `write!`
    /// and `write_char` give its `write_str` (ADR 0166).
    pub(super) fn user_write(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (DefId, ty::GenericArgsRef<'tcx>),
        mut values: Vec<Value<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let tcx = self.tcx;
        // `fmt::write(w, args)`, std's own `write_fmt` of the `dyn
        // fmt::Write` it takes (#1697): written through its `write_str`,
        // never its own `write_fmt`.
        let std_write_fmt = self.recognition().is_fmt_write(def_id);
        let (def_id, generic_args) = self.recognition().call_it_is(def_id, generic_args);
        // A `write!`'s, a piece at a time (#1652).
        if let [writer, Value::Writes(writes, fails)] = &mut values[..] {
            let (writer, writes) = (self.value_expr(writer.clone(), span)?, std::mem::take(writes));
            let writes = (writes, *fails);
            return self.written_into_writer(state, (def_id, generic_args), writer, writes, span, out);
        }
        // Given the function of an `Arguments` that writes into the sink it
        // claims (#1652, step 6): a writer of the crate's own writes it
        // through its sink, `$writingInto(f, args)`; a `W: fmt::Write`'s
        // dictionary's `writeFmt` is given it.
        let function = self.takes_arguments_function(state, (def_id, generic_args), 1);
        let sink = function && self.arguments_given_as_sink(state);
        if !sink {
            self.arguments_not_written(def_id, &values, span)?;
        }
        if sink
            && (std_write_fmt || !self.recognition().writes_through_a_dictionary(def_id, generic_args))
            && let [writer, args] = &values[..]
        {
            let (writer, args) = (
                self.value_expr(writer.clone(), span)?,
                self.value_expr(args.clone(), span)?,
            );
            let writes = (
                vec![SinkWrite::Helper("$writingInto", vec![args])],
                self.krate.failing.any,
            );
            return self.written_into_writer(state, (def_id, generic_args), writer, writes, span, out);
        }
        if sink {
            let exprs = (values.into_iter())
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            self.flush(state, out)?;
            return Ok(Value::Expr(
                self.trait_call(def_id, generic_args, exprs, span, out)?
                    .ok_or_else(|| self.unsupported(span, "this writer's `write_fmt`"))?,
            ));
        }
        Ok(Value::Expr({
            let exprs = values
                .into_iter()
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            let trait_id = tcx.trait_of_assoc(def_id).expect("`fmt::Write`'s method");
            let write_str = trait_method(tcx, trait_id, "write_str");
            self.flush(state, out)?;
            self.trait_call(write_str, generic_args, exprs, span, out)?
                .ok_or_else(|| self.unsupported(span, "this writer's `write_str`"))?
        }))
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
        // A generic writer's dictionary, or a `dyn`'s, has its own, or std's.
        let own_char = |cx: &Self| match cx.resolve_self_instance(write_char, generic_args) {
            _ if (generic_args.types().next())
                .is_some_and(|writer| crate::lower::recognition::known_by_a_dictionary(writer.peel_refs())) =>
            {
                true
            }
            Ok(Some(instance)) => cx.krate.fns.contains_key(&instance.def_id()),
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
        Err(self.refused_piecewise(
            span,
            "`write_fmt` of a `fmt::Arguments` made elsewhere into a writer of the crate's own or a generic one",
        ))
    }

    /// Whether the `Arguments` the `write_fmt` at `state.at` is given, its
    /// second, is the function that writes into the sink it claims
    /// (`sink_form_given`, #1652 step 6).
    pub(super) fn arguments_given_as_sink(&self, state: &State<'_, 'tcx>) -> bool {
        self.arguments_given_as_sink_at(state, 1)
    }

    /// Whether the call at `state.at` is given at `index` an `Arguments`
    /// that writes into the sink it claims (`sink_form_given`).
    pub(super) fn arguments_given_as_sink_at(&self, state: &State<'_, 'tcx>, index: usize) -> bool {
        match &state.body.basic_blocks[state.at].terminator().kind {
            TerminatorKind::Call { args, .. } => args
                .get(index)
                .is_some_and(|arg| self.sink_form_given(state.body, &arg.node)),
            _ => false,
        }
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
                        // The function of an `Arguments` that writes into the
                        // sink it claims, a parameter's (#1652, step 6):
                        // this writer's sink armed for it.
                        (true, false) if self.arguments_given_as_sink(state) => {
                            vec![SinkWrite::Helper("$writingInto", vec![value])]
                        }
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
                        // An `Arguments`' `Display`, `Display::fmt(&args, f)`,
                        // std's `fmt::write` of it into the `Formatter`'s
                        // writer, its options unread: the function that
                        // writes into the sink it claims, armed with this
                        // one; else refused, as its `write_fmt` is (step 6).
                        let arguments = kind == Std::FmtDisplay && self.is_lang_adt(ty, LangItem::FormatArguments);
                        let written = match kind {
                            _ if arguments && self.arguments_given_as_sink_at(state, 0) => {
                                Some(vec![SinkWrite::Helper("$writingInto", vec![value])])
                            }
                            _ if arguments => {
                                let what = "a `Display::fmt` of a `fmt::Arguments` made elsewhere";
                                return Err(self.refused_in_a_writer(what, span));
                            }
                            Std::FmtDebug => {
                                let pretty = self.writer_pretty().alternate();
                                self.debug_sink_writes(value, ty, (&options, &pretty), span)?
                            }
                            _ => self.sink_writes_given(value, (kind, ty), options, span)?,
                        };
                        match written {
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
            // `{:x}`, `{:e}` and the like of a std value given the
            // `Formatter`, `LowerHex::fmt(&self.0, f)`: its writes by the
            // `Formatter`'s options as it runs; of a writer of the crate's
            // own, as another's below.
            WriteCall::OtherFmt if !self.own_writer(def_id, generic_args) => {
                let trait_id = self.tcx.trait_of_assoc(def_id).expect("a formatting trait's `fmt`");
                let kind = match (self.radix_trait(trait_id), self.exp_trait(trait_id)) {
                    (Some(radix), _) => Some(Std::FmtRadix(radix)),
                    (_, Some(upper)) => Some(Std::FmtExp(upper)),
                    _ => None,
                };
                let ty = generic_args.type_at(0).peel_refs();
                let value = match values.next() {
                    Some(value) => self.shown_in_place(value, span)?,
                    None => return Err(self.unsupported(span, "this `fmt`'s value, from its MIR")),
                };
                let written = match kind {
                    Some(kind) => self.sink_writes_given(value, (kind, ty), options, span)?,
                    None => None,
                };
                match written {
                    Some(writes) => writes,
                    None => {
                        let what = format!("a `{{:x}}`, `{{:e}}` or the like of a `{ty}` given the `Formatter`");
                        return Err(self.refused_in_a_writer(&what, span));
                    }
                }
            }
            WriteCall::Function | WriteCall::Trait | WriteCall::OtherFmt => {
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
            // A derived `Debug`'s, `debug_struct_field2_finish(f, "P", "x",
            // &self.x, ..)`: core's builder, each field a step, its value
            // written by its `Debug` into the step's sink.
            // Of more than five fields, `debug_struct_fields_finish(f, "P",
            // &["a", ..], &[&self.a, ..])`: the same, of its arrays.
            WriteCall::Struct | WriteCall::Tuple | WriteCall::StructFields | WriteCall::TupleFields => {
                let named = matches!(operation, WriteCall::Struct | WriteCall::StructFields);
                let Some(type_name) = values.next() else {
                    return Err(self.unsupported(span, "this derived `Debug`, from its MIR"));
                };
                let type_name = self.shown_in_place(type_name, span)?;
                let options = self.given_to_writer(None).unwrap_or_else(Expr::undefined);
                let pretty = self.writer_pretty().alternate();
                let mut values: Vec<Value<'tcx>> = match operation {
                    WriteCall::StructFields | WriteCall::TupleFields => {
                        let lists: Vec<Value<'tcx>> = values.collect();
                        // The names, an array of strings; the values, each
                        // a `&dyn Debug` made here.
                        let mut lists = lists.into_iter().map(|list| match list {
                            Value::List(items, _) => Some(items),
                            Value::Expr(Expr {
                                kind: js::ExprKind::Array(names),
                                ..
                            }) => Some(names.into_iter().map(Value::Expr).collect()),
                            _ => None,
                        });
                        let (names, items) = match named {
                            true => (lists.next().flatten(), lists.next().flatten()),
                            false => (Some(Vec::new()), lists.next().flatten()),
                        };
                        let (Some(names), Some(items)) = (names, items) else {
                            return Err(self.unsupported(span, "this derived `Debug`'s fields, from its MIR"));
                        };
                        match named {
                            true => names
                                .into_iter()
                                .zip(items)
                                .flat_map(|(name, item)| [name, item])
                                .collect(),
                            false => items,
                        }
                    }
                    _ => values.collect(),
                };
                let mut values = values.drain(..);
                let mut fields = Vec::new();
                while let Some(first) = values.next() {
                    let (name, value) = match named {
                        true => (Some(self.shown_in_place(first, span)?), values.next()),
                        false => (None, Some(first)),
                    };
                    let Some(Value::DynDebug(from, _, value)) = value else {
                        return Err(self.refused_in_a_writer("a derived `Debug` of a `dyn Debug` given on", span));
                    };
                    let ty = self.pointee(from);
                    let Some(write) =
                        self.writing_fn(|cx, field| cx.debug_sink_writes(field, ty, (&options, &pretty), span))?
                    else {
                        let what = format!("a derived `Debug`'s field of a `{ty}`");
                        return Err(self.refused_in_a_writer(&what, span));
                    };
                    let mut field: Vec<Expr> = name.into_iter().collect();
                    field.extend([value, write]);
                    fields.push(Expr::array(field));
                }
                let helper = if named { "$writeDebugStruct" } else { "$writeDebugTuple" };
                vec![SinkWrite::Helper(helper, vec![type_name, Expr::array(fields), pretty])]
            }
            _ => return Err(self.refused_in_a_writer("this `Formatter`'s call", span)),
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
        self.refused_piecewise(span, &format!("{what} in a writer shown into a writer by `write!`"))
    }

    /// The refusal of `what`, written into a writer, whose split into
    /// pieces isn't followed yet: `what`, and why, std writes it a piece at
    /// a time, naming #1652.
    pub(in crate::lower) fn refused_piecewise(&self, span: Span, what: &str) -> rustc_span::ErrorGuaranteed {
        crate::lower::refusal::refuse(
            self.tcx,
            span,
            crate::lower::refusal::Refused::Because(
                what,
                "std writes it a piece at a time, each a call of the writer's, which isn't followed for it (#1652)",
            ),
        )
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A closure, or a writer through a `fn` pointer, given the `Formatter`
    /// of a writer shown into a writer, `call`, what it's called with but
    /// the `Formatter`, `w(n)`: called with this writer's sink armed for it,
    /// `$writingInto(f, w, n)`, as a writer function's is, which writes
    /// through a sink as its maker is framed (#1652). Its result, kept as a
    /// value, is what `$writeTry` caught.
    pub(in crate::lower::mir) fn called_with_the_sink_armed(
        &mut self,
        state: &mut State<'_, 'tcx>,
        call: Expr,
        destination: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        let js::ExprKind::Call(callee, args) = call.kind else {
            return Err(self.refused_in_a_writer("a writer function given the `Formatter`", span));
        };
        let mut given = vec![*callee];
        given.extend(args);
        let sink = Expr::var(&state.writer.as_ref().expect("a writer").1);
        let written = vec![SinkWrite::Helper("$writingInto", given).written_into(&sink, self.js_span(span))];
        use super::super::result_use::{ResultUse, result_use};
        let inspected = result_use(&self.recognition(), state.body, destination) == ResultUse::Inspected;
        if inspected {
            return Ok(Value::Expr(self.fmt_try(written)));
        }
        out.extend(written);
        Ok(Value::Expr(Expr::undefined()))
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A write, `def_id` of `generic_args`, through a `Formatter` held where
    /// it isn't its writer's, `handle`, of a writer that writes through a
    /// sink, which the handle is (#1652): `given`, a `write!`'s writes,
    /// each into it; the function of an `Arguments` that writes into the
    /// sink it claims, `$writingInto(handle, args)`; a `char`, a
    /// `write_char`; a string, a `write_str`. Its result, kept as a value,
    /// is what `$writeTry` caught.
    pub(super) fn written_into_held_formatter(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (def_id, generic_args): (DefId, ty::GenericArgsRef<'tcx>),
        handle: Expr,
        given: Value<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let writes = match given {
            Value::Writes(writes, _) => writes,
            given => {
                let value = self.value_expr(given, span)?;
                let literal = matches!(value.kind, js::ExprKind::Str(_));
                match () {
                    _ if self.recognition().is_write_char(def_id) => vec![SinkWrite::Char(value)],
                    _ if !self.recognition().formatter_write_fmt(def_id, generic_args) || literal => {
                        vec![SinkWrite::Str(value)]
                    }
                    _ if self.arguments_given_as_sink(state) => vec![SinkWrite::Helper("$writingInto", vec![value])],
                    _ => {
                        return Err(self.refused_in_a_writer(
                            "a `write_fmt` of a `fmt::Arguments` made elsewhere through a held `Formatter`",
                            span,
                        ));
                    }
                }
            }
        };
        self.flush(state, out)?;
        let handle = match handle.kind {
            js::ExprKind::Var(_) => handle,
            _ => self.spill(Base::Temporary, handle, out),
        };
        let js_span = self.js_span(span);
        let written: Vec<Stmt> = (writes.into_iter())
            .map(|write| write.written_into(&handle, js_span))
            .collect();
        Ok(self.fmt_try(written))
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `def_id` of `generic_args` resolves to a writer of the
    /// crate's own that writes through a sink, which a writer may arm.
    fn own_writer(&self, def_id: DefId, generic_args: ty::GenericArgsRef<'tcx>) -> bool {
        matches!(self.resolve_self_instance(def_id, generic_args), Ok(Some(instance))
            if instance.def_id().is_local() && self.writes_through_sink(instance.def_id()))
    }
}
