//! Temporaries made and not yet used: kept, folded where they're used, or
//! written as `const`s, in the order they're read.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `lower` in an evaluation of its own: none of what's around it made and
    /// not yet used, nor the cleanup that unwinds to, nor the copies back a
    /// call around it owes, which are the enclosing evaluation's again once
    /// it's done, however `lower` ends. What it gives, and the copies back
    /// its own calls owe, which its caller writes where they're owed.
    pub(super) fn in_fresh_evaluation<T>(
        &mut self,
        state: &mut State<'_, 'tcx>,
        lower: impl FnOnce(&mut Self, &mut State<'_, 'tcx>) -> T,
    ) -> (T, Vec<(Expr, String, bool)>) {
        let pending = std::mem::take(&mut state.pending);
        let unwind = state.unwind.take();
        let copy_backs = std::mem::take(&mut state.copy_backs);
        let lowered = lower(self, state);
        state.pending = pending;
        state.unwind = unwind;
        (lowered, std::mem::replace(&mut state.copy_backs, copy_backs))
    }

    /// What `js::is_defined` is given isn't read: it names what's asked of,
    /// `typeof` of its path, so a static's value MIR copies into its
    /// argument, `Symbol.toStringTag`, is no read before the test, which
    /// throws where the host hasn't the global (ADR 0647). Not a value
    /// `js::is_function` asks of, which it reads (`asks_of_value`).
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
            || !matches!(
                bindings::js_form(self.tcx, def_id),
                bindings::JsForm::Defined | bindings::JsForm::Function
            )
            || self.reads_a_value(state.body, def_id, args)
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
        for (local, mut value) in std::mem::take(&mut state.pending) {
            // A discriminant, which only a switch reads: kept for it, of
            // what reads the same anywhere, a call's value held first, here,
            // `*w.r()`'s (#171).
            if let Value::Discriminant(subject, ..) = &mut value {
                if !subject.reads_same() {
                    let made_once = std::mem::replace(subject, Expr::undefined());
                    *subject = self.spill("_$", made_once, &mut made);
                }
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
        self.settle(state, &[block], out);
    }

    /// As `block`'s switch branches, its test made, each value kept to be
    /// read in place that a drop after it would settle: read here, where
    /// every branch has it, not in the one branch that drops, past which the
    /// others would read a variable that branch alone set (#274). Once its
    /// test is made, which keeps in place what's made before it (`flush`):
    /// before, the value isn't there to settle, and a test further in, which
    /// a way past this one skips, settles it instead, the second of
    /// `p.to_string() + &(if a && (b || c) { .. } else { .. })` (ADR 0451,
    /// amended). Only past a drop that drops what JS sees: one of what it
    /// doesn't, a `String`'s, settles nothing, so React's `prefix + (key !=
    /// null && .. ? .. : '') + childKey` reads `prefix` where it's joined.
    pub(super) fn settle_before_branches(
        &mut self,
        state: &mut State<'_, 'tcx>,
        block: BasicBlock,
        out: &mut Vec<Stmt>,
    ) {
        let graph = state.facts.graph();
        if state.in_place.is_empty() || graph.succ[block.as_usize()].len() < 2 {
            return;
        }
        let drops: Vec<BasicBlock> = (graph.rpo.iter().copied())
            .filter(|&d| d != block && graph.dominates(block, d))
            .filter(|&d| match &state.body.basic_blocks[d].terminator().kind {
                TerminatorKind::Drop { place, .. } => state.facts.drops().is_some_and(|e| e.drops(d, *place)),
                _ => false,
            })
            .collect();
        self.settle(state, &drops, out);
    }

    /// Each value kept to be read in place that reads through a shared
    /// reference and is read past one of `drops`: assigned here.
    fn settle(&mut self, state: &mut State<'_, 'tcx>, drops: &[BasicBlock], out: &mut Vec<Stmt>) {
        if drops.is_empty() {
            return;
        }
        let references: HashSet<&str> = (state.body.local_decls.iter_enumerated())
            .filter(|(_, decl)| decl.ty.is_ref())
            .map(|(local, _)| state.locals.names[local].as_str())
            .filter(|name| self.locals.mir.plain.contains(*name))
            .collect();
        let reads_reference = |e: &Expr| {
            // Not an element or an object made where it's used, which reads
            // what a branch after sets (`capture_jsx`).
            if matches!(e.kind, js::ExprKind::Jsx(_) | js::ExprKind::Object(..)) {
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
                (drops.iter()).any(|&block| {
                    (state.body.basic_blocks[block].terminator().successors()).any(|b| read_from(state.body, b, l))
                })
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
        (self.frame.thir.exprs.iter())
            .find_map(|e| match e.kind {
                thir::ExprKind::If { cond, .. } if self.frame.thir[self.strip(cond)].span == span => {
                    Some(e.span.with_hi(span.hi()))
                }
                _ => None,
            })
            .unwrap_or(span)
    }

    /// The pattern of the arm written at `at` of the `match` whose switch MIR's
    /// at `span`, its `match x`, is, for the source map.
    pub(super) fn arm_span(&self, span: Span, at: rustc_span::BytePos) -> Option<Span> {
        Some(self.written_arm(span, at, |arm| arm.span)?.pattern.span)
    }

    /// The arm written at `at` of the `match` whose switch MIR's at `span`,
    /// where `written` says each arm is.
    fn written_arm(
        &self,
        span: Span,
        at: rustc_span::BytePos,
        written: impl Fn(&thir::Arm<'tcx>) -> Span,
    ) -> Option<&thir::Arm<'tcx>> {
        let arms = (self.frame.thir.exprs.iter()).find_map(|e| match &e.kind {
            thir::ExprKind::Match { arms, .. } if e.span.lo() == span.lo() && e.span.contains(span) => Some(arms),
            _ => None,
        })?;
        arms.iter()
            .map(|&arm| &self.frame.thir[arm])
            .find(|arm| written(arm).lo() <= at && at < written(arm).hi())
    }

    /// The variants the pattern of the arm written at `at` names, in the
    /// order it names them, `Pending | Uninitialized`'s: what its test
    /// tests in turn, as the program wrote them. Of a macro's `match`,
    /// `matches!`'s, where its call is: the program's pattern its first arm.
    pub(super) fn written_variants(&self, span: Span, at: rustc_span::BytePos) -> Vec<(VariantIdx, Span)> {
        fn named(pattern: &thir::Pat<'_>, found: &mut Vec<(VariantIdx, Span)>) {
            match &pattern.kind {
                thir::PatKind::Or { pats } => pats.iter().for_each(|p| named(p, found)),
                // Of a reference, match ergonomics' `&`.
                thir::PatKind::Deref { subpattern, .. } => named(subpattern, found),
                thir::PatKind::Variant { variant_index, .. } => found.push((*variant_index, pattern.span)),
                _ => {}
            }
        }
        let mut found = Vec::new();
        if let Some(arm) = self.written_arm(span, at, |arm| arm.span.source_callsite()) {
            named(&arm.pattern, &mut found);
        // An `if let`'s, `if let Kind::Function(_) | Kind::Object(_) = ..`,
        // whose switch is in its `let`, the innermost.
        } else if let Some((pattern, _)) = (self.frame.thir.exprs.iter())
            .filter_map(|e| match &e.kind {
                thir::ExprKind::Let { pat, .. } if e.span.contains(span) => Some((pat, e.span)),
                _ => None,
            })
            .min_by_key(|(_, written)| written.hi() - written.lo())
        {
            named(pattern, &mut found);
        }
        found
    }

    /// The temporaries made and not yet used that do something.
    pub(super) fn acting(&self, state: &State<'_, 'tcx>) -> Vec<Local> {
        let acting = state.pending.iter().filter(|(_, value)| !movable(state, value));
        acting.map(|(local, _)| *local).collect()
    }

    /// How many of the values made and not yet used act, a list's items
    /// each one: fewer once a statement held one of a list it read an item
    /// of, which stays made, its other items not read yet.
    pub(super) fn acting_parts(&self, state: &State<'_, 'tcx>) -> usize {
        // A list's item acts as `take_field` holds it: not movable, and
        // doing something, which what it holds it by doesn't.
        fn item(state: &State<'_, '_>, value: &Value<'_>) -> bool {
            !movable(state, value) && matches!(value, Value::Expr(e) if e.has_effects())
        }
        (state.pending.iter())
            .map(|(_, value)| match value {
                Value::List(items) => items.iter().filter(|value| item(state, value)).count(),
                value => usize::from(!movable(state, value)),
            })
            .sum()
    }

    /// Whether a temporary made and not yet used may throw, so runs before
    /// a call that unwinds elsewhere, outside its `try`; or runs the
    /// program's code, which one that never unwinds does too, `made(5)`,
    /// and so runs before what that call's JS does first, an `insert`'s
    /// drop of what it held (ADR 0902). A closure made, `cache => cache`,
    /// can't: it's made where it's read, `with`'s applied in place (ADR
    /// 0703).
    pub(super) fn may_throw_first(&self, state: &State<'_, 'tcx>) -> bool {
        (state.pending.iter()).any(|(_, value)| {
            !movable(state, value)
                && match value {
                    Value::Expr(e) => e.may_throw() || (e.has_effects() && e.runs_program_code()),
                    _ => true,
                }
        })
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
            name if state.facts.pointed().locals.contains(&local) => Ok(Value::Expr(Expr::held(Expr::var(name)))),
            name => Ok(Value::Expr(Expr::var(name))),
        }
    }

    /// Whether `operand` takes a temporary not yet written whose value does
    /// something, which can't be read past what runs before it.
    pub(super) fn acts_pending(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> bool {
        matches!(operand, Operand::Copy(place) | Operand::Move(place)
            if self.through_values(state.body, place, true)
                && state.pending.iter().any(|(l, value)| *l == place.local && !movable(state, value)))
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
            // Of a plain variable, but not a read a getter may answer, which
            // may give another value each time (`Expr::reads_same`).
            js::ExprKind::Member(..) => {
                let mut root = e;
                while let js::ExprKind::Member(object, _) = &root.kind {
                    if root.facts.has(js::Known::Getter) {
                        return false;
                    }
                    root = object;
                }
                matches!(&root.kind, js::ExprKind::Var(name) if self.locals.mir.plain.contains(name))
            }
            _ => e.is_constant(),
        };
        // A context's `Provider` or `Consumer`, `react::provider`'s, which
        // React never changes.
        let context_member = crate::lower::recognition::is_context_member(self.tcx, ty);
        // A closure that captures nothing, an arrow of what it's given and
        // the module's names alone: the same function made anywhere, so
        // where it's given, which may take the function it calls in its
        // place (ADR 0605, amended).
        if let ty::Closure(_, args) = ty.kind()
            && args.as_closure().upvar_tys().is_empty()
        {
            return true;
        }
        match &e.kind {
            js::ExprKind::Member(..) if context_member => true,
            // A thread-local made where it's first used, `key()`, the same
            // cell wherever it's read (`Known::LocalRead`): read where it's
            // used, after what its call is given, as Rust makes it there.
            js::ExprKind::Call(..) if e.facts.has(js::Known::LocalRead) => true,
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

    /// `discriminant`'s number, a `Value::Discriminant`'s: each variant's
    /// discriminant where `switch_test` says it's that one, the last's
    /// else, `o != null ? 1 : 0` of an `Option`.
    fn discriminant_number(&mut self, discriminant: &Value<'tcx>, span: Span) -> R<Expr> {
        let Value::Discriminant(_, enum_ty, _) = discriminant else {
            unreachable!("a discriminant");
        };
        let enum_ty = *enum_ty;
        let ty::Adt(adt, _) = enum_ty.kind() else {
            return Err(self.unsupported(span, "a discriminant of what isn't an enum, read as a value"));
        };
        // Each variant's discriminant, as its bits and as its number, a
        // signed one's sign-extended, `-1` of an `enum E { A = -1 }`.
        let mut values = Vec::new();
        for (_, d) in adt.discriminants(self.tcx) {
            let number = match d.ty.is_signed() {
                true => {
                    let layout = (self.tcx.layout_of(self.typing_env.as_query_input(d.ty)))
                        .map_err(|_| self.unsupported(span, "a discriminant of an unsized type"))?;
                    layout.size.sign_extend(d.val)
                }
                false => d.val as i128,
            };
            values.push((d.val, number));
        }
        let Some((&(_, last), rest)) = values.split_last() else {
            return Err(self.unsupported(span, "a discriminant of an enum of no variant, read as a value"));
        };
        let mut made = Expr::int(last);
        for &(bits, number) in rest.iter().rev() {
            let test = self.switch_test(discriminant, enum_ty, bits, span, None)?;
            made = Expr::cond(test, Expr::int(number), made);
        }
        Ok(made)
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
            // An enum's discriminant held as a value, as rustc's MIR does
            // where jump threading reads it in two blocks: its number, of
            // the variant `switch_test` finds it is (ADR 0899).
            Value::Discriminant(..) => self.discriminant_number(&value, span)?,
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
                js::ExprKind::Call(..) | js::ExprKind::Object(..) => true,
                _ => self.locals.mir.cells.iter().any(|cell| cell.written_alike(holder)),
            };
        held.then(|| (**holder).clone())
    }
}
