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
                &walk.body.basic_blocks[location.block].statements[location.statement_index].kind
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

/// Of each closure of `bodies` whose result, at each return, is an
/// `Option` of a slice's `get(index)` of a capture it refers to, the
/// capture's field and the most an index below that slice's length is:
/// `targets.get(i).cloned()` of `KEY.with_borrow(|targets| ..)`, whose
/// `Some` knows its caller's `i` below a length (#1701 C).
pub(super) fn closure_results<'tcx>(
    tcx: TyCtxt<'tcx>,
    bodies: &IndexMap<DefId, &Body<'tcx>>,
) -> IndexMap<DefId, (usize, i128)> {
    let mut results = IndexMap::default();
    for (&closure, body) in bodies {
        if !tcx.is_closure_like(closure) {
            continue;
        }
        let returns: IndexSet<BasicBlock> = (body.basic_blocks.iter_enumerated())
            .filter(|(_, data)| matches!(data.terminator().kind, TerminatorKind::Return))
            .map(|(block, _)| block)
            .collect();
        let typing_env = ty::TypingEnv::post_analysis(tcx, closure);
        let walk = walk(tcx, typing_env, body, Given::default());
        let ends = walk.ends(&returns);
        // The capture each return's `Some` knows below a length, one at
        // every return, and the most of them.
        let mut found: Option<(usize, i128)> = None;
        let mut every = !returns.is_empty();
        for block in &returns {
            let known = (ends.get(block))
                .and_then(|state| state.gets.get(&mir::RETURN_PLACE))
                .and_then(|&(index, most)| {
                    let field = walk.captured.iter().find(|&&(_, local)| local == index)?.0;
                    Some((field, most))
                });
            match (known, found) {
                (Some(now), None) => found = Some(now),
                (Some((field, most)), Some((was, before))) if field == was => found = Some((field, most.max(before))),
                _ => every = false,
            }
        }
        if every && let Some(result) = found {
            results.insert(closure, result);
        }
    }
    results
}

impl<'tcx> Walk<'_, 'tcx> {
    /// What `of` is assigned, and where, of a local assigned once in this
    /// body.
    pub(super) fn made(&self, of: Local) -> Option<(Location, &Rvalue<'tcx>)> {
        let mut writes = (self.body.basic_blocks.iter_enumerated()).flat_map(|(block, data)| {
            (data.statements.iter().enumerate()).filter_map(move |(statement_index, statement)| match &statement.kind {
                StatementKind::Assign(assign) if assign.0.as_local() == Some(of) => {
                    Some((Location { block, statement_index }, &assign.1))
                }
                _ => None,
            })
        });
        match (writes.next(), writes.next()) {
            (Some(made), None) => Some(made),
            _ => None,
        }
    }

    /// The local `operand`, a shared reference made once in this body,
    /// refers to: `i` of `&i`.
    fn referred(&self, operand: &Operand<'tcx>) -> Option<Local> {
        let (_, Rvalue::Ref(_, mir::BorrowKind::Shared, of)) = self.made(operand.place()?.as_local()?)? else {
            return None;
        };
        of.as_local()
    }

    /// The closure `local` holds, made once in this body: which, and the
    /// local each capture refers to, by field: by a shared reference, which
    /// borrowck holds unchanged while the closure lives; or by copy, of a
    /// closure made in the block whose call it's given, the local not
    /// written after it's made, so the call sees what it captured.
    pub(super) fn made_closure(&self, local: Local) -> Option<(DefId, Vec<Option<Local>>)> {
        let (at, Rvalue::Aggregate(kind, operands)) = self.made(local)? else {
            return None;
        };
        let mir::AggregateKind::Closure(closure, _) = **kind else {
            return None;
        };
        let data = &self.body.basic_blocks[at.block];
        let given_here = matches!(&data.terminator().kind, TerminatorKind::Call { args, .. }
            if args.iter().any(|arg| arg.node.place().and_then(|p| p.as_local()) == Some(local)));
        let written_after = |of: Local| {
            data.statements[at.statement_index + 1..].iter().any(|statement| {
                matches!(&statement.kind, StatementKind::Assign(assign)
                    if assign.0.local == of
                        || matches!(&assign.1, Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, place) | Rvalue::RawPtr(_, place) if place.local == of))
            })
        };
        let copied = |operand: &Operand<'tcx>| {
            let of = operand.place()?.as_local()?;
            (given_here && self.range(self.body.local_decls[of].ty).is_some() && !written_after(of)).then_some(of)
        };
        Some((
            closure,
            operands
                .iter()
                .map(|operand| self.referred(operand).or_else(|| copied(operand)))
                .collect(),
        ))
    }

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
        // A reference made where it's captured, `&n`.
        self.operand(&Operand::Copy(self.referred(operand)?.into()), state)
    }
}

/// The field a place of a closure reads, and whether it reads it through
/// it, a reference: none of one that reads the closure whole.
type FieldOf<'a, 'tcx> = &'a dyn Fn(&[mir::PlaceElem<'tcx>]) -> Option<(usize, bool)>;

