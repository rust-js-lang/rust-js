//! A writer's `Formatter` from MIR (ADRs 0054, 0136): which locals are it,
//! the `Debug` builders it makes, what's refused of it, and a call that
//! writes to it or steps a builder, whose JS `display.rs` writes.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A body's end: a writer's, `writer_epilogue`; and of a function that
    /// gives a value's `Result`, what `?` of a write's result threw on,
    /// caught and given as its `Err` (ADR 0462).
    pub(super) fn body_epilogue(&mut self, state: &mut State<'_, 'tcx>, out: &mut Vec<Stmt>) {
        if let Some((_, name)) = &state.writer {
            let name = name.clone();
            self.writer_epilogue(&name, out);
        }
        if let Some((_, conversion)) = state.thrown_writes.take() {
            let error = self.fresh("error");
            let caught = Expr::call_helper("$writeCaught", vec![Expr::var(&error)]);
            let converted = conversion.map_or_else(|| caught.clone(), |conversion| conversion.of(caught.clone()));
            let handler =
                vec![StmtKind::Return(Some(crate::lower::std_types::result::err(converted))).at(js::Span::NONE)];
            *out = vec![StmtKind::TryCatch(std::mem::take(out), Some(error), handler).at(js::Span::NONE)];
        }
    }

    /// A writer's body's end, `name` the string it writes to: one that may
    /// fail gives what it wrote before, in front of what what it called did
    /// (ADR 0187), `let f = ""; try { .. } catch (error) { .. } return f;`;
    /// another, as tidy writes it (`tidy::writer_body`).
    pub(super) fn writer_epilogue(&mut self, name: &str, out: &mut Vec<Stmt>) {
        match self.writer_fails() {
            // One that may fail gives what it wrote before, in front of
            // what what it called did (ADR 0187): `let f = ""; try { .. }
            // catch (error) { .. } return f;`.
            // One that writes nothing, its string made `""` and given as it
            // is, `return "";`, whose `let` tidy took: what it wrote before
            // is `""`, and its body gives what it gives.
            true if js::mentions_in(out, name) == 0 => {
                let error = self.fresh("error");
                let written = Expr::call(Expr::helper("$fmtWritten"), vec![Expr::var(&error), Expr::str("")]);
                let handler = vec![StmtKind::Throw(written).at(js::Span::NONE)];
                *out = vec![StmtKind::TryCatch(std::mem::take(out), Some(error), handler).at(js::Span::NONE)];
            }
            true => {
                let made = (out.iter()).position(|s| matches!(&s.kind, StmtKind::Let(n, _) if n == name));
                let mut body = out.split_off(made.map_or(0, |at| at + 1));
                // Its last `return f`, the one after the `try`.
                if matches!(body.last().map(|s| &s.kind), Some(StmtKind::Return(Some(r)))
                    if matches!(&r.kind, js::ExprKind::Var(v) if v == name))
                {
                    body.pop();
                }
                let error = self.fresh("error");
                let written = Expr::call(Expr::helper("$fmtWritten"), vec![Expr::var(&error), Expr::var(name)]);
                let handler = vec![StmtKind::Throw(written).at(js::Span::NONE)];
                out.push(StmtKind::TryCatch(body, Some(error), handler).at(js::Span::NONE));
                out.push(StmtKind::Return(Some(Expr::var(name))).at(js::Span::NONE));
            }
            false => tidy::writer_body(out, name),
        }
    }

    /// A call of a writer's: a step of its `Debug` builder, kept till its
    /// `finish`, which writes what it's been given (ADR 0136); or a write to
    /// its `Formatter`, `f += s` (ADR 0054). `None` of another.
    pub(super) fn mir_writer_call(
        &mut self,
        state: &mut State<'_, 'tcx>,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        destination: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Value<'tcx>>> {
        let decls = &state.body.local_decls;
        // A step of a writer's `Debug` builder, kept till its `finish`,
        // which writes what it's been given (ADR 0136).
        if let Some(root) = (args.first())
            .and_then(|a| a.node.place())
            .and_then(|p| state.builders.get(&p.local))
            .copied()
            && let Some((def_id, generic_args)) = fn_def(func.ty(decls, self.tcx))
            && let Some((_, name)) = state.writer.clone()
        {
            let Some(step) = crate::lower::recognition::builder_step(self.tcx, def_id) else {
                return Err(self.unsupported(span, "this `Debug` builder's step, from its MIR"));
            };
            let tys: Vec<Ty<'tcx>> = args[1..].iter().map(|a| a.node.ty(decls, self.tcx)).collect();
            let operands: Vec<&Operand<'tcx>> = args[1..].iter().map(|a| &a.node).collect();
            let values = self.take_operands(state, &operands, out)?;
            // Each read at its `finish`, in the order given: a chain's steps,
            // which nothing runs between.
            let mut typed = Vec::new();
            for (value, ty) in values.into_iter().zip(tys) {
                let value = self.value_expr(value, span)?;
                // `entries(v.iter())`'s fresh `$iter(v)`: all of `v`.
                let value = match step == BuilderStep::Entries {
                    true => crate::lower::iterators::iterated(value),
                    false => value,
                };
                typed.push((value, ty));
            }
            let Some((kind, type_name, mut steps, parts)) = state.built.remove(&root) else {
                return Err(self.unsupported(span, "this `Debug` builder, from its MIR"));
            };
            match (step, parts) {
                (BuilderStep::Finish { non_exhaustive }, None) => {
                    let written = self.builder_written(kind, type_name, steps, non_exhaustive, span, out)?;
                    self.flush(state, out)?;
                    crate::lower::display::append_written(&Expr::var(&name), written, false, self.js_span(span), out);
                }
                // Held in a variable: its parts so far, none perhaps, where a
                // struct's or a tuple's is its name alone (ADR 0532).
                (BuilderStep::Finish { non_exhaustive }, Some(parts)) => {
                    let all = vec![crate::lower::display::BuilderPart::Many(Expr::var(&parts))];
                    let written = self.builder_assembled(kind, type_name.clone(), all, non_exhaustive);
                    let written = match (kind, type_name) {
                        (DebugBuilder::Struct | DebugBuilder::Tuple, Some(type_name)) if !non_exhaustive => {
                            let none = Expr::bin(js::Op::Eq, Expr::member(Expr::var(&parts), "length"), Expr::int(0));
                            Expr::cond(none, type_name, written)
                        }
                        _ => written,
                    };
                    self.flush(state, out)?;
                    crate::lower::display::append_written(&Expr::var(&name), written, false, self.js_span(span), out);
                }
                (step, Some(parts)) => {
                    let part = self.builder_part(kind, (step, typed, generic_args), span, out)?;
                    let added = match part {
                        crate::lower::display::BuilderPart::One(text) => text,
                        crate::lower::display::BuilderPart::Many(texts) => Expr::spread(texts),
                    };
                    self.flush(state, out)?;
                    let push = Expr::call(Expr::member(Expr::var(&parts), "push"), vec![added]);
                    out.push(StmtKind::Expr(push).at(self.js_span(span)));
                    state.built.insert(root, (kind, type_name, steps, Some(parts)));
                }
                (step, None) => {
                    steps.push((step, typed, generic_args));
                    state.built.insert(root, (kind, type_name, steps, None));
                }
            }
            return Ok(Some(Value::Expr(Expr::undefined())));
        }
        // A writer callable given the writer's `Formatter`, `with_word(word,
        // f)`: what it writes, `f += withWord(word)`, as a writer function's.
        if let Some((aliases, name)) = state.writer.clone()
            && let Some(tuple) = args.get(1).and_then(|a| a.node.place())
            && self.callables_given(state.body, &aliases).contains(&tuple.local)
            && let Some((def_id, generic_args)) = fn_def(func.ty(decls, self.tcx))
            && let Some(at) =
                (self.recognition()).callable_formatter(def_id, generic_args, destination.ty(decls, self.tcx).ty)
        {
            let arg_tys: Vec<Ty<'tcx>> = args.iter().map(|a| a.node.ty(decls, self.tcx)).collect();
            let values = self.take_operands(state, &[&args[0].node, &args[1].node], out)?;
            let written = self.mir_closure_call(values, &arg_tys, Some(at), span)?;
            let written = self.value_expr(written, span)?;
            self.flush(state, out)?;
            crate::lower::display::append_written(&Expr::var(&name), written, false, self.js_span(span), out);
            return Ok(Some(Value::Expr(Expr::undefined())));
        }
        // A writer called through a `fn` pointer given the writer's
        // `Formatter`, `cb(x, f)`: what it writes, `f += cb(x)`, as a writer
        // callable's (ADR 0538).
        if let Some((aliases, name)) = state.writer.clone()
            && let callee = func.ty(decls, self.tcx)
            && let Some(at) = self.recognition().pointer_formatter(callee)
            && (args.get(at).and_then(|a| a.node.place()))
                .is_some_and(|p| p.projection.is_empty() && aliases.contains(&p.local))
        {
            let given: Vec<Ty<'tcx>> = args.iter().map(|a| a.node.ty(decls, self.tcx)).collect();
            let arg_tys = [callee, Ty::new_tup(self.tcx, &given)];
            let mut operands = vec![func];
            operands.extend(args.iter().map(|a| &a.node));
            let mut values = self.take_operands(state, &operands, out)?;
            let callee = values.remove(0);
            let written = self.mir_closure_call(vec![callee, Value::List(values)], &arg_tys, Some(at), span)?;
            let written = self.value_expr(written, span)?;
            self.flush(state, out)?;
            crate::lower::display::append_written(&Expr::var(&name), written, false, self.js_span(span), out);
            return Ok(Some(Value::Expr(Expr::undefined())));
        }
        // A write to the writer's `Formatter` (ADR 0054): `f += s`.
        if let Some((aliases, name)) = &state.writer
            && let Some(at) = args
                .iter()
                .position(|a| a.node.place().is_some_and(|p| aliases.contains(&p.local)))
            && let Some((def_id, generic_args)) = fn_def(func.ty(decls, self.tcx))
            && !self.given_formatter(def_id, generic_args, at)
        {
            // `f.alternate()`, `f.width()` and the like: what this writer's
            // `Formatter` was given (ADRs 0137, 0143).
            if let Some(query) = crate::lower::recognition::formatter_query(self.tcx, def_id) {
                return (self.formatter_answer(query))
                    .map(|answer| Some(Value::Expr(answer)))
                    .ok_or_else(|| self.unsupported(span, "asking a `Formatter` here"));
            }
            // `f.debug_struct("P")` and the like: a builder, its type's name.
            if state.builders.contains_key(&destination.local) {
                let kind = match destination.ty(decls, self.tcx).ty.kind() {
                    ty::Adt(builder, _) => crate::lower::recognition::debug_builder(self.tcx, builder.did()),
                    _ => None,
                };
                let Some(kind) = kind else {
                    return Err(self.unsupported(span, "this `Debug` builder, from its MIR"));
                };
                let type_name = match args.get(1) {
                    Some(a) if matches!(kind, DebugBuilder::Struct | DebugBuilder::Tuple) => {
                        let value = self.mir_operand(state, &a.node, out)?;
                        Some(self.value_expr(value, span)?)
                    }
                    _ => None,
                };
                // Held in a variable, which a branch may step or not: the
                // array its steps add their parts to (ADR 0532).
                let held = (state.body.var_debug_info.iter()).any(
                    |info| matches!(info.value, mir::VarDebugInfoContents::Place(p) if p.local == destination.local),
                );
                let parts = held.then(|| self.fresh("parts"));
                if let Some(parts) = &parts {
                    self.flush(state, out)?;
                    out.push(StmtKind::Let(parts.clone(), Some(Expr::array(Vec::new()))).at(self.js_span(span)));
                }
                state
                    .built
                    .insert(destination.local, (kind, type_name, Vec::new(), parts));
                return Ok(Some(Value::Expr(Expr::undefined())));
            }
            let name = name.clone();
            let others: Vec<&Operand<'tcx>> = (args.iter().enumerate())
                .filter(|&(i, _)| i != at)
                .map(|(_, a)| &a.node)
                .collect();
            let operation = self.recognition().write_call(def_id, self.is_rust_fn(def_id));
            let values = self.take_operands(state, &others, out)?;
            // A `write!` of what may fail: piece by piece, what came before
            // written (ADR 0187).
            let pieces = values.iter().any(|v| matches!(v, Value::Fmt(Std::FmtNew, ..)));
            let values = (values.into_iter())
                .map(|v| self.value_expr(v, span))
                .collect::<R<Vec<_>>>()?;
            let pretty = self.writer_pretty();
            let written = self.written_of(operation, def_id, generic_args, values, &pretty, span, out)?;
            self.flush(state, out)?;
            crate::lower::display::append_written(&Expr::var(&name), written, pieces, self.js_span(span), out);
            return Ok(Some(Value::Expr(Expr::undefined())));
        }
        Ok(None)
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The locals of a writer's `body` that are the arguments' tuple of a
    /// writer callable given its `Formatter`, one of `aliases`, where that
    /// writes, `with_word(word, f)`'s `(word, f)`: each made once, of the
    /// `Formatter` itself, and given to the call alone. rustc's borrows
    /// leave the `Formatter` nowhere else in it.
    pub(super) fn callables_given(&self, body: &mir::Body<'tcx>, aliases: &[Local]) -> Vec<Local> {
        let decls = &body.local_decls;
        let made = |tuple: Local| {
            let mut made = (body.basic_blocks.iter().flat_map(|b| &b.statements)).filter_map(|s| match &s.kind {
                StatementKind::Assign(assign) if assign.0.local == tuple => Some(&assign.1),
                _ => None,
            });
            match (made.next(), made.next()) {
                (Some(Rvalue::Aggregate(kind, items)), None) if matches!(**kind, mir::AggregateKind::Tuple) => {
                    Some(items.iter().collect::<Vec<_>>())
                }
                _ => None,
            }
        };
        (body.basic_blocks.iter())
            .filter_map(|data| match &data.terminator().kind {
                TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } => {
                    let (def_id, generic_args) = fn_def(func.ty(decls, self.tcx))?;
                    let output = destination.ty(decls, self.tcx).ty;
                    let at = self.recognition().callable_formatter(def_id, generic_args, output)?;
                    let tuple = args.get(1)?.node.place().filter(|p| p.projection.is_empty())?.local;
                    let items = made(tuple)?;
                    let given =
                        (items[at].place()).is_some_and(|p| p.projection.is_empty() && aliases.contains(&p.local));
                    let once = (body.basic_blocks.iter())
                        .filter(|b| {
                            matches!(&b.terminator().kind, TerminatorKind::Call { args, .. }
                            if args.iter().any(|a| a.node.place().is_some_and(|p| p.local == tuple)))
                        })
                        .count()
                        == 1;
                    (given && once).then_some(tuple)
                }
                _ => None,
            })
            .collect()
    }
}

