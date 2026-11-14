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
