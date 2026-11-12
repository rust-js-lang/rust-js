//! The writes over a place a raw pointer is taken to or into (#1484): a
//! local or a field of one, written whole after its first write, is
//! written over what it holds, `$assign(p, v)`, so its object, the place
//! its pointers are of, stays the one it is, as Rust's place does (ADR
//! 0562, amended).

use super::pointed_locals_of;
use crate::collections::{IndexMap, IndexSet};
use crate::lower::FnCx;
use rustc_data_structures::either::Either;
use rustc_middle::mir::visit::Visitor;
use rustc_middle::mir::{self, Local, PlaceElem, Rvalue, StatementKind};
use rustc_middle::ty;
use rustc_mir_dataflow::Analysis;
use rustc_mir_dataflow::impls::{MaybeInitializedPlaces, MaybeUninitializedPlaces};
use rustc_mir_dataflow::move_paths::{LookupResult, MoveData};

/// Whether `place` is a local or reached from it by its fields and its
/// items, `s.f` or `a[0].f`: no `Deref`, nothing a pointer reads through.
pub(in crate::lower) fn within_items(place: &mir::Place<'_>) -> bool {
    (place.projection.iter()).all(|elem| {
        matches!(
            elem,
            PlaceElem::Field(..) | PlaceElem::Index(_) | PlaceElem::ConstantIndex { .. }
        )
    })
}

/// Whether `inner` is `outer` or reached from it: the same fields, and an
/// item any item, as which one an index is isn't known here; a write in
/// place of one that isn't is still over what it holds.
fn reaches_into<'tcx>(inner: &[PlaceElem<'tcx>], outer: &[PlaceElem<'tcx>]) -> bool {
    let item = |elem: &PlaceElem<'_>| matches!(elem, PlaceElem::Index(_) | PlaceElem::ConstantIndex { .. });
    inner.len() >= outer.len() && inner.iter().zip(outer).all(|(a, b)| a == b || (item(a) && item(b)))
}

/// How a local a raw pointer is taken to the whole of, an object JS holds
/// as itself, is written where it's assigned whole (#1484). Rust writes the
/// same place each time, so its pointers, its fields' too, keep reading it;
/// JS's `p = v` would make the variable another object, at another address.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Rewritten {
    /// Over what it holds, `$assign(p, v)`, where it surely holds a value:
    /// the object is the place.
    InPlace,
    /// Made, `p = v`, where it surely holds none and no pointer to it can
    /// have been made: its first.
    Made,
    /// Where it may hold one or not, or holds none after a move with a
    /// pointer to it made: neither would be Rust's place.
    Refused,
}

/// Each place `body` writes whole, a local or a field of one, `p = v` or
/// `s.f = v`, that a raw pointer is taken to or into, `&raw const s.f` or
/// `&raw const s.f.x`: each write of it after its first is over what it
/// holds in Rust, so its pointers keep reading it (#1484). The one answer
/// of which places #1484's writes are of, which the lowering writes
/// (`rewritten`) and the crate's analysis of what changes in place reads
/// (`mutation.rs`), by where each write is. With each, the places inside
/// it a pointer is taken to or into, `s.f` of `&raw const s.f.b` when `s`
/// is written, deepest first: each an object written over in turn, kept
/// where the write's value would put another.
pub(in crate::lower) fn rewritten_places<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
) -> Vec<(mir::Location, mir::Place<'tcx>, Vec<mir::Place<'tcx>>)> {
    let mut pointed: Vec<mir::Place<'tcx>> = (pointed_locals_of(body, IndexSet::default()).locals.into_iter())
        .map(mir::Place::from)
        .collect();
    for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
        if let StatementKind::Assign(assign) = &statement.kind
            && let Rvalue::RawPtr(kind, of) = &assign.1
            && *kind != mir::RawPtrKind::FakeForPtrMetadata
            && !of.projection.is_empty()
            && within_items(of)
        {
            pointed.push(*of);
        }
    }
    let again = assigned_again(body);
    let mut rewritten = Vec::new();
    for (at, written) in whole_writes(body) {
        let within: Vec<_> = (pointed.iter())
            .filter(|p| p.local == written.local && reaches_into(p.projection, written.projection))
            .collect();
        // A local's first write makes it; a field's is over what the local
        // was made with.
        if within.is_empty() || (written.projection.is_empty() && !again.contains(&written.local)) {
            continue;
        }
        let mut kept: Vec<mir::Place<'tcx>> = Vec::new();
        for p in within {
            // The write's own place, its index the write's, and what the
            // pointer reaches past it.
            for end in written.projection.len() + 1..=p.projection.len() {
                let projection: Vec<PlaceElem<'tcx>> = (written.projection.iter())
                    .chain(p.projection[written.projection.len()..end].iter().copied())
                    .collect();
                let inner = mir::Place {
                    local: p.local,
                    projection: tcx.mk_place_elems(&projection),
                };
                if !kept.contains(&inner) {
                    kept.push(inner);
                }
            }
        }
        kept.sort_by_key(|inner| std::cmp::Reverse(inner.projection.len()));
        rewritten.push((at, written, kept));
    }
    rewritten
}