/// `param`, a writer's `Formatter`, and the locals that are it: each a
/// reborrow or a copy of one, `_5 = &mut (*_2)`.
pub(super) fn formatter_aliases(body: &mir::Body<'_>, given: &[Local], captured: Option<FieldIdx>) -> Vec<Local> {
    let mut aliases = given.to_vec();
    // A closure's capture of it, its environment's `field`, through as many
    // references as it's behind: `deref_copy (*_1).0` (ADR 0539).
    let of_capture = |from: &Place<'_>| {
        from.local == Local::from_usize(1)
            && captured.is_some_and(|field| {
                let mut fields = from.projection.iter().filter(|e| *e != PlaceElem::Deref);
                matches!((fields.next(), fields.next()), (Some(PlaceElem::Field(f, _)), None) if f == field)
            })
    };
    loop {
        let found = (body.basic_blocks.iter().flat_map(|b| &b.statements)).find_map(|s| match &s.kind {
            StatementKind::Assign(assign)
                if assign.0.projection.is_empty()
                    && !aliases.contains(&assign.0.local)
                    && let Rvalue::Ref(_, _, from)
                    | Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _)
                    | Rvalue::CopyForDeref(from) = &assign.1
                    && ((aliases.contains(&from.local) && from.projection.iter().all(|e| e == PlaceElem::Deref))
                        || of_capture(from)) =>
            {
                Some(assign.0.local)
            }
            _ => None,
        });
        match found {
            Some(alias) => aliases.push(alias),
            None => return aliases,
        }
    }
}

