//! What each `Drop` of a MIR body drops (ADR 0364), as rustc's drop
//! elaboration finds it: rustc's dataflow of what's initialized says,
//! before each, whether what it drops is there, isn't, or may be, which a
//! flag of its own then says as the body runs, set where rustc sets it.

use std::collections::{HashMap, HashSet};

use rustc_abi::FieldIdx;
use rustc_index::IndexVec;
use rustc_middle::mir::{
    self, BasicBlock, Location, Place, ProjectionElem, StatementKind, TerminatorKind, UnwindAction,
};
use rustc_middle::ty::{self, Ty};
use rustc_mir_dataflow::impls::{MaybeInitializedPlaces, MaybeUninitializedPlaces};
use rustc_mir_dataflow::move_paths::{LookupResult, MoveData, MovePathIndex};
use rustc_mir_dataflow::{
    Analysis, DropFlagState, drop_flag_effects_for_function_entry, drop_flag_effects_for_location,
    move_path_children_matching, on_all_children_bits, on_lookup_result_bits,
};
use rustc_span::Span;

use super::State;
use crate::js::{self, Expr, Stmt, StmtKind};
use crate::lower::drops::Drops;
use crate::lower::{FnCx, R};

/// How a drop drops what it drops, rustc's `DropStyle`: not at all, as
/// it's never there; as it's always there; as its flag says; or part by
/// part, as some of it has moved.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Style {
    Dead,
    Static,
    Conditional,
    Open,
}

pub(super) struct Elaboration<'tcx> {
    pub(super) move_data: MoveData<'tcx>,
    /// Before each `Drop`, of each path under what it drops: whether it
    /// may be initialized, and whether it may not be.
    states: HashMap<(BasicBlock, MovePathIndex), (bool, bool)>,
    /// The paths a flag says the drop of.
    pub(super) flagged: IndexVec<MovePathIndex, bool>,
    /// Each flag as the body starts: a parameter's is set.
    pub(super) initial: IndexVec<MovePathIndex, bool>,
    /// The flags set before a statement or a terminator.
    pub(super) sets: HashMap<Location, Vec<(MovePathIndex, bool)>>,
    /// The blocks whose `Drop` drops what's behind a pointer that drops
    /// nothing itself, `*self`'s field: there, as borrowck says.
    behind: HashSet<BasicBlock>,
}

impl<'tcx> Elaboration<'tcx> {
    /// Whether the `Drop` ending `block` drops `place`, or may.
    pub(super) fn drops(&self, block: BasicBlock, place: Place<'tcx>) -> bool {
        match self.move_data.rev_lookup.find(place.as_ref()) {
            LookupResult::Exact(path) => self.style(block, path, true) != Style::Dead,
            LookupResult::Parent(parent) => parent.is_some() || self.behind.contains(&block),
        }
    }

    /// How the `Drop` ending `block` drops `path`: all of it, `deep`, or
    /// only what isn't a part of its own, as rustc's `drop_style`.
    pub(super) fn style(&self, block: BasicBlock, path: MovePathIndex, deep: bool) -> Style {
        let state = |path| self.states.get(&(block, path)).copied().unwrap_or((false, false));
        let ((init, uninit), parts) = match deep {
            true => {
                let (mut init, mut uninit, mut count) = (false, false, 0);
                on_all_children_bits(&self.move_data, path, |child| {
                    let (i, u) = state(child);
                    init |= i;
                    uninit |= u;
                    count += 1;
                });
                ((init, uninit), count != 1)
            }
            false => (state(path), false),
        };
        match (init, uninit, parts) {
            (false, _, _) => Style::Dead,
            (true, false, _) => Style::Static,
            (true, true, false) => Style::Conditional,
            (true, true, true) => Style::Open,
        }
    }

    /// The paths each `Drop` reads, of a local: one that's there, or may be.
    pub(super) fn read(&self, body: &mir::Body<'tcx>) -> Vec<mir::Local> {
        let mut read = Vec::new();
        for (block, data) in body.basic_blocks.iter_enumerated() {
            if let TerminatorKind::Drop { place, .. } = &data.terminator().kind
                && self.drops(block, *place)
            {
                read.push(place.local);
            }
        }
        read
    }
}