/// How each of `writes` in `body` is written (`Rewritten`), by rustc's
/// dataflow of what's there before it.
pub(in crate::lower) fn rewritten<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    writes: Vec<(mir::Location, mir::Place<'tcx>, Vec<mir::Place<'tcx>>)>,
) -> IndexMap<mir::Location, (Rewritten, Vec<mir::Place<'tcx>>)> {
    let mut rewritten = IndexMap::default();
    if writes.is_empty() {
        return rewritten;
    }
    let move_data = MoveData::gather_moves(body, tcx, |_| true);
    let mut inits = MaybeInitializedPlaces::new(tcx, body, &move_data)
        .iterate_to_fixpoint(tcx, body, None)
        .into_results_cursor(body);
    let mut uninits = MaybeUninitializedPlaces::new(tcx, body, &move_data)
        .iterate_to_fixpoint(tcx, body, None)
        .into_results_cursor(body);
    for (at, written, kept) in writes {
        let path = match move_data.rev_lookup.find(written.as_ref()) {
            LookupResult::Exact(path) | LookupResult::Parent(Some(path)) => path,
            LookupResult::Parent(None) => continue,
        };
        inits.seek_before_primary_effect(at);
        uninits.seek_before_primary_effect(at);
        let (init, uninit) = (inits.get().contains(path), uninits.get().contains(path));
        // What a `Drop` left, `d = v` of a value with drop glue, rustc's
        // drop first: the object nothing else holds, written over.
        let how = match (init, uninit) {
            (true, false) => Rewritten::InPlace,
            (false, true) if !reaches(body, written, at, pointer_made) => Rewritten::Made,
            (false, true) if !reaches(body, written, at, moved_out) => Rewritten::InPlace,
            _ => Rewritten::Refused,
        };
        rewritten.insert(at, (how, kept));
    }
    rewritten
}

/// Each write of a whole local or a field of one in `body`, by where it
/// is: an assignment's, `p = v`, and a call's result's, `p = f()`.
fn whole_writes<'tcx>(body: &mir::Body<'tcx>) -> Vec<(mir::Location, mir::Place<'tcx>)> {
    let mut writes = Vec::new();
    for (block, data) in body.basic_blocks.iter_enumerated() {
        for (statement_index, statement) in data.statements.iter().enumerate() {
            if let StatementKind::Assign(assign) = &statement.kind
                && within_items(&assign.0)
            {
                writes.push((mir::Location { block, statement_index }, assign.0));
            }
        }
        if let mir::TerminatorKind::Call { destination, .. } = &data.terminator().kind
            && within_items(destination)
        {
            writes.push((body.terminator_loc(block), *destination));
        }
    }
    writes
}

/// Whether a pointer or a reference to `place` or into it is made at `at`.
fn pointer_made<'tcx>(body: &mir::Body<'tcx>, place: mir::Place<'tcx>, at: mir::Location) -> bool {
    matches!(body.stmt_at(at), Either::Left(statement) if matches!(&statement.kind, StatementKind::Assign(assign)
        if matches!(&assign.1, Rvalue::RawPtr(_, of) | Rvalue::Ref(_, _, of)
            if of.local == place.local && reaches_into(of.projection, place.projection))))
}