impl<'tcx> State<'_, 'tcx> {
    /// The string a writer makes, `let f = ""`, which it gives back: not a
    /// closure's that writes its maker's (ADR 0539).
    pub(super) fn made_writing(&self) -> Option<&String> {
        self.writer
            .as_ref()
            .filter(|_| !self.writes_captured)
            .map(|(_, name)| name)
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The writer `body` is (ADR 0054), if it's one: of one given a
    /// `Formatter`, its parameter `formatter` and its name, the locals that are it, and
    /// the string it writes to; of a closure that captured its maker's,
    /// given `captures`, its maker's string (ADR 0539). Refused, what's
    /// refused of it.
    pub(super) fn body_writer(
        &mut self,
        state: &mut State<'_, 'tcx>,
        body: &mir::Body<'tcx>,
        formatter: Option<(Local, &str)>,
        captures: Option<&[(Expr, bool)]>,
    ) -> R<Option<(Vec<Local>, String)>> {
        let (aliases, name) = match (formatter, captures) {
            (Some((param, named)), _) => {
                let name = match named {
                    "" => self.fresh("f"),
                    name => name.to_string(),
                };
                (formatter_aliases(body, &[param], None), name)
            }
            (None, Some(captures)) => match self.captured_formatter(body, captures)? {
                Some((field, name)) => {
                    state.writes_captured = true;
                    (formatter_aliases(body, &[], Some(field)), name)
                }
                None => return Ok(None),
            },
            (None, None) => return Ok(None),
        };
        if let Some((span, what)) = self.writer_refusal(body, &aliases) {
            return Err(self.unsupported(span, &what));
        }
        Ok(Some((aliases, name)))
    }

    /// A writer's `Formatter`'s options, `options` its parameter of them,
    /// where the crate gives any (ADRs 0058, 0137); a closure's that writes
    /// its maker's, its maker's (ADR 0539).
    pub(super) fn writer_options(&self, state: &State<'_, 'tcx>, options: Option<&str>) -> Option<Expr> {
        match state.writes_captured {
            true => self.given_to_writer(None),
            false => options.map(Expr::var),
        }
    }

    /// Of a closure, `body`, that captured its maker's `Formatter`, given
    /// `captures`, what it took, as its maker read them: which of its
    /// environment's fields it is, and the string its maker writes to,
    /// which JS's closure writes as its maker does (ADR 0539).
    fn captured_formatter(&self, body: &mir::Body<'tcx>, captures: &[(Expr, bool)]) -> R<Option<(FieldIdx, String)>> {
        let ty::Closure(_, args) = self.tcx.type_of(body.source.def_id()).skip_binder().kind() else {
            return Ok(None);
        };
        let upvars = args.as_closure().upvar_tys();
        let Some(at) = upvars
            .iter()
            .position(|ty| self.recognition().is_formatter_type(ty.peel_refs()))
        else {
            return Ok(None);
        };
        match captures.get(at).map(|(e, _)| &e.kind) {
            Some(js::ExprKind::Var(name)) => Ok(Some((FieldIdx::from_usize(at), name.clone()))),
            _ => Err(self.unsupported(self.tcx.def_span(body.source.def_id()), "this `Formatter`")),
        }
    }

    /// Whether a call of `def_id` takes a writer's `Formatter`, its `at`th
    /// argument, where a writer goes: the crate's own generic `fmt::Write`
    /// code, not a writer, which writes to it (ADR 0180).
    pub(super) fn given_formatter(
        &self,
        def_id: rustc_span::def_id::DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        at: usize,
    ) -> bool {
        // The crate's own, or another rust-js crate's, which its library
        // exports (ADR 0100): bitflags' `to_writer` its macro's `Debug` calls.
        self.is_rust_fn(def_id)
            && self.formatter_param(def_id).is_none()
            && !self.recognition().fmt_write_on_formatter(def_id, generic_args)
            && (self.tcx.fn_sig(def_id).skip_binder().skip_binder().inputs())
                .get(at)
                .is_some_and(|ty| ty.peel_refs().is_param(0) || matches!(ty.peel_refs().kind(), ty::Param(_)))
    }

    /// A writer's `Debug` builders, of its `Formatter`, `aliases` (ADR
    /// 0136): each local that's one, to the one `f.debug_struct(..)` made,
    /// of what a step gives back, `&mut` to it, or a reference to one.
    pub(super) fn writer_builders(&self, body: &mir::Body<'tcx>, aliases: &[Local]) -> HashMap<Local, Local> {
        let mut builders: HashMap<Local, Local> = HashMap::new();
        let made = |builders: &HashMap<Local, Local>, from: &Place<'tcx>| {
            builders
                .get(&from.local)
                .copied()
                .filter(|_| from.projection.iter().all(|e| e == PlaceElem::Deref))
        };
        loop {
            let before = builders.len();
            for data in body.basic_blocks.iter() {
                for statement in &data.statements {
                    if let StatementKind::Assign(assign) = &statement.kind
                        && assign.0.projection.is_empty()
                        && let Rvalue::Ref(_, _, from) | Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _) =
                            &assign.1
                        && let Some(root) = made(&builders, from)
                    {
                        builders.entry(assign.0.local).or_insert(root);
                    }
                }
                let TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } = &data.terminator().kind
                else {
                    continue;
                };
                let (Some((def_id, _)), Some(first)) = (
                    fn_def(func.ty(&body.local_decls, self.tcx)),
                    args.first().and_then(|a| a.node.place()),
                ) else {
                    continue;
                };
                let on_formatter = self.tcx.inherent_impl_of_assoc(def_id).is_some_and(|imp| {
                    let owner = self.tcx.type_of(imp).instantiate_identity().skip_normalization();
                    self.recognition().is_formatter_type(owner)
                });
                if aliases.contains(&first.local)
                    && on_formatter
                    && crate::lower::recognition::makes_debug_builder(self.tcx, def_id)
                {
                    builders.entry(destination.local).or_insert(destination.local);
                } else if let Some(root) = made(&builders, &first)
                    && matches!(
                        crate::lower::recognition::builder_step(self.tcx, def_id),
                        Some(BuilderStep::Field | BuilderStep::Entry | BuilderStep::Entries)
                    )
                {
                    builders.entry(destination.local).or_insert(root);
                }
            }
            if builders.len() == before {
                return builders;
            }
        }
    }
}

