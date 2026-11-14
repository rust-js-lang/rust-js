//! What a closure captures is what it was made with (#1701, ADR 1137): a
//! value it captures by copy, or what a shared reference it captures
//! refers to, which borrowck holds unchanged while the closure lives, is
//! the interval its maker's walk knows where it makes it, joined over each
//! place it's made. Its makers are walked first, by how deep in closures
//! each is.

use super::*;
use rustc_span::def_id::DefId;

/// Each closure's captures, of `bodies`, into what `given` gives it: one
/// none of whose makers is walked before it, made where a closure as deep
/// or deeper is, an inlined copy, gets none.
pub(super) fn capture_intervals<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &IndexMap<DefId, &Body<'tcx>>,
    given: &mut IndexMap<DefId, Given>,
) {
    let depth = |mut id: DefId| {
        let mut depth = 0;
        while tcx.is_closure_like(id) {
            depth += 1;
            id = tcx.parent(id);
        }
        depth
    };
    // Where each closure is made, by the body that makes it.
    let mut made: IndexMap<DefId, Vec<(DefId, Location)>> = IndexMap::default();
    for (&maker, body) in bodies {
        for (block, data) in body.basic_blocks.iter_enumerated() {
            for (statement_index, statement) in data.statements.iter().enumerate() {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let Rvalue::Aggregate(kind, _) = &assign.1
                    && let mir::AggregateKind::Closure(closure, _) = **kind
                    && closure.is_local()
                {
                    made.entry(closure)
                        .or_default()
                        .push((maker, Location { block, statement_index }));
                }
            }
        }
    }
    let spoiled: IndexSet<DefId> = (made.iter())
        .filter(|&(&closure, sites)| sites.iter().any(|&(maker, _)| depth(maker) >= depth(closure)))
        .map(|(&closure, _)| closure)
        .collect();
    let mut makers: Vec<DefId> = (made.values().flatten())
        .map(|&(maker, _)| maker)
        .collect::<IndexSet<_>>()
        .into_iter()
        .collect();
    makers.sort_by_key(|&maker| depth(maker));
    let mut found: IndexMap<DefId, Vec<Option<Interval>>> = IndexMap::default();
    for maker in makers {
        let body = bodies[&maker];
        let sites: Vec<Location> = (made.iter())
            .flat_map(|(_, sites)| {
                sites
                    .iter()
                    .filter(|&&(at, _)| at == maker)
                    .map(|&(_, location)| location)
            })
            .collect();
        let typing_env = ty::TypingEnv::post_analysis(tcx, maker);
        let walk = walk(tcx, typing_env, body, given.get(&maker).cloned().unwrap_or_default());
        for (location, state) in sites.iter().zip(walk.at(&sites)) {
            let StatementKind::Assign(assign) =
                &body.basic_blocks[location.block].statements[location.statement_index].kind
            else {
                continue;
            };
            let Rvalue::Aggregate(kind, operands) = &assign.1 else {
                continue;
            };
            let mir::AggregateKind::Closure(closure, _) = **kind else {
                continue;
            };
            let captured: Vec<Option<Interval>> = match &state {
                Some(state) => operands
                    .iter()
                    .map(|operand| walk.captured_at(operand, state))
                    .collect(),
                None => vec![None; operands.len()],
            };
            match found.get_mut(&closure) {
                Some(known) => {
                    for (known, now) in known.iter_mut().zip(captured) {
                        *known = known.zip(now).map(|(a, b)| a.join(b));
                    }
                }
                None => {
                    found.insert(closure, captured);
                }
            }
        }
    }
    for (closure, captures) in found {
        if !spoiled.contains(&closure) {
            given.entry(closure).or_default().captures = captures;
        }
    }
}

impl<'tcx> Walk<'_, 'tcx> {
    /// What's known just before each of `locations`' statements: none of
    /// one no flow reaches.
    fn at(&self, locations: &[Location]) -> Vec<Option<State>> {
        let entry = self.entries();
        (locations.iter())
            .map(|location| {
                let mut state = entry[location.block].clone()?;
                for statement in &self.body.basic_blocks[location.block].statements[..location.statement_index] {
                    self.statement(statement, &mut state);
                }
                Some(state)
            })
            .collect()
    }

    /// What a closure made of `operand` captures of it: its value, or, of a
    /// shared reference to a local, the local's, which borrowck holds
    /// unchanged while the reference lives.
    fn captured_at(&self, operand: &Operand<'tcx>, state: &State) -> Option<Interval> {
        let local = operand.place()?.as_local()?;
        if self.range(self.body.local_decls[local].ty).is_some() {
            return self.operand(operand, state);
        }
        // The one write of a reference made where it's captured, `&n`.
        let mut writes = (self.body.basic_blocks.iter())
            .flat_map(|data| &data.statements)
            .filter_map(|statement| match &statement.kind {
                StatementKind::Assign(assign) if assign.0.as_local() == Some(local) => Some(&assign.1),
                _ => None,
            });
        let (Some(Rvalue::Ref(_, mir::BorrowKind::Shared, of)), None) = (writes.next(), writes.next()) else {
            return None;
        };
        self.operand(&Operand::Copy(of.as_local()?.into()), state)
    }

    /// Of a closure's body, what a read of `place` knows of a capture: its
    /// value's interval, by copy, or, by a shared reference, what it refers
    /// to, of one the body never writes. None of any other place.
    pub(super) fn capture(&self, place: &mir::Place<'tcx>) -> Option<Interval> {
        let def_id = self.body.source.def_id();
        if !self.tcx.is_closure_like(def_id) || place.local != Local::from_usize(1) {
            return None;
        }
        // `(*_1).k` of a closure the call lends, `_1.k` of one it's given,
        // and of a reference it captures, `*(*_1).k`: a shared one's alone
        // is given (`captured_at`).
        let (field, by_reference) = match place.projection.as_slice() {
            [mir::PlaceElem::Deref, mir::PlaceElem::Field(k, _)] | [mir::PlaceElem::Field(k, _)] => (*k, false),
            [
                mir::PlaceElem::Deref,
                mir::PlaceElem::Field(k, _),
                mir::PlaceElem::Deref,
            ]
            | [mir::PlaceElem::Field(k, _), mir::PlaceElem::Deref] => (*k, true),
            _ => return None,
        };
        if !by_reference && self.captures_written.contains(&field.index()) {
            return None;
        }
        *self.from_callers.captures.get(field.index())?
    }
}

/// The captures `body`, a closure's, writes, by their field's index: a
/// write of one, or a `&mut` or raw pointer of it, which may.
pub(super) fn captures_written(body: &Body<'_>) -> IndexSet<usize> {
    let field = |place: &mir::Place<'_>| {
        if place.local != Local::from_usize(1) {
            return None;
        }
        match place.projection.as_slice() {
            [mir::PlaceElem::Deref, mir::PlaceElem::Field(k, _), ..] | [mir::PlaceElem::Field(k, _), ..] => {
                Some(k.index())
            }
            _ => None,
        }
    };
    let mut written = IndexSet::default();
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            if let StatementKind::Assign(assign) = &statement.kind {
                written.extend(field(&assign.0));
                if let Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, place) | Rvalue::RawPtr(_, place) = &assign.1 {
                    written.extend(field(place));
                }
            }
        }
        if let TerminatorKind::Call { destination, .. } = &data.terminator().kind {
            written.extend(field(destination));
        }
    }
    written
}
