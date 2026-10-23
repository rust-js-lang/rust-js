//! Temporaries made and not yet used: kept, folded where they're used, or
//! written as `const`s, in the order they're read.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What `js::is_defined` is given isn't read: it names what's asked of,
    /// `typeof` of its path, so a static's value MIR copies into its
    /// argument, `Symbol.toStringTag`, is no read before the test, which
    /// throws where the host hasn't the global (ADR 0647).
    pub(super) fn unread_defined_operands(
        &self,
        state: &mut State<'_, 'tcx>,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
    ) {
        let Some((def_id, _)) = fn_def(func.ty(&state.body.local_decls, self.tcx)) else {
            return;
        };
        if !bindings::is_binding(self.tcx, def_id)
            || !matches!(bindings::js_form(self.tcx, def_id), bindings::JsForm::Defined)
        {
            return;
        }
        for arg in args {
            if let Operand::Move(place) | Operand::Copy(place) = &arg.node
                && let Some(local) = place.as_local()
            {
                state.pending.retain(|(l, _)| *l != local);
            }
        }
    }

    /// Each temporary made and not yet used, made here, as `const`s: what
    /// runs next must not run before them, nor change what they read.
    pub(super) fn flush(&mut self, state: &mut State<'_, 'tcx>, out: &mut Vec<Stmt>) -> R<()> {
        let unwind = state.unwind.take();
        let mut made = Vec::new();
        let mut kept = Vec::new();
        for (local, value) in std::mem::take(&mut state.pending) {
            // A discriminant, which only a switch reads, of what reads the
            // same anywhere: kept for it.
            if let Value::Discriminant(subject, ..) = &value
                && subject.reads_same()
            {
                kept.push((local, value));
                continue;
            }
            // And an `Option`'s whose test is known, which reads what its
            // switch reads it with, unchanged meanwhile (ADR 0642).
            if let Value::Tested(_) = &value {
                kept.push((local, value));
                continue;
            }
            let mut expr = self.value_expr(value, Span::default())?;
            let named = state.named.contains(&local);
            if self.reads_same_anywhere(&expr, state.body.local_decls[local].ty) && !named {
                state.in_place.insert(local, expr);
                continue;
            }
            // An element, or an object, what it's made of read here, made
            // where it's used.
            let before = made.len();
            if !named && (self.capture_jsx(&mut expr, &mut made) || self.capture_object(&mut expr, &mut made)) {
                for stmt in &made[before..] {
                    match &stmt.kind {
                        StmtKind::Const(name, _) => {
                            self.locals.mir.captured.insert(name.clone());
                            self.locals.mir.alike.insert(name.clone());
                            state.own_names.insert(name.clone());
                        }
                        // What a branch's element reads, set where its test
                        // holds: no `const`, but MIR's too.
                        StmtKind::Let(name, None) => {
                            self.locals.mir.captured.insert(name.clone());
                            state.own_names.insert(name.clone());
                        }
                        _ => {}
                    }
                }
                state.in_place.insert(local, expr);
                continue;
            }
            let name = state.locals.names[local].clone();
            state.locals.declared[local] = true;
            made.push(StmtKind::Const(name, expr).at(js::Span::NONE));
        }
        state.pending = kept;
        self.unwinding_to(state, unwind, made, out)
    }

    /// Each value kept to be read in place that reads through a shared
    /// reference, which nothing changes while it's borrowed, `tmp.length` of
    /// a `&Vec`: read here, before `block`'s drop, which may change what it
    /// refers to once the borrow's over, a `D: Deref`'s `d.len()` returned
    /// (ADR 0451). Not one read only before it, as a branch's that's done
    /// with, whose reference isn't there.
    pub(super) fn settle_reads(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock, out: &mut Vec<Stmt>) {
        let references: HashSet<&str> = (state.body.local_decls.iter_enumerated())
            .filter(|(_, decl)| decl.ty.is_ref())
            .map(|(local, _)| state.locals.names[local].as_str())
            .filter(|name| self.locals.mir.plain.contains(*name))
            .collect();
        let reads_reference = |e: &Expr| {
            // Not an element or an object made where it's used, which reads
            // what a branch after sets (`capture_jsx`).
            if matches!(e.kind, js::ExprKind::Jsx(_) | js::ExprKind::Object(_)) {
                return false;
            }
            let mut found = false;
            e.visit_vars(&mut |name| found |= references.contains(name));
            found
        };
        let mut settled: Vec<Local> = state
            .in_place
            .iter()
            .filter(|(_, e)| reads_reference(e))
            .map(|(&l, _)| l)
            .filter(|&l| {
                state.body.basic_blocks[block]
                    .terminator()
                    .successors()
                    .any(|b| read_from(state.body, b, l))
            })
            .collect();
        settled.sort();
        for local in settled {
            let expr = state.in_place.remove(&local).expect("found");
            // Assigned, its `let` where the function's are, as it may be
            // read past the block it's dropped in.
            let name = state.locals.names[local].clone();
            if !state.locals.declared[local] {
                state.locals.declared[local] = true;
                state.settled.push(local);
            }
            out.push(StmtKind::Assign(Expr::var(&name), expr).at(js::Span::NONE));
        }
    }

    /// `made`, run so that a panic in it drops what `unwind`, its cleanup,
    /// drops (`drops.rs`).
    pub(super) fn unwinding_to(
        &mut self,
        state: &mut State<'_, 'tcx>,
        unwind: Option<BasicBlock>,
        made: Vec<Stmt>,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        match unwind {
            Some(cleanup) if !made.is_empty() => {
                let cleanup = self.mir_unwind(state, mir::UnwindAction::Cleanup(cleanup))?;
                self.unwinding(made, cleanup, js::Span::NONE, out);
            }
            _ => out.extend(made),
        }
        Ok(())
    }

    /// Where the `if` whose condition MIR's switch is was written, `if c`, its
    /// `if` first, for the source map: the switch is where its condition is.
    pub(super) fn if_span(&self, span: Span) -> Span {
        (self.thir.exprs.iter())
            .find_map(|e| match e.kind {
                thir::ExprKind::If { cond, .. } if self.thir[self.strip(cond)].span == span => {
                    Some(e.span.with_hi(span.hi()))
                }
                _ => None,
            })
            .unwrap_or(span)
    }

    /// The pattern of the arm written at `at` of the `match` whose switch MIR's
    /// at `span`, its `match x`, is, for the source map.
    pub(super) fn arm_span(&self, span: Span, at: rustc_span::BytePos) -> Option<Span> {
        let arms = (self.thir.exprs.iter()).find_map(|e| match &e.kind {
            thir::ExprKind::Match { arms, .. } if e.span.lo() == span.lo() && e.span.contains(span) => Some(arms),
            _ => None,
        })?;
        let arm = arms
            .iter()
            .map(|&arm| &self.thir[arm])
            .find(|arm| arm.span.lo() <= at && at < arm.span.hi())?;
        Some(arm.pattern.span)
    }

    /// The temporaries made and not yet used that do something.
    pub(super) fn acting(&self, state: &State<'_, 'tcx>) -> Vec<Local> {
        let acting = state.pending.iter().filter(|(_, value)| !movable(state, value));
        acting.map(|(local, _)| *local).collect()
    }

    /// A local's value: its temporary's, taken where it's used, or its
    /// variable. A temporary used before others made after it, which would
    /// change their order, has all of them made first.
    pub(super) fn read_local_value(
        &mut self,
        state: &mut State<'_, 'tcx>,
        local: Local,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        if state.pending.iter().any(|(l, _)| *l == local) && self.admit(state, &[local], true, out)? {
            let at = state.pending.iter().position(|(l, _)| *l == local).expect("admitted");
            return Ok(state.pending.remove(at).1);
        }
        if let Some(value) = state.in_place.get(&local) {
            return Ok(Value::Expr(value.clone()));
        }
        if let Some(place) = state.refs.get(&local) {
            return Ok(Value::Ref(place.clone()));
        }
        if let Some(value) = state.places.get(&local) {
            return Ok(value.clone());
        }
        match state.locals.names[local].as_str() {
            "" => Ok(Value::Expr(Expr::undefined())),
            // A cell a pointer is taken to (ADR 0407): its `value`.
            name if state.pointed.locals.contains(&local) => Ok(Value::Expr(Expr::held(Expr::var(name)))),
            name => Ok(Value::Expr(Expr::var(name))),
        }
    }

    /// Operands read in this order, `f(a, b)`'s: the temporaries they take
    /// written in place where they're the latest made, in the order they
    /// were made, and what's read before the last of them can't change
    /// meanwhile, a constant or a local nothing borrows. Otherwise what's
    /// made is made first, and they're read by name.
    pub(super) fn take_operands(
        &mut self,
        state: &mut State<'_, 'tcx>,
        operands: &[&Operand<'tcx>],
        out: &mut Vec<Stmt>,
    ) -> R<Vec<Value<'tcx>>> {
        let pending = |state: &State<'_, 'tcx>, operand: &Operand<'tcx>| match operand {
            Operand::Copy(place) | Operand::Move(place) if self.through_values(state.body, place, true) => state
                .pending
                .iter()
                .any(|(l, _)| *l == place.local)
                .then_some(place.local),
            _ => None,
        };
        let stable = |state: &State<'_, 'tcx>, operand: &Operand<'tcx>| match operand {
            Operand::Constant(_) => true,
            Operand::Copy(place) | Operand::Move(place) => {
                place.projection.is_empty() && !state.locals.borrowed[place.local] && !state.locals.changed[place.local]
            }
            #[allow(unreachable_patterns)]
            _ => false,
        };
        let pends: Vec<Option<Local>> = operands.iter().map(|o| pending(state, o)).collect();
        let stables: Vec<bool> = operands.iter().map(|o| stable(state, o)).collect();
        let taken: Vec<Local> = pends.iter().flatten().copied().collect();
        // What's read before the last taken that does something can't change meanwhile.
        let last = pends.iter().rposition(|p| {
            p.is_some_and(|l| {
                let value = &state.pending.iter().find(|(p, _)| *p == l).expect("pending").1;
                !movable(state, value)
            })
        });
        let others_stable = last.is_none_or(|last| (0..last).all(|i| pends[i].is_some() || stables[i]));
        let admitted = taken.is_empty() || self.admit(state, &taken, others_stable, out)?;
        let mut values = Vec::new();
        for (operand, pend) in operands.iter().zip(pends) {
            values.push(match pend.filter(|_| admitted) {
                Some(local) => {
                    let at = state.pending.iter().position(|(l, _)| *l == local).expect("pending");
                    state.pending.remove(at).1
                }
                None => self.mir_operand(state, operand, out)?,
            });
        }
        Ok(values)
    }

    /// Whether temporaries `taken`, in the order they're read, can be
    /// written where they're read: those that do something read in the
    /// order they were made, what was made after the first of them movable,
    /// and `others_stable`, what else is read before them unchanged
    /// meanwhile. What was made before them is made first, here. If they
    /// can't be, everything made is, and false.
    pub(super) fn admit(
        &mut self,
        state: &mut State<'_, 'tcx>,
        taken: &[Local],
        others_stable: bool,
        out: &mut Vec<Stmt>,
    ) -> R<bool> {
        let at = |l: Local| state.pending.iter().position(|(p, _)| *p == l).expect("pending");
        let acting: Vec<usize> = taken
            .iter()
            .map(|&l| at(l))
            .filter(|&i| !movable(state, &state.pending[i].1))
            .collect();
        let ordered = acting.windows(2).all(|w| w[0] < w[1]);
        let first = acting.iter().copied().min();
        let positions: Vec<usize> = taken.iter().map(|&l| at(l)).collect();
        let after_movable = first.is_none_or(|first| {
            (first..state.pending.len())
                .filter(|i| !positions.contains(i))
                .all(|i| movable(state, &state.pending[i].1))
        });
        if !(ordered && after_movable && (others_stable || acting.is_empty())) {
            self.flush(state, out)?;
            return Ok(false);
        }
        // What was made before the first that does something, made first.
        if let Some(first) = first {
            let before: Vec<usize> = (0..first).filter(|i| !positions.contains(i)).collect();
            if !before.iter().all(|&i| movable(state, &state.pending[i].1)) {
                let mut made = Vec::new();
                for i in before.into_iter().rev() {
                    made.push(state.pending.remove(i));
                }
                made.reverse();
                let kept = std::mem::replace(&mut state.pending, made);
                self.flush(state, out)?;
                state.pending = kept;
            }
        }
        Ok(true)
    }

    /// Whether `e` reads the same wherever it's read, so it's written there:
    /// a constant, what the module names, a function or a static, a variable
    /// nothing changes or a field of plain data in one, of type `ty`.
    pub(super) fn reads_same_anywhere(&self, e: &Expr, ty: Ty<'tcx>) -> bool {
        let alike = |e: &Expr| match &e.kind {
            js::ExprKind::Var(name) => self.module_names.contains(name) || self.locals.mir.alike.contains(name),
            js::ExprKind::Symbol(_) => true,
            js::ExprKind::Member(..) => {
                let mut root = e;
                while let js::ExprKind::Member(object, _) = &root.kind {
                    root = object;
                }
                matches!(&root.kind, js::ExprKind::Var(name) if self.locals.mir.plain.contains(name))
            }
            _ => e.is_constant(),
        };
        // A context's `Provider` or `Consumer`, `react::provider`'s, which
        // React never changes.
        let context_member = crate::lower::recognition::is_context_member(self.tcx, ty);
        match &e.kind {
            js::ExprKind::Member(..) if context_member => true,
            // A pointer, one handle for each place, of a place that's the same
            // anywhere (ADR 0407): `$ptr(COUNT, "value")`.
            js::ExprKind::Call(callee, args) if matches!(callee.helper_name(), Some("$ptr" | "$ptrTo")) => {
                args.iter().all(|arg| alike(arg) || arg.is_constant())
            }
            _ => alike(e),
        }
    }

    /// How an `Assert`, `terminator`, isn't an `if` that throws: left out,
    /// as THIR knows it passes or the helper that runs next checks it, or
    /// made where the element is read. Neither where its panic has something
    /// to drop as it unwinds: that's the `if`'s. `None`, the `if`.
    pub(super) fn assert_check(
        &self,
        body: &mir::Body<'tcx>,
        drops: Option<&drops::Elaboration<'tcx>>,
        terminator: &mir::Terminator<'tcx>,
    ) -> Option<Check> {
        let TerminatorKind::Assert {
            cond,
            expected,
            msg,
            target,
            unwind,
        } = &terminator.kind
        else {
            return None;
        };
        if self.known_in_bounds(msg, terminator.source_info.span) || constant_truth(body, cond) == Some(*expected) {
            return Some(Check::Unchecked);
        }
        if drops::cleans_up(body, drops, *unwind).is_some() {
            return None;
        }
        if checked_by_division(body, msg, *target) {
            return Some(Check::Unchecked);
        }
        deferred_check(body, msg, *target).map(|(index, write)| Check::Deferred(index, write))
    }

    pub(super) fn read_local(&mut self, state: &mut State<'_, 'tcx>, local: Local, out: &mut Vec<Stmt>) -> R<Expr> {
        let value = self.read_local_value(state, local, out)?;
        self.value_expr(value, state.body.local_decls[local].source_info.span)
    }

    /// A value as JS: a `format_args!` argument, the string it shows.
    pub(super) fn value_expr(&mut self, value: Value<'tcx>, span: Span) -> R<Expr> {
        Ok(match value {
            Value::Expr(e) => e,
            Value::Fmt(Std::FmtDisplay, ty, e) => self.display_string(e, ty, span)?,
            Value::Fmt(Std::FmtDebug, ty, e) => self.debug_string(e, ty, span)?,
            Value::Fmt(_, _, e) => e,
            Value::List(items) => Expr::array(
                items
                    .into_iter()
                    .map(|v| self.value_expr(v, span))
                    .collect::<R<Vec<_>>>()?,
            ),
            Value::Discriminant(..) => return Err(self.unsupported(span, "an enum's discriminant read as a value")),
            Value::Place(e) => e,
            Value::Branch(..) => return Err(self.unsupported(span, "what `?` made, read as a value, from its MIR")),
            Value::Tested(..) | Value::Cut(..) => {
                return Err(self.unsupported(span, "an `Option` read as a test, as a value, from its MIR"));
            }
            // Kept or given: a handle on its place, which reads and writes it
            // (ADR 0099), where the place is always the same one.
            // A handle on the `value` of what a variable holds, a box, a handle
            // or a cell, is that: what it's read and written through. Of what
            // a call gives, `$strPart(..)`'s, the one it gives, there.
            // So is one a call gives in what it gives, a store's
            // `lm_get_mut(i).unwrap().1`, made there once.
            Value::Ref(ref place) if let Some(cell) = self.held_cell(place) => cell,
            Value::Ref(place) if fixed_place(&place) => Expr::handle(place),
            // A field or an item of an object reached another way, `$index(v,
            // 0)[0]` of `&mut v[0].0`: a handle on it, `$handleOn`, the object
            // reached once, where it's borrowed, the same while it lasts.
            Value::Ref(Expr {
                kind: js::ExprKind::Member(object, field),
                ..
            }) => Expr::call_helper("$handleOn", vec![*object, Expr::str(&field)]),
            Value::Ref(Expr {
                kind: js::ExprKind::Index(object, key),
                ..
            }) if key.is_constant() || matches!(key.kind, js::ExprKind::Var(_)) => {
                Expr::call_helper("$handleOn", vec![*object, *key])
            }
            // Of what a generic `Option`'s `Some` holds, `$someValue(o)`: a
            // handle that reads it so and writes `o` a `Some`, given a handle
            // on `o` (ADR 0051).
            Value::Ref(ref place)
                if let Some(option) = crate::lower::std_types::option::some_value_of(place)
                    && fixed_place(option) =>
            {
                let on = self.value_expr(Value::Ref(option.clone()), span)?;
                Expr::call_helper("$someMut", vec![on])
            }
            // Of a `()`, which has no place: nothing, as writing one is.
            Value::Ref(place) if matches!(place.kind, js::ExprKind::Undefined) => place,
            Value::Ref(_) => return Err(self.unsupported(span, "a `&mut` of a number kept or given, from its MIR")),
        })
    }

    // ── Statements ──────────────────────────────────────────────────────
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The cell, box or handle whose `value` `place`, a `&mut`'s, is: one a
    /// call gives, `$strPart(..)`'s, one made there, or one a place holds,
    /// a reborrow's, `&mut **u` (`locals.mir.cells`). What a `&mut` to the
    /// place is given as, the cell, read and written through it.
    pub(super) fn held_cell(&self, place: &Expr) -> Option<Expr> {
        let js::ExprKind::Member(holder, field) = &place.kind else {
            return None;
        };
        let held = field == js::HELD
            && match &call_part(holder).kind {
                js::ExprKind::Call(..) | js::ExprKind::Object(_) => true,
                _ => self.locals.mir.cells.iter().any(|cell| cell.written_alike(holder)),
            };
        held.then(|| (**holder).clone())
    }
}
