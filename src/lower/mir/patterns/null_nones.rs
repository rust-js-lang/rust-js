//! Which `Option` locals hold `null` for `None` at each write
//! (`OptionWrites::null_nones`): what makes them and what reads them,
//! nullable fields and parameters, so a `None` is `null` where it's made.

use super::*;
use crate::collections::{IndexMap, IndexSet};
use crate::lower::representation::{NonePlace, none_form};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The locals `OptionWrites::null_nones` says: each write a `Some(..)`, a
    /// `None` or a call, one that may give `None` at least, and each read a
    /// nullable field's or binding parameter's, or a `#[rust_js::nullable]`
    /// thread-local's `set` or `replace` (#683), or a copy's only they read
    /// (ADR 0781, amended by 0841); or a
    /// `#[rust_js::nullable]` function's result, by its return.
    /// A `let` a call writes too, where a nullable parameter of the crate's
    /// own reads it, and each other read is one too, through a shared
    /// `as_ref()` or `as_deref()`, or a test that reads `null` and
    /// `undefined` alike (`alike_reads`): React's `previousMeasurements`
    /// (rd632, the user's decision (B)).
    /// Never a parameter, which holds what its caller gave before any write
    /// the body makes, `undefined` for `None` as a call of the crate's gives
    /// it (#1248).
    pub(super) fn mir_null_nones(
        &self,
        body: &mir::Body<'tcx>,
        promoted: &IndexVec<Promoted, mir::Body<'tcx>>,
        locals: &Locals,
    ) -> IndexSet<Local> {
        let mut made: IndexMap<Local, (usize, bool)> = Default::default();
        let mut given: IndexMap<Local, usize> = Default::default();
        // What a call makes, and what JS reads as it's stored, a field's or
        // a binding's parameter's: a call's `Option` only the crate's own
        // functions' nullable parameters read is given as it is, their
        // bodies reading `null` and `undefined` alike (ADR 0861).
        let mut called: IndexSet<Local> = Default::default();
        let mut stored: IndexSet<Local> = Default::default();
        // What the crate's own function's nullable parameter reads (rd632).
        let mut toward: IndexSet<Local> = Default::default();
        // Each copy of a variable, `_2 = copy _1`, MIR's of `owner` in `Info
        // { owner }`: the variable's read, given where the copy is.
        let mut copies: Vec<(Local, Local)> = Vec::new();
        // A `#[rust_js::nullable]` function's result, which a variable it
        // returns is given to: `let mut found = None; .. found = Some(n); ..
        // found`'s `None` is `null` where it's made, `let found = null;`,
        // and the variable returned as it is (ADR 0957, amended, the user's
        // decision, 2026-10-27).
        let returns_nullable = none_form(self.tcx, NonePlace::FnResult(body.source.def_id())).stores_null();
        // Each local assigned once, and what it's assigned: what a
        // thread-local's key is read through (`queries::referred_key`).
        let assigned = crate::lower::body_queries::assigned_once(body);
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
                    && none_form(self.tcx, NonePlace::Field(&fields[field], None)).stores_null()
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
            // What a `#[rust_js::nullable]` thread-local is set, `KEY.set(v)`
            // or `KEY.replace(v)`'s `v`, which it stores `null` for `None`
            // as a nullable field does: `importStringRequest = __DEV__ &&
            // debug ? null : request` of a test's `None` and `Some` (#683).
            if crate::lower::recognition::local_key_cell_method(self.tcx, def_id)
                == Some(crate::lower::recognition::LocalKeyCellMethod::Writes)
                && let [key, value] = &args[..]
                && let Some(read) = value.node.place()
                && read.projection.is_empty()
                && queries::referred_key(promoted, |local| assigned.get(&local).copied().flatten(), &key.node)
                    .is_some_and(|key| none_form(self.tcx, NonePlace::Local(key)).stores_null())
            {
                *given.entry(read.local).or_default() += 1;
            }
            let nullable = crate::lower::representation::null_params(self.tcx, def_id);
            // What a `#[rust_js::nullable]` thread-local is made of, its
            // `init`'s call's value: each `Option` given for a type
            // parameter's, `create_cursor(None)`'s (ADR 0586, amended).
            let made = destination.local == mir::RETURN_PLACE
                && destination.projection.is_empty()
                && (self.tcx.opt_parent(body.source.def_id()))
                    .is_some_and(|key| none_form(self.tcx, NonePlace::Local(key)).stores_null());
            if nullable.is_empty() && !made {
                continue;
            }
            let inputs = self.tcx.fn_sig(def_id).skip_binder().skip_binder().inputs();
            let binding = bindings::is_binding(self.tcx, def_id);
            for (i, arg) in args.iter().enumerate() {
                let named =
                    !nullable.is_empty() && none_form(self.tcx, NonePlace::BindingArg(def_id, i, None)).stores_null();
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
        let mut items: IndexSet<Local> = Default::default();
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
                        && none_form(self.tcx, NonePlace::Field(field, None)).stores_null()
                        && bindings::nullable_items(self.tcx, ty).is_some()
                    {
                        items.insert(read.local);
                    }
                }
            }
        }
        let mut options: IndexSet<Local> = Default::default();
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
        let mut alike: IndexMap<Local, usize> = Default::default();
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
        let mut lets: IndexSet<Local> = (body.var_debug_info.iter())
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
        let mut moved_in = IndexSet::default();
        for &(copy, of) in &copies.iter().copied().collect::<IndexSet<_>>() {
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
        let result = none_form(self.tcx, NonePlace::Result(body.source.def_id())).stores_null();
        // What a `let` is written, besides what it makes, of its arms' own
        // `let`s or temporaries, each written once by what makes an
        // `Option`, read once, there: `let x = if c { let y = f(); y }
        // else { g() };`'s `y` (rd632), `null` for `None` where it's
        // written as `x` is, so `x`'s copy of it is as it is.
        let arm = |of: Local| {
            locals.writes[of] == 1 && locals.reads[of] == 1 && made.get(&of).is_some_and(|&(writes, _)| writes == 1)
        };
        let mut nulled = IndexSet::default();
        let candidates: IndexSet<Local> = (made.keys().copied())
            .chain(copies.iter().filter(|&&(_, of)| arm(of)).map(|&(copy, _)| copy))
            .collect();
        for local in candidates {
            // A parameter's first value is its caller's, no write of the
            // body's: `undefined` for `None` where that's what it's given.
            if (1..=body.arg_count).contains(&local.as_usize()) {
                continue;
            }
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
}
