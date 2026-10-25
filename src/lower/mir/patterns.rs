//! The locals a pattern binds: an enum's payloads, a `let`'s and a
//! parameter's patterns taken apart, and the `Option`s they write.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The locals that patterns bind in an enum's variant, by THIR's
    /// patterns: `if let`s', `match`es' and `let`s'.
    pub(super) fn mir_payloads(&self, body: &mir::Body<'tcx>) -> std::collections::HashSet<Local> {
        let mut spans = Vec::new();
        let mut bound = |pat: &thir::Pat<'tcx>| {
            pat.walk(|p| match &p.kind {
                // Not a `for` loop's `Some(x)`: its item, a variable.
                thir::PatKind::Variant { subpatterns, .. }
                    if !p.span.is_desugaring(rustc_span::DesugaringKind::ForLoop) =>
                {
                    for field in subpatterns {
                        field.pattern.walk_always(|b| {
                            if let thir::PatKind::Binding { .. } = b.kind {
                                spans.push(b.span);
                            }
                        });
                    }
                    false
                }
                _ => true,
            })
        };
        for stmt in self.frame.thir.stmts.iter() {
            if let thir::StmtKind::Let { pattern, .. } = &stmt.kind {
                bound(pattern);
            }
        }
        for expr in self.frame.thir.exprs.iter() {
            if let thir::ExprKind::Let { pat, .. } = &expr.kind {
                bound(pat);
            }
        }
        for arm in self.frame.thir.arms.iter() {
            bound(&arm.pattern);
        }
        // An arm's of the value whole, `x if x.len() > 1`, as a variant's part.
        spans.extend((self.frame.thir.arms.iter()).filter_map(|arm| match arm.pattern.kind {
            thir::PatKind::Binding { .. } => Some(arm.pattern.span),
            _ => None,
        }));
        (body.var_debug_info.iter())
            .filter_map(|info| match info.value {
                mir::VarDebugInfoContents::Place(place)
                    if place.projection.is_empty() && spans.contains(&info.source_info.span) =>
                {
                    Some(place.local)
                }
                _ => None,
            })
            .collect()
    }

    /// The locals `OptionWrites::null_nones` says: each write a `Some(..)`, a
    /// `None` or a call, one that may give `None` at least, and each read a
    /// nullable field's or binding parameter's, or a copy's only they read
    /// (ADR 0781, amended by 0841); or a
    /// `#[rust_js::nullable]` function's result, by its return.
    fn mir_null_nones(&self, body: &mir::Body<'tcx>, locals: &Locals) -> std::collections::HashSet<Local> {
        let mut made: std::collections::HashMap<Local, (usize, bool)> = Default::default();
        let mut given: std::collections::HashMap<Local, usize> = Default::default();
        // What a call makes, and what JS reads as it's stored, a field's or
        // a binding's parameter's: a call's `Option` only the crate's own
        // functions' nullable parameters read is given as it is, their
        // bodies reading `null` and `undefined` alike (ADR 0861).
        let mut called: std::collections::HashSet<Local> = Default::default();
        let mut stored: std::collections::HashSet<Local> = Default::default();
        // Each copy of a variable, `_2 = copy _1`, MIR's of `owner` in `Info
        // { owner }`: the variable's read, given where the copy is.
        let mut copies: Vec<(Local, Local)> = Vec::new();
        for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
            let mir::StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            if let Rvalue::Use(mir::Operand::Copy(read) | mir::Operand::Move(read), _) = &assign.1
                && read.projection.is_empty()
                && assign.0.projection.is_empty()
            {
                copies.push((assign.0.local, read.local));
            }
            let Rvalue::Aggregate(kind, operands) = &assign.1 else {
                continue;
            };
            let AggregateKind::Adt(did, variant, ..) = **kind else {
                continue;
            };
            if assign.0.projection.is_empty() && self.tcx.is_lang_item(did, LangItem::Option) {
                let (writes, none) = made.entry(assign.0.local).or_default();
                *writes += 1;
                *none |= variant.as_u32() == 0;
            }
            let fields = &self.tcx.adt_def(did).variant(variant).fields;
            for (field, operand) in operands.iter_enumerated() {
                if let Some(read) = operand.place()
                    && read.projection.is_empty()
                    && bindings::is_nullable(self.tcx, &fields[field])
                {
                    *given.entry(read.local).or_default() += 1;
                    stored.insert(read.local);
                }
            }
        }
        // And a binding's parameter it names `#[rust_js::nullable(..)]`. An
        // `Option` a call gives, which may be `undefined`, is written so,
        // where the call's made (`nullable_written`): `c ? f(c) : null` of
        // `map`'s `undefined`.
        for block in body.basic_blocks.iter() {
            let mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &block.terminator().kind
            else {
                continue;
            };
            // Not a clone, which `nulled` gives what it's a clone of.
            let cloned = func.const_fn_def().is_some_and(|(def_id, _)| {
                (self.tcx.trait_of_assoc(def_id)).is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Clone))
            });
            if !cloned
                && destination.projection.is_empty()
                && matches!(destination.ty(body, self.tcx).ty.kind(),
                    ty::Adt(adt, _) if self.tcx.is_lang_item(adt.did(), LangItem::Option))
            {
                let (writes, none) = made.entry(destination.local).or_default();
                *writes += 1;
                *none = true;
                called.insert(destination.local);
            }
            let Some((def_id, _)) = func.const_fn_def() else {
                continue;
            };
            let nullable = bindings::nullable_params(self.tcx, def_id);
            if nullable.is_empty() {
                continue;
            }
            let idents = self.tcx.fn_arg_idents(def_id);
            let binding = bindings::is_binding(self.tcx, def_id);
            for (i, arg) in args.iter().enumerate() {
                if let Some(read) = arg.node.place()
                    && read.projection.is_empty()
                    && (idents.get(i).copied().flatten()).is_some_and(|ident| nullable.contains(&ident.name))
                {
                    *given.entry(read.local).or_default() += 1;
                    if binding {
                        stored.insert(read.local);
                    }
                }
            }
        }
        // What makes a `#[rust_js::nullable]` field's `Vec<Option<T>>`, its
        // cell's `new` and the call that gives its items, `RefCell::new(
        // create_lane_map(None))`: each `Option` it's given, its `None`
        // `null` where it's made, as upstream's `createLaneMap(null)` (ADR
        // 0586, amended).
        let mut items: std::collections::HashSet<Local> = Default::default();
        for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
            if let mir::StatementKind::Assign(assign) = &statement.kind
                && let Rvalue::Aggregate(kind, operands) = &assign.1
                && let AggregateKind::Adt(did, variant, _, _, None) = **kind
            {
                let fields = &self.tcx.adt_def(did).variant(variant).fields;
                for (field, operand) in fields.iter().zip(operands.iter()) {
                    let ty = operand.ty(body, self.tcx);
                    if let Some(read) = operand.place()
                        && read.projection.is_empty()
                        && bindings::is_nullable(self.tcx, field)
                        && bindings::nullable_items(self.tcx, ty).is_some()
                    {
                        items.insert(read.local);
                    }
                }
            }
        }
        let mut options: std::collections::HashSet<Local> = Default::default();
        let mut grown = true;
        while grown {
            grown = false;
            for block in body.basic_blocks.iter() {
                if let mir::TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } = &block.terminator().kind
                    && destination.projection.is_empty()
                    && items.contains(&destination.local)
                    && locals.reads[destination.local] == 1
                {
                    let cell_new = func.const_fn_def().is_some_and(|(def_id, generic_args)| {
                        self.recognition().classify(def_id, generic_args) == Some(Std::CellNew)
                    });
                    for arg in args.iter() {
                        let Some(read) = arg.node.place().filter(|read| read.projection.is_empty()) else {
                            continue;
                        };
                        grown |= match cell_new {
                            true => items.insert(read.local),
                            false => self.option_of(read.ty(body, self.tcx).ty).is_some() && options.insert(read.local),
                        };
                    }
                }
            }
        }
        for option in options {
            *given.entry(option).or_default() += 1;
        }
        // A copy made once and only given to nullable fields or parameters
        // gives what it's a copy of to them: `let owner = None;`'s `None` is `null` where
        // it's made (ADR 0781).
        for (copy, of) in copies {
            if locals.writes[copy] == 1 && given.get(&copy).is_some_and(|&n| n == locals.reads[copy]) {
                *given.entry(of).or_default() += 1;
                if stored.contains(&copy) {
                    stored.insert(of);
                }
            }
        }
        // A `#[rust_js::nullable]` function's result is read by its return.
        let result = bindings::nullable_result(self.tcx, body.source.def_id());
        (made.into_iter())
            .filter(|&(local, (writes, none))| {
                // Of a function's result, each write: what isn't an
                // `Option` it makes is `nullable_written`'s.
                match local == mir::RETURN_PLACE && result {
                    true => true,
                    false => {
                        none && writes == locals.writes[local]
                            && given.get(&local) == Some(&locals.reads[local])
                            && (!called.contains(&local) || stored.contains(&local))
                    }
                }
            })
            .map(|(local, _)| local)
            .collect()
    }

    /// Each variable a `let` binds: where the `let` is written, which its
    /// declaration maps to (`State::let_spans`).
    pub(super) fn mir_let_spans(&self, body: &mir::Body<'tcx>) -> HashMap<Local, js::Span> {
        let lets: Vec<(Span, Span)> = (self.frame.thir.stmts.iter())
            .filter_map(|s| match &s.kind {
                thir::StmtKind::Let { pattern, span, .. } => Some((pattern.span, *span)),
                _ => None,
            })
            .collect();
        (body.var_debug_info.iter())
            .filter_map(|info| match info.value {
                mir::VarDebugInfoContents::Place(place) if place.projection.is_empty() => lets
                    .iter()
                    .find(|(pattern, _)| pattern.contains(info.source_info.span))
                    .map(|(_, at)| (place.local, self.js_span(*at))),
                _ => None,
            })
            .collect()
    }

    /// What `body`'s writes say of its `Option`s.
    pub(super) fn mir_option_writes(
        &self,
        body: &mir::Body<'tcx>,
        promoted: &IndexVec<Promoted, mir::Body<'tcx>>,
        locals: &Locals,
    ) -> OptionWrites {
        OptionWrites {
            null_nones: self.mir_null_nones(body, locals),
            made_some: self.mir_made_some(body, promoted, locals),
            registered: self.mir_registered(body),
            thrown: self.mir_thrown(body, locals),
            present: self.mir_present(body),
            held: self.frame.facts.held_guards.clone(),
        }
    }

    /// The locals `OptionWrites::thrown` says, each written once: a
    /// `downcast_ref` that finds what was thrown (`finds_thrown`), its
    /// `Some`'s discriminant; and `?` of one, its `Continue`'s.
    fn mir_thrown(&self, body: &mir::Body<'tcx>, locals: &Locals) -> HashMap<Local, u128> {
        let tcx = self.tcx;
        // The discriminant of `ty`'s variant `variant`, a lang item.
        let discriminant = |ty: Ty<'tcx>, variant: LangItem| match ty.kind() {
            ty::Adt(adt, _) => {
                let id = tcx.lang_items().get(variant)?;
                Some(adt.discriminant_for_variant(tcx, adt.variant_index_with_id(id)).val)
            }
            _ => None,
        };
        let calls = || {
            (body.basic_blocks.iter()).filter_map(|data| match &data.terminator().kind {
                TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } if destination.projection.is_empty() && locals.writes[destination.local] == 1 => {
                    let (def_id, generic_args) = fn_def(func.ty(&body.local_decls, tcx))?;
                    Some((def_id, generic_args, args.first()?, destination.local))
                }
                _ => None,
            })
        };
        let downcast = Std::Any(crate::lower::std_types::any::AnyOp::DowncastRef);
        let mut thrown: HashMap<Local, u128> = calls()
            .filter_map(|(def_id, generic_args, receiver, local)| {
                let any = receiver.node.ty(&body.local_decls, tcx).peel_refs();
                let found = self.recognition().classify(def_id, generic_args) == Some(downcast)
                    && self.finds_thrown(any, generic_args.types().last()?);
                let ty = body.local_decls[local].ty;
                found
                    .then(|| Some((local, discriminant(ty, LangItem::OptionSome)?)))
                    .flatten()
            })
            .collect();
        let tried: Vec<(Local, u128)> = calls()
            .filter_map(|(def_id, _, tried, local)| {
                let place = tried.node.place()?;
                let of_thrown = place.projection.is_empty() && thrown.contains_key(&place.local);
                let ty = body.local_decls[local].ty;
                (of_thrown && tcx.is_lang_item(def_id, LangItem::TryTraitBranch))
                    .then(|| Some((local, discriminant(ty, LangItem::ControlFlowContinue)?)))
                    .flatten()
            })
            .collect();
        thrown.extend(tried);
        thrown
    }

    /// Of each guard that holds (`OptionWrites::held`): what it binds,
    /// `bindings`, read as the value it's the `Some` of, `value`, not a
    /// variable of its own. Its arm's alternatives go on to the arm, past
    /// the guard's test, as an arm's with no guard do, one case, `Object |
    /// Array`'s: `LocalFacts`'s `held_arms`, and its graph.
    pub(super) fn mir_held_guards(&self, state: &mut State<'_, 'tcx>, bindings: &[(Local, Local)]) {
        for &(bound, of) in bindings {
            let read = Expr::var(&state.locals.names[of]);
            state.in_place.insert(bound, read);
            state.locals.names[bound] = String::new();
            state.locals.declared[bound] = true;
        }
    }

    /// Each variable a guard that holds binds (`OptionWrites::held`), `v` of
    /// `if let Some(v) = value`, and the local it's the `Some` of: `value`,
    /// which it is wherever it's read, as a `Some(x)` of a reference is `x`
    /// (ADR 0030) and `value` never changes.
    pub(super) fn mir_held_bindings(
        &self,
        body: &mir::Body<'tcx>,
        held: &std::collections::HashSet<Span>,
    ) -> Vec<(Local, Local)> {
        let user: std::collections::HashSet<Local> = (body.var_debug_info.iter())
            .filter_map(|info| match info.value {
                mir::VarDebugInfoContents::Place(place) if place.projection.is_empty() => Some(place.local),
                _ => None,
            })
            .collect();
        (body.basic_blocks.iter())
            .flat_map(|data| &data.statements)
            .filter(|statement| held.iter().any(|guard| guard.contains(statement.source_info.span)))
            .filter_map(|statement| {
                let mir::StatementKind::Assign(assign) = &statement.kind else {
                    return None;
                };
                let (bound, Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _)) = &**assign else {
                    return None;
                };
                let some = matches!(from.projection.as_slice(), [mir::ProjectionElem::Downcast(_, v), mir::ProjectionElem::Field(f, _)]
                    if v.as_u32() == 1 && f.as_u32() == 0);
                (bound.projection.is_empty() && user.contains(&bound.local) && some).then_some((bound.local, from.local))
            })
            .collect()
    }

    /// Each block on the way to an arm past a guard that holds
    /// (`OptionWrites::held`), and the arm's first block: of what does
    /// nothing in JS, a fake borrow, the guard's test of a `Some` and what
    /// it binds (`bound`, read as the value it's of), and false edges.
    pub(super) fn mir_held_arms(
        &self,
        body: &mir::Body<'tcx>,
        held: &std::collections::HashSet<Span>,
        bound: &std::collections::HashSet<Local>,
    ) -> HashMap<BasicBlock, BasicBlock> {
        let in_held = |span: Span| held.iter().any(|guard| guard.contains(span));
        let nothing = |statement: &mir::Statement<'tcx>| match &statement.kind {
            StatementKind::StorageLive(_)
            | StatementKind::StorageDead(_)
            | StatementKind::Nop
            | StatementKind::PlaceMention(_)
            | StatementKind::FakeRead(_)
            | StatementKind::AscribeUserType(..) => true,
            StatementKind::Assign(assign) => match &assign.1 {
                Rvalue::Ref(_, BorrowKind::Fake(_), _) => true,
                Rvalue::Discriminant(_) => in_held(statement.source_info.span),
                _ => assign.0.projection.is_empty() && bound.contains(&assign.0.local),
            },
            _ => false,
        };
        let mut arms = HashMap::new();
        if held.is_empty() {
            return arms;
        }
        for start in body.basic_blocks.indices() {
            let (mut at, mut past_guard) = (start, false);
            let mut seen = std::collections::HashSet::new();
            while seen.insert(at) {
                let data = &body.basic_blocks[at];
                if !data.statements.iter().all(nothing) {
                    break;
                }
                at = match &data.terminator().kind {
                    TerminatorKind::Goto { target } | TerminatorKind::FalseEdge { real_target: target, .. } => *target,
                    // The guard's test, of a discriminant read here: its `Some`.
                    TerminatorKind::SwitchInt {
                        discr: Operand::Copy(d) | Operand::Move(d),
                        targets,
                    } if (data.statements.iter()).any(|s| {
                        matches!(&s.kind, StatementKind::Assign(a) if a.0 == *d && matches!(a.1, Rvalue::Discriminant(_)))
                            && in_held(s.source_info.span)
                    }) =>
                    {
                        past_guard = true;
                        targets.target_for_value(1)
                    }
                    _ => break,
                };
            }
            if past_guard {
                arms.insert(start, at);
            }
        }
        arms
    }

    /// The locals `OptionWrites::present` says: each a call
    /// `body_queries::present_calls` says gives what's there.
    fn mir_present(&self, body: &mir::Body<'tcx>) -> std::collections::HashSet<Local> {
        let calls = &self.frame.facts.present_calls;
        (body.basic_blocks.iter())
            .filter_map(|data| match &data.terminator().kind {
                TerminatorKind::Call {
                    destination, fn_span, ..
                } if destination.projection.is_empty() && calls.contains(fn_span) => Some(destination.local),
                _ => None,
            })
            .collect()
    }

    /// The locals `OptionWrites::registered` says: each a call of
    /// `js::SymbolFor::from_value` gives, whose value the body only tests
    /// against the registry's symbols (`registered_reads`, ADR 0634).
    fn mir_registered(&self, body: &mir::Body<'tcx>) -> std::collections::HashSet<Local> {
        let reads = &self.frame.facts.registered_reads;
        (body.basic_blocks.iter())
            .filter_map(|data| match &data.terminator().kind {
                TerminatorKind::Call {
                    func,
                    destination,
                    fn_span,
                    ..
                } if destination.projection.is_empty()
                    && reads.contains(fn_span)
                    && fn_def(func.ty(&body.local_decls, self.tcx))
                        .is_some_and(|(def_id, _)| crate::lower::recognition::is_symbol_for_read(self.tcx, def_id)) =>
                {
                    Some(destination.local)
                }
                _ => None,
            })
            .collect()
    }

    /// The locals `OptionWrites::made_some` says: each write a `Some(..)`, or a
    /// shared reference to one of those, written once; a promoted
    /// `&Some(C)` is one too (ADR 0771).
    fn mir_made_some(
        &self,
        body: &mir::Body<'tcx>,
        promoted: &IndexVec<Promoted, mir::Body<'tcx>>,
        locals: &Locals,
    ) -> std::collections::HashSet<Local> {
        let mut somes: std::collections::HashMap<Local, usize> = Default::default();
        let mut refs = Vec::new();
        for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
            let mir::StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            if !assign.0.projection.is_empty() {
                continue;
            }
            match &assign.1 {
                Rvalue::Aggregate(kind, _)
                    if let AggregateKind::Adt(did, variant, ..) = **kind
                        && self.tcx.is_lang_item(did, LangItem::Option)
                        && variant.as_u32() == 1 =>
                {
                    *somes.entry(assign.0.local).or_default() += 1;
                }
                // Or a reborrow of a reference to one, `&*r`.
                Rvalue::Ref(_, BorrowKind::Shared, of)
                    if of.projection.is_empty() || of.projection[..] == [mir::ProjectionElem::Deref] =>
                {
                    refs.push((assign.0.local, of.local));
                }
                Rvalue::Use(Operand::Constant(c), ..)
                    if let mir::Const::Unevaluated(uv, _) = c.const_
                        && let Some(at) = uv.promoted
                        && super::queries::promoted_some(self.tcx, &promoted[at]).is_some() =>
                {
                    *somes.entry(assign.0.local).or_default() += 1;
                }
                _ => {}
            }
        }
        let mut made: std::collections::HashSet<Local> = (somes.into_iter())
            .filter(|&(local, writes)| writes == locals.writes[local])
            .map(|(local, _)| local)
            .collect();
        for (reference, of) in refs {
            if made.contains(&of) && locals.writes[reference] == 1 {
                made.insert(reference);
            }
        }
        made
    }

    /// The locals a `let`'s struct pattern binds its fields to, through a
    /// reference too, `let S { a } = use_context(..)`'s; each whether it's
    /// the rest, a `Rest` or a flattened struct's (ADR 0195, 0204).
    pub(super) fn let_pattern_bindings(&self, body: &mir::Body<'tcx>) -> Vec<(Local, bool)> {
        let mut spans = Vec::new();
        for stmt in self.frame.thir.stmts.iter() {
            let thir::StmtKind::Let { pattern, .. } = &stmt.kind else {
                continue;
            };
            let mut pattern = &**pattern;
            while let thir::PatKind::Deref { subpattern, .. } = &pattern.kind {
                pattern = subpattern;
            }
            if let thir::PatKind::Leaf { subpatterns } = &pattern.kind {
                for field in subpatterns {
                    if let thir::PatKind::Binding { subpattern: None, .. } = field.pattern.kind {
                        let rest = bindings::is_rest_field(self.tcx, pattern.ty, field.field.as_usize());
                        spans.push((field.pattern.span, rest));
                    }
                }
            }
        }
        (body.var_debug_info.iter())
            .filter_map(|info| match info.value {
                mir::VarDebugInfoContents::Place(place) if place.projection.is_empty() => (spans.iter())
                    .find(|(span, _)| *span == info.source_info.span)
                    .map(|&(_, rest)| (place.local, rest)),
                _ => None,
            })
            .collect()
    }

    /// What the body's `let`s bind, by the names `state` gives them: a
    /// struct or tuple pattern's bindings (`let_pattern_bindings`); a name
    /// alone, `let init = lazy.init;`, a statement of its own, which no
    /// pattern of a part read beside it is (ADR 0598, amended); and of
    /// those, a variable given another type, `let lazy: &LazyComponent =
    /// unsafe { js::cast(children) };`, a binding that's its argument
    /// (`JsForm::This`), as TypeScript's `as` gives one, which JS has as it
    /// is; and one an `if let` takes apart, its scrutinee the variable
    /// alone, `let types = t.types.borrow().clone(); if let Some(types) =
    /// types` (ADR 0866).
    pub(super) fn let_names(&self, body: &mir::Body<'tcx>, state: &State<'_, 'tcx>) -> super::tidy::LetNames {
        let thir = self.frame.thir;
        // Through an `unsafe { .. }` of the value alone, and what `strip` is through.
        let through = |mut e: thir::ExprId| loop {
            let inner = match &thir[e].kind {
                thir::ExprKind::Block { block } if thir[*block].stmts.is_empty() => thir[*block].expr,
                _ => Some(crate::lower::body_queries::strip(thir, e)),
            };
            match inner {
                Some(inner) if inner != e => e = inner,
                _ => return e,
            }
        };
        let cast_of_variable = |e: thir::ExprId| {
            let thir::ExprKind::Call { fun, ref args, .. } = thir[through(e)].kind else {
                return false;
            };
            let ty::FnDef(def_id, _) = *thir[fun].ty.kind() else {
                return false;
            };
            bindings::is_binding(self.tcx, def_id)
                && matches!(bindings::js_form(self.tcx, def_id), bindings::JsForm::This)
                && matches!(&args[..], [arg] if matches!(thir[through(*arg)].kind, thir::ExprKind::VarRef { .. }))
        };
        let mut named = Vec::new();
        let mut casts = Vec::new();
        let mut lets = Vec::new();
        for stmt in thir.stmts.iter() {
            if let thir::StmtKind::Let {
                pattern, initializer, ..
            } = &stmt.kind
                && let thir::PatKind::Binding {
                    subpattern: None, var, ..
                } = pattern.kind
            {
                named.push(pattern.span);
                lets.push((var, pattern.span));
                if initializer.is_some_and(cast_of_variable) {
                    casts.push(pattern.span);
                }
            }
        }
        // Of those, each an `if let`'s scrutinee, the variable alone.
        let scrutinees: Vec<thir::LocalVarId> = (thir.exprs.iter())
            .filter_map(|e| match e.kind {
                thir::ExprKind::Let { expr, .. } => match thir[crate::lower::body_queries::strip(thir, expr)].kind {
                    thir::ExprKind::VarRef { id } => Some(id),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        let taken_apart: Vec<Span> = (lets.iter())
            .filter(|(var, _)| scrutinees.contains(var))
            .map(|&(_, span)| span)
            .collect();
        let names = |spans: &[Span]| {
            (body.var_debug_info.iter())
                .filter_map(|info| match info.value {
                    mir::VarDebugInfoContents::Place(place)
                        if place.projection.is_empty() && spans.contains(&info.source_info.span) =>
                    {
                        Some(state.locals.names[place.local].clone())
                    }
                    _ => None,
                })
                .collect()
        };
        super::tidy::LetNames {
            patterns: (self.let_pattern_bindings(body).into_iter())
                .map(|(l, _)| state.locals.names[l].clone())
                .collect(),
            named: names(&named),
            casts: names(&casts),
            taken_apart: names(&taken_apart),
        }
    }

    /// The temporaries a `let`'s struct or tuple pattern takes apart: what
    /// its bindings, of its fields, are read from.
    pub(super) fn mir_patterned(&self, body: &mir::Body<'tcx>) -> std::collections::HashSet<Local> {
        let bound: Vec<Local> = self.let_pattern_bindings(body).into_iter().map(|(l, _)| l).collect();
        let mut patterned = std::collections::HashSet::new();
        for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
            // Bound by value or, through a reference, by one: `&(*_2).0`.
            if let StatementKind::Assign(assign) = &statement.kind
                && let (
                    to,
                    Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _)
                    | Rvalue::Ref(_, BorrowKind::Shared, from),
                ) = &**assign
                && to.as_local().is_some_and(|l| bound.contains(&l))
                && let [derefs @ .., PlaceElem::Field(..)] = from.projection.as_slice()
                && derefs.iter().all(|p| matches!(p, PlaceElem::Deref))
                && body.local_kind(from.local) == LocalKind::Temp
            {
                patterned.insert(from.local);
            }
        }
        patterned
    }

    /// Whether struct pattern `pat` binds each field that has a destructor,
    /// by value, whole: what it leaves has none to drop.
    fn binds_dropped_fields(&self, pat: &thir::Pat<'tcx>) -> bool {
        let (thir::PatKind::Leaf { subpatterns }, ty::Adt(adt, args)) = (&pat.kind, pat.ty.kind()) else {
            return false;
        };
        adt.is_struct()
            && (adt.non_enum_variant().fields.iter_enumerated()).all(|(i, field)| {
                !self.has_drops(field.ty(self.tcx, args).skip_normalization())
                    || subpatterns.iter().any(|sub| {
                        sub.field == i
                            && matches!(
                                sub.pattern.kind,
                                thir::PatKind::Binding {
                                    mode: rustc_hir::BindingMode(rustc_hir::ByRef::No, _),
                                    subpattern: None,
                                    ..
                                }
                            )
                    })
            })
    }

    /// Parameters taken apart where they're given: a props struct's, its
    /// fields' defaults and its rest among them (ADRs 0195, 0212). Each its
    /// index, THIR's pattern, which names what it binds, and the local MIR
    /// binds each to, by value.
    pub(super) fn mir_param_patterns(&mut self, body: &mir::Body<'tcx>) -> Vec<ParamPattern> {
        let mut patterns = Vec::new();
        for (i, param) in self.frame.thir.params.iter().enumerate() {
            let Some(pat) = param.pat.as_deref() else { continue };
            if !matches!(pat.kind, thir::PatKind::Leaf { .. }) {
                continue;
            }
            // One with a destructor, only where each part that has one is
            // bound, moved out whole: none is left of it to drop.
            if self.has_drops(pat.ty) && !self.binds_dropped_fields(pat) {
                continue;
            }
            // Each binding, and the local MIR binds it to, by its span.
            let mut bound = Vec::new();
            let mut by_value = true;
            pat.walk_always(|p| match p.kind {
                thir::PatKind::Binding {
                    var,
                    mode: rustc_hir::BindingMode(rustc_hir::ByRef::No, _),
                    subpattern: None,
                    ..
                } => bound.push((var, p.span)),
                thir::PatKind::Binding { .. } => by_value = false,
                _ => {}
            });
            let locals: Option<Vec<Local>> = (bound.iter())
                .map(|&(_, span)| {
                    body.var_debug_info.iter().find_map(|info| match info.value {
                        mir::VarDebugInfoContents::Place(place)
                            if place.projection.is_empty() && info.source_info.span == span =>
                        {
                            Some(place.local)
                        }
                        _ => None,
                    })
                })
                .collect();
            let Some(locals) = locals.filter(|_| by_value) else {
                continue;
            };
            let Some((pattern, _)) = self.js_pattern(pat) else {
                continue;
            };
            let named: Option<Vec<(Local, String)>> = (bound.iter().zip(locals))
                .map(
                    |(&(var, _), local)| match self.locals.vars.get(&var).map(|v| &v.place.kind) {
                        Some(js::ExprKind::Var(name)) => Some((local, name.clone())),
                        _ => None,
                    },
                )
                .collect();
            if let Some(named) = named {
                patterns.push((i, pattern, named));
            }
        }
        patterns
    }
}