/// Whether `stmt` reads or writes one of `locals`.
fn mentions_locals(locals: &[Local], visit: impl FnOnce(&mut Mentions<'_>)) -> bool {
    let mut mentions = Mentions { locals, found: false };
    visit(&mut mentions);
    mentions.found
}

struct Mentions<'l> {
    locals: &'l [Local],
    found: bool,
}

impl<'tcx> Visitor<'tcx> for Mentions<'_> {
    fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
        self.found |= self.locals.contains(&local) && !matches!(context, PlaceContext::NonUse(_));
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What's refused of a writer (ADR 0054), and where: a use of its
    /// `Formatter`, `param`, that isn't a write to it, `write!`, `{}` of a
    /// value, another writer given it, a question of it, a `Debug` builder's
    /// chain, or the crate's generic `fmt::Write` code given it.
    pub(super) fn writer_refusal(&self, body: &mir::Body<'tcx>, aliases: &[Local]) -> Option<(Span, String)> {
        let this = || "this `Formatter`".to_string();
        let aliases = aliases.to_vec();
        let builders = self.writer_builders(body, &aliases);
        let all: Vec<Local> = aliases.iter().chain(builders.keys()).copied().collect();
        let aliases = all;
        // The arguments' tuple of a writer callable given this `Formatter`
        // where it writes, `with_word(word, f)`.
        let callables_given = self.callables_given(body, &aliases);
        let mut location = mir::Location::START;
        for (block, data) in body.basic_blocks.iter_enumerated() {
            location.block = block;
            for (i, statement) in data.statements.iter().enumerate() {
                location.statement_index = i;
                // What borrowck alone reads, `let mut dbg = f.debug_struct(..)`'s.
                if matches!(
                    statement.kind,
                    StatementKind::FakeRead(_) | StatementKind::PlaceMention(_)
                ) {
                    continue;
                }
                let of_alias = matches!(&statement.kind, StatementKind::Assign(assign)
                    if assign.0.projection.is_empty() && aliases.contains(&assign.0.local));
                let given = matches!(&statement.kind, StatementKind::Assign(assign)
                    if assign.0.projection.is_empty() && callables_given.contains(&assign.0.local));
                // A struct that holds it, `FormatterSink { f }`: a handle on
                // its text, which what the struct is given writes (ADR 0461).
                // Or a closure that captured it, which writes to it as it does
                // (ADR 0539).
                let held = match &statement.kind {
                    StatementKind::Assign(assign) => match &assign.1 {
                        Rvalue::Aggregate(kind, items) => {
                            matches!(**kind, mir::AggregateKind::Adt(..) | mir::AggregateKind::Closure(..))
                                && items.iter().all(|item| {
                                    item.place().is_some_and(|p| p.projection.is_empty())
                                        || !mentions_locals(&aliases, |m| m.visit_operand(item, location))
                                })
                        }
                        _ => false,
                    },
                    _ => false,
                };
                if !of_alias && !given && !held && mentions_locals(&aliases, |m| m.visit_statement(statement, location))
                {
                    return Some((statement.source_info.span, this()));
                }
            }
            location.statement_index = data.statements.len();
            let terminator = data.terminator();
            let span = terminator.source_info.span;
            match &terminator.kind {
                TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } => {
                    let at: Vec<usize> = (args.iter().enumerate())
                        .filter(|(_, a)| mentions_locals(&aliases, |m| m.visit_operand(&a.node, location)))
                        .map(|(i, _)| i)
                        .collect();
                    let [at] = at[..] else {
                        if at.is_empty() {
                            continue;
                        }
                        return Some((span, this()));
                    };
                    let whole = args[at].node.place().is_some_and(|p| p.projection.is_empty());
                    // A writer called through a `fn` pointer, `cb(x, f)` (ADR 0538).
                    let callee = func.ty(&body.local_decls, self.tcx);
                    if whole && self.recognition().pointer_formatter(callee) == Some(at) {
                        continue;
                    }
                    let Some((def_id, generic_args)) = fn_def(callee) else {
                        return Some((span, this()));
                    };
                    // A builder's step, or a builder its `Formatter` makes.
                    let of_builder = args[at].node.place().is_some_and(|p| builders.contains_key(&p.local));
                    if of_builder || builders.contains_key(&destination.local) {
                        let known = crate::lower::recognition::builder_step(self.tcx, def_id).is_some();
                        if whole && at == 0 && (known || !of_builder) {
                            continue;
                        }
                        let step = self.tcx.item_name(def_id);
                        return Some((span, format!("a `Debug` builder's `{step}`")));
                    }
                    if whole && at == 0 && crate::lower::recognition::formatter_query(self.tcx, def_id).is_some() {
                        continue;
                    }
                    if whole && self.given_formatter(def_id, generic_args, at) {
                        continue;
                    }
                    let writes = self.formatter_param(def_id) == Some(at)
                        || (at == 0 && self.recognition().fmt_write_on_formatter(def_id, generic_args));
                    let operation = self.recognition().write_call(def_id, self.is_rust_fn(def_id));
                    if matches!(operation, crate::lower::recognition::WriteCall::Unsupported) && writes && whole {
                        return Some((span, crate::lower::refusal::calling(self.tcx, def_id)));
                    }
                    if !whole || !writes || aliases.contains(&destination.local) {
                        return Some((span, this()));
                    }
                }
                _ => {
                    if mentions_locals(&aliases, |m| m.visit_terminator(terminator, location)) {
                        return Some((span, this()));
                    }
                }
            }
        }
        None
    }
}
