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

    /// The locals `OptionWrites::null_nones` says: each write a `Some(..)` or a
    /// `None`, one at least, and each read a nullable field's, or a
    /// `#[rust_js::nullable]` function's result, by its return.
    fn mir_null_nones(&self, body: &mir::Body<'tcx>, locals: &Locals) -> std::collections::HashSet<Local> {
        let mut made: std::collections::HashMap<Local, (usize, bool)> = Default::default();
        let mut given: std::collections::HashMap<Local, usize> = Default::default();
        for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
            let mir::StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
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
                    false => none && writes == locals.writes[local] && given.get(&local) == Some(&locals.reads[local]),
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
    pub(super) fn mir_option_writes(&self, body: &mir::Body<'tcx>, locals: &Locals) -> OptionWrites {
        OptionWrites {
            null_nones: self.mir_null_nones(body, locals),
            made_some: self.mir_made_some(body, locals),
            registered: self.mir_registered(body),
            present: self.mir_present(body),
        }
    }

    /// The locals `OptionWrites::present` says: each a call
    /// `body_queries::present_calls` says gives what's there.
    fn mir_present(&self, body: &mir::Body<'tcx>) -> std::collections::HashSet<Local> {
        let calls = crate::lower::body_queries::present_calls(self.tcx, self.frame.thir);
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
        let reads = crate::lower::body_queries::registered_reads(self.tcx, self.frame.thir);
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
    /// shared reference to one of those, written once.
    fn mir_made_some(&self, body: &mir::Body<'tcx>, locals: &Locals) -> std::collections::HashSet<Local> {
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
                Rvalue::Ref(_, BorrowKind::Shared, of) if of.projection.is_empty() => {
                    refs.push((assign.0.local, of.local));
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
}
