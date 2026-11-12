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
/// items, `s.f` or `a[0].f`, and the contents of each `Box` so reached,
/// `(*s.b).f`, which are the `Box`'s own as a local's fields are its own:
/// nothing a pointer or a reference reads through.
pub(in crate::lower) fn within_items<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    place: &mir::Place<'tcx>,
) -> bool {
    (place.iter_projections()).all(|(base, elem)| match elem {
        PlaceElem::Deref => base.ty(body, tcx).ty.is_box(),
        _ => a_field_or_an_item(&elem),
    })
}

/// A step to a field or an item, `.f` or `[i]`.
fn a_field_or_an_item(elem: &PlaceElem<'_>) -> bool {
    matches!(
        elem,
        PlaceElem::Field(..) | PlaceElem::Index(_) | PlaceElem::ConstantIndex { .. }
    )
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
    /// Where it may hold one or not: over what it holds where it holds an
    /// object, as a move copies (`moved_apart_places`), and made where it
    /// holds none, `undefined`, its first (#1500).
    Tested,
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
    let pointed = pointed_places(tcx, body);
    let again = assigned_again(tcx, body);
    let mut rewritten = Vec::new();
    for (at, written) in whole_writes(tcx, body) {
        // A local's first write makes it; a field's is over what the local
        // was made with.
        let Some(kept) = kept_within(tcx, &pointed, written) else {
            continue;
        };
        if written.projection.is_empty() && !again.contains(&written.local) {
            continue;
        }
        rewritten.push((at, written, kept));
    }
    rewritten
}

/// Each place `body` moves out, `let kept = m;` or `f(m)`, that a raw
/// pointer is taken to or into, by where it's moved. The value moved is its
/// own allocation, at another address, as each live local is (#1535), so
/// it's a copy, and the place moved out keeps its object, which a write
/// after the move is over (#1500).
pub(in crate::lower) fn moved_apart_places<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
) -> IndexSet<(mir::Location, mir::Place<'tcx>)> {
    struct Moves<'a, 'tcx>(
        ty::TyCtxt<'tcx>,
        &'a mir::Body<'tcx>,
        Vec<(mir::Location, mir::Place<'tcx>)>,
    );
    impl<'tcx> Visitor<'tcx> for Moves<'_, 'tcx> {
        fn visit_operand(&mut self, operand: &mir::Operand<'tcx>, at: mir::Location) {
            if let mir::Operand::Move(moved) = operand
                && within_items(self.0, self.1, moved)
            {
                self.2.push((at, *moved));
            }
        }
    }
    let pointed = pointed_places(tcx, body);
    let mut moves = Moves(tcx, body, Vec::new());
    moves.visit_body(body);
    (moves.2.into_iter())
        .filter(|(_, moved)| {
            (pointed.iter()).any(|p| p.local == moved.local && reaches_into(p.projection, moved.projection))
        })
        .collect()
}

/// The places a raw pointer is taken to the whole of, the locals
/// `pointed_locals_of` names, or into, `&raw const s.f`.
fn pointed_places<'tcx>(tcx: ty::TyCtxt<'tcx>, body: &mir::Body<'tcx>) -> Vec<mir::Place<'tcx>> {
    let mut pointed: Vec<mir::Place<'tcx>> = (pointed_locals_of(body, IndexSet::default()).locals.into_iter())
        .map(mir::Place::from)
        .collect();
    for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
        if let StatementKind::Assign(assign) = &statement.kind
            && let Rvalue::RawPtr(kind, of) = &assign.1
            && *kind != mir::RawPtrKind::FakeForPtrMetadata
            && !of.projection.is_empty()
            && within_items(tcx, body, of)
        {
            pointed.push(*of);
        }
    }
    pointed
}

/// Of a `place` one of `pointed` is or is inside, the places inside it
/// a pointer is into, deepest first: each an object a write over `place`
/// keeps. None where no pointer is to or into it.
fn kept_within<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    pointed: &[mir::Place<'tcx>],
    place: mir::Place<'tcx>,
) -> Option<Vec<mir::Place<'tcx>>> {
    let within: Vec<_> = (pointed.iter())
        .filter(|p| p.local == place.local && reaches_into(p.projection, place.projection))
        .collect();
    if within.is_empty() {
        return None;
    }
    let mut kept: Vec<mir::Place<'tcx>> = Vec::new();
    for p in within {
        // The place's own, its index the place's, and what the pointer
        // reaches past it.
        for end in place.projection.len() + 1..=p.projection.len() {
            let projection: Vec<PlaceElem<'tcx>> = (place.projection.iter())
                .chain(p.projection[place.projection.len()..end].iter().copied())
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
    Some(kept)
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
            // Unmade on some way here, `let mut d; if c { d = .. }`, though
            // rustc's drop of what it held before the write, which drops
            // only what's there, leaves it unmade on each: tested (#1500).
            _ if unmade_on_some_way(body, written, at) => Rewritten::Tested,
            // Moved out or dropped since a pointer to it was made, made on
            // each way: a move copied it (`moved_apart_places`), so it holds
            // its object still (#1500).
            _ => Rewritten::InPlace,
        };
        rewritten.insert(at, (how, kept));
    }
    rewritten
}

