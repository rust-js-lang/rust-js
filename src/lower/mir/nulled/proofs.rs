//! Where an `Option` is proved `Some`: every path to a read takes the edge
//! of a test of it that `None` never takes (#1902, #1933), and the tests,
//! writes and constants that proof reads. Split by concept from
//! `nulled.rs` at its cap (#2112's first step), unchanged.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `local`, an `Option`, is proved there where the block
    /// lowered now runs: every path to it takes the edge of a test of it
    /// that `None` never takes, and `local` is a parameter nothing writes or
    /// has one whole write, which comes before the test, and is never lent
    /// as a `&mut` (#1902, #1933). The tests: `js::truthy(local)`, JS's `if
    /// (local)`, and `local.is_some()`, their true edge; `local.is_none()`,
    /// its false one; a `match` or `if let` of it, `Some`'s arm. A read of
    /// it there is `Presence::Present`, so a nullable result or place stores it
    /// as it is, no `?? null`, and an exit gives it as it is, no `??
    /// undefined`.
    /// And a local written once, never lent, by a move or a copy of such a
    /// local, proved where it's written: it holds that value wherever it's
    /// read, React's Flight client's `*value = deferred`, its `deferred`
    /// moved into a temporary under `deferred.is_some()` (#1933's removal).
    pub(in crate::lower::mir) fn proved_present(&self, state: &State<'_, 'tcx>, local: Local) -> bool {
        if self.proved_by(state, local, Self::present_edge, state.at) || self.defaulted_present(state.body, local) {
            return true;
        }
        if self.ended_present(state, local) || self.present_on_all_paths(state, local, state.at) {
            return true;
        }
        let Some([Write::Assign(Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _))]) =
            self.whole_writes(state, local).as_deref()
        else {
            return false;
        };
        let written = (state.body.basic_blocks.iter_enumerated()).find_map(|(block, data)| {
            (data.statements.iter())
                .any(|s| matches!(&s.kind, mir::StatementKind::Assign(assign) if assign.0.local == local))
                .then_some(block)
        });
        of.projection.is_empty()
            && state.locals.writes[local] == 1
            && local.as_usize() > state.body.arg_count
            && written.is_some_and(|at| {
                self.proved_by(state, of.local, Self::present_edge, at)
                    || self.present_on_all_paths(state, of.local, at)
            })
    }

    /// Whether `local`, an `Option`, is written once, never lent, by a
    /// slice's `first()` or `last()` of one MIR proves not empty there
    /// (`present_proved`): `Some` wherever it's read, a read in place too,
    /// `keys.at(-1)` (#1860).
    fn ended_present(&self, state: &State<'_, 'tcx>, local: Local) -> bool {
        matches!(self.whole_writes(state, local).as_deref(), Some([Write::Call(block, ..)])
            if crate::lower::analysis::present_proved(self.tcx, state.body, *block))
    }

    /// Whether `local` is proved there by a test whose edge `edge` says,
    /// of the switch on its flag: `proved_present`'s, of a test that `None`
    /// never passes, and `proved_defined`'s, of one `undefined` never does.
    pub(super) fn proved_by(
        &self,
        state: &State<'_, 'tcx>,
        local: Local,
        edge: impl Fn(&Self, &State<'_, 'tcx>, Local, &mir::SwitchTargets, Local) -> Option<BasicBlock>,
        read: BasicBlock,
    ) -> bool {
        let parameter = (1..=state.body.arg_count).contains(&local.as_usize());
        // Each write whole, never lent as a `&mut` nor a raw pointer, which
        // a closure that captures it by `&mut` is too (`whole_writes`): one,
        // before the test, or more, none of them between the test and the
        // read (`unwritten_since`), React's getIteratorFn's `maybeIterator`,
        // a `match`'s in each arm (#1933 step 4 (A)).
        let Some(writes) = self.whole_writes(state, local) else {
            return false;
        };
        let once = writes.len() == usize::from(!parameter);
        let blocks = &state.body.basic_blocks;
        let dominators = blocks.dominators();
        blocks.iter_enumerated().any(|(_, data)| {
            let TerminatorKind::SwitchInt { discr, targets } = &data.terminator().kind else {
                return false;
            };
            let Some(flag) = discr.place().filter(|p| p.projection.is_empty()) else {
                return false;
            };
            // Entered from the test alone: a `match`'s false edges, which
            // MIR's borrow check reads and no run takes, aside.
            let entered = |edge: BasicBlock| {
                blocks.predecessors()[edge]
                    .iter()
                    .filter(|&&from| {
                        !matches!(blocks[from].terminator().kind,
                            TerminatorKind::FalseEdge { imaginary_target, real_target } if imaginary_target == edge && real_target != edge)
                    })
                    .count()
                    == 1
            };
            // An edge of a test of `local` that every way to `to` takes, and
            // past which nothing writes `local` up to the read.
            let proves = |flag: Local, targets: &mir::SwitchTargets, to: BasicBlock| {
                edge(self, state, flag, targets, local).is_some_and(|edge| {
                    entered(edge) && dominators.dominates(edge, to) && (once || self.unwritten_since(state, flag, local, read))
                })
            };
            if proves(flag.local, targets, read) {
                return true;
            }
            // A `bool` set to constants alone, `matches!`'s in each arm of
            // its `match` of a test of `local`: of the edge a value takes,
            // each write of that value is past an edge of a test that
            // proves `local` there, as MIR says which arm wrote it, so the
            // edge proves it too; one no write gives is never taken (#1933
            // step 4 (D)).
            let Some(written) = self.constant_writes(state, flag.local) else {
                return false;
            };
            let tests: Vec<(Local, &mir::SwitchTargets)> = (blocks.iter())
                .filter_map(|data| match &data.terminator().kind {
                    TerminatorKind::SwitchInt { discr, targets } => {
                        Some((discr.place().filter(|p| p.projection.is_empty())?.local, targets))
                    }
                    _ => None,
                })
                .filter(|&(tested, _)| tested != flag.local)
                .collect();
            let [(0, no)] = targets.iter().collect::<Vec<_>>()[..] else {
                return false;
            };
            [(false, no), (true, targets.otherwise())].into_iter().any(|(value, taken)| {
                entered(taken)
                    && dominators.dominates(taken, read)
                    && (written.iter().filter(|&&(_, v)| v == value))
                        .all(|&(at, _)| tests.iter().any(|&(tested, targets)| proves(tested, targets, at)))
            })
        })
    }

    /// Where each write of `flag` is and the value it writes, where `flag`
    /// is a `bool` set to constants alone, as a `matches!` sets its result
    /// in each arm of its `match`, or one written once of such a `bool` or
    /// its negation, `let defined = !matches!(..)`'s, where that `bool` is
    /// written: `None` where another write of it, or a lend, is too, or of
    /// a parameter, which its caller wrote.
    pub(super) fn constant_writes(&self, state: &State<'_, 'tcx>, flag: Local) -> Option<Vec<(BasicBlock, bool)>> {
        if !state.body.local_decls[flag].ty.is_bool() {
            return None;
        }
        let mut flag = flag;
        let mut negated = false;
        for _ in 0..4 {
            match self.whole_writes(state, flag)?.as_slice() {
                [Write::Assign(Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _))] if of.projection.is_empty() => {
                    flag = of.local;
                }
                [Write::Assign(Rvalue::UnaryOp(mir::UnOp::Not, Operand::Copy(of) | Operand::Move(of)))]
                    if of.projection.is_empty() =>
                {
                    flag = of.local;
                    negated = !negated;
                }
                _ => break,
            }
        }
        if (1..=state.body.arg_count).contains(&flag.as_usize()) {
            return None;
        }
        let mut written = Vec::new();
        for (block, data) in state.body.basic_blocks.iter_enumerated() {
            for statement in &data.statements {
                if let mir::StatementKind::Assign(assign) = &statement.kind
                    && assign.0.local == flag
                    && !data.is_cleanup
                {
                    let Rvalue::Use(Operand::Constant(constant), _) = &assign.1 else {
                        return None;
                    };
                    written.push((block, constant.const_.try_to_bool()? != negated));
                }
            }
        }
        (!written.is_empty() && self.whole_writes(state, flag)?.len() == written.len()).then_some(written)
    }

    /// Whether nothing writes `local` on any way from where `flag`, a test
    /// of it, is written to the block lowered now (`ways_from_test`), nor
    /// after the test in its own block: what the test read is what's read.
    pub(super) fn unwritten_since(&self, state: &State<'_, 'tcx>, flag: Local, local: Local, read: BasicBlock) -> bool {
        let blocks = &state.body.basic_blocks;
        let Some((test, after)) = (blocks.iter_enumerated()).find_map(|(block, data)| {
            let assigned = (data.statements.iter()).position(
                |statement| matches!(&statement.kind, mir::StatementKind::Assign(assign) if assign.0.local == flag),
            );
            match (assigned, &data.terminator().kind) {
                (Some(at), _) => Some((block, at + 1)),
                (None, TerminatorKind::Call { destination, .. }) if destination.local == flag => {
                    Some((block, data.statements.len()))
                }
                _ => None,
            }
        }) else {
            return false;
        };
        let writes = |statement: &mir::Statement<'tcx>| {
            matches!(&statement.kind, mir::StatementKind::Assign(assign) if assign.0.local == local)
                || matches!(&statement.kind, mir::StatementKind::SetDiscriminant { place, .. } if place.local == local)
        };
        let calls_into = |data: &mir::BasicBlockData<'tcx>| matches!(&data.terminator().kind, TerminatorKind::Call { destination, .. } if destination.local == local);
        if blocks[test].statements[after..].iter().any(writes) || (test != read && calls_into(&blocks[test])) {
            return false;
        }
        let Some(ways) = self.ways_from_test_to(state, test, read) else {
            return false;
        };
        ways.iter().all(|&block| {
            let data = &blocks[block];
            !data.statements.iter().any(writes) && (block == read || !calls_into(data))
        })
    }

    /// The blocks on each way from `test`, a block that ends with a test, to
    /// the block lowered now: forward of its successors, and back of the
    /// block lowered now, not through `test`, as a way through it again
    /// tests anew. `None` where the block lowered now isn't forward of it.
    /// The one answer of what runs between a test and a read, which
    /// `proved_present` and `field_reads` ask (#1933).
    pub(in crate::lower::mir) fn ways_from_test(
        &self,
        state: &State<'_, 'tcx>,
        test: BasicBlock,
    ) -> Option<IndexSet<BasicBlock>> {
        self.ways_from_test_to(state, test, state.at)
    }

    /// `ways_from_test`'s, to `read`, a block of the body.
    pub(super) fn ways_from_test_to(
        &self,
        state: &State<'_, 'tcx>,
        test: BasicBlock,
        read: BasicBlock,
    ) -> Option<IndexSet<BasicBlock>> {
        let blocks = &state.body.basic_blocks;
        let mut ahead: IndexSet<BasicBlock> = IndexSet::default();
        let mut work: Vec<BasicBlock> = blocks[test].terminator().successors().collect();
        while let Some(b) = work.pop() {
            if !ahead.insert(b) {
                continue;
            }
            work.extend(blocks[b].terminator().successors());
        }
        if !ahead.contains(&read) {
            return None;
        }
        let mut behind: IndexSet<BasicBlock> = IndexSet::default();
        let mut work = vec![read];
        while let Some(b) = work.pop() {
            if b == test || !behind.insert(b) {
                continue;
            }
            work.extend(blocks.predecessors()[b].iter().copied());
        }
        Some(ahead.into_iter().filter(|b| behind.contains(b)).collect())
    }

    /// The edge of a switch on `flag` that `local` is `Some` on: of a test
    /// `js::truthy(local)` or `local.is_some()`, its true edge; of
    /// `local.is_none()`, its false one; of `discriminant(local)`, `Some`'s;
    /// of `discriminant(js::classify(local))`, a variant's neither
    /// `undefined` nor `null` (`classified_present_edge`).
    pub(super) fn present_edge(
        &self,
        state: &State<'_, 'tcx>,
        flag: Local,
        targets: &mir::SwitchTargets,
        local: Local,
    ) -> Option<BasicBlock> {
        let values: Vec<(u128, BasicBlock)> = targets.iter().collect();
        match self.whole_writes(state, flag)?.as_slice() {
            // `js::type_of(local) == "function"`, of a kind `None`'s,
            // `"undefined"`, nor `null`'s, `"object"`, never is: its true
            // edge.
            [Write::Call(_, func, [a, b])] => {
                let [(0, _)] = values.as_slice() else {
                    return None;
                };
                let (callee, _) = func.const_fn_def()?;
                let eq = crate::lower::recognition::is_partial_eq(self.tcx, callee);
                let kind = |side: &Operand<'tcx>| self.type_of_test(state, side);
                let text = |side: &Operand<'tcx>| self.constant_text(state, side);
                let tested = match (kind(&a.node), kind(&b.node)) {
                    (Some(tested), None) => (tested, text(&b.node)?),
                    (None, Some(tested)) => (tested, text(&a.node)?),
                    _ => return None,
                };
                (eq && tested.0 == local && tested.1 != "undefined" && tested.1 != "object")
                    .then(|| targets.otherwise())
            }
            [Write::Call(_, func, [given])] => {
                // A test's: `0` its false edge, its only value, `otherwise`
                // its true one.
                let [(0, no)] = values.as_slice() else {
                    return None;
                };
                let (callee, _) = func.const_fn_def()?;
                let present = match crate::lower::recognition::is_js_presence_test(self.tcx, callee) {
                    true => targets.otherwise(),
                    false => match crate::lower::recognition::option_presence_test(self.tcx, callee)? {
                        true => targets.otherwise(),
                        false => *no,
                    },
                };
                (self.tested_local(state, &given.node, Some(local))? == local).then_some(present)
            }
            // A variant's of `k = js::classify(local)` that isn't `undefined`
            // or `null`, `if let Kind::Function(_) = js::classify(exports)`.
            [Write::Assign(Rvalue::Discriminant(of))] if of.projection.is_empty() && of.local != local => {
                self.classified_present_edge(state, of.local, targets, local)
            }
            // `Some`'s discriminant, 1, `None`'s 0 (`Option`'s variants).
            [Write::Assign(Rvalue::Discriminant(of))] if of.projection.is_empty() && of.local == local => {
                match values.as_slice() {
                    [(0, _)] => Some(targets.otherwise()),
                    _ => values.iter().find(|(value, _)| *value == 1).map(|&(_, block)| block),
                }
            }
            _ => None,
        }
    }

    /// The local `given`, a test's argument, is of: through the copies MIR
    /// makes of it for the call, `_11 = copy _3`, the reference it lends,
    /// `_12 = &_3`, and that reference's reborrows.
    pub(super) fn tested_local(
        &self,
        state: &State<'_, 'tcx>,
        given: &Operand<'tcx>,
        stop: Option<Local>,
    ) -> Option<Local> {
        let mut tested = given.place().filter(|p| p.projection.is_empty())?.local;
        for _ in 0..4 {
            if stop == Some(tested) {
                break;
            }
            match self.whole_writes(state, tested).as_deref() {
                Some(
                    [
                        Write::Assign(
                            Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _)
                            | Rvalue::Ref(_, BorrowKind::Shared, of),
                        ),
                    ],
                ) if of.projection.is_empty() => {
                    tested = of.local;
                }
                // A reborrow, `&(*_13)`, of what `_13` lends.
                Some([Write::Assign(Rvalue::Ref(_, BorrowKind::Shared, of))])
                    if of.projection[..] == [mir::ProjectionElem::Deref] =>
                {
                    tested = of.local;
                }
                _ => break,
            }
        }
        Some(tested)
    }

    /// The local `side`, a comparison's operand, is `js::type_of` of.
    pub(super) fn type_of_test(&self, state: &State<'_, 'tcx>, side: &Operand<'tcx>) -> Option<Local> {
        let kind = self.tested_local(state, side, None)?;
        let writes = self.whole_writes(state, kind)?;
        let [Write::Call(_, func, [given])] = writes.as_slice() else {
            return None;
        };
        let (callee, _) = func.const_fn_def()?;
        crate::lower::recognition::is_js_type_of(self.tcx, callee)
            .then(|| self.tested_local(state, &given.node, None))?
    }

    /// The text constant `side`, a comparison's operand, is, through the
    /// locals MIR holds it in and a promoted reference to it, `&"function"`.
    pub(super) fn constant_text(&self, state: &State<'_, 'tcx>, side: &Operand<'tcx>) -> Option<String> {
        let constant = match side {
            Operand::Constant(c) => **c,
            _ => {
                let held = self.tested_local(state, side, None)?;
                let writes = self.whole_writes(state, held)?;
                let [Write::Assign(Rvalue::Use(Operand::Constant(c), _))] = writes.as_slice() else {
                    return None;
                };
                **c
            }
        };
        self.text_of(state, &constant, 0)
    }

    pub(super) fn text_of(
        &self,
        state: &State<'_, 'tcx>,
        constant: &mir::ConstOperand<'tcx>,
        depth: usize,
    ) -> Option<String> {
        match constant.const_ {
            mir::Const::Val(value, ty) if ty.peel_refs().is_str() && ty.is_ref() && !ty.peel_refs().is_ref() => {
                let bytes = value.try_get_slice_bytes_for_diagnostics(self.tcx)?;
                String::from_utf8(bytes.to_vec()).ok()
            }
            mir::Const::Unevaluated(uv, _) if depth < 2 => {
                let referred = crate::lower::analysis::mir_reads::referred::referred_constant(
                    &state.facts.promoted()[uv.promoted?],
                )?;
                self.text_of(state, &referred, depth + 1)
            }
            _ => None,
        }
    }

    /// Each write of `local`, if each is whole, an assignment or a call's
    /// result, and nothing lends it as a `&mut` or sets its variant. Not a
    /// cleanup's, which writes the value a body's write of it does as the
    /// old one's drop unwinds, `active = closest(f)`'s of a `let mut active
    /// = js::null()` twice, and which the body's count of its writes leaves
    /// out (`Locals::writes`, #1933).
    pub(super) fn whole_writes<'b>(&self, state: &'b State<'_, 'tcx>, local: Local) -> Option<Vec<Write<'b, 'tcx>>> {
        let mut writes = Vec::new();
        for (block, data) in state.body.basic_blocks.iter_enumerated() {
            for statement in &data.statements {
                match &statement.kind {
                    mir::StatementKind::Assign(assign) => {
                        if assign.0.local == local {
                            if !assign.0.projection.is_empty() {
                                return None;
                            }
                            if !data.is_cleanup {
                                writes.push(Write::Assign(&assign.1));
                            }
                        }
                        // A `&mut` that never takes a `Some` away is no write
                        // of a `None` (`lend_keeps_some`, #1933 step 4 (B)).
                        if let Rvalue::Ref(_, BorrowKind::Mut { .. }, lent) = &assign.1
                            && lent.local == local
                            && !self.lend_keeps_some(state, *lent, assign.0)
                        {
                            return None;
                        }
                        if let Rvalue::RawPtr(mir::RawPtrKind::Mut, lent) = &assign.1
                            && lent.local == local
                        {
                            return None;
                        }
                    }
                    mir::StatementKind::SetDiscriminant { place, .. } if place.local == local => return None,
                    _ => {}
                }
            }
            if let mir::TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
                && destination.local == local
            {
                if !destination.projection.is_empty() {
                    return None;
                }
                writes.push(Write::Call(block, func, args));
            }
        }
        Some(writes)
    }

    /// The place `local` holds a copy of, a move of or a borrow of, where its
    /// one whole write is that (`whole_writes`), read in `block`: what a
    /// temporary MIR reads a variable or a capture into before it's given,
    /// `_5 = copy (*_1).0`, written in `block`; or of a call that goes on to
    /// `block` and gives its `Option` through (`passes_through`),
    /// `found.map(js::unknown)`'s or `js::cast(found)`'s, its argument; with
    /// the block it's read in. Only so, straight-line code with no test
    /// between, so what's proved of the place where the block lowered now
    /// runs, a proof no write since its test breaks, held where it was
    /// copied; a copy taken before a test, which may hold what the place
    /// held then, isn't followed (#2112 C S4).
    // helper: what `NoneForms::presence` follows a copy to, of the proofs it asks
    pub(in crate::lower::mir) fn copied_from(
        &self,
        state: &State<'_, 'tcx>,
        (local, block): (Local, BasicBlock),
    ) -> Option<(Place<'tcx>, BasicBlock)> {
        let writes = self.whole_writes(state, local)?;
        match &writes[..] {
            [Write::Call(at, func, args)] => {
                let into = matches!(
                    state.body.basic_blocks[*at].terminator().kind,
                    mir::TerminatorKind::Call { target: Some(target), .. } if target == block
                );
                let from = self.passes_through(state, local, func, args)?;
                into.then_some((from, *at))
            }
            [Write::Assign(rvalue)] => {
                let statements = &state.body.basic_blocks[block].statements;
                let copied_here = (statements.iter())
                    .any(|st| matches!(&st.kind, mir::StatementKind::Assign(a) if a.0.local == local));
                let from = match rvalue {
                    Rvalue::Use(Operand::Copy(place) | Operand::Move(place), _) | Rvalue::CopyForDeref(place) => *place,
                    Rvalue::Ref(_, mir::BorrowKind::Fake(_), _) => return None,
                    Rvalue::Ref(_, _, place) | Rvalue::Reborrow(_, _, place) => *place,
                    _ => return None,
                };
                copied_here.then_some((from, block))
            }
            _ => None,
        }
    }

    /// The `Option` a call of `func` on `args`, written to `local`, gives
    /// through: `Option::map`'s receiver, `Some` where it is; a binding
    /// that's its argument in JS of an `Option` (`is_its_argument`),
    /// `js::cast(found)`'s, the same value.
    // helper: what a call gives through, of `copied_from` and `NoneForms::call_presence`
    pub(in crate::lower::mir) fn passes_through(
        &self,
        state: &State<'_, 'tcx>,
        local: Local,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
    ) -> Option<Place<'tcx>> {
        let (callee, generic_args) = func.const_fn_def()?;
        let mapped =
            self.recognition().classify(callee, generic_args) == Some(crate::lower::recognition::Std::OptionMap);
        let from = args.first()?.node.place()?;
        match mapped {
            true => Some(from),
            false => self.gives_its_option(state.body, local, func, args),
        }
    }

    /// The `Option` a call of `func` on `args`, written to `local`, is in
    /// JS as it is: `js::cast(found)`'s argument, a binding that's its
    /// argument (`is_its_argument`), and a shared `o.as_ref()`'s,
    /// `Std::Pointee`'s, the `Option` its reference is to (ADR 0023). Its
    /// `None` is that one's, and so is its presence.
    // helper: what a call is as it is, of `passes_through` and `mir_nones`
    pub(in crate::lower::mir) fn gives_its_option(
        &self,
        body: &mir::Body<'tcx>,
        local: Local,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
    ) -> Option<Place<'tcx>> {
        let (callee, generic_args) = func.const_fn_def()?;
        let [arg] = args else { return None };
        let from = arg.node.place()?;
        let cast = self.is_its_argument(callee) && self.option_of(body.local_decls[local].ty).is_some();
        let lent = self.recognition().classify(callee, generic_args) == Some(crate::lower::recognition::Std::Pointee)
            && matches!(from.ty(body, self.tcx).ty.kind(), ty::Ref(_, _, ty::Mutability::Not));
        (cast || lent).then_some(from)
    }
}
