//! What each `Drop` of a MIR body drop            match state.body.basic_blocks[block].terminator().kind { (ADR 0364), as rustc's drop
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

impl<'a, 'tcx> FnCx<'a, 'tcx> {
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
        let mut inits = MaybeInitializedPlaces::new(tcx, body, &move_data)
            .exclude_inactive_in_otherwise()
            .skipping_unreachable_unwind()
            .iterate_to_fixpoint(tcx, body, None)
            .into_results_cursor(body);
        let mut uninits = MaybeUninitializedPlaces::new(tcx, body, &move_data)
            .mark_inactive_variants_as_uninit()
            .iterate_to_fixpoint(tcx, body, None)
            .into_results_cursor(body);
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
            inits.seek_before_primary_effect(at);
            uninits.seek_before_primary_effect(at);
            on_all_children_bits(&move_data, path, |child| {
                let state = (inits.get().contains(child), uninits.get().contains(child));
                states.insert((block, child), state);
                // One that may be there and may not: a flag says (`collect_drop_flags`).
                if state.0 && state.1 {
                    flagged[child] = true;
                }
            });
        }
        drop(inits);
        drop(uninits);
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
    /// The flags set before `at`, as rustc sets them, after what's made
    /// before: a call's cleanup reads them as they were where it was made.
    pub(super) fn flag_sets(&mut self, state: &mut State<'_, 'tcx>, at: Location, out: &mut Vec<Stmt>) -> R<()> {
        let Some(sets) = state.drops.as_ref().and_then(|drops| drops.sets.get(&at)).cloned() else {
            return Ok(());
        };
        if !state.pending.is_empty() {
            self.flush(state, out)?;
        }
        for (path, present) in sets {
            let flag = state.flags[path].clone().expect("a flagged path");
            out.push(StmtKind::Assign(Expr::var(&flag), Expr::bool(present)).at(js::Span::NONE));
        }
        Ok(())
    }

    /// The cleanup a panic of a terminator unwinds to, if it drops
    /// anything there.
    pub(super) fn cleans_up(&self, state: &State<'_, 'tcx>, unwind: UnwindAction) -> Option<BasicBlock> {
        let (UnwindAction::Cleanup(start), Some(drops)) = (unwind, &state.drops) else {
            return None;
        };
        let mut block = start;
        loop {
            match state.body.basic_blocks[block].terminator().kind {
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
        let Some(drops) = &state.drops else {
            return Ok(());
        };
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
        match state.drops.as_ref().expect("elaborated").style(block, path, true) {
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
        match state.drops.as_ref().expect("elaborated").style(block, path, false) {
            Style::Dead => Ok(()),
            Style::Static => self.drop_place(state, place, span, out),
            Style::Conditional | Style::Open => {
                let flag = state.flags[path].clone().expect("a flagged path");
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
            ty::Closure(_, args) => args.as_closure().upvar_tys().iter().collect(),
            ty::Adt(adt, args) if adt.is_struct() => adt
                .non_enum_variant()
                .fields
                .iter()
                .map(|f| self.field_ty(f, args))
                .collect(),
            ty::Adt(adt, args) if adt.is_enum() => {
                return self.open_drop_variants(state, block, (place, path), (*adt, args), cleanup, span, out);
            }
            _ => {
                let what = format!("dropping what's left of a `{ty}` some of which has moved, from its MIR");
                return Err(self.unsupported(span, &what));
            }
        };
        self.drop_ladder(state, block, (place, path, path), fields, cleanup, span, out)?;
        if !cleanup && let Some(flag) = &state.flags[path] {
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
            let drops = state.drops.as_ref().expect("elaborated");
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
        let subject = self.mir_place(state, place, span, out)?;
        let mut arms = Vec::new();
        let (mut otherwise, mut glue) = (false, false);
        for (variant, def) in adt.variants().iter_enumerated() {
            let drops = state.drops.as_ref().expect("elaborated");
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
        // The variants not taken apart: all of it, or nothing.
        let mut chain: Option<Vec<Stmt>> = match (otherwise, glue) {
            (true, true) => {
                let mut whole = Vec::new();
                self.drop_value(subject.clone(), ty, span, &mut whole)?;
                Some(whole)
            }
            (true, false) => Some(Vec::new()),
            // The last variant taken apart is what's left.
            (false, _) => arms.pop().map(|(_, dropped)| dropped),
        };
        let js_span = self.js_span(span);
        for (variant, dropped) in arms.into_iter().rev() {
            let test = self.variant_test(subject.clone(), ty, adt, variant);
            chain = Some(match (dropped.is_empty(), chain) {
                (true, Some(rest)) if !rest.is_empty() => {
                    vec![StmtKind::If(Expr::unary(js::UnaryOp::Not, test), rest, None).at(js_span)]
                }
                (true, _) => Vec::new(),
                (false, rest) => vec![StmtKind::If(test, dropped, rest.filter(|r| !r.is_empty())).at(js_span)],
            });
        }
        out.extend(chain.unwrap_or_default());
        if !cleanup && let Some(flag) = &state.flags[path] {
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
    pub(super) fn mir_unwind(&mut self, state: &mut State<'_, 'tcx>, unwind: UnwindAction, span: Span) -> R<Vec<Stmt>> {
        let UnwindAction::Cleanup(mut block) = unwind else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        loop {
            let data = &state.body.basic_blocks[block];
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
                _ => return Err(self.unsupported(span, "this cleanup, from its MIR")),
            }
        }
    }

    /// `body`, run so that a panic in it drops `cleanup` as it unwinds: a
    /// `try` whose `catch` drops it and throws the panic on. What `body`
    /// declares, it assigns, declared before.
    pub(super) fn unwinding(&mut self, body: Vec<Stmt>, cleanup: Vec<Stmt>, span: js::Span, out: &mut Vec<Stmt>) {
        // What can't throw needs no `catch`: a value made of values.
        if !body.iter().any(may_throw) {
            out.extend(body);
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

/// Whether running `stmt` may throw: whether it does anything but make
/// values and assign them.
fn may_throw(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Const(_, value) | StmtKind::Let(_, Some(value)) | StmtKind::Expr(value) => value.has_effects(),
        StmtKind::Assign(target, value) => target.has_effects() || value.has_effects(),
        StmtKind::Let(_, None) => false,
        _ => true,
    }
}