/// Each write of a whole local or a field of one in `body`, by where it
/// is: an assignment's, `p = v`, and a call's result's, `p = f()`.
fn whole_writes<'tcx>(tcx: ty::TyCtxt<'tcx>, body: &mir::Body<'tcx>) -> Vec<(mir::Location, mir::Place<'tcx>)> {
    let mut writes = Vec::new();
    for (block, data) in body.basic_blocks.iter_enumerated() {
        for (statement_index, statement) in data.statements.iter().enumerate() {
            if let StatementKind::Assign(assign) = &statement.kind
                && within_items(tcx, body, &assign.0)
            {
                writes.push((mir::Location { block, statement_index }, assign.0));
            }
        }
        if let mir::TerminatorKind::Call { destination, .. } = &data.terminator().kind
            && within_items(tcx, body, destination)
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

/// Whether `place` may be unmade at `at`: some way here from the start of
/// its local's life, its `StorageLive` or the function's start, writes it
/// nowhere, `let m; if c { m = .. }`. A parameter is made at the start.
fn unmade_on_some_way<'tcx>(body: &mir::Body<'tcx>, place: mir::Place<'tcx>, at: mir::Location) -> bool {
    let makes = |location: mir::Location| match body.stmt_at(location) {
        Either::Left(statement) => matches!(&statement.kind, StatementKind::Assign(assign)
            if assign.0.local == place.local && reaches_into(place.projection, assign.0.projection)),
        Either::Right(terminator) => matches!(&terminator.kind, mir::TerminatorKind::Call { destination, .. }
            if destination.local == place.local && reaches_into(place.projection, destination.projection)),
    };
    // Whether a way back from `end` in `block` is made, unmade, or goes on.
    let in_block = |block: mir::BasicBlock, end: usize| -> Option<bool> {
        for statement_index in (0..end).rev() {
            let location = mir::Location { block, statement_index };
            if makes(location) {
                return Some(false);
            }
            if let Either::Left(statement) = body.stmt_at(location)
                && matches!(statement.kind, StatementKind::StorageLive(local) if local == place.local)
            {
                return Some(true);
            }
        }
        None
    };
    let parameter = place.local.as_usize() <= body.arg_count;
    let mut stack = vec![(at.block, at.statement_index)];
    let mut seen = IndexSet::default();
    while let Some((block, end)) = stack.pop() {
        match in_block(block, end) {
            Some(true) => return true,
            Some(false) => continue,
            None => {
                let before = &body.basic_blocks.predecessors()[block];
                if before.is_empty() && !parameter {
                    return true;
                }
                for &earlier in before {
                    if seen.insert(earlier) {
                        stack.push((earlier, body.basic_blocks[earlier].statements.len() + 1));
                    }
                }
            }
        }
    }
    false
}

/// The locals `body` assigns whole more than once, `p = P { .. }` after
/// `let mut p = ..`, or in a block a loop runs again; or once, of a
/// parameter, made where it's given.
fn assigned_again<'tcx>(tcx: ty::TyCtxt<'tcx>, body: &mir::Body<'tcx>) -> IndexSet<Local> {
    let mut seen: IndexSet<Local> = body.args_iter().collect();
    let mut again = IndexSet::default();
    for (at, written) in whole_writes(tcx, body) {
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

    /// `moved_apart_places` of objects JS holds as themselves.
    pub(in crate::lower) fn moved_apart_moves(
        &self,
        body: &mir::Body<'tcx>,
    ) -> IndexSet<(mir::Location, mir::Place<'tcx>)> {
        let object = |place: &mir::Place<'tcx>| {
            let ty = place.ty(&body.local_decls, self.tcx).ty;
            self.is_object(ty) && !self.mut_ref_form(ty).is_cell()
        };
        (moved_apart_places(self.tcx, body).into_iter())
            .filter(|(_, moved)| object(moved))
            .collect()
    }
}