/// A closure's body with each of its captures a local of its own (#1701
/// B): what it captures by a reference only ever read through, the place
/// it refers to, `*(*_1).k`; any other, the field, `(*_1).k`. The walk's
/// rules of a local are then the capture's: a test narrows it, a copy is
/// one of it, and a `&mut` of it lends it (#1782). Its blocks and
/// statements are the body's, each where it was. None of a body that isn't
/// a closure's, or that reads itself whole, `&*_1` given a call, which
/// may reach any capture. With it, each capture given what it was made
/// with, by field: one by a shared reference, and one by copy its body
/// never writes.
pub(super) fn as_locals<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> Option<(Body<'tcx>, Vec<(usize, Local)>)> {
    use rustc_middle::mir::visit::{MutVisitor, PlaceContext, Visitor};
    const ENV: Local = Local::from_u32(1);
    if !tcx.is_closure_like(body.source.def_id()) || body.arg_count == 0 {
        return None;
    }
    let env = body.local_decls[ENV].ty;
    let by_reference = env.builtin_deref(true).is_some();
    let ty::Closure(_, args) = *env.builtin_deref(true).unwrap_or(env).kind() else {
        return None;
    };
    let upvars = args.as_closure().upvar_tys();
    // The field a place of the closure reads, and whether it reads it
    // through it, a reference: none of one that reads the closure whole.
    let field = |projection: &[mir::PlaceElem<'tcx>]| -> Option<(usize, bool)> {
        let rest = match (by_reference, projection) {
            (true, [mir::PlaceElem::Deref, rest @ ..]) | (false, rest) => rest,
            _ => return None,
        };
        match rest {
            [mir::PlaceElem::Field(k, _), then @ ..] => {
                Some((k.index(), matches!(then.first(), Some(mir::PlaceElem::Deref))))
            }
            _ => None,
        }
    };
    struct Uses<'a, 'tcx> {
        field: FieldOf<'a, 'tcx>,
        whole: bool,
        itself: IndexSet<usize>,
    }
    impl<'tcx> Visitor<'tcx> for Uses<'_, 'tcx> {
        fn visit_place(&mut self, place: &mir::Place<'tcx>, _: PlaceContext, _: Location) {
            if place.local != ENV {
                return;
            }
            match (self.field)(place.projection.as_slice()) {
                Some((k, false)) => {
                    self.itself.insert(k);
                }
                Some((_, true)) => {}
                None => self.whole = true,
            }
        }
    }
    let mut uses = Uses {
        field: &field,
        whole: false,
        itself: IndexSet::default(),
    };
    uses.visit_body(body);
    if uses.whole {
        return None;
    }
    let written = captures_written(body);
    let mut rewritten = body.clone();
    // Each field's local, and whether it's what its reference refers to.
    let mut locals: Vec<(Local, bool)> = Vec::new();
    let mut seeded: Vec<(usize, Local)> = Vec::new();
    for (k, upvar) in upvars.iter().enumerate() {
        // A reference's alone: what a raw pointer or a `Box` it captures
        // points at other code may reach.
        let referent = match *upvar.kind() {
            ty::Ref(_, to, _) if !uses.itself.contains(&k) => Some(to),
            _ => None,
        };
        let local = rewritten
            .local_decls
            .push(mir::LocalDecl::new(referent.unwrap_or(upvar), body.span));
        locals.push((local, referent.is_some()));
        // A `&mut` one's referent is given nothing where it's made
        // (`captured_at`).
        if referent.is_some() || ty_is_value(upvar) && !written.contains(&k) {
            seeded.push((k, local));
        }
    }
    struct Rewrite<'a, 'tcx> {
        tcx: TyCtxt<'tcx>,
        field: FieldOf<'a, 'tcx>,
        locals: &'a [(Local, bool)],
        by_reference: bool,
    }
    impl<'tcx> MutVisitor<'tcx> for Rewrite<'_, 'tcx> {
        fn tcx(&self) -> TyCtxt<'tcx> {
            self.tcx
        }
        fn visit_place(&mut self, place: &mut mir::Place<'tcx>, _: PlaceContext, _: Location) {
            if place.local != ENV {
                return;
            }
            let Some((k, _)) = (self.field)(place.projection.as_slice()) else {
                return;
            };
            let (local, referent) = self.locals[k];
            // Past the closure's deref, if a reference, and the field; and
            // past the field's deref of a referent.
            let skip = usize::from(self.by_reference) + 1 + usize::from(referent);
            *place = mir::Place {
                local,
                projection: self.tcx.mk_place_elems(&place.projection[skip..]),
            };
        }
    }
    Rewrite {
        tcx,
        field: &field,
        locals: &locals,
        by_reference,
    }
    .visit_body(&mut rewritten);
    Some((rewritten, seeded))
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

/// Whether `ty` is a value a capture by copy holds itself, not a pointer
/// to one.
fn ty_is_value(ty: Ty<'_>) -> bool {
    ty.builtin_deref(true).is_none()
}
