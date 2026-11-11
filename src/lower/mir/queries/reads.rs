//! Where a body's MIR reads and writes a local or a place, asked of the
//! body alone: whether it's only read once it's assigned, read again from a
//! block or past a statement, written before it's read, or read by a call
//! before what it's reached through is written (ADR 0853); and the places a
//! statement writes or lends to be written, and whether a write reaches a
//! place.

use super::*;

/// Whether `local` is only read, by its parts or lent shared, once it's
/// assigned: never moved, copied whole, nor lent `&mut`.
pub(in crate::lower::mir) fn only_read(body: &mir::Body<'_>, local: Local) -> bool {
    struct Reads {
        local: Local,
        only: bool,
    }
    impl<'tcx> Visitor<'tcx> for Reads {
        fn visit_place(&mut self, place: &Place<'tcx>, context: PlaceContext, _: mir::Location) {
            if place.local != self.local {
                return;
            }
            self.only &= match context {
                PlaceContext::NonUse(_) | PlaceContext::MutatingUse(MutatingUseContext::Store) => {
                    place.projection.is_empty()
                }
                PlaceContext::NonMutatingUse(
                    NonMutatingUseContext::SharedBorrow
                    | NonMutatingUseContext::FakeBorrow
                    | NonMutatingUseContext::Inspect
                    | NonMutatingUseContext::PlaceMention,
                ) => true,
                PlaceContext::NonMutatingUse(NonMutatingUseContext::Copy) => !place.projection.is_empty(),
                _ => false,
            };
        }
    }
    let mut reads = Reads { local, only: true };
    reads.visit_body(body);
    reads.only
}

/// Whether `local` is read in `from`, or a block it goes on to, before it's
/// assigned again: what it holds now is read there.
pub(in crate::lower::mir) fn read_from(body: &mir::Body<'_>, from: BasicBlock, local: Local) -> bool {
    read_in(body, from, 0, local)
}

/// Whether `local` is read past what's at `at`, a statement or its block's
/// terminator, before it's assigned again: what it holds there is read
/// again, in its block or a block it goes on to.
pub(in crate::lower::mir) fn read_past(body: &mir::Body<'_>, at: mir::Location, local: Local) -> bool {
    read_in(body, at.block, at.statement_index + 1, local)
}

/// Whether `local` is read in `from`, its statements from `start` on, its
/// terminator unless `start` is past it, or a block it goes on to, before
/// it's assigned again.
fn read_in(body: &mir::Body<'_>, from: BasicBlock, start: usize, local: Local) -> bool {
    /// The first thing a statement does with `local`: reads it, `Some(true)`,
    /// or assigns it, `Some(false)`.
    struct First {
        local: Local,
        found: Option<bool>,
    }
    impl<'tcx> Visitor<'tcx> for First {
        fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
            if local != self.local || self.found.is_some() {
                return;
            }
            self.found = match context {
                PlaceContext::MutatingUse(MutatingUseContext::Store | MutatingUseContext::Call) => Some(false),
                PlaceContext::NonUse(_) => None,
                _ => Some(true),
            };
        }
    }
    // Each block from its start, `from`'s too where a way goes back to it.
    let mut seen = IndexSet::default();
    let mut next = vec![(from, start)];
    'blocks: while let Some((block, start)) = next.pop() {
        if !seen.insert((block, start)) {
            continue;
        }
        let data = &body.basic_blocks[block];
        for (index, statement) in data.statements.iter().enumerate().skip(start) {
            let at = mir::Location {
                block,
                statement_index: index,
            };
            let mut first = First { local, found: None };
            // What's assigned is read first, then where it goes.
            match &statement.kind {
                StatementKind::Assign(assign) => {
                    first.visit_rvalue(&assign.1, at);
                    if first.found.is_none() && assign.0.local == local {
                        first.found = Some(!assign.0.projection.is_empty());
                    }
                }
                _ => first.visit_statement(statement, at),
            }
            match first.found {
                Some(true) => return true,
                Some(false) => continue 'blocks,
                None => {}
            }
        }
        if start <= data.statements.len() {
            let mut first = First { local, found: None };
            let at = mir::Location {
                block,
                statement_index: data.statements.len(),
            };
            first.visit_terminator(data.terminator(), at);
            match first.found {
                Some(true) => return true,
                Some(false) => continue,
                None => {}
            }
        }
        next.extend(data.terminator().successors().map(|b| (b, 0)));
    }
    false
}

