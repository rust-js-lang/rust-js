//! The locals a pattern binds: an enum's payloads, a `let`'s and a
//! parameter's patterns taken apart, and the `Option`s they write.

mod let_names;

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The locals that patterns bind in an enum's variant, by THIR's
    /// patterns: `if let`s', `match`es' and `let`s'.
    pub(super) fn mir_payloads(&self, body: &mir::Body<'tcx>, locals: &Locals) -> std::collections::HashSet<Local> {
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
        // Not one a closure takes by value and something else reads too: each
        // that took it would read the place apart, its own copy where it's
        // made, `const p = x.bound; const p$1 = x.bound;`, and what reads it
        // after the place, where the program wrote one variable, which they
        // close over (rd640). One only a closure reads is that closure's.
        let captured: std::collections::HashSet<Local> = (super::queries::captured_by_closures(body).into_iter())
            .filter(|&local| locals.reads[local] > 1)
            .collect();
        (body.var_debug_info.iter())
            .filter_map(|info| match info.value {
                mir::VarDebugInfoContents::Place(place)
                    if place.projection.is_empty()
                        && spans.contains(&info.source_info.span)
                        && !captured.contains(&place.local) =>
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
    /// A `let` a call writes too, where a nullable parameter of the crate's
    /// own reads it, and each other read is one too, through a shared
    /// `as_ref()` or `as_deref()`, or a test that reads `null` and
    /// `undefined` alike (`alike_reads`): React's `previousMeasurements`
    /// (rd632, the user's decision (B)).
    fn mir_null_nones(&self, body: &mir::Body<'tcx>, locals: &Locals) -> std::collections::HashSet<Local> {
        let mut made: std::collections::HashMap<Local, (usize, bool)> = Default::default();
        let mut given: std::collections::HashMap<Local, usize> = Default::default();
        // What a call makes, and what JS reads as it's stored, a field's or
        // a binding's parameter's: a call's `Option` only the crate's own
        // functions' nullable parameters read is given as it is, their
        // bodies reading `null` and `undefined` alike (ADR 0861).
        let mut called: std::collections::HashSet<Local> = Default::default();
        let mut stored: std::collections::HashSet<Local> = Default::default();
        // What the crate's own function's nullable parameter reads (rd632).
        let mut toward: std::collections::HashSet<Local> = Default::default();
        // Each copy of a variable, `_2 = copy _1`, MIR's of `owner` in `Info
        // { owner }`: the variable's read, given where the copy is.
        let mut copies: Vec<(Local, Local)> = Vec::new();
        // A `#[rust_js::nullable]` function's result, which a variable it
        // returns is given to: `let mut found = None; .. found = Some(n); ..
        // found`'s `None` is `null` where it's made, `let found = null;`,
        // and the variable returned as it is (ADR 0957, amended, the user's
        // decision, 2026-10-27).
        let returns_nullable = bindings::nullable_result(self.tcx, body.source.def_id());
        let statements = (body.basic_blocks.iter_enumerated())
            .flat_map(|(block, data)| data.statements.iter().enumerate().map(move |(i, s)| (block, i, s)));
        for (block, statement_index, statement) in statements {
            let mir::StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            // A copy of a place just tested `is_some()`, `if
            // options.callbacks.is_some() { callbacks = options.callbacks;
            // }`: a write of a `Some` (rd623).
            if let Rvalue::Use(mir::Operand::Copy(read) | mir::Operand::Move(read), _) = &assign.1
                && !read.projection.is_empty()
                && assign.0.projection.is_empty()
                && self.option_of(assign.0.ty(body, self.tcx).ty).is_some()
                && self.tested_some(body, mir::Location { block, statement_index }, *read)
            {
                made.entry(assign.0.local).or_default().0 += 1;
            }
            if let Rvalue::Use(mir::Operand::Copy(read) | mir::Operand::Move(read), _) = &assign.1
                && read.projection.is_empty()
                && assign.0.projection.is_empty()
            {
                copies.push((assign.0.local, read.local));
                if returns_nullable && assign.0.local == mir::RETURN_PLACE {
                    *given.entry(read.local).or_default() += 1;
                }
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
                // But one whose `None` is `null`, `js::null()`'s (rd623).
                let null = (func.const_fn_def())
                    .and_then(|(def_id, _)| crate::lower::calls::none_given_by(self.tcx, def_id))
                    == Some(js::Known::NoneNull);
                if !null {
                    called.insert(destination.local);
                }
            }
            let Some((def_id, _)) = func.const_fn_def() else {
                continue;
            };
            let nullable = bindings::nullable_params(self.tcx, def_id);
            // What a `#[rust_js::nullable]` thread-local is made of, its
            // `init`'s call's value: each `Option` given for a type
            // parameter's, `create_cursor(None)`'s (ADR 0586, amended).
            let made = destination.local == mir::RETURN_PLACE
                && destination.projection.is_empty()
                && bindings::nullable_init(self.tcx, body.source.def_id());
            if nullable.is_empty() && !made {
                continue;
            }
            let inputs = self.tcx.fn_sig(def_id).skip_binder().skip_binder().inputs();
            let binding = bindings::is_binding(self.tcx, def_id);
            for (i, arg) in args.iter().enumerate() {
                let named = !nullable.is_empty() && bindings::takes_nullable(self.tcx, def_id, i);
                let of_a_type_parameter =
                    made && inputs.get(i).is_some_and(|input| matches!(input.kind(), ty::Param(_)));
                if let Some(read) = arg.node.place()
                    && read.projection.is_empty()
                    && (named || of_a_type_parameter)
                {
                    *given.entry(read.local).or_default() += 1;
                    match binding {
                        true => stored.insert(read.local),
                        false => toward.insert(read.local),
                    };
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
        // A `let`'s reads through a shared `as_ref()` or `as_deref()` given
        // on, and its tests that read `null` and `undefined` alike (rd632).
        // Only a `let` a call writes is asked of them, so a `let x;` that
        // only `None`s and `Some`s write is as it was, `undefined`.
        let mut alike: std::collections::HashMap<Local, usize> = Default::default();
        for (local, to_param) in self.alike_reads(body, locals, &given, &toward) {
            *alike.entry(local).or_default() += 1;
            if to_param {
                toward.insert(local);
            }
        }
        // A copy made once and only given to nullable fields or parameters
        // gives what it's a copy of to them: `let owner = None;`'s `None` is `null` where
        // it's made (ADR 0781).
        // A `let`, the program's variable, not MIR's temporary, and what MIR
        // makes it of, each arm's value of `let x = if c { a } else { b };`.
        let mut lets: std::collections::HashSet<Local> = (body.var_debug_info.iter())
            .filter_map(|info| match info.value {
                mir::VarDebugInfoContents::Place(place) if place.projection.is_empty() => Some(place.local),
                _ => None,
            })
            .collect();
        // So does one a test reads too, alike, which the `let` it's a copy of
        // is read alike by (rd632).
        for &(copy, of) in &copies {
            let (to, alike_to) = (
                given.get(&copy).copied().unwrap_or_default(),
                alike.get(&copy).copied().unwrap_or_default(),
            );
            if locals.writes[copy] == 1 && to + alike_to == locals.reads[copy] && to + alike_to > 0 {
                match alike_to {
                    0 => *given.entry(of).or_default() += 1,
                    _ => *alike.entry(of).or_default() += 1,
                }
                if stored.contains(&copy) {
                    stored.insert(of);
                }
                if toward.contains(&copy) {
                    toward.insert(of);
                }
                if lets.contains(&copy) && locals.reads[of] == 1 {
                    lets.insert(of);
                }
            }
        }
        // A `Some(..)` MIR makes in a temporary of its own, read once, then
        // moves in, `found = Some(Rc::clone(&n))`'s: a write of a `Some`.
        let mut moved_in = std::collections::HashSet::new();
        for &(copy, of) in &copies.iter().copied().collect::<std::collections::HashSet<_>>() {
            if made
                .get(&of)
                .is_some_and(|&(writes, none)| !none && writes == locals.writes[of])
                && locals.reads[of] == 1
            {
                made.entry(copy).or_default().0 += 1;
                moved_in.insert((copy, of));
            }
        }
        // A `#[rust_js::nullable]` function's result is read by its return,
        // and a nullable thread-local's of an `Option`, its `init`'s.
        let result = bindings::null_result(self.tcx, body.source.def_id());
        // What a `let` is written, besides what it makes, of its arms' own
        // `let`s or temporaries, each written once by what makes an
        // `Option`, read once, there: `let x = if c { let y = f(); y }
        // else { g() };`'s `y` (rd632), `null` for `None` where it's
        // written as `x` is, so `x`'s copy of it is as it is.
        let arm = |of: Local| {
            locals.writes[of] == 1 && locals.reads[of] == 1 && made.get(&of).is_some_and(|&(writes, _)| writes == 1)
        };
        let mut nulled = std::collections::HashSet::new();
        let candidates: std::collections::HashSet<Local> = (made.keys().copied())
            .chain(copies.iter().filter(|&&(_, of)| arm(of)).map(|&(copy, _)| copy))
            .collect();
        for local in candidates {
            let (writes, none) = made.get(&local).copied().unwrap_or_default();
            // Of a function's result, each write: what isn't an `Option` it
            // makes is `nullable_written`'s.
            if local == mir::RETURN_PLACE && result {
                nulled.insert(local);
                continue;
            }
            let given = given.get(&local).copied().unwrap_or_default();
            let arms: std::collections::BTreeSet<Local> = (copies.iter())
                .filter(|&&(copy, of)| copy == local && arm(of))
                .map(|&(_, of)| of)
                .collect();
            // Each arm's write once: a `Some` one moves in is among `writes`
            // already (rd647).
            let moved = arms.iter().filter(|&&of| moved_in.contains(&(local, of))).count();
            let called = called.contains(&local) || arms.iter().any(|of| called.contains(of));
            if !(called && !stored.contains(&local)) {
                if made.contains_key(&local) && none && writes == locals.writes[local] && given == locals.reads[local] {
                    nulled.insert(local);
                }
                continue;
            }
            // A `let` a call writes, read by the crate's own nullable
            // parameter and alike otherwise (rd632, the user's (B)).
            let alike = alike.get(&local).copied().unwrap_or_default();
            if toward.contains(&local)
                && lets.contains(&local)
                && given + alike == locals.reads[local]
                && writes + arms.len() - moved == locals.writes[local]
            {
                nulled.insert(local);
                nulled.extend(arms);
            }
        }
        nulled
    }

    /// Whether `place`, an `Option` read at `at`, is a `Some` there: read
    /// first where `place.is_some()` is true, its block the test's alone
    /// goes on to, and `place` a field, or a field's, of what a shared
    /// reference refers to, which nothing writes between (rd623).
    fn tested_some(&self, body: &mir::Body<'tcx>, at: mir::Location, place: Place<'tcx>) -> bool {
        let shared = matches!(
            body.local_decls[place.local].ty.kind(),
            ty::Ref(_, _, ty::Mutability::Not)
        );
        let fields = (place.projection.iter().skip(1)).all(|elem| matches!(elem, mir::ProjectionElem::Field(..)));
        if !shared || place.projection.first() != Some(&mir::ProjectionElem::Deref) || !fields {
            return false;
        }
        let written = |statement: &mir::Statement<'tcx>| matches!(&statement.kind, StatementKind::Assign(assign) if assign.0.local == place.local);
        if body.basic_blocks[at.block].statements[..at.statement_index]
            .iter()
            .any(written)
        {
            return false;
        }
        let predecessors = body.basic_blocks.predecessors();
        let [switch] = predecessors[at.block][..] else {
            return false;
        };
        let data = &body.basic_blocks[switch];
        let TerminatorKind::SwitchInt {
            discr: Operand::Copy(test) | Operand::Move(test),
            targets,
        } = &data.terminator().kind
        else {
            return false;
        };
        if targets.target_for_value(0) == at.block || !test.projection.is_empty() || data.statements.iter().any(written)
        {
            return false;
        }
        // The test: `is_some()` of a shared reference made once to `place`,
        // whose call goes on to the switch.
        let [call] = predecessors[switch][..] else {
            return false;
        };
        let TerminatorKind::Call {
            func,
            args,
            destination,
            target: Some(next),
            ..
        } = &body.basic_blocks[call].terminator().kind
        else {
            return false;
        };
        let reference = match &args[..] {
            [arg] => arg.node.place().and_then(|p| p.as_local()),
            _ => None,
        };
        *next == switch
            && destination.as_local() == Some(test.local)
            && func.const_fn_def().is_some_and(|(def_id, generic_args)| {
                self.recognition().classify(def_id, generic_args) == Some(Std::IsSome)
            })
            && reference.is_some_and(|reference| {
                (body.basic_blocks.iter().flat_map(|b| &b.statements)).any(|s| {
                    matches!(&s.kind, StatementKind::Assign(assign)
                        if assign.0.as_local() == Some(reference)
                            && matches!(&assign.1, Rvalue::Ref(_, BorrowKind::Shared, of) if *of == place))
                }) && (body.basic_blocks.iter().flat_map(|b| &b.statements))
                    .filter(|s| matches!(&s.kind, StatementKind::Assign(assign) if assign.0.local == reference))
                    .count()
                    == 1
            })
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

    /// The locals `OptionWrites::nones` says: an `Option`, or a shared
    /// reference to one, each of whose writes is a `Some(..)` or gives one
    /// `None` alone, the same: the crate's `None`, `undefined`; a call that
    /// gives one (`calls::none_given_by`), `window.event`'s `undefined` or
    /// a `#[rust_js::nullable]` function's `null`; a copy of another such,
    /// or a borrow of one. Never a parameter, which JS may give either, but
    /// a function's own nullable one each of whose calls gives it `null` or
    /// a value, which the body sets `null` for `None` (ADR 0861, amended,
    /// `analysis/nulled_params.rs`); nor one written any other way, a
    /// field's read, a cast's, a `take()`'s, through a `&mut` or a pointer,
    /// or in part (ADR 0298, amended).
    fn mir_nones(&self, body: &mir::Body<'tcx>, locals: &Locals) -> HashMap<Local, js::Known> {
        // What a write gives: a `None` of one kind, a `Some` (`None` here),
        // or what another local holds.
        enum Write {
            Made(Option<js::Known>),
            Of(Local),
        }
        let tcx = self.tcx;
        let option = |ty: Ty<'tcx>| matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
        let mut writes: HashMap<Local, Vec<Write>> = HashMap::new();
        let mut unknown: std::collections::HashSet<Local> = Default::default();
        // Each local's whole writes by a statement, which a nullable
        // parameter's store `null` for `None` (`Storage::NullForNone`).
        let mut assigned: HashMap<Local, usize> = HashMap::new();
        for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
            if let mir::StatementKind::Assign(assign) = &statement.kind
                && assign.0.projection.is_empty()
            {
                *assigned.entry(assign.0.local).or_default() += 1;
            }
            let assign = match &statement.kind {
                mir::StatementKind::Assign(assign) => assign,
                mir::StatementKind::SetDiscriminant { place, .. } => {
                    unknown.insert(place.local);
                    continue;
                }
                _ => continue,
            };
            // Borrowed whole to be written, or pointed to: what's written
            // through it, `*slot = given`, is another's write.
            match &assign.1 {
                Rvalue::RawPtr(_, of) => {
                    unknown.insert(of.local);
                }
                Rvalue::Ref(_, BorrowKind::Mut { .. }, of)
                    if !of.is_indirect()
                        && !matches!(of.projection.first(), Some(mir::ProjectionElem::Downcast(..))) =>
                {
                    unknown.insert(of.local);
                }
                _ => {}
            }
            if !assign.0.projection.is_empty() {
                unknown.insert(assign.0.local);
                continue;
            }
            let write = match &assign.1 {
                Rvalue::Aggregate(kind, _)
                    if let AggregateKind::Adt(did, variant, ..) = **kind
                        && tcx.is_lang_item(did, LangItem::Option) =>
                {
                    Write::Made((variant.as_u32() == 0).then_some(js::Known::NoneUndefined))
                }
                Rvalue::Use(Operand::Copy(read) | Operand::Move(read), _) if read.projection.is_empty() => {
                    Write::Of(read.local)
                }
                Rvalue::Use(Operand::Copy(read) | Operand::Move(read), _)
                    if self.reads_null_or_value(body, *read, 0) =>
                {
                    Write::Made(Some(js::Known::NoneNull))
                }
                Rvalue::Ref(_, BorrowKind::Shared, of)
                    if matches!(of.projection.as_slice(), [] | [mir::ProjectionElem::Deref]) =>
                {
                    Write::Of(of.local)
                }
                _ => {
                    unknown.insert(assign.0.local);
                    continue;
                }
            };
            writes.entry(assign.0.local).or_default().push(write);
        }
        for block in body.basic_blocks.iter() {
            match &block.terminator().kind {
                mir::TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } => {
                    let given =
                        (func.const_fn_def()).and_then(|(def_id, _)| crate::lower::calls::none_given_by(tcx, def_id));
                    // `as_ref()` of a field that's `null` or a value (rd647).
                    let given = given.or_else(|| self.as_ref_of_null_or_value(body, func, args));
                    match given {
                        Some(none) if destination.projection.is_empty() => {
                            writes
                                .entry(destination.local)
                                .or_default()
                                .push(Write::Made(Some(none)));
                        }
                        _ => {
                            unknown.insert(destination.local);
                        }
                    }
                }
                mir::TerminatorKind::Yield { resume_arg, .. } => {
                    unknown.insert(resume_arg.local);
                }
                _ => {}
            }
        }
        // Each written only so: no parameter, nor the return, every write
        // of it one of those above, none in part.
        let known = |local: Local| {
            local != mir::RETURN_PLACE
                && local.as_usize() > body.arg_count
                && !unknown.contains(&local)
                && !locals.borrowed[local]
                && option(body.local_decls[local].ty)
                && writes.get(&local).is_some_and(|w| w.len() == locals.writes[local])
        };
        // What each holds, until none changes: `Err` of one that may hold
        // either, `Ok(None)` of one only ever `Some`.
        let mut held: HashMap<Local, Result<Option<js::Known>, ()>> =
            (writes.keys()).filter(|&&l| known(l)).map(|&l| (l, Ok(None))).collect();
        // A function's own nullable parameter each call gives `null` or a
        // value, written only whole by statements, each `null` for `None`,
        // and never borrowed: its `None` is `null`, whatever it's set to.
        let def_id = body.source.def_id();
        let fixed: Vec<Local> = (1..=body.arg_count)
            .map(Local::from_usize)
            .filter(|&param| {
                self.krate.nulled_params.is_nulled(def_id, param.as_usize() - 1)
                    && !locals.borrowed[param]
                    && option(body.local_decls[param].ty)
                    && assigned.get(&param).copied().unwrap_or(0) == locals.writes[param]
            })
            .collect();
        for &param in &fixed {
            held.insert(param, Ok(Some(js::Known::NoneNull)));
        }
        loop {
            let mut changed = false;
            for (&local, each) in &writes {
                if fixed.contains(&local) {
                    continue;
                }
                let Some(&now) = held.get(&local) else { continue };
                let mut next = now;
                for write in each {
                    let given = match *write {
                        Write::Made(none) => Ok(none),
                        Write::Of(of) => held.get(&of).copied().unwrap_or(Err(())),
                    };
                    next = match (next, given) {
                        (Err(()), _) | (_, Err(())) => Err(()),
                        (Ok(None), other) | (other, Ok(None)) => other,
                        (Ok(Some(a)), Ok(Some(b))) if a == b => Ok(Some(a)),
                        _ => Err(()),
                    };
                }
                if next != now {
                    held.insert(local, next);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        (held.into_iter())
            .filter_map(|(local, none)| Some((local, none.ok()??)))
            .collect()
    }

    /// What `body`'s writes say of its `Option`s.
    pub(super) fn mir_option_writes(
        &self,
        body: &mir::Body<'tcx>,
        promoted: &IndexVec<Promoted, mir::Body<'tcx>>,
        locals: &Locals,
        result_held: bool,
    ) -> OptionWrites {
        let (nested, held_results) = self.mir_nested(body, &locals.writes, result_held);
        OptionWrites {
            null_nones: self.mir_null_nones(body, locals),
            nones: self.mir_nones(body, locals),
            made_some: self.mir_made_some(body, promoted, locals),
            nested,
            registered: self.mir_registered(body),
            thrown: self.mir_thrown(body, locals),
            present: self.mir_present(body),
            held: self.frame.facts.held_guards.clone(),
            held_results,
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

    /// Each block whose test is that an `Option` is `Some`, `Some(7) => ..`,
    /// which goes on to a test of its payload alone, a number, a `bool` or a
    /// `char` MIR switches on, and whose `None` goes where that test's
    /// `otherwise` does: that test's block, whose test of the payload is the
    /// `Option`'s own, `sub === 7` or `flag === false`, which no `None`
    /// passes, as no `null` is `7` or `false` (a `bool`'s exactly, ADR 0298,
    /// the user's decision); no `sub != null` before it, and its `otherwise`
    /// reached once.
    pub(super) fn mir_payload_tests(&self, body: &mir::Body<'tcx>) -> HashMap<BasicBlock, BasicBlock> {
        let nothing = |statement: &mir::Statement<'tcx>| {
            matches!(
                statement.kind,
                StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop
            )
        };
        let predecessors = body.basic_blocks.predecessors();
        let mut tests = HashMap::new();
        for (block, data) in body.basic_blocks.iter_enumerated() {
            let TerminatorKind::SwitchInt {
                discr: Operand::Copy(d) | Operand::Move(d),
                targets,
            } = &data.terminator().kind
            else {
                continue;
            };
            let Some(option) = data.statements.iter().find_map(|s| match &s.kind {
                StatementKind::Assign(assign) if assign.0 == *d => match assign.1 {
                    Rvalue::Discriminant(option) => Some(option),
                    _ => None,
                },
                _ => None,
            }) else {
                continue;
            };
            let option_ty = option.ty(&body.local_decls, self.tcx).ty;
            if self.option_of(option_ty).is_none() {
                continue;
            }
            let (some, none) = (targets.target_for_value(1), targets.target_for_value(0));
            let payload = &body.basic_blocks[some];
            let TerminatorKind::SwitchInt {
                discr: Operand::Copy(read) | Operand::Move(read),
                targets: payload_targets,
            } = &payload.terminator().kind
            else {
                continue;
            };
            let of_payload = read.local == option.local
                && read.projection.len() == option.projection.len() + 2
                && read.projection[..option.projection.len()] == option.projection[..]
                && matches!(
                    read.projection[option.projection.len()..],
                    [mir::ProjectionElem::Downcast(_, variant), mir::ProjectionElem::Field(field, _)]
                        if variant.as_u32() == 1 && field.as_u32() == 0
                );
            if of_payload
                && payload.statements.iter().all(nothing)
                && payload_targets.otherwise() == none
                && predecessors[some].len() == 1
            {
                tests.insert(block, some);
            }
        }
        tests
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

    /// `mir`, each `match`'s and `if let`'s subject a getter may answer, a
    /// field of a struct JS may have made (`fields_answered_by_getters`),
    /// `view.margin` of `if let Some(margin) = view.margin`, borrowed once
    /// where it's tested, and what its patterns read of it, its bindings and
    /// its parts' tests, read through that borrow: the field read once, as
    /// Rust reads it, `const margin = view.margin; if (margin != null)`, not
    /// tested by one read and bound by another, each the getter's (#430; ADR
    /// 1022, amended). A borrow of the subject, `&view.name`, is read so
    /// already. `None` where there's none.
    pub(super) fn held_subjects(&self, mir: &Mir<'tcx>) -> Option<Mir<'tcx>> {
        use rustc_middle::mir::visit::MutVisitor;
        let (tcx, body) = (self.tcx, &mir.body);
        if !body.source.def_id().is_local() {
            return None;
        }
        // Each subject's span, and its patterns', of THIR's matches and lets.
        let mut subjects: Vec<(Span, Vec<Span>)> = Vec::new();
        let thir = &self.frame.thir;
        for expr in thir.exprs.iter() {
            match &expr.kind {
                thir::ExprKind::Match { scrutinee, arms, .. } => subjects.push((
                    thir[*scrutinee].span,
                    arms.iter().map(|&arm| thir[arm].pattern.span).collect(),
                )),
                thir::ExprKind::Let { expr, pat } => subjects.push((thir[*expr].span, vec![pat.span])),
                _ => {}
            }
        }
        for stmt in thir.stmts.iter() {
            if let thir::StmtKind::Let {
                pattern,
                initializer: Some(init),
                ..
            } = &stmt.kind
            {
                subjects.push((thir[*init].span, vec![pattern.span]));
            }
        }
        // Whether a read of `place` is a getter's: a field of such a struct.
        let getters = |place: Place<'tcx>| {
            (place.iter_projections()).any(|(base, elem)| {
                let base = base.ty(&body.local_decls, tcx);
                matches!(elem, PlaceElem::Field(..))
                    && base.variant_index.is_none()
                    && self.fields_answered_by_getters(base.ty)
            })
        };
        // Each tested here, its block's last statement read by its switch:
        // the block, the place, its patterns' spans, and the borrow made.
        struct Held<'tcx> {
            block: BasicBlock,
            place: Place<'tcx>,
            subject: Span,
            patterns: Vec<Span>,
            borrow: Local,
        }
        let dominators = body.basic_blocks.dominators();
        let mut held: Vec<Held<'tcx>> = Vec::new();
        let mut decls = body.local_decls.clone();
        for (block, data) in mir::traversal::reverse_postorder(body) {
            let (Some(last), TerminatorKind::SwitchInt { discr, .. }) =
                (data.statements.last(), &data.terminator().kind)
            else {
                continue;
            };
            let StatementKind::Assign(assign) = &last.kind else {
                continue;
            };
            let (tested, Rvalue::Discriminant(place)) = &**assign else {
                continue;
            };
            if discr.place() != Some(*tested) || place.projection.is_empty() || !getters(*place) {
                continue;
            }
            let span = last.source_info.span;
            let Some((subject, patterns)) = subjects.iter().find(|(subject, _)| *subject == span) else {
                continue;
            };
            // A part of one its match holds already, `Some(Some(x))`'s
            // inner, is read through its borrow.
            let within = held.iter().any(|h| {
                h.subject == span
                    && h.place.local == place.local
                    && place.projection.starts_with(h.place.projection)
                    && dominators.dominates(h.block, block)
            });
            if within {
                continue;
            }
            let ty = place.ty(&body.local_decls, tcx).ty;
            let borrow = decls.push(mir::LocalDecl::new(
                ty::Ty::new_imm_ref(tcx, tcx.lifetimes.re_erased, ty),
                span,
            ));
            held.push(Held {
                block,
                place: *place,
                subject: *subject,
                patterns: patterns.clone(),
                borrow,
            });
        }
        if held.is_empty() {
            return None;
        }
        // Reads of a held place, a guard's fake borrow of it, or of its
        // part, `(place as Some).0`, of its own match: where its subject or
        // its patterns are, past the borrow, the nearest one.
        struct Through<'a, 'tcx> {
            tcx: ty::TyCtxt<'tcx>,
            held: &'a [Held<'tcx>],
            dominators: &'a rustc_data_structures::graph::dominators::Dominators<BasicBlock>,
            span: Span,
        }
        impl<'tcx> MutVisitor<'tcx> for Through<'_, 'tcx> {
            fn tcx(&self) -> ty::TyCtxt<'tcx> {
                self.tcx
            }
            fn visit_place(&mut self, place: &mut Place<'tcx>, _: PlaceContext, location: mir::Location) {
                let span = self.span;
                let nearest = (self.held.iter())
                    .filter(|h| {
                        h.place.local == place.local
                            && place.projection.starts_with(h.place.projection)
                            && (place.projection.get(h.place.projection.len()))
                                .is_none_or(|elem| matches!(elem, PlaceElem::Downcast(..)))
                            && (span == h.subject || h.patterns.iter().any(|p| p.contains(span)))
                            && h.block != location.block
                            && self.dominators.dominates(h.block, location.block)
                    })
                    .reduce(|a, b| match self.dominators.dominates(a.block, b.block) {
                        true => b,
                        false => a,
                    });
                if let Some(h) = nearest {
                    let rest = &place.projection[h.place.projection.len()..];
                    let projection: Vec<PlaceElem<'tcx>> =
                        std::iter::once(PlaceElem::Deref).chain(rest.iter().copied()).collect();
                    *place = Place {
                        local: h.borrow,
                        projection: self.tcx.mk_place_elems(&projection),
                    };
                }
            }
        }
        let mut body = body.clone();
        body.local_decls = decls;
        for h in &held {
            let data = &mut body.basic_blocks_mut()[h.block];
            let last = data.statements.last_mut().expect("a discriminant read");
            let source_info = last.source_info;
            let StatementKind::Assign(assign) = &mut last.kind else {
                unreachable!("a discriminant read")
            };
            let tested = assign.0;
            let borrowed = Place::from(h.borrow);
            assign.0 = borrowed;
            assign.1 = Rvalue::Ref(tcx.lifetimes.re_erased, BorrowKind::Shared, h.place);
            let through = Place {
                local: h.borrow,
                projection: tcx.mk_place_elems(&[PlaceElem::Deref]),
            };
            data.statements.push(mir::Statement::new(
                source_info,
                StatementKind::Assign(Box::new((tested, Rvalue::Discriminant(through)))),
            ));
        }
        let dominators = body.basic_blocks.dominators().clone();
        let blocks: Vec<BasicBlock> = body.basic_blocks.indices().collect();
        for block in blocks {
            let data = &mut body.basic_blocks_mut()[block];
            for (i, statement) in data.statements.iter_mut().enumerate() {
                let mut through = Through {
                    tcx,
                    held: &held,
                    dominators: &dominators,
                    span: statement.source_info.span,
                };
                through.visit_statement(
                    statement,
                    mir::Location {
                        block,
                        statement_index: i,
                    },
                );
            }
        }
        Some(Mir {
            body,
            promoted: mir.promoted.clone(),
            awaits: mir.awaits.clone(),
            cases: mir.cases.clone(),
        })
    }
}
