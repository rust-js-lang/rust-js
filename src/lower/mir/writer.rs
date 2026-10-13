//! A writer's `Formatter` from MIR (ADRs 0054, 0136): which locals are it,
//! the `Debug` builders it makes, what's refused of it, and a call that
//! writes to it or steps a builder, whose JS `display.rs` writes.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
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
            let Some((kind, type_name, mut steps)) = state.built.remove(&root) else {
                return Err(self.unsupported(span, "this `Debug` builder, from its MIR"));
            };
            if let BuilderStep::Finish { non_exhaustive } = step {
                let written = self.builder_written(kind, type_name, steps, non_exhaustive, span)?;
                self.flush(state, out)?;
                crate::lower::display::append_written(&Expr::var(&name), written, false, self.js_span(span), out);
            } else {
                steps.push((step, typed, generic_args));
                state.built.insert(root, (kind, type_name, steps));
            }
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
                state.built.insert(destination.local, (kind, type_name, Vec::new()));
                return Ok(Some(Value::Expr(Expr::undefined())));
            }
            let name = name.clone();
            let others: Vec<&Operand<'tcx>> = (args.iter().enumerate())
                .filter(|&(i, _)| i != at)
                .map(|(_, a)| &a.node)
                .collect();
            let operation = self
                .recognition()
                .write_call(def_id, self.krate.fns.contains_key(&def_id));
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

/// `param`, a writer's `Formatter`, and the locals that are it: each a
/// reborrow or a copy of one, `_5 = &mut (*_2)`.
pub(super) fn formatter_aliases(body: &mir::Body<'_>, param: Local) -> Vec<Local> {
    let mut aliases = vec![param];
    loop {
        let found = (body.basic_blocks.iter().flat_map(|b| &b.statements)).find_map(|s| match &s.kind {
            StatementKind::Assign(assign)
                if assign.0.projection.is_empty()
                    && !aliases.contains(&assign.0.local)
                    && let Rvalue::Ref(_, _, from) | Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _) =
                        &assign.1
                    && aliases.contains(&from.local)
                    && from.projection.iter().all(|e| e == PlaceElem::Deref) =>
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

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether a call of `def_id` takes a writer's `Formatter`, its `at`th
    /// argument, where a writer goes: the crate's own generic `fmt::Write`
    /// code, not a writer, which writes to it (ADR 0180).
    pub(super) fn given_formatter(
        &self,
        def_id: rustc_span::def_id::DefId,
        generic_args: ty::GenericArgsRef<'tcx>,
        at: usize,
    ) -> bool {
        def_id.is_local()
            && self.krate.fns.contains_key(&def_id)
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
    pub(super) fn writer_refusal(&self, body: &mir::Body<'tcx>, param: usize) -> Option<(Span, String)> {
        let this = || "this `Formatter`".to_string();
        let aliases = formatter_aliases(body, Local::from_usize(param + 1));
        let builders = self.writer_builders(body, &aliases);
        // A builder kept in a variable, which a loop may step: MIR's are a
        // chain's.
        let named = |local: Local| {
            (body.var_debug_info.iter())
                .any(|info| matches!(info.value, mir::VarDebugInfoContents::Place(p) if p.local == local))
        };
        if let Some(&local) = builders.keys().find(|&&local| named(local)) {
            let span = body.local_decls[local].source_info.span;
            return Some((span, "a `Debug` builder kept in a variable".to_string()));
        }
        let all: Vec<Local> = aliases.iter().chain(builders.keys()).copied().collect();
        let aliases = all;
        let mut location = mir::Location::START;
        for (block, data) in body.basic_blocks.iter_enumerated() {
            location.block = block;
            for (i, statement) in data.statements.iter().enumerate() {
                location.statement_index = i;
                let of_alias = matches!(&statement.kind, StatementKind::Assign(assign)
                    if assign.0.projection.is_empty() && aliases.contains(&assign.0.local));
                if !of_alias && mentions_locals(&aliases, |m| m.visit_statement(statement, location)) {
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
                    let Some((def_id, generic_args)) = fn_def(func.ty(&body.local_decls, self.tcx)) else {
                        return Some((span, this()));
                    };
                    let whole = args[at].node.place().is_some_and(|p| p.projection.is_empty());
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
                    let operation = self
                        .recognition()
                        .write_call(def_id, self.krate.fns.contains_key(&def_id));
                    if matches!(operation, crate::lower::recognition::WriteCall::Unsupported) && writes && whole {
                        return Some((span, format!("calling `{}`", self.tcx.def_path_str(def_id))));
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
