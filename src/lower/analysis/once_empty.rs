//! Which `OnceCell`s a body knows are still empty where it sets them, of
//! its MIR (ADR 0847, #1672 M3): one it made, `Rc::new(Node { cache:
//! OnceCell::new(), .. })`, that nothing could have set since. A cell is
//! known by the local whose value holds it and the fields to it, through
//! `Rc`s and `Box`es: `(fresh, [1])` of `fresh.cache`. What a move or a
//! struct made of it, or an `Rc` or a `Box` of it, gives a local holds what
//! it held there. Only what reaches the cell could set it: the local that
//! holds it, and each made of that one or cloned of its `Rc`. A cell is
//! forgotten where one of them is used any other way than these, given to a
//! call, captured, lent on or dropped where dropping it may run code; and
//! where it's set, as it's set then.

use crate::collections::{IndexMap, IndexSet};
use crate::lower::recognition::{is_pointer_new, is_rc_clone, is_rc_deref, is_std_crate, once_method};
use crate::lower::std_types::once::OnceOp;
use rustc_index::IndexVec;
use rustc_middle::mir::{
    self, AggregateKind, BasicBlock, Body, Local, Operand, Place, PlaceElem, Rvalue, StatementKind, TerminatorKind,
};
use rustc_middle::ty::{self, Ty, TyCtxt};
use std::collections::BTreeSet;

/// A cell: the local whose value holds it, and the fields to it.
type Cell = (Local, Vec<usize>);

/// What's known at a point: each cell known empty; each local a reference
/// is, by the place it lends, its local and the fields to it; and what each
/// local reaches, the locals whose cells it could set.
#[derive(Clone, PartialEq, Default)]
struct State {
    empty: IndexSet<Cell>,
    refs: IndexMap<Local, Cell>,
    reaches: IndexMap<Local, IndexSet<Local>>,
}

impl State {
    fn join(&self, other: &State) -> State {
        let mut reaches = self.reaches.clone();
        for (local, of) in &other.reaches {
            reaches.entry(*local).or_default().extend(of);
        }
        State {
            empty: self.empty.intersection(&other.empty).cloned().collect(),
            refs: (self.refs.iter())
                .filter(|&(k, v)| other.refs.get(k) == Some(v))
                .map(|(k, v)| (*k, v.clone()))
                .collect(),
            reaches,
        }
    }

    /// `local` written: what it held, and what it lent, gone.
    fn written(&mut self, local: Local) {
        self.empty.retain(|(of, _)| *of != local);
        self.refs.shift_remove(&local);
        self.reaches.shift_remove(&local);
    }

    /// What `local` reaches may have been given to code that could set it:
    /// none of those cells is known empty; nor, of a reference, the place
    /// it lends.
    fn escaped(&mut self, local: Local) {
        let mut of: IndexSet<Local> = self.reaches.get(&local).cloned().unwrap_or_default();
        of.insert(local);
        if let Some((lent, _)) = self.refs.get(&local) {
            of.insert(*lent);
            of.extend(self.reaches.get(lent).into_iter().flatten().copied());
        }
        self.empty.retain(|(holder, _)| !of.contains(holder));
    }

    /// `to` made of `from`, moved, in a struct or a pointer at `field`: the
    /// cells `from` held, `to`'s there, and what `from` reached reached.
    fn moved(&mut self, from: Local, to: Local, field: Option<usize>) {
        let cells: Vec<Vec<usize>> = (self.empty.iter())
            .filter(|(holder, _)| *holder == from)
            .map(|(_, path)| path.clone())
            .collect();
        self.empty.retain(|(holder, _)| *holder != from);
        for path in cells {
            self.empty.insert((to, field.into_iter().chain(path).collect()));
        }
        let mut reached = self.reaches.shift_remove(&from).unwrap_or_default();
        reached.insert(from);
        self.reaches.entry(to).or_default().extend(reached);
    }
}