/// The blocks whose `Drop` would unwind to a cleanup but drops what JS
/// drops nothing of, so runs nothing and can't panic.
fn quiet_drops<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    drops: &impl Fn(Ty<'tcx>) -> bool,
) -> HashSet<BasicBlock> {
    (body.basic_blocks.iter_enumerated())
        .filter(|(_, data)| {
            matches!(&data.terminator().kind, TerminatorKind::Drop { place, unwind: UnwindAction::Cleanup(_), .. }
                if !drops(place.ty(&body.local_decls, tcx).ty))
        })
        .map(|(block, _)| block)
        .collect()
}

/// `body` with no unwind from the `Drop` ending each of `quiet`.
fn without_unwinds<'tcx>(body: &mir::Body<'tcx>, quiet: &HashSet<BasicBlock>) -> mir::Body<'tcx> {
    let mut body = body.clone();
    for (block, data) in body.basic_blocks.as_mut().iter_enumerated_mut() {
        if quiet.contains(&block)
            && let TerminatorKind::Drop { unwind, .. } = &mut data.terminator_mut().kind
        {
            *unwind = UnwindAction::Unreachable;
        }
    }
    body
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `place`, a call's destination, is a local holding an iterator
    /// dropped by closing it that the body drops, a variable's or a
    /// temporary's: held, `$held(..)`, so its owner drops what it steps
    /// where Rust does, at its scope's or its statement's end, not as it's
    /// done or an adapter in it ends (ADR 0570 amended, #206). Not one only
    /// moved, into a consumer that takes it, which drops it at its end.
    pub(super) fn holds_dropped_iterator(&self, state: &State<'_, 'tcx>, place: Place<'tcx>) -> bool {
        place.projection.is_empty()
            && self.dropped_by_closing(state.body.local_decls[place.local].ty)
            && self.owned_where_run(state, place.local, 0)
    }

    /// Whether the body drops `local` where it runs on, not only as a panic
    /// unwinds past it; or moves it into a call whose value it drops so,
    /// of a type holding it that its drop glue drops, `Peekable<Take<..>>`'s,
    /// which drops it there (#243). Not one moved into a consumer,
    /// `collect`'s, whose value holds no iterator; nor into an adapter
    /// dropped by closing, held itself, whose `hold()` holds it.
    fn owned_where_run(&self, state: &State<'_, 'tcx>, local: mir::Local, depth: usize) -> bool {
        let Some(drops) = state.facts.drops() else {
            return false;
        };
        let body = state.body;
        let dropped = (body.basic_blocks.iter_enumerated()).any(|(block, data)| {
            !data.is_cleanup
                && matches!(&data.terminator().kind, TerminatorKind::Drop { place: dropped, .. }
                    if dropped.local == local && dropped.projection.is_empty() && drops.drops(block, *dropped))
        });
        let moved = |arg: &mir::Operand<'tcx>| matches!(arg, mir::Operand::Move(p) if p.local == local && p.projection.is_empty());
        dropped
            || depth < 4
                && body.basic_blocks.iter().any(|data| {
                    !data.is_cleanup
                        && matches!(&data.terminator().kind, TerminatorKind::Call { args, destination, .. }
                            if destination.projection.is_empty()
                                && destination.local != local
                                && args.iter().any(|arg| moved(&arg.node))
                                && (body.local_decls[destination.local].ty.walk())
                                    .any(|arg| arg.as_type() == Some(body.local_decls[local].ty))
                                && !self.dropped_by_closing(body.local_decls[destination.local].ty)
                                && self.owned_where_run(state, destination.local, depth + 1))
                })
    }

    /// What `body`'s drops drop, `None` if none drops anything JS sees.
    pub(super) fn mir_elaboration(&self, body: &mir::Body<'tcx>) -> Option<Elaboration<'tcx>> {
        let tcx = self.tcx;
        let drops = |ty| self.drops(ty) != Drops::Nothing;
        let any = body.basic_blocks.iter().any(|data| {
            matches!(&data.terminator().kind, TerminatorKind::Drop { place, .. }
                if drops(place.ty(&body.local_decls, tcx).ty))
        });
        if !any {
            return None;
        }
        // rustc's `ElaborateDrops`, of the paths whose drop JS sees.
        let move_data = MoveData::gather_moves(body, tcx, drops);
        // A std body, read from the MIR the toolchain ships, has its drops
        // elaborated by rustc already: each `Drop` left drops what's there,
        // and its flags are the body's own locals. Its dataflow isn't run,
        // which its optimized switches wouldn't let be.
        let elaborated = !body.source.def_id().is_local();
        // A `Drop` of what JS drops nothing of runs nothing, so it never
        // unwinds: the cleanup it would unwind to isn't reached from it, as
        // rustc's elaboration finds of a drop of what isn't there
        // (`compute_dead_unwinds`). The dataflow runs on the body without
        // those edges, so what's made after a `Box::new(closure)` isn't
        // taken to be dropped where the box's drop would unwind.
        let quiet = quiet_drops(tcx, body, &drops);
        let pruned = (!quiet.is_empty()).then(|| without_unwinds(body, &quiet));
        let flow = pruned.as_ref().unwrap_or(body);
        let mut cursors = (!elaborated).then(|| {
            let inits = MaybeInitializedPlaces::new(tcx, flow, &move_data)
                .exclude_inactive_in_otherwise()
                .skipping_unreachable_unwind()
                .iterate_to_fixpoint(tcx, flow, None)
                .into_results_cursor(flow);
            let uninits = MaybeUninitializedPlaces::new(tcx, flow, &move_data)
                .mark_inactive_variants_as_uninit()
                .iterate_to_fixpoint(tcx, flow, None)
                .into_results_cursor(flow);
            (inits, uninits)
        });
        let mut states = HashMap::new();
        let mut flagged = IndexVec::from_elem(false, &move_data.move_paths);
        let mut behind = HashSet::new();
        for (block, data) in body.basic_blocks.iter_enumerated() {
            let TerminatorKind::Drop { place, .. } = &data.terminator().kind else {
                continue;
            };
            let path = match move_data.rev_lookup.find(place.as_ref()) {
                LookupResult::Exact(path) => path,
                LookupResult::Parent(None) if place.is_indirect() && drops(place.ty(&body.local_decls, tcx).ty) => {
                    behind.insert(block);
                    continue;
                }
                LookupResult::Parent(_) => continue,
            };
            let at = body.terminator_loc(block);
            if let Some((inits, uninits)) = &mut cursors {
                inits.seek_before_primary_effect(at);
                uninits.seek_before_primary_effect(at);
            }
            on_all_children_bits(&move_data, path, |child| {
                let state = match &cursors {
                    Some((inits, uninits)) => (inits.get().contains(child), uninits.get().contains(child)),
                    None => (true, false),
                };
                states.insert((block, child), state);
                // One that may be there and may not: a flag says (`collect_drop_flags`).
                if state.0 && state.1 {
                    flagged[child] = true;
                }
            });
        }
        drop(cursors);
        let mut initial = IndexVec::from_elem(false, &move_data.move_paths);
        drop_flag_effects_for_function_entry(body, &move_data, |path, state| {
            initial[path] = state == DropFlagState::Present;
        });
        let mut sets: HashMap<Location, Vec<(MovePathIndex, bool)>> = HashMap::new();
        let mut set = |at: Location, path: MovePathIndex, present: bool| {
            if flagged[path] {
                sets.entry(at).or_default().push((path, present));
            }
        };
        // A call's result, where its unwinding cleans up: as it returns.
        for data in body.basic_blocks.iter() {
            if let TerminatorKind::Call {
                destination,
                target: Some(target),
                unwind: UnwindAction::Cleanup(_),
                ..
            } = data.terminator().kind
            {
                let at = Location {
                    block: target,
                    statement_index: 0,
                };
                on_lookup_result_bits(&move_data, move_data.rev_lookup.find(destination.as_ref()), |child| {
                    set(at, child, true)
                });
            }
        }
        // What each statement and terminator moves and makes; a `Drop`'s
        // is the drop's own.
        for (block, data) in body.basic_blocks.iter_enumerated() {
            for index in 0..=data.statements.len() {
                if index == data.statements.len() && matches!(data.terminator().kind, TerminatorKind::Drop { .. }) {
                    continue;
                }
                let at = Location {
                    block,
                    statement_index: index,
                };
                drop_flag_effects_for_location(body, &move_data, at, |path, state| {
                    set(at, path, state == DropFlagState::Present)
                });
            }
            // A call's result, where it doesn't unwind to a cleanup: before it.
            if let TerminatorKind::Call {
                destination,
                target: Some(_),
                unwind: UnwindAction::Continue | UnwindAction::Unreachable | UnwindAction::Terminate(_),
                ..
            } = data.terminator().kind
            {
                let at = body.terminator_loc(block);
                on_lookup_result_bits(&move_data, move_data.rev_lookup.find(destination.as_ref()), |child| {
                    set(at, child, true)
                });
            }
        }
        Some(Elaboration {
            move_data,
            states,
            flagged,
            initial,
            sets,
            behind,
        })
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The flag that says whether `place` is there to drop: its variable's
    /// name, a part's by its field's, `pair$a$live`. A temporary's is MIR's
    /// local's until tidy names it for what its drop is of (`tidy/flags.rs`).
    pub(super) fn flag_name(&mut self, state: &State<'_, 'tcx>, place: Place<'tcx>) -> String {
        let mut name = match state.locals.names[place.local].as_str() {
            "" => super::tidy::temporary(place.local.as_usize()),
            name => name.to_string(),
        };
        let mut ty = mir::PlaceTy::from_ty(state.body.local_decls[place.local].ty);
        for elem in place.projection {
            if let ProjectionElem::Field(field, _) = elem {
                let key = match (ty.ty.kind(), ty.variant_index) {
                    (ty::Adt(adt, _), variant) => crate::lower::bindings::field_key(
                        self.tcx,
                        &adt.variant(variant.unwrap_or(rustc_abi::FIRST_VARIANT)).fields[field],
                    ),
                    _ => field.as_usize().to_string(),
                };
                name = format!("{name}${key}");
            }
            ty = ty.projection_ty(self.tcx, elem);
        }
        self.fresh(&format!("{name}$live"))
    }

    /// The flags set before `at`, as rustc sets them, after what's made
    /// before: a call's cleanup reads them as they were where it was made.
    pub(super) fn flag_sets(&mut self, state: &mut State<'_, 'tcx>, at: Location, out: &mut Vec<Stmt>) -> R<()> {
        let Some(sets) = state.facts.drops().and_then(|drops| drops.sets.get(&at)).cloned() else {
            return Ok(());
        };
        if !state.pending.is_empty() {
            self.flush(state, out)?;
        }
        for (path, present) in sets {
            let flag = state.naming.flags()[path].clone().expect("a flagged path");
            out.push(StmtKind::Assign(Expr::var(&flag), Expr::bool(present)).at(js::Span::NONE));
        }
        Ok(())
    }

    /// The `Drop` that ends `block`, of `place`: what's there of it,
    /// dropped. As a panic unwinds, `cleanup`, its flags aren't read again.
    pub(super) fn mir_drop(
        &mut self,
        state: &mut State<'_, 'tcx>,
        block: BasicBlock,
        place: Place<'tcx>,
        cleanup: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let Some(drops) = state.facts.drops() else {
            return Ok(());
        };
        // A guard of a field's `RefCell` no borrow of which can meet another
        // counted nothing, so its drop releases nothing (ADR 0863, amended).
        if place.projection.is_empty() && state.facts.quiet_cells().guards.contains(&place.local) {
            return Ok(());
        }
        match drops.move_data.rev_lookup.find(place.as_ref()) {
            LookupResult::Exact(path) => self.elaborate_drop(state, block, (place, path), cleanup, span, out),
            // Behind a pointer, which borrowck says is there.
            LookupResult::Parent(_) if drops.drops(block, place) => self.drop_place(state, place, span, out),
            // Of nothing JS sees.
            LookupResult::Parent(_) => Ok(()),
        }
    }

    /// rustc's `elaborate_drop`: all of `path`, as it's there.
    fn elaborate_drop(
        &mut self,
        state: &mut State<'_, 'tcx>,
        block: BasicBlock,
        (place, path): (Place<'tcx>, MovePathIndex),
        cleanup: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        match state.facts.drops().expect("elaborated").style(block, path, true) {
            Style::Dead => Ok(()),
            Style::Static => self.drop_place(state, place, span, out),
            Style::Conditional => self.complete_drop(state, block, (place, path), span, out),
            Style::Open => self.open_drop(state, block, (place, path), cleanup, span, out),
        }
    }

    /// rustc's `complete_drop`: `place`, as the flag of `path` says.
    fn complete_drop(
        &mut self,
        state: &mut State<'_, 'tcx>,
        block: BasicBlock,
        (place, path): (Place<'tcx>, MovePathIndex),
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        match state.facts.drops().expect("elaborated").style(block, path, false) {
            Style::Dead => Ok(()),
            Style::Static => self.drop_place(state, place, span, out),
            Style::Conditional | Style::Open => {
                let flag = state.naming.flags()[path].clone().expect("a flagged path");
                let mut then = Vec::new();
                self.drop_place(state, place, span, &mut then)?;
                out.push(StmtKind::If(Expr::var(&flag), then, None).at(self.js_span(span)));
                Ok(())
            }
        }
    }

    /// rustc's `open_drop`: of a value some of which has moved, each part
    /// that's there, in order, then its flag cleared.
    fn open_drop(
        &mut self,
        state: &mut State<'_, 'tcx>,
        block: BasicBlock,
        (place, path): (Place<'tcx>, MovePathIndex),
        cleanup: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let tcx = self.tcx;
        let ty = place.ty(&state.body.local_decls, tcx).ty;
        let fields: Vec<Ty<'tcx>> = match ty.kind() {
            ty::Tuple(items) => items.iter().collect(),
            ty::Closure(..) | ty::Coroutine(..) | ty::CoroutineClosure(..) => {
                crate::lower::recognition::held_types(ty).map_or_else(Vec::new, |(_, held)| held.to_vec())
            }
            ty::Adt(adt, args) if adt.is_struct() => adt
                .non_enum_variant()
                .fields
                .iter()
                .map(|f| self.field_ty(f, args))
                .collect(),
            ty::Adt(adt, args) if adt.is_enum() => {
                return self.open_drop_variants(state, block, (place, path), (*adt, args), cleanup, span, out);
            }
            ty::Array(_, len) if let Some(size) = len.try_to_target_usize(tcx) => {
                return self.open_drop_array(state, block, (place, path), size, cleanup, span, out);
            }
            _ => {
                let what = format!("dropping what's left of a `{ty}` some of which has moved, from its MIR");
                return Err(self.unsupported(span, &what));
            }
        };
        self.drop_ladder(state, block, (place, path, path), fields, cleanup, span, out)?;
        if !cleanup && let Some(flag) = &state.naming.flags()[path] {
            out.push(StmtKind::Assign(Expr::var(flag), Expr::bool(false)).at(js::Span::NONE));
        }
        Ok(())
    }

    /// rustc's `open_drop_for_array`: of an array of `size` items some of
    /// which have moved, `[a, b, c] = items`, what's left: each item moved
    /// by its own path, and each run of the rest, as its flag says, a
    /// subslice, one drop of them all, the last first.
    #[allow(clippy::too_many_arguments)]
    fn open_drop_array(
        &mut self,
        state: &mut State<'_, 'tcx>,
        block: BasicBlock,
        (place, path): (Place<'tcx>, MovePathIndex),
        size: u64,
        cleanup: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        // An item's path: a subslice moved, `[a, rest @ ..]`, is each of its
        // items' too, as rustc's move paths have an array's.
        let drops = state.facts.drops().expect("elaborated");
        let item_path = |i: u64| {
            move_path_children_matching(
                &drops.move_data,
                path,
                |elem| matches!(elem, ProjectionElem::ConstantIndex { offset, .. } if offset == i),
            )
        };
        let paths: Vec<Option<MovePathIndex>> = (0..size).map(item_path).collect();
        // Each item moved by its own path, and each run of the rest between.
        let mut pieces = Vec::new();
        let mut i = 0;
        while i < size {
            let end = match paths[i as usize] {
                Some(_) => i + 1,
                None => (i..size).find(|&j| paths[j as usize].is_some()).unwrap_or(size),
            };
            let elem = match end - i {
                1 => ProjectionElem::ConstantIndex {
                    offset: i,
                    min_length: size,
                    from_end: false,
                },
                _ => ProjectionElem::Subslice {
                    from: i,
                    to: end,
                    from_end: false,
                },
            };
            pieces.push((self.tcx.mk_place_elem(place, elem), paths[i as usize]));
            i = end;
        }
        // Last first, as rustc's ladder has them, each run in order.
        for (part, subpath) in pieces.into_iter().rev() {
            match subpath {
                Some(subpath) => self.elaborate_drop(state, block, (part, subpath), cleanup, span, out)?,
                None => self.complete_drop(state, block, (part, path), span, out)?,
            }
        }
        if !cleanup && let Some(flag) = &state.naming.flags()[path] {
            out.push(StmtKind::Assign(Expr::var(flag), Expr::bool(false)).at(js::Span::NONE));
        }
        Ok(())
    }

    /// rustc's `drop_ladder`: each of `fields` of `place`, a struct's or a
    /// variant's whose path is `parts`, that's there, in order. One with no
    /// path of its own is there as `path`'s flag says.
    #[allow(clippy::too_many_arguments)]
    fn drop_ladder(
        &mut self,
        state: &mut State<'_, 'tcx>,
        block: BasicBlock,
        (place, path, parts): (Place<'tcx>, MovePathIndex, MovePathIndex),
        fields: Vec<Ty<'tcx>>,
        cleanup: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        for (i, field_ty) in fields.into_iter().enumerate() {
            if self.drops(field_ty) == Drops::Nothing {
                continue;
            }
            let field = FieldIdx::from_usize(i);
            let part = self.tcx.mk_place_field(place, field, field_ty);
            let drops = state.facts.drops().expect("elaborated");
            let subpath = move_path_children_matching(
                &drops.move_data,
                parts,
                |elem| matches!(elem, ProjectionElem::Field(at, _) if at == field),
            );
            match subpath {
                Some(subpath) if drops.style(block, subpath, true) == Style::Dead => {}
                Some(subpath) => self.elaborate_drop(state, block, (part, subpath), cleanup, span, out)?,
                None => self.complete_drop(state, block, (part, path), span, out)?,
            }
        }
        Ok(())
    }

    /// rustc's `open_drop_for_multivariant`: of an enum some of which has
    /// moved, each variant some of whose fields have, those left, if it's
    /// that variant; any other, all of it, if it holds what drops something.
    /// Then its flag cleared.
    #[allow(clippy::too_many_arguments)]
    fn open_drop_variants(
        &mut self,
        state: &mut State<'_, 'tcx>,
        block: BasicBlock,
        (place, path): (Place<'tcx>, MovePathIndex),
        (adt, args): (ty::AdtDef<'tcx>, ty::GenericArgsRef<'tcx>),
        cleanup: bool,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let ty = place.ty(&state.body.local_decls, self.tcx).ty;
        let mut arms = Vec::new();
        let (mut otherwise, mut glue) = (false, false);
        for (variant, def) in adt.variants().iter_enumerated() {
            let drops = state.facts.drops().expect("elaborated");
            let parts = move_path_children_matching(
                &drops.move_data,
                path,
                |elem| matches!(elem, ProjectionElem::Downcast(_, at) if at == variant),
            );
            let fields: Vec<Ty<'tcx>> = def.fields.iter().map(|f| self.field_ty(f, args)).collect();
            match parts {
                Some(parts) => {
                    let base = self.tcx.mk_place_downcast(place, adt, variant);
                    let mut dropped = Vec::new();
                    self.drop_ladder(state, block, (base, path, parts), fields, cleanup, span, &mut dropped)?;
                    arms.push((variant, dropped));
                }
                None => {
                    otherwise = true;
                    glue |= fields.iter().any(|&f| self.drops(f) != Drops::Nothing);
                }
            }
        }
        // What `?` made, `Try::branch(r)`: `r`, whose `Err` or `None` is its
        // `Break`, its value its `Continue`.
        let branch = match place.projection.is_empty() {
            true => state.branches.get(&place.local).cloned(),
            false => None,
        };
        let subject = match &branch {
            Some((tried, _)) => tried.clone(),
            None => self.mir_place(state, place, span, out)?,
        };
        // The variants not taken apart: all of it, or nothing.
        let mut chain: Option<Vec<Stmt>> = match (otherwise, glue) {
            (true, true) => {
                let mut whole = Vec::new();
                let whole_ty = branch.as_ref().map_or(ty, |(_, tried_ty)| *tried_ty);
                self.drop_value(subject.clone(), whole_ty, span, &mut whole)?;
                Some(whole)
            }
            (true, false) => Some(Vec::new()),
            // The last variant taken apart is what's left.
            (false, _) => arms.pop().map(|(_, dropped)| dropped),
        };
        let js_span = self.js_span(span);
        for (variant, dropped) in arms.into_iter().rev() {
            let test = match &branch {
                Some((_, tried_ty)) => {
                    let failed = self.tried_failed(subject.clone(), *tried_ty);
                    match variant.as_u32() {
                        0 => Expr::unary(js::UnaryOp::Not, failed),
                        _ => failed,
                    }
                }
                None => self.mir_variant_test(subject.clone(), ty, adt, &[variant], &[], false),
            };
            chain = Some(match (dropped.is_empty(), chain) {
                (true, Some(rest)) if !rest.is_empty() => {
                    vec![StmtKind::If(Expr::unary(js::UnaryOp::Not, test), rest, None).at(js_span)]
                }
                (true, _) => Vec::new(),
                (false, rest) => vec![StmtKind::If(test, dropped, rest.filter(|r| !r.is_empty())).at(js_span)],
            });
        }
        out.extend(chain.unwrap_or_default());
        if !cleanup && let Some(flag) = &state.naming.flags()[path] {
            out.push(StmtKind::Assign(Expr::var(flag), Expr::bool(false)).at(js::Span::NONE));
        }
        Ok(())
    }

    /// `place`, dropped: its own `drop`, then each part's.
    fn drop_place(
        &mut self,
        state: &mut State<'_, 'tcx>,
        place: Place<'tcx>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let ty = place.ty(&state.body.local_decls, self.tcx).ty;
        let value = self.mir_place(state, place, span, out)?;
        self.drop_value(value, ty, span, out)
    }

    /// What a panic drops as it unwinds from a terminator: its cleanup
    /// blocks' drops, in order, nothing if it has none.
    pub(super) fn mir_unwind(&mut self, state: &mut State<'_, 'tcx>, unwind: UnwindAction) -> R<Vec<Stmt>> {
        let UnwindAction::Cleanup(block) = unwind else {
            return Ok(Vec::new());
        };
        self.mir_cleanup(state, block)
    }

    /// What cleanup `block` and those after it drop, in order. A test of a
    /// drop flag optimized MIR keeps, another crate's (ADRs 0561, 0914),
    /// `switchInt(copy _4) -> [0: bb3, otherwise: bb4]`, is an `if` of what
    /// each way drops, each to its end.
    fn mir_cleanup(&mut self, state: &mut State<'_, 'tcx>, mut block: BasicBlock) -> R<Vec<Stmt>> {
        let mut out = Vec::new();
        loop {
            let data = &state.body.basic_blocks[block];
            // Where its Rust is, as a cleanup has no span of its own.
            let span = data.terminator().source_info.span;
            for (index, statement) in data.statements.iter().enumerate() {
                match &statement.kind {
                    StatementKind::StorageDead(_) | StatementKind::StorageLive(_) | StatementKind::Nop => {}
                    // A `Drop`'s replacing value, written as the old one's
                    // drop unwinds: of variables, made of nothing made before.
                    StatementKind::Assign(assign) => {
                        let at = Location {
                            block,
                            statement_index: index,
                        };
                        let made = std::mem::take(&mut state.pending);
                        let (place, rvalue) = &**assign;
                        let written = self
                            .flag_sets(state, at, &mut out)
                            .and_then(|()| self.mir_assign(state, place, rvalue, span, &mut out));
                        let left = std::mem::replace(&mut state.pending, made);
                        written?;
                        if !left.is_empty() {
                            return Err(self.unsupported(span, "this cleanup, from its MIR"));
                        }
                    }
                    _ => return Err(self.unsupported(span, "this cleanup, from its MIR")),
                }
            }
            match data.terminator().kind {
                TerminatorKind::Drop { place, target, .. } => {
                    self.mir_drop(state, block, place, true, span, &mut out)?;
                    block = target;
                }
                TerminatorKind::Goto { target } => block = target,
                TerminatorKind::UnwindResume | TerminatorKind::UnwindTerminate(_) => return Ok(out),
                TerminatorKind::SwitchInt { ref discr, ref targets }
                    if discr.ty(&state.body.local_decls, self.tcx).is_bool()
                        && let [(0, unset)] = targets.iter().collect::<Vec<_>>()[..] =>
                {
                    let flag = self.mir_operand(state, discr, &mut out)?;
                    let flag = self.value_expr(flag, span)?;
                    let set = self.mir_cleanup(state, targets.otherwise())?;
                    let unset = self.mir_cleanup(state, unset)?;
                    let js_span = self.js_span(span);
                    out.push(match (set.is_empty(), unset.is_empty()) {
                        (true, true) => return Ok(out),
                        (false, true) => StmtKind::If(flag, set, None).at(js_span),
                        (true, false) => StmtKind::If(Expr::unary(js::UnaryOp::Not, flag), unset, None).at(js_span),
                        (false, false) => StmtKind::If(flag, set, Some(unset)).at(js_span),
                    });
                    return Ok(out);
                }
                _ => return Err(self.unsupported(span, "this cleanup, from its MIR")),
            }
        }
    }

    /// `body`, run so that a panic in it drops `cleanup` as it unwinds: a
    /// `try` whose `catch` drops it and throws the panic on. What `body`
    /// declares, it assigns, declared before. A cleanup that runs nothing,
    /// of what's all moved, an `FnOnce`'s own self given away, needs none.
    pub(super) fn unwinding(&mut self, body: Vec<Stmt>, cleanup: Vec<Stmt>, span: js::Span, out: &mut Vec<Stmt>) {
        if cleanup.is_empty() {
            out.extend(body);
            return;
        }
        // What can't throw needs no `catch`: a value made of values, and
        // what's made so before what can, a closure the call is given.
        let first = body.iter().take_while(|s| s.only_makes_values()).count();
        let mut body = body.into_iter();
        out.extend(body.by_ref().take(first));
        let mut body = body.peekable();
        if body.peek().is_none() {
            return;
        }
        let mut inner = Vec::new();
        for stmt in body {
            match stmt.kind {
                StmtKind::Const(name, value) | StmtKind::Let(name, Some(value)) => {
                    out.push(StmtKind::Let(name.clone(), None).at(stmt.span));
                    inner.push(StmtKind::Assign(Expr::var(&name), value).at(stmt.span));
                }
                StmtKind::Let(name, None) => out.push(StmtKind::Let(name, None).at(stmt.span)),
                kind => inner.push(Stmt { kind, span: stmt.span }),
            }
        }
        let error = self.fresh("error");
        let mut handler = cleanup;
        handler.push(StmtKind::Throw(Expr::var(&error)).at(span));
        out.push(StmtKind::TryCatch(inner, Some(error), handler).at(span));
    }
}

/// The cleanup a panic of a terminator unwinds to, if it drops
/// anything there.
pub(super) fn cleans_up<'tcx>(
    body: &mir::Body<'tcx>,
    drops: Option<&Elaboration<'tcx>>,
    unwind: UnwindAction,
) -> Option<BasicBlock> {
    let (UnwindAction::Cleanup(start), Some(drops)) = (unwind, drops) else {
        return None;
    };
    let mut block = start;
    loop {
        match body.basic_blocks[block].terminator().kind {
            TerminatorKind::Drop { place, target, .. } => {
                if drops.drops(block, place) {
                    return Some(start);
                }
                block = target;
            }
            TerminatorKind::Goto { target } => block = target,
            TerminatorKind::UnwindResume | TerminatorKind::UnwindTerminate(_) => return None,
            // What `mir_unwind` says it doesn't support.
            _ => return Some(start),
        }
    }
}
