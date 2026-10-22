//! Where each block goes, made structured JS by Ramsey's "Beyond
//! Relooper": its tree, its branches, its statements and its terminator, and
//! the tests a switch and a check make.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `block`, and what it dominates (Ramsey's `doTree`): a loop of it if
    /// an edge goes back to it.
    pub(super) fn mir_tree(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock) -> R<Vec<Stmt>> {
        let merges = state.graph.merge_children(block);
        if state.graph.headers.contains(&block) {
            self.flush(state, &mut Vec::new())?;
            state.labels.push(block);
            let mut body = self.mir_statements(state, block)?;
            body.extend(self.mir_within(state, block, &merges)?);
            state.labels.pop();
            // Where it starts, what the program wrote: a `while`'s test.
            let data = &state.body.basic_blocks[block];
            let start = (data.statements.iter())
                .find(|s| {
                    !matches!(
                        s.kind,
                        StatementKind::StorageLive(_) | StatementKind::StorageDead(_) | StatementKind::Nop
                    )
                })
                .map_or(data.terminator().source_info.span, |s| s.source_info.span);
            return Ok(vec![
                StmtKind::While {
                    label: Some(label(block)),
                    cond: Expr::bool(true),
                    body,
                }
                .at(self.js_span(start.source_callsite())),
            ]);
        }
        let mut out = self.mir_statements(state, block)?;
        out.extend(self.mir_within(state, block, &merges)?);
        Ok(out)
    }

    /// Where `block` goes, inside a labeled block for each of `merges`, each
    /// followed by its own code (Ramsey's `nodeWithin`). Its statements come
    /// before, as none of them branches: what they bind is seen where its
    /// branches meet.
    fn mir_within(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock, merges: &[BasicBlock]) -> R<Vec<Stmt>> {
        let Some((&merge, inner)) = merges.split_first() else {
            return self.mir_terminator(state, block);
        };
        // What's made here and read after the block, where its branches meet,
        // is declared here, not in the block: all but those the branch reads
        // last, which it reads where it's written.
        let read = terminator_reads(state.body.basic_blocks[block].terminator());
        if let Some(last) = state.pending.iter().rposition(|(local, _)| !read.contains(local)) {
            let rest = state.pending.split_off(last + 1);
            let mut out = Vec::new();
            self.flush(state, &mut out)?;
            state.pending = rest;
            out.extend(self.mir_within(state, block, merges)?);
            return Ok(out);
        }
        state.labels.push(merge);
        let mut within = self.mir_within(state, block, inner)?;
        state.labels.pop();
        self.flush(state, &mut within)?;
        let mut out = vec![StmtKind::Labeled(label(merge), within).at(js::Span::NONE)];
        out.extend(self.mir_tree(state, merge)?);
        Ok(out)
    }

    /// Going from `from` to `to`: on, back to a loop's start, or out to a
    /// block that follows.
    fn mir_branch(&mut self, state: &mut State<'_, 'tcx>, from: BasicBlock, to: BasicBlock) -> R<Vec<Stmt>> {
        if state.graph.backward(from, to) {
            let mut out = Vec::new();
            self.flush(state, &mut out)?;
            out.push(StmtKind::Continue(Some(label(to))).at(js::Span::NONE));
            return Ok(out);
        }
        // To the block that returns, and does nothing else: the `return`,
        // where a person writes it; at a `return` the program wrote, where
        // what jumps there is, if it's one.
        if is_return(state.body, to) {
            let mut out = self.mir_terminator(state, to)?;
            // Back through the drops on the way, to where the value is made.
            let mut blocks = vec![from];
            let preds = state.body.basic_blocks.predecessors();
            while let Some(&block) = blocks.last()
                && !(state.body.basic_blocks[block].statements.iter())
                    .any(|s| matches!(&s.kind, StatementKind::Assign(a) if a.0.local == mir::RETURN_PLACE))
                && let [only] = preds[block].as_slice()
                && !blocks.contains(only)
            {
                blocks.push(*only);
            }
            // What each does, not a temporary's storage ending, which a `?`'s
            // has where it's written, on its way on too.
            let spans = (blocks.iter())
                .flat_map(|&b| {
                    let data = &state.body.basic_blocks[b];
                    (data.statements.iter())
                        .filter(|s| !matches!(s.kind, StatementKind::StorageLive(_) | StatementKind::StorageDead(_)))
                        .map(|s| s.source_info.span)
                        .chain([data.terminator().source_info.span])
                })
                .map(|span| self.js_span(span));
            let written = spans
                .filter_map(|s| (state.returns.iter()).find(|w| !s.is_none() && w.lo <= s.lo && s.hi <= w.hi))
                .next()
                .copied();
            if let Some(written) = written
                && let Some(stmt) = out.last_mut().filter(|s| matches!(s.kind, StmtKind::Return(_)))
            {
                stmt.span = written;
            }
            return Ok(out);
        }
        if state.graph.merges.contains(&to) {
            let mut out = Vec::new();
            self.flush(state, &mut out)?;
            out.push(StmtKind::Break(Some(label(to))).at(js::Span::NONE));
            return Ok(out);
        }
        self.mir_tree(state, to)
    }

    /// A block's statements.
    fn mir_statements(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock) -> R<Vec<Stmt>> {
        state.at = block;
        let data = &state.body.basic_blocks[block];
        let mut out = Vec::new();
        for (index, statement) in data.statements.iter().enumerate() {
            let span = statement.source_info.span;
            // A statement that runs a call made before, which may panic
            // as something's there to drop, is that call's `try`.
            let unwind = state.unwind.take();
            let acting = self.acting(state);
            let mut made = Vec::new();
            let at = mir::Location {
                block,
                statement_index: index,
            };
            self.flag_sets(state, at, &mut made)?;
            match &statement.kind {
                StatementKind::Assign(assign) => {
                    let (place, rvalue) = &**assign;
                    self.mir_assign(state, place, rvalue, span, &mut made)?;
                }
                StatementKind::SetDiscriminant { .. } => {
                    return Err(self.unsupported(span, "an enum made by parts, from its MIR"));
                }
                // What borrowck alone reads, and what says nothing that runs.
                StatementKind::FakeRead(_)
                | StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::PlaceMention(_)
                | StatementKind::AscribeUserType(..)
                | StatementKind::Coverage(_)
                | StatementKind::ConstEvalCounter
                | StatementKind::Nop
                | StatementKind::BackwardIncompatibleDropHint { .. } => {}
                StatementKind::Intrinsic(_) => {
                    return Err(self.unsupported(span, "an intrinsic, from its MIR"));
                }
            }
            let ran = acting.iter().any(|l| !state.pending.iter().any(|(p, _)| p == l));
            self.unwinding_to(state, unwind.filter(|_| ran), made, &mut out)?;
            state.unwind = unwind.filter(|_| !self.acting(state).is_empty());
        }
        Ok(out)
    }

    /// What code a program that's right never reaches does, MIR's
    /// unreachable code's or `hint::unreachable_unchecked()`'s: Rust's is
    /// undefined, so JS's, a throw, which says where it went wrong.
    pub(in crate::lower) fn unreachable_code(&self, span: Span) -> Stmt {
        StmtKind::Throw(Expr::new_(
            Expr::var("Error"),
            vec![Expr::str("internal error: entered unreachable code")],
        ))
        .at(self.js_span(span))
    }

    /// Where a block goes.
    fn mir_terminator(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock) -> R<Vec<Stmt>> {
        let mut out = Vec::new();
        let terminator = state.body.basic_blocks[block].terminator();
        // What's made that may panic runs before a branch, in its own `try`.
        if state.unwind.is_some() && !matches!(terminator.kind, TerminatorKind::Call { .. }) {
            self.flush(state, &mut out)?;
        }
        self.flag_sets(state, state.body.terminator_loc(block), &mut out)?;
        let span = terminator.source_info.span;
        match &terminator.kind {
            TerminatorKind::Goto { target } => out.extend(self.mir_branch(state, block, *target)?),
            TerminatorKind::FalseEdge { real_target, .. } | TerminatorKind::FalseUnwind { real_target, .. } => {
                out.extend(self.mir_branch(state, block, *real_target)?)
            }
            TerminatorKind::Return => {
                let ret = mir::RETURN_PLACE;
                let value = match (&state.writer, state.locals.names[ret].is_empty()) {
                    // A closure's that writes its maker's string, nothing.
                    (Some(_), _) if state.writes_captured => None,
                    // A writer's, the string it wrote.
                    (Some((_, name)), _) => Some(Expr::var(name)),
                    (None, true) => None,
                    (None, false) => Some(self.read_local(state, ret, &mut out)?),
                };
                // A `#[rust_js::nullable]` function's: a `None` it didn't make
                // `null` already is `null` too, `heap[0] ?? null`.
                let value = match value {
                    Some(value)
                        if bindings::nullable_result(self.tcx, state.body.source.def_id())
                            && !state.options.null_nones.contains(&ret) =>
                    {
                        Some(match value.kind {
                            js::ExprKind::Undefined => Expr::null(),
                            _ => Expr::bin(Op::Coalesce, value, Expr::null()),
                        })
                    }
                    value => value,
                };
                self.flush(state, &mut out)?;
                out.push(StmtKind::Return(value).at(self.js_span(span)));
            }
            TerminatorKind::Unreachable => {
                self.flush(state, &mut out)?;
                out.push(self.unreachable_code(span));
            }
            TerminatorKind::SwitchInt { discr, targets } => {
                let mut subject = self.mir_operand(state, discr, &mut out)?;
                self.flush(state, &mut out)?;
                let ty = discr.ty(&state.body.local_decls, self.tcx);
                // The cases, those that can't be reached left out.
                let reached = |b: BasicBlock| !is_unreachable(state.body, b);
                // What more than one case tests, and does something as it's
                // made, is made once, as Rust makes it: a call's value,
                // `match note(1, x) { 1 => .., 2 => .., _ => .. }`, held
                // first, not called by each case's test (a generated
                // program's, seed 50147). One that only reads, `i % 3`, is
                // read by each.
                if targets.iter().filter(|&(_, target)| reached(target)).count() > 1
                    && let Value::Expr(made) | Value::Discriminant(made, _) = &mut subject
                    && made.has_effects()
                {
                    let value = std::mem::replace(made, Expr::undefined());
                    *made = self.spill("_$", value, &mut out);
                }
                let mut cases: Vec<(Expr, BasicBlock)> = Vec::new();
                let mut always = None;
                for (value, target) in targets.iter() {
                    if reached(target) {
                        let test = self.switch_test(&subject, ty, value, span)?;
                        // A test that's a constant: its branch alone, or none.
                        match test.kind {
                            js::ExprKind::Bool(false) => {}
                            js::ExprKind::Bool(true) => always = always.or(Some(target)),
                            _ => cases.push((test, target)),
                        }
                    }
                }
                if let Some(target) = always {
                    out.extend(self.mir_branch(state, block, target)?);
                    return Ok(out);
                }
                let mut otherwise = Some(targets.otherwise()).filter(|&b| reached(b));
                // Values that go to one block, an or-pattern's `'+' | '-'`:
                // one test of either, the block once, as the program wrote it.
                let mut grouped: Vec<(Expr, BasicBlock)> = Vec::new();
                for (test, target) in cases {
                    match grouped.iter_mut().find(|(_, t)| *t == target) {
                        Some((either, _)) => {
                            *either = Expr::bin(Op::Or, std::mem::replace(either, Expr::undefined()), test);
                        }
                        None => grouped.push((test, target)),
                    }
                }
                let mut cases = grouped;
                // Each case tested, an enum's `match`'s: its arms in the order
                // the program wrote them, not its variants'.
                if otherwise.is_none()
                    && let Some(at) = (cases.iter())
                        .map(|&(_, b)| written_at(state.body, b))
                        .collect::<Option<Vec<_>>>()
                {
                    let mut ordered: Vec<_> = at.into_iter().zip(std::mem::take(&mut cases)).collect();
                    ordered.sort_by_key(|&(at, _)| at);
                    cases = ordered.into_iter().map(|(_, case)| case).collect();
                }
                // `if c { a } else { b }`, MIR's test of `false`, `!c`, first:
                // the program's, `c`, its branch `a` first. Not `if !c { a }
                // else { b }`'s, which MIR tests as `c`, `a` its `false`.
                if ty.is_bool()
                    && targets.iter().all(|(value, _)| value == 0)
                    && let [(test, target)] = cases.as_mut_slice()
                    && let Some(yes) = otherwise
                    && !matches!((written_at(state.body, *target), written_at(state.body, yes)), (Some(no), Some(yes)) if no < yes)
                {
                    *test = std::mem::replace(test, Expr::undefined()).negated_test();
                    otherwise = Some(std::mem::replace(target, yes));
                }
                let mut chain_panics = otherwise.is_some_and(|b| panics(state.body, b));
                let mut chain: Option<Vec<Stmt>> = match otherwise {
                    Some(b) => Some(self.mir_branch(state, block, b)?),
                    None => None,
                };
                // Without an `otherwise`, the last case is what's left.
                if chain.is_none()
                    && let Some((_, last)) = cases.pop()
                {
                    chain_panics = panics(state.body, last);
                    chain = Some(self.mir_branch(state, block, last)?);
                }
                for (test, target) in cases.into_iter().rev() {
                    let then = self.mir_branch(state, block, target)?;
                    let branches_panic = [panics(state.body, target), chain_panics];
                    // Its test where its arm's pattern is, a `match`'s, or its
                    // `if`, which its `if` takes once it's tidied (`tidy`): till
                    // then, as tidying reads it, where it's tested.
                    let arm = written_at(state.body, target).and_then(|at| self.arm_span(span, at));
                    let test = Expr {
                        span: self.js_span(arm.unwrap_or_else(|| self.if_span(span))),
                        ..test
                    };
                    chain = Some(early_exit(test, then, chain, branches_panic, self.js_span(span)));
                    chain_panics = false;
                }
                out.extend(chain.unwrap_or_default());
            }
            TerminatorKind::Assert { target, .. }
                if let Some(check) = self.assert_check(state.body, state.drops.as_ref(), terminator) =>
            {
                if let Check::Deferred(index, write) = check {
                    state.deferred.insert(index, write);
                }
                out.extend(self.mir_branch(state, block, *target)?);
            }
            TerminatorKind::Assert {
                cond,
                expected,
                msg,
                target,
                unwind,
            } => {
                let cond = self.mir_expr(state, cond, &mut out)?;
                let message = self.assert_message(state, msg, span, &mut out)?;
                self.flush(state, &mut out)?;
                let failed = match expected {
                    true => Expr::unary(js::UnaryOp::Not, cond),
                    false => cond,
                };
                let js_span = self.js_span(span);
                // The panic drops what's there as it unwinds, then is thrown.
                let mut panic = self.mir_unwind(state, *unwind)?;
                panic.push(StmtKind::Throw(Expr::new_(Expr::var("Error"), vec![message])).at(js_span));
                out.push(StmtKind::If(failed, panic, None).at(js_span));
                out.extend(self.mir_branch(state, block, *target)?);
            }
            TerminatorKind::Drop {
                place, target, unwind, ..
            } => {
                // What's dropped runs after what's made before it.
                let mut dropped = Vec::new();
                self.mir_drop(state, block, *place, false, span, &mut dropped)?;
                if !dropped.is_empty() {
                    self.flush(state, &mut out)?;
                    self.settle_reads(state, block, &mut out);
                    let cleanup = self.mir_unwind(state, *unwind)?;
                    match cleanup.is_empty() {
                        true => out.extend(dropped),
                        false => self.unwinding(dropped, cleanup, self.js_span(span), &mut out),
                    }
                }
                out.extend(self.mir_branch(state, block, *target)?);
            }
            TerminatorKind::Call {
                func,
                args,
                destination,
                target,
                fn_span,
                unwind,
                ..
            } => {
                // A call that may panic as something's there to drop runs in
                // a `try` whose `catch` drops it: the statement that runs it,
                // as it may be written where it's used. What's made before
                // that unwinds elsewhere runs first.
                let unwind = drops::cleans_up(state.body, state.drops.as_ref(), *unwind);
                if state.unwind != unwind && !self.acting(state).is_empty() {
                    self.flush(state, &mut out)?;
                }
                state.unwind = None;
                if let Some(future) = fn_def(func.ty(&state.body.local_decls, self.tcx))
                    .and_then(|call| self.recognition().awaited_unlowered_future(call))
                {
                    return Err(self.unlowered_future_awaited(future, *fn_span));
                }
                if let Some(given) = fn_def(func.ty(&state.body.local_decls, self.tcx))
                    .and_then(|call| self.recognition().own_into_future_given(call))
                {
                    let what = format!("`{given}` where any `IntoFuture` goes");
                    let why = "its `IntoFuture` is the crate's own, which generic code would await as a promise";
                    return Err(refuse(self.tcx, *fn_span, Refused::Because(&what, why)));
                }
                let mut called = Vec::new();
                let value = match state.awaits.contains(&block) {
                    true => self.mir_await(state, func, args, *fn_span, &mut called),
                    false => self.mir_call(state, func, args, *destination, *fn_span, &mut called),
                };
                let value = value?;
                // What the call writes, a setter's `n.textContent = "x"` or a
                // cell's `count = 5`, after what was made before it, which
                // may read what it writes.
                if called.iter().any(|s| matches!(s.kind, StmtKind::Assign(..))) && !state.pending.is_empty() {
                    let mut before = Vec::new();
                    self.flush(state, &mut before)?;
                    before.append(&mut called);
                    called = before;
                }
                match target {
                    Some(target) => {
                        self.mir_store(state, *destination, value, span, &mut called)?;
                        // Each box given, copied back once the call returns.
                        if !state.copy_backs.is_empty() {
                            self.flush(state, &mut called)?;
                            for (place, boxed, object) in std::mem::take(&mut state.copy_backs) {
                                let js_span = self.js_span(span);
                                called.push(self.copied_back(place, &boxed, object, js_span));
                            }
                        }
                        self.unwinding_to(state, unwind, called, &mut out)?;
                        state.unwind = unwind.filter(|_| !self.acting(state).is_empty());
                        out.extend(self.mir_branch(state, block, *target)?);
                    }
                    // A call that never returns, a panic's: what it does is all.
                    None => {
                        let value = self.value_expr(value, span)?;
                        self.flush(state, &mut called)?;
                        if !matches!(value.kind, js::ExprKind::Undefined) {
                            called.push(StmtKind::Expr(value).at(self.js_span(span)));
                        }
                        self.unwinding_to(state, unwind, called, &mut out)?;
                    }
                }
            }
            TerminatorKind::UnwindResume
            | TerminatorKind::UnwindTerminate(_)
            | TerminatorKind::CoroutineDrop
            | TerminatorKind::Yield { .. }
            | TerminatorKind::InlineAsm { .. }
            | TerminatorKind::TailCall { .. } => {
                return Err(self.unsupported(span, "this terminator, from its MIR"));
            }
        }
        Ok(out)
    }

    // ── Folding ─────────────────────────────────────────────────────────

    /// The test that `subject`, of type `ty`, is `value`: of an enum's
    /// discriminant, its variant.
    pub(super) fn switch_test(&mut self, subject: &Value<'tcx>, ty: Ty<'tcx>, value: u128, span: Span) -> R<Expr> {
        match subject {
            Value::Branch(tried, tried_ty) => {
                let failed = self.branch_failed(tried.clone(), *tried_ty);
                Ok(match value {
                    0 => failed.negated_test(),
                    _ => failed,
                })
            }
            Value::Discriminant(enum_value, enum_ty) => {
                let ty::Adt(adt, _) = enum_ty.kind() else {
                    return Err(self.unsupported(span, "a discriminant of this type, from its MIR"));
                };
                let variant = adt
                    .discriminants(self.tcx)
                    .find(|(_, d)| d.val == value)
                    .map(|(v, _)| v)
                    .ok_or_else(|| self.unsupported(span, "this discriminant, from its MIR"))?;
                // An untagged enum's variants JS can't tell apart: refused (ADR 0214).
                if bindings::is_untagged(self.tcx, adt.did())
                    && let Some(other) = self.untagged_alike(*enum_ty, adt.variant(variant))
                {
                    let message = format!(
                        "`{}` and `{other}` hold values of one kind, which JS can't tell apart in an untagged enum (ADR 0214)",
                        adt.variant(variant).name
                    );
                    return Err(self.tcx.dcx().span_err(span, message));
                }
                Ok(self.mir_variant_test(enum_value.clone(), *enum_ty, *adt, variant))
            }
            _ => {
                let subject = self.value_expr(subject.clone(), span)?;
                if ty.is_bool() {
                    return Ok(match value {
                        0 => Expr::unary(js::UnaryOp::Not, subject),
                        _ => subject,
                    });
                }
                if ty.is_char() {
                    let c = char::from_u32(value as u32).ok_or_else(|| self.unsupported(span, "this `char`"))?;
                    return Ok(Expr::bin(Op::Eq, subject, Expr::str(c.to_string())));
                }
                let num = self.num(ty, span)?;
                let literal = match num.signed() {
                    true => {
                        let bits = num.bits();
                        let v = ((value << (128 - bits)) as i128) >> (128 - bits);
                        num.literal(v)
                    }
                    false => num.literal(value as i128),
                };
                Ok(Expr::bin(Op::Eq, subject, literal))
            }
        }
    }

    /// Whether what `?` tried returns early; a write's never does (ADRs
    /// 0054, 0132).
    fn branch_failed(&self, tried: Expr, ty: Ty<'tcx>) -> Expr {
        match self.is_write_result(ty) {
            true => Expr::bool(false),
            false => self.tried_failed(tried, ty),
        }
    }

    /// That `subject` is `variant` of its enum, as a pattern tests it
    /// (`variant_test`): a test function an untagged one's calls,
    /// `isValidElement(child)`, gives a `bool`, which tidy writes a test of
    /// as one.
    pub(in crate::lower) fn mir_variant_test(
        &mut self,
        subject: Expr,
        ty: Ty<'tcx>,
        adt: ty::AdtDef<'tcx>,
        variant: VariantIdx,
    ) -> Expr {
        let test = self.variant_test(subject, ty, adt, variant);
        if bindings::is_untagged(self.tcx, adt.did()) {
            test.clone().each_mut(&mut |e| {
                if let Some(path) = tidy::call_path(e) {
                    self.locals.mir.paths.entry(path).or_insert(true);
                }
            });
        }
        test
    }

    /// What a failed check says, as Rust's panic does.
    fn assert_message(
        &mut self,
        state: &mut State<'_, 'tcx>,
        msg: &AssertKind<Operand<'tcx>>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        Ok(match msg {
            AssertKind::BoundsCheck { len, index } => {
                let [len, index]: [Value<'tcx>; 2] = self
                    .take_operands(state, &[len, index], out)?
                    .try_into()
                    .ok()
                    .expect("two");
                let len = self.value_expr(len, span)?;
                let index = self.value_expr(index, span)?;
                let usize = self.tcx.types.usize;
                let len = self.display_string(len, usize, span)?;
                let index = self.display_string(index, usize, span)?;
                crate::lower::display::join(vec![
                    Expr::str("index out of bounds: the len is "),
                    len,
                    Expr::str(" but the index is "),
                    index,
                ])
            }
            AssertKind::Overflow(..)
            | AssertKind::OverflowNeg(_)
            | AssertKind::DivisionByZero(_)
            | AssertKind::RemainderByZero(_) => {
                for operand in assert_operands(msg) {
                    self.mir_expr(state, operand, out)?;
                }
                Expr::str(match msg {
                    AssertKind::Overflow(op, ..) => overflow_message(*op),
                    AssertKind::OverflowNeg(_) => "attempt to negate with overflow",
                    AssertKind::DivisionByZero(_) => "attempt to divide by zero",
                    _ => "attempt to calculate the remainder with a divisor of zero",
                })
            }
            _ => return Err(self.unsupported(span, "this check, from its MIR")),
        })
    }

    // ── Calls ───────────────────────────────────────────────────────────
}