/// Whether `root` may be written on a way from `at`, where `local` is
/// made, to a read of `local`: assigned, whole or in part, a call's result
/// put in it, or lent `&mut`, in a block that's both reached from `at` and
/// reaches a read (#121). A `mut` parameter assigned after its `match` is
/// written on none of its arms' ways.
pub(in crate::lower::mir) fn written_before_read(
    body: &mir::Body<'_>,
    at: mir::Location,
    root: Local,
    local: Local,
) -> bool {
    use mir::visit::{PlaceContext, Visitor};
    struct Uses {
        root: Local,
        local: Local,
        writes: Vec<mir::Location>,
        reads: Vec<mir::Location>,
    }
    impl<'tcx> Visitor<'tcx> for Uses {
        fn visit_local(&mut self, local: Local, context: PlaceContext, at: mir::Location) {
            match context {
                PlaceContext::MutatingUse(_) if local == self.root => self.writes.push(at),
                PlaceContext::NonMutatingUse(_) if local == self.local => self.reads.push(at),
                _ => {}
            }
        }
    }
    let mut uses = Uses {
        root,
        local,
        writes: Vec::new(),
        reads: Vec::new(),
    };
    uses.visit_body(body);
    // The blocks reached from `at`, and those that reach a read.
    let mut reached = IndexSet::default();
    let mut next: Vec<BasicBlock> = body.basic_blocks[at.block].terminator().successors().collect();
    while let Some(block) = next.pop() {
        if reached.insert(block) {
            next.extend(body.basic_blocks[block].terminator().successors());
        }
    }
    let mut reaching = IndexSet::default();
    let mut next: Vec<BasicBlock> = uses.reads.iter().map(|read| read.block).collect();
    while let Some(block) = next.pop() {
        if reaching.insert(block) {
            next.extend(body.basic_blocks.predecessors()[block].iter().copied());
        }
    }
    uses.writes.iter().any(|write| {
        let after = write.block == at.block && write.statement_index > at.statement_index;
        (after || reached.contains(&write.block)) && reaching.contains(&write.block)
    })
}

/// Whether a path from `at`, where `local` is assigned, writes `root`, or
/// what it points at, before it reads `local`: along each path until
/// `local` is assigned again, a loop's next turn, after which a read is of
/// another value.
pub(in crate::lower::mir) fn written_before_its_read(
    body: &mir::Body<'_>,
    at: mir::Location,
    root: Local,
    local: Local,
) -> bool {
    use mir::visit::{PlaceContext, Visitor};
    /// Of one statement or terminator: whether it reads `local`, and
    /// whether it writes `root`.
    struct Uses {
        root: Local,
        local: Local,
        reads: bool,
        writes: bool,
    }
    impl<'tcx> Visitor<'tcx> for Uses {
        fn visit_place(&mut self, place: &mir::Place<'tcx>, context: PlaceContext, at: mir::Location) {
            if place.local == self.root && matches!(context, PlaceContext::MutatingUse(_)) {
                self.writes = true;
            }
            self.super_place(place, context, at);
        }
        fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
            if local == self.local && matches!(context, PlaceContext::NonMutatingUse(_)) {
                self.reads = true;
            }
        }
    }
    let uses = |location: mir::Location| {
        let mut uses = Uses {
            root,
            local,
            reads: false,
            writes: false,
        };
        match body.stmt_at(location) {
            rustc_data_structures::either::Either::Left(statement) => uses.visit_statement(statement, location),
            rustc_data_structures::either::Either::Right(terminator) => uses.visit_terminator(terminator, location),
        }
        (uses.reads, uses.writes)
    };
    let mut seen = IndexSet::default();
    let mut next = vec![(at.block, at.statement_index + 1, false)];
    while let Some((block, from, mut written)) = next.pop() {
        let data = &body.basic_blocks[block];
        for statement_index in from..=data.statements.len() {
            let location = mir::Location { block, statement_index };
            if location == at {
                break;
            }
            let (reads, writes) = uses(location);
            if reads && written {
                return true;
            }
            written |= writes;
            if statement_index == data.statements.len() {
                for successor in data.terminator().successors() {
                    if seen.insert((successor, written)) {
                        next.push((successor, 0, written));
                    }
                }
            }
        }
    }
    false
}

/// Whether `place` is read through a shared reference a part of a value
/// holds, `*((*inner).s)` of a `&str` field: what it refers to borrowck
/// keeps the same, not the part that holds it, which may be written while
/// the reference is held.
pub(in crate::lower::mir) fn through_held_reference<'tcx>(
    body: &mir::Body<'tcx>,
    tcx: ty::TyCtxt<'tcx>,
    place: Place<'tcx>,
) -> bool {
    (place.iter_projections()).any(|(base, elem)| {
        elem == PlaceElem::Deref
            && !base.projection.is_empty()
            && matches!(base.ty(body, tcx).ty.kind(), ty::Ref(_, _, ty::Mutability::Not))
    })
}