/// Whether `place`, or what holds it, is moved out at `at`, `let kept = m;`.
fn moved_out<'tcx>(body: &mir::Body<'tcx>, place: mir::Place<'tcx>, at: mir::Location) -> bool {
    struct Moves<'tcx>(mir::Place<'tcx>, bool);
    impl<'tcx> Visitor<'tcx> for Moves<'tcx> {
        fn visit_operand(&mut self, operand: &mir::Operand<'tcx>, _: mir::Location) {
            if let mir::Operand::Move(moved) = operand
                && moved.local == self.0.local
                && (reaches_into(self.0.projection, moved.projection)
                    || reaches_into(moved.projection, self.0.projection))
            {
                self.1 = true;
            }
        }
    }
    let mut moves = Moves(place, false);
    match body.stmt_at(at) {
        Either::Left(statement) => moves.visit_statement(statement, at),
        Either::Right(terminator) => moves.visit_terminator(terminator, at),
    }
    moves.1
}

/// Whether `found` holds at some location that may run before `at` in
/// the local's same life: not before its `StorageLive`, where a loop's
/// `for r in ..` makes its binding a new local each time round (#1484).
fn reaches<'tcx>(
    body: &mir::Body<'tcx>,
    place: mir::Place<'tcx>,
    at: mir::Location,
    found: fn(&mir::Body<'tcx>, mir::Place<'tcx>, mir::Location) -> bool,
) -> bool {
    // Whether it's found in `block` before `end`, looking back; and
    // whether the local's life begins there, which ends the search.
    let in_block = |block: mir::BasicBlock, end: usize| -> (bool, bool) {
        for statement_index in (0..end).rev() {
            let location = mir::Location { block, statement_index };
            if found(body, place, location) {
                return (true, true);
            }
            if let Either::Left(statement) = body.stmt_at(location)
                && matches!(statement.kind, StatementKind::StorageLive(local) if local == place.local)
            {
                return (false, true);
            }
        }
        (false, false)
    };
    if let (found, true) = in_block(at.block, at.statement_index) {
        return found;
    }
    let mut stack: Vec<mir::BasicBlock> = (body.basic_blocks.predecessors()[at.block]).to_vec();
    let mut seen = IndexSet::default();
    while let Some(block) = stack.pop() {
        if !seen.insert(block) {
            continue;
        }
        match in_block(block, body.basic_blocks[block].statements.len() + 1) {
            (true, _) => return true,
            (false, true) => continue,
            (false, false) => stack.extend(body.basic_blocks.predecessors()[block].iter().copied()),
        }
    }
    false
}

/// The locals `body` assigns whole more than once, `p = P { .. }` after
/// `let mut p = ..`, or in a block a loop runs again.
fn assigned_again(body: &mir::Body<'_>) -> IndexSet<Local> {
    let mut seen = IndexSet::default();
    let mut again = IndexSet::default();
    for (at, written) in whole_writes(body) {
        if written.projection.is_empty() && (!seen.insert(written.local) || in_a_loop(body, at.block)) {
            again.insert(written.local);
        }
    }
    again
}

/// Whether `block` can be reached again from itself.
fn in_a_loop(body: &mir::Body<'_>, block: mir::BasicBlock) -> bool {
    let mut stack: Vec<mir::BasicBlock> = body.basic_blocks[block].terminator().successors().collect();
    let mut seen = IndexSet::default();
    while let Some(next) = stack.pop() {
        if next == block {
            return true;
        }
        if seen.insert(next) {
            stack.extend(body.basic_blocks[next].terminator().successors());
        }
    }
    false
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `rewritten` of `body`'s places `rewritten_places` says, each an object
    /// JS holds as itself: a local that isn't is a cell already
    /// (`pointed_locals`), and a field's a property its pointer is on.
    pub(in crate::lower) fn rewritten_writes(
        &self,
        body: &mir::Body<'tcx>,
    ) -> IndexMap<mir::Location, (Rewritten, Vec<mir::Place<'tcx>>)> {
        let object = |place: &mir::Place<'tcx>| {
            let ty = place.ty(&body.local_decls, self.tcx).ty;
            self.is_object(ty) && !self.mut_ref_form(ty).is_cell()
        };
        let objects: Vec<_> = (rewritten_places(self.tcx, body).into_iter())
            .filter(|(_, written, _)| object(written))
            .map(|(at, written, kept)| (at, written, kept.into_iter().filter(object).collect()))
            .collect();
        rewritten(self.tcx, body, objects)
    }
}
