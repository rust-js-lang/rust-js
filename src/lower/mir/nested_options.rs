//! The locals an `Option` of an `Option` is held in as its payload (ADR
//! 1017): its `None` `undefined`, its `Some(None)` `null` and its
//! `Some(Some(v))` `v`, where each of its reads knows what its writes made.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The locals `OptionWrites::nested` says (ADR 1017): each an `Option`
    /// of an `Option` whose payload never looks like `None`
    /// (`nested_payload`), of no drop, neither a parameter nor the result,
    /// and each shared reference to one an `==` is given. Each written only
    /// `None`, `Some(..)` or another such, and read only by another such,
    /// by its discriminant, by its `Some`'s payload, `(x as Some).0`, or,
    /// through a reference, by an `==` of two such: so each read knows
    /// what its writes made. One read any other way, given to a function,
    /// returned, captured or borrowed `&mut`, isn't, nor what it's copied
    /// to or from, or compared with.
    pub(super) fn mir_nested(&self, body: &mir::Body<'tcx>, locals: &Locals) -> std::collections::HashSet<Local> {
        let decls = &body.local_decls;
        let mut nested: std::collections::HashSet<Local> = (decls.indices())
            .filter(|&local| local.as_usize() > body.arg_count)
            .filter(|&local| {
                let ty = decls[local].ty;
                self.nested_payload(ty).is_some() && !ty.needs_drop(self.tcx, self.typing_env)
            })
            .collect();
        if nested.is_empty() {
            return nested;
        }
        // Each shared reference to one, made once.
        let mut refs: HashMap<Local, Local> = HashMap::new();
        for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
            if let mir::StatementKind::Assign(assign) = &statement.kind
                && let Some(reference) = assign.0.as_local()
                && let Rvalue::Ref(_, BorrowKind::Shared, of) = &assign.1
                && let Some(of) = of.as_local()
                && nested.contains(&of)
                && locals.writes[reference] == 1
            {
                refs.insert(reference, of);
            }
        }
        // A read of what its `Some` holds, and through it.
        let payload = |place: &Place<'tcx>| {
            matches!(place.projection.as_slice(), [PlaceElem::Downcast(_, some), PlaceElem::Field(field, _), ..]
                if some.as_u32() == 1 && field.as_u32() == 0)
        };
        // Every mention of every local, but where it lives and dies.
        struct Mentions(Vec<(mir::Location, Local)>);
        impl<'tcx> Visitor<'tcx> for Mentions {
            fn visit_statement(&mut self, statement: &mir::Statement<'tcx>, location: mir::Location) {
                if !matches!(
                    statement.kind,
                    mir::StatementKind::StorageLive(_)
                        | mir::StatementKind::StorageDead(_)
                        | mir::StatementKind::FakeRead(..)
                        | mir::StatementKind::PlaceMention(..)
                        | mir::StatementKind::AscribeUserType(..)
                        | mir::StatementKind::Nop
                ) {
                    self.super_statement(statement, location);
                }
            }
            // A variable's name, which reads nothing.
            fn visit_var_debug_info(&mut self, _: &mir::VarDebugInfo<'tcx>) {}
            fn visit_local(&mut self, local: Local, _: PlaceContext, location: mir::Location) {
                self.0.push((location, local));
            }
        }
        let mut mentions = Mentions(Vec::new());
        mentions.visit_body(body);
        let mentions = mentions.0;
        // Where each is mentioned as it may be, and which must agree.
        let mut allowed: std::collections::HashSet<(mir::Location, Local)> = Default::default();
        let mut links: Vec<(Local, Local)> = Vec::new();
        for (block, data) in body.basic_blocks.iter_enumerated() {
            for (statement_index, statement) in data.statements.iter().enumerate() {
                let at = mir::Location { block, statement_index };
                let mir::StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                let (to, rvalue) = (&assign.0, &assign.1);
                let whole = to.as_local().filter(|local| nested.contains(local));
                match rvalue {
                    Rvalue::Aggregate(kind, _)
                        if let Some(to) = whole
                            && matches!(**kind, AggregateKind::Adt(..)) =>
                    {
                        allowed.insert((at, to));
                    }
                    Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _)
                        if let Some(to) = whole
                            && let Some(from) = from.as_local()
                            && nested.contains(&from) =>
                    {
                        allowed.extend([(at, to), (at, from)]);
                        links.push((to, from));
                    }
                    Rvalue::Ref(_, BorrowKind::Shared, of)
                        if let Some(reference) = to.as_local()
                            && let Some(of) = of.as_local()
                            && refs.get(&reference) == Some(&of) =>
                    {
                        allowed.extend([(at, reference), (at, of)]);
                    }
                    Rvalue::Discriminant(of)
                        if nested.contains(&of.local) && (of.projection.is_empty() || payload(of)) =>
                    {
                        allowed.insert((at, of.local));
                    }
                    // What its `Some` holds: its `Some`'s payload, never
                    // `None`-like; or the inner `Option` itself, whose
                    // `None` is `null` here, only where nothing tells that
                    // from the `undefined` a `None` is everywhere else
                    // (`only_tested`): a callback given it would be
                    // given `null`, and a `None` of any shape's value
                    // would be `null`, `Kind::Null`, whichever it was.
                    Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _)
                    | Rvalue::Ref(_, BorrowKind::Shared, of)
                    | Rvalue::CopyForDeref(of)
                        if whole.is_none()
                            && nested.contains(&of.local)
                            && payload(of)
                            && (!of.projection[2..].iter().all(|elem| *elem == PlaceElem::Deref)
                                || to
                                    .as_local()
                                    .is_some_and(|held| self.only_tested(body, held, locals, &mentions))) =>
                    {
                        allowed.insert((at, of.local));
                    }
                    _ => {}
                }
            }
            // `is_some()` or `is_none()` of one, through a reference; and an
            // `==` of one with a constant `Some(None)`, `last == Some(None)`
            // (ADR 1017, amended).
            if let mir::TerminatorKind::Call { func, args, .. } = &data.terminator().kind
                && let Some((def_id, generic_args)) = func.const_fn_def()
            {
                let through = |operand: &Operand<'tcx>| {
                    (operand.place())
                        .and_then(|place| place.as_local())
                        .filter(|local| refs.contains_key(local))
                };
                let tested = match &args[..] {
                    [a] if matches!(
                        self.recognition().classify(def_id, generic_args),
                        Some(Std::IsSome | Std::IsNone)
                    ) =>
                    {
                        through(&a.node)
                    }
                    [a, b]
                        if (self.tcx.trait_of_assoc(def_id)).is_some_and(|trait_id| {
                            matches!(
                                self.recognition().trait_call(def_id, trait_id),
                                Some(TraitCall::Equality { .. })
                            )
                        }) =>
                    {
                        match (through(&a.node), through(&b.node)) {
                            (Some(a), None) if self.nested_some_none(body, &b.node) => Some(a),
                            (None, Some(b)) if self.nested_some_none(body, &a.node) => Some(b),
                            _ => None,
                        }
                    }
                    _ => None,
                };
                if let Some(reference) = tested {
                    allowed.insert((body.terminator_loc(block), reference));
                }
            }
            // A keyed read of a value whose `None` its collection holds as
            // `null`, `map.get(key)`: JS's read tells the three apart (ADR
            // 0884, amended).
            if let mir::TerminatorKind::Call { func, destination, .. } = &data.terminator().kind
                && let Some(to) = destination.as_local()
                && nested.contains(&to)
                && let Some((def_id, args)) = func.const_fn_def()
                && bindings::reads_a_nullable_value(self.tcx, def_id, args)
            {
                allowed.insert((body.terminator_loc(block), to));
            }
            // An `==` of two of them, each through a reference.
            if let mir::TerminatorKind::Call { func, args, .. } = &data.terminator().kind
                && let Some((def_id, _)) = func.const_fn_def()
                && (self.tcx.trait_of_assoc(def_id)).is_some_and(|trait_id| {
                    matches!(
                        self.recognition().trait_call(def_id, trait_id),
                        Some(TraitCall::Equality { .. })
                    )
                })
                && let [a, b] = &args[..]
                && let (Some(a), Some(b)) = (a.node.place(), b.node.place())
                && let (Some(a), Some(b)) = (a.as_local(), b.as_local())
                && let (Some(&x), Some(&y)) = (refs.get(&a), refs.get(&b))
            {
                let at = body.terminator_loc(block);
                allowed.extend([(at, a), (at, b)]);
                links.push((x, y));
            }
        }
        // Every other mention.
        let mut out: std::collections::HashSet<Local> = (mentions.into_iter())
            .filter(|&(at, local)| {
                (nested.contains(&local) || refs.contains_key(&local)) && !allowed.contains(&(at, local))
            })
            .map(|(_, local)| local)
            .collect();
        links.extend(refs.iter().map(|(&reference, &of)| (reference, of)));
        loop {
            let before = out.len();
            for &(x, y) in &links {
                if out.contains(&x) || out.contains(&y) {
                    out.extend([x, y]);
                }
            }
            if out.len() == before {
                break;
            }
        }
        nested.extend(refs.keys());
        nested.retain(|local| !out.contains(local));
        nested
    }

    /// Whether `held`, the inner `Option` a `Some` of one held as its
    /// payload holds, copied out of it or borrowed, written once, is only
    /// tested (`mir_nested`): its discriminant read, its `Some`'s payload
    /// read, or it, or a shared reference to it made once, given to
    /// `is_some()` or `is_none()`, each of which reads its `null` as the
    /// `None` it is (ADR 1017, amended).
    fn only_tested(
        &self,
        body: &mir::Body<'tcx>,
        held: Local,
        locals: &Locals,
        mentions: &[(mir::Location, Local)],
    ) -> bool {
        let derefs = |place: &Place<'tcx>| place.projection.iter().all(|elem| elem == PlaceElem::Deref);
        let its_payload = |place: &Place<'tcx>| {
            let past: Vec<_> = (place.projection.iter())
                .skip_while(|elem| *elem == PlaceElem::Deref)
                .collect();
            matches!(past.as_slice(), [PlaceElem::Downcast(_, some), PlaceElem::Field(field, _), ..]
                if some.as_u32() == 1 && field.as_u32() == 0)
        };
        let tests = |func: &Operand<'tcx>| {
            (func.const_fn_def()).is_some_and(|(def_id, args)| {
                matches!(
                    self.recognition().classify(def_id, args),
                    Some(Std::IsSome | Std::IsNone)
                )
            })
        };
        // It, and each shared reference to it made once.
        let mut tested = std::collections::HashSet::from([held]);
        for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
            if let mir::StatementKind::Assign(assign) = &statement.kind
                && let Some(reference) = assign.0.as_local()
                && let Rvalue::Ref(_, BorrowKind::Shared, of) = &assign.1
                && of.local == held
                && derefs(of)
                && locals.writes[reference] == 1
            {
                tested.insert(reference);
            }
        }
        locals.writes[held] == 1
            && (mentions.iter())
                .filter(|(_, local)| tested.contains(local))
                .all(|&(at, local)| match body.stmt_at(at).left() {
                    Some(statement) => {
                        let mir::StatementKind::Assign(assign) = &statement.kind else {
                            return false;
                        };
                        let (to, rvalue) = (&assign.0, &assign.1);
                        // Its one write, and what's given to a test.
                        if to.as_local() == Some(local) {
                            return true;
                        }
                        local == held
                            && match rvalue {
                                Rvalue::Discriminant(of) => of.local == held && derefs(of),
                                Rvalue::Ref(_, BorrowKind::Shared, of)
                                    if derefs(of) && to.as_local().is_some_and(|to| tested.contains(&to)) =>
                                {
                                    of.local == held
                                }
                                Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _)
                                | Rvalue::Ref(_, BorrowKind::Shared, of)
                                | Rvalue::CopyForDeref(of) => of.local == held && its_payload(of),
                                _ => false,
                            }
                    }
                    None => matches!(
                        &body.basic_blocks[at.block].terminator().kind,
                        mir::TerminatorKind::Call { func, args, destination, .. }
                            if tests(func)
                                && destination.local != local
                                && matches!(&args[..], [arg] if arg.node.place() == Some(Place::from(local)))
                    ),
                })
    }
}
