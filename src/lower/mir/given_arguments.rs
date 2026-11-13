//! A `fmt::Arguments` given on, as `arguments_use` says, is a function
//! that makes its text (#1565): made where the `format_args!` is written,
//! its operands evaluated there, and called where std formats it, so its
//! values' `Display` and `Debug` run there, as often as it's formatted.
//! One given to a parameter a function writes into a writer a piece at a
//! time (`WrittenArguments`) writes each piece into the sink it claims,
//! `const f = $sink(); ..; return f.done();`: a write of it arms that sink
//! with its writer's, `$writingInto(f, args)`, so each piece is a call of
//! the writer's, and called plainly it gives its text (#1652, step 6 of
//! ADR 1135).

use super::arguments_use::{ArgumentsUse, arguments_use, holds_text, resolves_to_own};
use super::*;
use crate::lower::display::SinkWrite;
use rustc_middle::mir::{Local, Rvalue, StatementKind, TerminatorKind};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether the call of `known` into `destination` makes an `Arguments`
    /// the body gives on, which is a function of its text.
    pub(super) fn makes_arguments_given_on(
        &self,
        state: &State<'_, 'tcx>,
        known: Option<Std>,
        destination: Place<'tcx>,
    ) -> bool {
        matches!(known, Some(Std::FmtNew | Std::FmtStr))
            && (destination.as_local())
                .is_none_or(|local| arguments_use(&self.recognition(), state.body, local) == ArgumentsUse::GivenOn)
    }

    /// Of an `Arguments` given on, what `format_args!` gives it, `values`:
    /// each operand evaluated now, held where it's anything but a variable
    /// or a constant, so what the function formats is what Rust evaluated
    /// where the macro is written, however late it's called.
    pub(super) fn arguments_operands_held(
        &mut self,
        state: &mut State<'_, 'tcx>,
        values: &mut [Value<'tcx>],
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        self.flush(state, out)?;
        if let Some(Value::List(items, _)) = values.get_mut(1) {
            for item in items.iter_mut() {
                if let Value::Fmt(_, _, value) = item
                    && !matches!(value.kind, js::ExprKind::Var(_))
                    && !value.is_constant()
                {
                    *value = self.spill(Base::Temporary, std::mem::replace(value, Expr::undefined()), out);
                }
            }
        }
        Ok(())
    }

    /// The function of an `Arguments` given on, of what `format_args!`
    /// made of it, `made`: `() => text`.
    /// One that may fail makes its text piece by piece, what it made
    /// before the error given with it, `$fmtWritten`'s, as one formatted
    /// where it's made writes it (ADR 0187).
    /// One given to a writer a piece at a time, `made` its writes (or a
    /// literal's text, one `write_str`), writes each into the sink it
    /// claims first: `() => { const f = $sink(); f.write("a"); ..; return
    /// f.done(); }`, what it wrote before an error given with it.
    pub(super) fn arguments_given_on(&mut self, made: Value<'tcx>, sink: bool, span: Span) -> R<Value<'tcx>> {
        if sink {
            let (writes, fails) = match made {
                Value::Writes(writes, fails) => (writes, fails),
                made => (vec![SinkWrite::Str(self.value_expr(made, span)?)], false),
            };
            let f = self.fresh(Role::F);
            let done = Expr::call(Expr::member(Expr::var(&f), "done"), Vec::new());
            let written: Vec<Stmt> = (writes.into_iter())
                .map(|write| write.written_into(&Expr::var(&f), js::Span::NONE))
                .collect();
            let mut body = vec![StmtKind::Const(f.clone(), Expr::call_helper("$sink", Vec::new())).at(js::Span::NONE)];
            match fails || self.krate.failing.any {
                true => {
                    let error = self.fresh(Role::Error);
                    let thrown = Expr::call_helper("$fmtWritten", vec![Expr::var(&error), done.clone()]);
                    body.push(
                        StmtKind::TryCatch(written, Some(error), vec![StmtKind::Throw(thrown).at(js::Span::NONE)])
                            .at(js::Span::NONE),
                    );
                }
                false => body.extend(written),
            }
            body.push(StmtKind::Return(Some(done)).at(js::Span::NONE));
            return Ok(Value::Expr(Expr::arrow(Vec::new(), body)));
        }
        let (text, fails) = match made {
            Value::Printing(text, fails) => (text, fails),
            Value::Fmt(_, _, text) => (text, true),
            made => (self.value_expr(made, span)?, false),
        };
        let body = match fails {
            false => vec![StmtKind::Return(Some(text)).at(js::Span::NONE)],
            true => {
                let name = self.fresh(Role::Text);
                let made = Expr::var(&name);
                let mut written = Vec::new();
                crate::lower::display::append_written(&made, text, true, js::Span::NONE, &mut written);
                let error = self.fresh(Role::Error);
                let thrown = Expr::call_helper("$fmtWritten", vec![Expr::var(&error), made.clone()]);
                vec![
                    StmtKind::Let(name, Some(Expr::str(""))).at(js::Span::NONE),
                    StmtKind::TryCatch(written, Some(error), vec![StmtKind::Throw(thrown).at(js::Span::NONE)])
                        .at(js::Span::NONE),
                    StmtKind::Return(Some(made)).at(js::Span::NONE),
                ]
            }
        };
        Ok(Value::Expr(Expr::arrow(Vec::new(), body)))
    }

    /// Each `Arguments` std's call `callee` is given by value, its
    /// `operands`' `values`, that's a function, of a place not made its
    /// text here (`holds_text`): called, its text made where std formats
    /// it, as one that may print or fail where any writer of the crate's
    /// may, as a `dyn`'s value is.
    pub(super) fn arguments_formatted(
        &mut self,
        state: &State<'_, 'tcx>,
        (callee, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (operands, arg_tys): (&[&Operand<'tcx>], &[Ty<'tcx>]),
        values: &mut [Value<'tcx>],
        span: Span,
    ) -> R<()> {
        // Not a std trait's method an impl of the crate's defines, which
        // takes the function as any of the crate's does (#1695).
        if !crate::lower::recognition::is_std_crate(self.tcx, callee)
            || resolves_to_own(&self.recognition(), callee, generic_args)
        {
            return Ok(());
        }
        for (i, value) in values.iter_mut().enumerate() {
            let (Some(operand), Some(&ty)) = (operands.get(i), arg_tys.get(i)) else {
                continue;
            };
            if !self.is_lang_adt(ty, LangItem::FormatArguments) {
                continue;
            }
            let text = (operand.place())
                .and_then(|place| place.as_local())
                .is_some_and(|local| holds_text(&self.recognition(), state.body, local));
            if text || self.takes_arguments_function(state, (callee, generic_args), i) {
                continue;
            }
            let given = self.value_expr(std::mem::replace(value, Value::Expr(Expr::undefined())), span)?;
            let called = Expr::call(given, Vec::new());
            let fails = self.krate.failing.any;
            *value = match self.krate.prints.any_writer_prints() {
                true => Value::Printing(called, fails),
                false if fails => Value::Fmt(Std::FmtNew, ty, called),
                false => Value::Expr(called),
            };
        }
        Ok(())
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `local`, an `Arguments` the body makes and gives on, is given,
    /// as it is or through what it's copied to, to a parameter a call writes
    /// into a writer a piece at a time (`WrittenArguments::writes`): the
    /// function that writes into the sink it claims (#1652, step 6).
    pub(super) fn given_to_a_writer(&self, body: &mir::Body<'tcx>, local: Local) -> bool {
        crate::lower::analysis::given_to_a_write(&self.recognition(), &self.krate.written.arguments, body, local)
    }

    /// Whether `operand`, an `Arguments` given to what writes it into a
    /// writer a piece at a time, is the function that writes into the sink
    /// it claims (#1652, step 6): through what it's copied from, a
    /// `format_args!` the body makes, which `given_to_a_writer` makes so,
    /// or a parameter of the function being lowered that it writes so
    /// (`WrittenArguments`), whose every caller gives one.
    pub(super) fn sink_form_given(&self, body: &mir::Body<'tcx>, operand: &Operand<'tcx>) -> bool {
        let Some(mut local) = operand.place().filter(|p| p.projection.is_empty()).map(|p| p.local) else {
            return false;
        };
        let recognition = self.recognition();
        for _ in 0..body.local_decls.len() {
            if body.local_kind(local) == mir::LocalKind::Arg {
                let index = local.as_u32() - 1;
                return self
                    .krate
                    .written
                    .arguments
                    .marks(&recognition, self.item)
                    .contains(&index);
            }
            let made = body.basic_blocks.iter().any(|data| {
                matches!(&data.terminator().kind, TerminatorKind::Call { func, destination, .. }
                    if destination.as_local() == Some(local)
                        && crate::lower::fn_def(func.ty(&body.local_decls, self.tcx))
                            .and_then(|(id, generic_args)| recognition.classify(id, generic_args))
                            .is_some_and(|known| matches!(known, Std::FmtNew | Std::FmtStr)))
            });
            if made {
                return arguments_use(&recognition, body, local) == ArgumentsUse::GivenOn;
            }
            let assigned: Vec<&Operand<'tcx>> = (body.basic_blocks.iter())
                .flat_map(|data| data.statements.iter())
                .filter_map(|statement| match &statement.kind {
                    StatementKind::Assign(assign) if assign.0.as_local() == Some(local) => match &assign.1 {
                        Rvalue::Use(from, _) => Some(from),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
            match assigned[..] {
                [from] => match from.place().filter(|p| p.projection.is_empty()) {
                    Some(from) => local = from.local,
                    None => return false,
                },
                _ => return false,
            }
        }
        false
    }

    /// Refuses a call of `callee` that gives a parameter it writes into a
    /// writer a piece at a time an `Arguments` that isn't the function that
    /// writes into the sink it claims, `operands`' (`sink_form_given`): one
    /// a function returned, or a field held, which the writer's sink can't
    /// be armed for (#1652). std's `write_fmt` refuses its own
    /// (`arguments_not_written`).
    pub(super) fn arguments_given_to_writers(
        &self,
        state: &State<'_, 'tcx>,
        (callee, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        operands: &[&Operand<'tcx>],
        span: Span,
    ) -> R<()> {
        let recognition = self.recognition();
        if recognition.writes_arguments_piecewise(callee, generic_args)
            || recognition.formatter_write_fmt(callee, generic_args)
        {
            return Ok(());
        }
        for (i, operand) in operands.iter().enumerate() {
            if self
                .krate
                .written
                .arguments
                .writes(&recognition, (callee, generic_args), i)
                && self.is_lang_adt(operand.ty(&state.body.local_decls, self.tcx), LangItem::FormatArguments)
                && !self.sink_form_given(state.body, operand)
            {
                return Err(self.refused_piecewise(
                    span,
                    "an `fmt::Arguments` made elsewhere given to a function that writes it into a writer",
                ));
            }
        }
        Ok(())
    }

    /// Whether the call of `callee` at `state.at` takes the `Arguments` it's
    /// given at `index` as the function it is, which it writes into a
    /// writer a piece at a time: a `W: fmt::Write`'s `write_fmt`, given it
    /// for its dictionary's `writeFmt`; a writer's of the crate's own, or a
    /// `Formatter`'s of a writer that writes through a sink, given one its
    /// function writes so, `$writingInto(f, args)` (#1652, step 6). Not
    /// called to make its text first.
    pub(super) fn takes_arguments_function(
        &self,
        state: &State<'_, 'tcx>,
        (callee, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        index: usize,
    ) -> bool {
        let recognition = self.recognition();
        if index != 1
            || !self
                .krate
                .written
                .arguments
                .writes(&recognition, (callee, generic_args), index)
        {
            return false;
        }
        if recognition.writes_through_a_dictionary(callee, generic_args) {
            return true;
        }
        let given = match &state.body.basic_blocks[state.at].terminator().kind {
            TerminatorKind::Call { args, .. } => args
                .get(index)
                .is_some_and(|arg| self.sink_form_given(state.body, &arg.node)),
            _ => false,
        };
        given && (!recognition.formatter_write_fmt(callee, generic_args) || self.writes_through_a_sink(state))
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Of a `write_fmt`, `callee`, into a writer only a dictionary says, a
    /// `W: fmt::Write`'s or a `dyn`'s, whose text may fail, `values`'
    /// second (#1696): its text made by a function, and what it made before
    /// the error written into the writer by its `write_str`, as std's
    /// `write_fmt` writes each piece before it, `$fmtTextOr(() => {..},
    /// (text) => W.writeStr(w, text))`; a `String`'s keeps it. Till ADR
    /// 1135's switch-over makes each piece a write, where a dictionary's
    /// `writeFmt` is given the function (step 6).
    pub(super) fn partial_written_through_a_dictionary(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (callee, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        values: &mut [Value<'tcx>],
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let tcx = self.tcx;
        if crate::lower::recognition::writes_piecewise() || !crate::lower::recognition::is_write_fmt(tcx, callee) {
            return Ok(());
        }
        let by_dictionary = (generic_args.types().next())
            .is_some_and(|writer| crate::lower::recognition::known_by_a_dictionary(writer.peel_refs()));
        let [writer, text @ Value::Fmt(Std::FmtNew, ..)] = values else {
            return Ok(());
        };
        if !by_dictionary {
            return Ok(());
        }
        self.flush(state, out)?;
        let given = self.value_expr(std::mem::replace(writer, Value::Expr(Expr::undefined())), span)?;
        let given = match given.kind {
            js::ExprKind::Var(_) => given,
            _ => self.spill(Base::Temporary, given, out),
        };
        *writer = Value::Expr(given.clone());
        let Value::Expr(make) =
            self.arguments_given_on(std::mem::replace(text, Value::Expr(Expr::undefined())), false, span)?
        else {
            return Err(self.unsupported(span, "this `write!`'s text, from its MIR"));
        };
        let trait_id = tcx.trait_of_assoc(callee).expect("`fmt::Write`'s method");
        let write_str = crate::lower::recognition::trait_method(tcx, trait_id, "write_str");
        let partial = self.fresh(Role::Text);
        let mut body = Vec::new();
        let written = self
            .trait_call(
                write_str,
                generic_args,
                vec![given, Expr::var(&partial)],
                span,
                &mut body,
            )?
            .ok_or_else(|| self.unsupported(span, "this writer's `write_str`"))?;
        body.push(StmtKind::Return(Some(written)).at(js::Span::NONE));
        let partial = Expr::arrow(vec![js::Pattern::Name(partial)], body);
        *text = Value::Expr(Expr::call_helper("$fmtTextOr", vec![make, partial]));
        Ok(())
    }
}