/// Whether `local` is assigned in `from`, or a block it goes on to.
pub(in crate::lower::mir) fn written_from(body: &mir::Body<'_>, from: BasicBlock, local: Local) -> bool {
    let mut seen = IndexSet::default();
    let mut next = vec![from];
    while let Some(block) = next.pop() {
        if !seen.insert(block) {
            continue;
        }
        let data = &body.basic_blocks[block];
        let writes = (data.statements.iter())
            .any(|s| matches!(&s.kind, StatementKind::Assign(a) if a.0.as_local() == Some(local)))
            || matches!(&data.terminator().kind, TerminatorKind::Call { destination, .. } if destination.as_local() == Some(local));
        if writes {
            return true;
        }
        next.extend(data.terminator().successors());
    }
    false
}

/// Whether what the statement at `at` makes, a temporary MIR reads once, a
/// `&(*node).v` through a reference written again, is read by `takes`, a
/// call of what keeps nothing of it, before what it's reached through is
/// written, every way from `at` (ADR 0853): its place, read there, is the
/// one it was, so nothing need hold what that is meanwhile. What's written
/// there is MIR's assignment of a place `written` says reaches it
/// (`writes_reach`): borrowck lets no `&mut` to it, a closure's capture of
/// it, write it between, as it's read before.
pub(in crate::lower::mir) fn read_before_written<'tcx>(
    state: &State<'_, 'tcx>,
    (at, rvalue): (mir::Location, &Rvalue<'tcx>),
    takes: impl Fn(&mir::Terminator<'tcx>, Local) -> bool,
    written: impl Fn(&Place<'tcx>) -> bool,
) -> bool {
    /// Whether a statement or terminator names `local`.
    struct Names {
        local: Local,
        found: bool,
    }
    impl<'tcx> Visitor<'tcx> for Names {
        fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
            self.found |= local == self.local && !matches!(context, PlaceContext::NonUse(_));
        }
    }
    let body = state.body;
    let Some(StatementKind::Assign(assign)) = (body.basic_blocks[at.block].statements)
        .get(at.statement_index)
        .map(|s| &s.kind)
    else {
        return false;
    };
    let made = assign.0;
    // Of the statement being lowered: not a cleanup's, say, lowered apart.
    if !std::ptr::addr_eq(&assign.1, rvalue)
        || !made.projection.is_empty()
        || state.locals.user[made.local]
        || state.locals.reads[made.local] != 1
    {
        return false;
    }
    let names = |visit: &dyn Fn(&mut Names)| {
        let mut names = Names {
            local: made.local,
            found: false,
        };
        visit(&mut names);
        names.found
    };
    let mut seen = IndexSet::default();
    let mut next = vec![(at.block, at.statement_index + 1)];
    while let Some((block, from)) = next.pop() {
        if !seen.insert((block, from)) {
            continue;
        }
        let data = &body.basic_blocks[block];
        for (index, statement) in data.statements.iter().enumerate().skip(from) {
            let here = mir::Location {
                block,
                statement_index: index,
            };
            if names(&|n| n.visit_statement(statement, here)) {
                return false;
            }
            if written_or_lent(statement).any(&written) {
                return false;
            }
        }
        let terminator = data.terminator();
        let here = mir::Location {
            block,
            statement_index: data.statements.len(),
        };
        if names(&|n| n.visit_terminator(terminator, here)) {
            if takes(terminator, made.local) {
                continue;
            }
            return false;
        }
        if let TerminatorKind::Call { destination, .. } = &terminator.kind
            && written(destination)
        {
            return false;
        }
        next.extend(terminator.successors().map(|b| (b, 0)));
    }
    true
}

/// The places `statement` writes, or lends to be written: an assignment's
/// place, and what a `&mut` or a `*mut` it makes is of, `&mut node.0`
/// that a closure's capture writes, `|| node.0 = b` (#1034).
pub(in crate::lower::mir) fn written_or_lent<'a, 'tcx>(
    statement: &'a mir::Statement<'tcx>,
) -> impl Iterator<Item = &'a Place<'tcx>> {
    let StatementKind::Assign(assign) = &statement.kind else {
        return None.into_iter().flatten();
    };
    let lent = match &assign.1 {
        Rvalue::Ref(_, BorrowKind::Mut { .. }, lent) | Rvalue::RawPtr(mir::RawPtrKind::Mut, lent) => Some(lent),
        _ => None,
    };
    Some(std::iter::once(&assign.0).chain(lent)).into_iter().flatten()
}

/// Whether writing `written` changes what `place` is: it's the place or
/// what holds it, `node` or `node.0` of `(*node.0).v`, a box's content of
/// `**b`, or an index it's at, `i` of `v[i]` (#1034).
pub(in crate::lower::mir) fn writes_reach<'tcx>(place: &Place<'tcx>, written: &Place<'tcx>) -> bool {
    (written.local == place.local && crate::lower::mir::rvalues::within(place.projection, written.projection))
        || (written.projection.is_empty() && place.projection.contains(&PlaceElem::Index(written.local)))
}