/// The blocks of `body` whose terminator is a `OnceCell`'s `set` of a cell
/// known empty there, every time it's reached.
pub(in crate::lower) fn empty_sets<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &Body<'tcx>,
) -> IndexSet<BasicBlock> {
    let walk = Walk { tcx, typing_env, body };
    let entry = walk.fixed_point();
    let mut proved = IndexSet::default();
    for (block, data) in body.basic_blocks.iter_enumerated() {
        let Some(mut state) = entry[block].clone() else {
            continue;
        };
        for statement in &data.statements {
            walk.statement(statement, &mut state);
        }
        if let TerminatorKind::Call { func, args, .. } = &data.terminator().kind
            && walk.once_op(func) == Some(OnceOp::Set)
            && let Some(cell) = args.first().and_then(|a| walk.lent_cell(&a.node, &state))
            && state.empty.contains(&cell)
        {
            proved.insert(block);
        }
    }
    proved
}

struct Walk<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &'a Body<'tcx>,
}

impl<'tcx> Walk<'_, 'tcx> {
    /// What's known as each block begins, at its fixed point: each block
    /// waiting once, by its place in reverse postorder.
    fn fixed_point(&self) -> IndexVec<BasicBlock, Option<State>> {
        let blocks = self.body.basic_blocks.len();
        let mut entry: IndexVec<BasicBlock, Option<State>> = IndexVec::from_elem_n(None, blocks);
        entry[mir::START_BLOCK] = Some(State::default());
        let mut rank: IndexVec<BasicBlock, usize> = IndexVec::from_elem_n(usize::MAX, blocks);
        for (at, &block) in self.body.basic_blocks.reverse_postorder().iter().enumerate() {
            rank[block] = at;
        }
        let mut work: BTreeSet<(usize, BasicBlock)> = BTreeSet::from([(rank[mir::START_BLOCK], mir::START_BLOCK)]);
        while let Some((_, block)) = work.pop_first() {
            let Some(mut state) = entry[block].clone() else {
                continue;
            };
            let data = &self.body.basic_blocks[block];
            for statement in &data.statements {
                self.statement(statement, &mut state);
            }
            self.terminator(data.terminator(), &mut state);
            for target in data.terminator().successors() {
                let joined = match &entry[target] {
                    Some(old) => old.join(&state),
                    None => state.clone(),
                };
                if entry[target].as_ref() != Some(&joined) {
                    entry[target] = Some(joined);
                    work.insert((rank[target], target));
                }
            }
        }
        entry
    }

    /// The `OnceCell` method `func` calls.
    fn once_op(&self, func: &Operand<'tcx>) -> Option<OnceOp> {
        func.const_fn_def()
            .and_then(|(def_id, _)| once_method(self.tcx, def_id))
    }

    /// The cell, or the value holding cells, `place` is: its local, or the
    /// place a reference lends, and the fields to it, through a reference's,
    /// a `Box`'s or an `Rc`'s `Deref`.
    fn cell_of(&self, place: &Place<'tcx>, state: &State) -> Option<Cell> {
        let mut at: Cell = (place.local, Vec::new());
        for (i, elem) in place.projection.iter().enumerate() {
            match elem {
                PlaceElem::Deref => {
                    let base = Place {
                        local: place.local,
                        projection: self.tcx.mk_place_elems(&place.projection[..i]),
                    };
                    let ty = base.ty(self.body, self.tcx).ty;
                    if i == 0
                        && let Some(lent) = state.refs.get(&place.local)
                    {
                        at = lent.clone();
                    } else if !ty.is_box() {
                        return None;
                    }
                }
                PlaceElem::Field(field, _) => at.1.push(field.as_usize()),
                _ => return None,
            }
        }
        Some(at)
    }

    /// The cell a reference operand lends.
    fn lent_cell(&self, operand: &Operand<'tcx>, state: &State) -> Option<Cell> {
        let place = operand.place()?;
        let local = place.as_local()?;
        state.refs.get(&local).cloned()
    }

    fn statement(&self, statement: &mir::Statement<'tcx>, state: &mut State) {
        match &statement.kind {
            StatementKind::Assign(assign) => {
                let (place, rvalue) = &**assign;
                self.rvalue(rvalue, state, place);
            }
            StatementKind::StorageDead(local) | StatementKind::StorageLive(local) => state.written(*local),
            _ => {}
        }
    }

    /// `rvalue` written to `place`: what it gives `place`, or each local it
    /// reads escaped.
    fn rvalue(&self, rvalue: &Rvalue<'tcx>, state: &mut State, place: &Place<'tcx>) {
        let dest = place.as_local();
        match (rvalue, dest) {
            // A reference to a place: what it lends, the cell it reaches.
            (Rvalue::Ref(_, _, of), Some(dest)) => {
                let lent = self.cell_of(of, state);
                state.written(dest);
                match lent {
                    Some(lent) => {
                        state.refs.insert(dest, lent);
                    }
                    None => self.reads_place(of, state),
                }
            }
            // A move or copy of a local, whole: what it held, `dest`'s.
            (Rvalue::Use(Operand::Move(of) | Operand::Copy(of), _), Some(dest)) if of.as_local().is_some() => {
                let from = of.as_local().expect("a local");
                let lent = state.refs.get(&from).cloned();
                state.written(dest);
                match lent {
                    Some(lent) => {
                        state.refs.insert(dest, lent);
                    }
                    None => state.moved(from, dest, None),
                }
            }
            // A struct or a tuple of fields: each field's cells, its own.
            (Rvalue::Aggregate(kind, fields), Some(dest))
                if matches!(**kind, AggregateKind::Adt(..) | AggregateKind::Tuple) =>
            {
                state.written(dest);
                let field_of = |i: usize| match **kind {
                    AggregateKind::Adt(_, _, _, _, Some(active)) => active.as_usize(),
                    _ => i,
                };
                for (i, operand) in fields.iter_enumerated() {
                    match operand {
                        Operand::Move(of) | Operand::Copy(of)
                            if let Some(from) = of.as_local()
                                && !state.refs.contains_key(&from) =>
                        {
                            state.moved(from, dest, Some(field_of(i.as_usize())));
                        }
                        other => self.reads(other, state),
                    }
                }
            }
            _ => {
                if let Some(dest) = dest {
                    state.written(dest);
                } else {
                    // A write through a place: of a cell's holder, what it
                    // held may be another's now.
                    if let Some((holder, _)) = self.cell_of(place, state) {
                        state.escaped(holder);
                    }
                }
                self.reads_rvalue(rvalue, state);
            }
        }
    }

    /// Each local `rvalue` reads, escaped.
    fn reads_rvalue(&self, rvalue: &Rvalue<'tcx>, state: &mut State) {
        match rvalue {
            Rvalue::Use(operand, _)
            | Rvalue::Repeat(operand, _)
            | Rvalue::Cast(_, operand, _)
            | Rvalue::UnaryOp(_, operand) => self.reads(operand, state),
            Rvalue::BinaryOp(_, operands) => {
                self.reads(&operands.0, state);
                self.reads(&operands.1, state);
            }
            Rvalue::Aggregate(_, fields) => fields.iter().for_each(|o| self.reads(o, state)),
            Rvalue::Ref(_, _, place)
            | Rvalue::RawPtr(_, place)
            | Rvalue::Discriminant(place)
            | Rvalue::CopyForDeref(place) => self.reads_place(place, state),
            _ => {}
        }
    }

    fn reads(&self, operand: &Operand<'tcx>, state: &mut State) {
        if let Some(place) = operand.place() {
            self.reads_place(&place, state);
        }
    }

    fn reads_place(&self, place: &Place<'tcx>, state: &mut State) {
        state.escaped(place.local);
    }

    fn terminator(&self, terminator: &mir::Terminator<'tcx>, state: &mut State) {
        match &terminator.kind {
            TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => {
                let called = func.const_fn_def();
                let first = args.first().map(|a| &a.node);
                let dest = destination.as_local();
                match called {
                    // `get()` and `set()` of a cell lent: none of it escapes,
                    // and a `set` sets it; of one not known empty, what
                    // reaches it is forgotten too. The value set escapes.
                    Some((def_id, _)) if let Some(op @ (OnceOp::Get | OnceOp::Set)) = once_method(self.tcx, def_id) => {
                        if op == OnceOp::Set {
                            match first.and_then(|f| self.lent_cell(f, state)) {
                                Some(cell) => {
                                    if !state.empty.swap_remove(&cell) {
                                        state.escaped(cell.0);
                                    }
                                }
                                None => {
                                    if let Some(f) = first {
                                        self.reads(f, state);
                                    }
                                }
                            }
                            for arg in args.iter().skip(1) {
                                self.reads(&arg.node, state);
                            }
                        } else if first.and_then(|f| self.lent_cell(f, state)).is_none()
                            && let Some(f) = first
                        {
                            self.reads(f, state);
                        }
                        if let Some(dest) = dest {
                            state.written(dest);
                        }
                    }
                    // A cell made: empty.
                    Some((def_id, _))
                        if once_method(self.tcx, def_id) == Some(OnceOp::New)
                            && let Some(dest) = dest =>
                    {
                        state.written(dest);
                        state.empty.insert((dest, Vec::new()));
                    }
                    // An `Rc`, an `Arc` or a `Box` of a value: what it held.
                    Some((def_id, _))
                        if is_pointer_new(self.tcx, def_id)
                            && let Some(dest) = dest
                            && let Some(Operand::Move(of)) = first
                            && let Some(from) = of.as_local() =>
                    {
                        state.written(dest);
                        state.moved(from, dest, None);
                    }
                    // An `Rc`'s `Deref`: what it lends, the `Rc`'s value.
                    Some((def_id, generic_args))
                        if is_rc_deref(self.tcx, def_id, generic_args)
                            && let Some(dest) = dest
                            && let Some(lent) = first.and_then(|f| self.lent_cell(f, state)) =>
                    {
                        state.written(dest);
                        state.refs.insert(dest, lent);
                    }
                    // An `Rc`'s clone: what the `Rc` reaches, and the `Rc`.
                    Some((def_id, generic_args))
                        if is_rc_clone(self.tcx, def_id, generic_args)
                            && let Some(dest) = dest
                            && let Some((rc, _)) = first.and_then(|f| self.lent_cell(f, state)) =>
                    {
                        state.written(dest);
                        let mut reached = state.reaches.get(&rc).cloned().unwrap_or_default();
                        reached.insert(rc);
                        state.reaches.insert(dest, reached.clone());
                        for of in reached {
                            state.reaches.entry(of).or_default().insert(dest);
                        }
                    }
                    _ => {
                        for arg in args {
                            self.reads(&arg.node, state);
                        }
                        self.reads(func, state);
                        match dest {
                            Some(dest) => state.written(dest),
                            None => {
                                if let Some((holder, _)) = self.cell_of(destination, state) {
                                    state.escaped(holder);
                                }
                            }
                        }
                    }
                }
            }
            // A drop whose code could set a cell it reaches.
            TerminatorKind::Drop { place, .. } => {
                let ty = place.ty(self.body, self.tcx).ty;
                if drop_runs_code(self.tcx, self.typing_env, ty) {
                    state.escaped(place.local);
                }
            }
            TerminatorKind::Yield { value, .. } => self.reads(value, state),
            TerminatorKind::InlineAsm { .. } => {
                let all: Vec<Local> = state.empty.iter().map(|(holder, _)| *holder).collect();
                for local in all {
                    state.escaped(local);
                }
            }
            _ => {}
        }
    }
}

/// Whether dropping a `ty` may run code that could set a cell: a
/// destructor of the crate's or a binding's, or one rust-js can't see, a
/// `dyn`'s or a type parameter's. std's own run none but their items'.
pub(in crate::lower) fn drop_runs_code<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> bool {
    let mut seen = IndexSet::default();
    let mut work = vec![ty];
    while let Some(ty) = work.pop() {
        if !ty.needs_drop(tcx, typing_env) || !seen.insert(ty) {
            continue;
        }
        match ty.kind() {
            ty::Adt(adt, args) if is_std_crate(tcx, adt.did()) => work.extend(args.types()),
            ty::Adt(adt, args) => {
                if tcx.adt_destructor(adt.did()).is_some() {
                    return true;
                }
                work.extend(adt.all_fields().map(|f| f.ty(tcx, args).skip_normalization()));
            }
            ty::Tuple(items) => work.extend(items.iter()),
            ty::Array(item, _) | ty::Slice(item) => work.push(*item),
            ty::Closure(_, args) => work.extend(args.as_closure().upvar_tys()),
            _ => return true,
        }
    }
    false
}
