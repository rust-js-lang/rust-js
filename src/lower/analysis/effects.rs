//! What a statement may write: the one answer the MIR intervals ask of
//! each fact they keep (#2112 B, ADR 1174). A fact survives a statement
//! only where no local it's of may be written there; what can't be seen
//! may write, so it fails closed.
//!
//! What a pointer may point into is a target: a local's own storage
//! (`Whole`), or what a local that holds a pointer points to (`Pointee`).
//! A local that is a pointer, a reference, a `Box`, an `Rc`, a guard,
//! stands for what it points to as well, one pointer down: a fact of a
//! reference's length is of the reference. Which pointers point where is a
//! flow-insensitive map of what each target's memory holds, each pointer
//! with what it may do: replace what it points to (`&mut`, a raw pointer,
//! a `Box`), or change only what's inside it, through a shared reference
//! to what holds a cell (`changes_through_shared`, rustc's `Freeze`
//! refined to what it reaches). A target that is a cell itself is replaced
//! through either. Each copy, value holding one, call given one and place
//! written through one, of what can hold a pointer.
//!
//! A target is written whole where a statement assigns its local; where a
//! write goes through a pointer that may replace it, of a whole local only
//! one rustc's `MaybeBorrowedLocals` says may be borrowed there, and of a
//! scalar written through a reference only what a scalar can be; where a
//! call or a drop is given such a pointer to it; or where code that isn't
//! std's runs past where one escaped. What std's signatures can't say is
//! named here and nowhere else (`writes_nothing`): a `RefCell`'s
//! `borrow()`, a `Ref`'s or an `Rc`'s `Deref`, a range's `next()`, whose
//! start only grows, and `IndexMut` and `DerefMut`, which give a pointer
//! and write nothing (`keeps_vec_length`). A pointer escapes where it's
//! made raw, given a call that may keep it (`pointers::argument_may_keep`,
//! the escape owner asked), or written into what's escaped or the body
//! can't see: from there on, a forward flow. What a parameter that can be
//! changed through an alias its caller keeps points to has escaped where
//! the body starts. A dead local isn't written: it's read again only once
//! it's assigned.

use crate::collections::{IndexMap, IndexSet};
use crate::lower::pointers::{Libraries, argument_may_keep, raw_pointee};
use crate::lower::recognition::{
    SliceLength, deref_mut_call, is_std_crate, known_by_bounds, range_step, rc_deref, rc_pointee, ref_deref,
    refcell_borrow, slice_length,
};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_index::IndexVec;
use rustc_index::bit_set::DenseBitSet;
use rustc_middle::mir::{
    self, BasicBlock, Body, Local, Location, Operand, Place, Rvalue, StatementKind, TerminatorKind,
};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_mir_dataflow::Analysis;
use rustc_mir_dataflow::impls::MaybeBorrowedLocals;
use rustc_span::def_id::DefId;

mod guards;
mod named;
mod types;
use named::writes_nothing;
pub(in crate::lower) use named::{keeps_vec_length, reads_only};
pub(in crate::lower) use types::changes_through_shared;
use types::{fixed_length, is_a_cell, is_a_pointer, pointee, pointers_in, scalar};

/// What a pointer may point into: a local's own storage, or what a local
/// that holds a pointer points to.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Target {
    Whole(Local),
    Pointee(Local),
}

impl Target {
    fn local(self) -> Local {
        match self {
            Target::Whole(l) | Target::Pointee(l) => l,
        }
    }
}

/// What a pointer may do to what it points to: change only what's inside
/// a cell of it, a shared reference's, `INSIDE`; change its items but not
/// its length, a reference to an item or a slice of it, `ITEMS`; or
/// replace it, `WHOLE`.
type Cap = u8;
const INSIDE: Cap = 0;
const ITEMS: Cap = 1;
const WHOLE: Cap = 2;

/// What a write does to what it reaches, as the transfer forgets of it:
/// its items written, a table's rows; its length grown, a `push`, whose
/// `x < len` holds; or replaced, nothing of it known.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(in crate::lower) enum Written {
    Items,
    Grows,
    Whole,
}

/// The targets a pointer may point into, each with what it may do to it;
/// and whether it may point where the body can't see, into any target that
/// has escaped.
#[derive(Clone, Default, PartialEq, Debug)]
struct Targets {
    places: IndexMap<Target, Cap>,
    unknown: bool,
}

impl Targets {
    /// `other`'s targets added, each replacing where either may: whether
    /// any was new.
    fn extend(&mut self, other: &Targets) -> bool {
        let mut changed = !self.unknown && other.unknown;
        self.unknown |= other.unknown;
        for (&t, &cap) in &other.places {
            match self.places.get_mut(&t) {
                Some(at) => {
                    if cap > *at {
                        *at = cap;
                        changed = true;
                    }
                }
                None => {
                    self.places.insert(t, cap);
                    changed = true;
                }
            }
        }
        changed
    }

    /// These targets, none replacing: what a shared reference to them may
    /// do.
    fn inside(self) -> Targets {
        self.capped(INSIDE)
    }

    /// These targets, none with more than `cap`.
    fn capped(mut self, cap: Cap) -> Targets {
        self.places.values_mut().for_each(|at| *at = (*at).min(cap));
        self
    }
}

/// What a body's statements may write (`Effects::of`).
pub(in crate::lower) struct Effects {
    /// Of each location, the targets it replaces whatever has escaped,
    /// and whether code not std's runs there or it writes through a
    /// pointer the body can't see, so it replaces each escaped target too.
    written: IndexMap<Location, (IndexMap<Target, Written>, bool)>,
    /// Of each local, the targets its own facts are of beside itself: of
    /// a pointer, what it points to, one pointer down; and whether one the
    /// body can't see.
    of: IndexVec<Local, Targets>,
    /// The targets escaped before each block, and where each escapes,
    /// each whether what escaped may replace it.
    escaped_before: IndexVec<BasicBlock, IndexMap<Target, Cap>>,
    escapes: IndexMap<Location, IndexMap<Target, Cap>>,
    /// The targets that are cells themselves, which what reaches them at
    /// all may replace; and those whose length never changes, a slice's,
    /// an array's, a `str`'s, whose items alone a write changes.
    cells: IndexSet<Target>,
    fixed_length: IndexSet<Target>,
    /// Of each location that runs code, what a `RefCell` guard held there
    /// points to: no code but through the guard writes it, as another
    /// borrow of the cell panics, in JS too, where `cell_borrows` counts
    /// the guard or has proved nothing borrows it; not where the code may
    /// be what it can't name, which may borrow it (`unnamed_may_borrow`).
    guarded: IndexMap<Location, IndexSet<Target>>,
}

impl Effects {
    /// What `body`'s statements may write. `libraries` is what
    /// `pointer_may_keep` asks; of none, every call not std's may keep
    /// what it's given. Each local from `made` on is one a rewrite of the
    /// body made of a place it reads through a pointer, a parameter's
    /// field, a capture or what a guard lends (`intervals::fields`,
    /// `captures`): what can be changed through a shared alias of it has
    /// escaped where the body starts, as a parameter's pointee has.
    pub(in crate::lower) fn of<'tcx>(
        tcx: TyCtxt<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
        body: &Body<'tcx>,
        libraries: Option<Libraries<'_, 'tcx>>,
        made: usize,
        unnamed_may_borrow: Option<&dyn Fn(Ty<'tcx>) -> bool>,
    ) -> Effects {
        let mut flow = Flow {
            tcx,
            typing_env,
            body,
            libraries,
            writes: IndexMap::default(),
            reaches: IndexMap::default(),
        };
        flow.seed(made);
        while flow.pass() {}
        let cells = flow.cells();
        let escapes = flow.escapes(made, &cells);
        let escaped_before = flow.escaped_before(&escapes);
        let of = (body.local_decls.indices())
            .map(|local| match is_a_pointer(tcx, body.local_decls[local].ty) {
                true => flow.reaches.get(&Target::Whole(local)).cloned().unwrap_or_default(),
                false => Targets::default(),
            })
            .collect();
        let borrowed = borrowed_at_writes(tcx, body);
        let mut written = IndexMap::default();
        for (block, data) in body.basic_blocks.iter_enumerated() {
            for statement_index in 0..=data.statements.len() {
                let location = Location { block, statement_index };
                written.insert(location, flow.written(location, borrowed.get(&location), &cells));
            }
        }
        let guarded = unnamed_may_borrow
            .map(|unnamed| flow.guarded(unnamed))
            .unwrap_or_default();
        let fixed_length = (body.local_decls.indices())
            .flat_map(|l| [Target::Whole(l), Target::Pointee(l)])
            .filter(|&t| flow.type_of(t).is_some_and(fixed_length))
            .collect();
        Effects {
            written,
            of,
            escaped_before,
            escapes,
            cells,
            fixed_length,
            guarded,
        }
    }

    /// Each local nothing known of survives what's at `location`: each
    /// whose own storage it may replace, or, of a pointer, what it points
    /// to.
    pub(in crate::lower) fn stale(&self, location: Location) -> IndexMap<Local, Written> {
        self.stale_guarded(location, true)
    }

    /// `stale`, but of no guard held: what the shadow says a guard keeps.
    pub(in crate::lower) fn stale_unguarded(&self, location: Location) -> IndexMap<Local, Written> {
        self.stale_guarded(location, false)
    }

    fn stale_guarded(&self, location: Location, guards: bool) -> IndexMap<Local, Written> {
        let Some((fixed, code)) = self.written.get(&location) else {
            return IndexMap::default();
        };
        let mut written = fixed.clone();
        let mut escaped_written = false;
        if *code {
            let guarded = self.guarded.get(&location).filter(|_| guards);
            for (t, cap) in self.escaped_at(location) {
                if guarded.is_some_and(|guarded| guarded.contains(&t)) {
                    continue;
                }
                if let Some(extent) = extent_of(t, cap, &self.cells, &self.fixed_length) {
                    let at = written.entry(t).or_insert(extent);
                    *at = (*at).max(extent);
                    escaped_written = true;
                }
            }
        }
        let mut stale: IndexMap<Local, Written> = IndexMap::default();
        let mut add = |local: Local, extent: Written| {
            let at = stale.entry(local).or_insert(extent);
            *at = (*at).max(extent);
        };
        for (&t, &extent) in &written {
            if let Target::Whole(l) = t {
                add(l, extent);
            }
        }
        for (local, of) in self.of.iter_enumerated() {
            for t in [Target::Pointee(local)].iter().chain(of.places.keys()) {
                if let Some(&extent) = written.get(t) {
                    add(local, extent);
                }
            }
            if of.unknown && escaped_written {
                add(local, Written::Whole);
            }
        }
        stale
    }

    /// The targets escaped just before `location`.
    fn escaped_at(&self, location: Location) -> IndexMap<Target, Cap> {
        let mut escaped = self.escaped_before[location.block].clone();
        for statement_index in 0..location.statement_index {
            let at = Location {
                block: location.block,
                statement_index,
            };
            for (&t, &cap) in self.escapes.get(&at).into_iter().flatten() {
                let entry = escaped.entry(t).or_insert(cap);
                *entry = (*entry).max(cap);
            }
        }
        escaped
    }
}

/// What code given what `start` holds may write: each target, and what
/// `reachable` reaches from it, with what it may do to it; of `whole`, the
/// place's own storage, what it reaches alone. Each target's cap a join of
/// every way to it, never an overwrite, so the answer is the targets', not
/// the order they come in (#2112 B 3b: an `insert` here made it the
/// order's, `ReactFlightClient`'s `written` one write more or less).
fn reached_from(start: &Targets, whole: Option<Target>, reachable: impl Fn(Target) -> Targets) -> Targets {
    let mut given = Targets {
        places: IndexMap::default(),
        unknown: start.unknown,
    };
    for (&t, &cap) in &start.places {
        if Some(t) == whole {
            given.extend(&reachable(t));
            continue;
        }
        let at = given.places.entry(t).or_insert(cap);
        *at = (*at).max(cap);
        let reached = reachable(t);
        given.extend(&if cap >= ITEMS { reached } else { reached.inside() });
    }
    given
}

/// The targets `place` reaches through its derefs, of the map `held`: of
/// none, its local's storage. Each replaced only where each pointer it
/// goes through may replace, or what it reaches is a cell.
fn through(held: &IndexMap<Target, Targets>, cells: Option<&IndexSet<Target>>, place: &Place<'_>) -> Targets {
    let mut at = Targets::default();
    at.places.insert(Target::Whole(place.local), WHOLE);
    for elem in place.projection.iter() {
        if elem == mir::PlaceElem::Deref {
            let mut next = Targets {
                places: IndexMap::default(),
                unknown: at.unknown,
            };
            for (t, &cap) in &at.places {
                if let Some(targets) = held.get(t) {
                    // Through what may change only inside a cell, what it
                    // holds may only so, but where it's a cell itself.
                    let through_a_cell = cells.is_some_and(|cells| cells.contains(t));
                    let targets = match cap >= ITEMS || through_a_cell {
                        true => targets.clone(),
                        false => targets.clone().inside(),
                    };
                    next.extend(&targets);
                }
            }
            at = next;
        }
    }
    at
}

/// The points-to map's fixed point, made (`Effects::of`).
struct Flow<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &'a Body<'tcx>,
    libraries: Option<Libraries<'a, 'tcx>>,
    /// Of each target, the pointers its memory may hold that may write
    /// what they point to; and `reaches`, of any pointer.
    writes: IndexMap<Target, Targets>,
    reaches: IndexMap<Target, Targets>,
}

impl<'tcx> Flow<'_, 'tcx> {
    /// Each local that is a pointer points to what it points to; and each
    /// that holds one and whose value comes from outside the body, a
    /// parameter, a local the rewrites made from `made` on, to what that
    /// holds. What else holds a pointer holds what the body put in it.
    fn seed(&mut self, made: usize) {
        for local in self.body.local_decls.indices() {
            let ty = self.body.local_decls[local].ty;
            let outside = (1..=self.body.arg_count).contains(&local.as_usize()) || local.as_usize() >= made;
            if !outside && !is_a_pointer(self.tcx, ty) {
                continue;
            }
            let held = pointers_in(self.tcx, self.typing_env, ty);
            if held.holds {
                (self.reaches.entry(Target::Whole(local)).or_default().places).insert(Target::Pointee(local), INSIDE);
            }
            if held.writes {
                let cap = if held.replaces { WHOLE } else { INSIDE };
                (self.writes.entry(Target::Whole(local)).or_default().places).insert(Target::Pointee(local), cap);
            }
        }
    }

    /// The type of what `target` is: a local's, or what a pointer it is
    /// points to; none where that can't be said.
    fn type_of(&self, target: Target) -> Option<Ty<'tcx>> {
        let ty = self.body.local_decls[target.local()].ty;
        match target {
            Target::Whole(_) => Some(ty),
            Target::Pointee(_) => pointee(self.tcx, ty),
        }
    }

    /// Each target that is a cell, `RefCell<_>`, `Cell<_>`, or a pointer to
    /// one, `Rc<RefCell<_>>`, which what reaches it at all may replace: what
    /// isn't `Freeze` past its pointers, or what can't be said. Not a `Vec`
    /// whose items hold one, whose length no shared reference changes.
    fn cells(&self) -> IndexSet<Target> {
        (self.body.local_decls.indices())
            .flat_map(|l| [Target::Whole(l), Target::Pointee(l)])
            .filter(|&t| {
                (self.writes.values().chain(self.reaches.values())).any(|targets| targets.places.contains_key(&t))
                    || matches!(t, Target::Whole(_))
            })
            .filter(|&t| {
                self.type_of(t)
                    .is_none_or(|ty| is_a_cell(self.tcx, self.typing_env, ty))
            })
            .collect()
    }

    /// Whether `target`'s memory may hold a pointer: a local's, as its type
    /// says; what a pointer points to, as what it points to says, or may.
    fn holds(&self, target: Target) -> bool {
        self.type_of(target)
            .is_none_or(|ty| pointers_in(self.tcx, self.typing_env, ty).holds)
    }

    /// `(writes, reaches)` stored into `target`'s memory, where it may hold
    /// a pointer: whether anything changed.
    fn store_into(&mut self, target: Target, (writes, reaches): &(Targets, Targets)) -> bool {
        let none = |t: &Targets| t.places.is_empty() && !t.unknown;
        if (none(writes) && none(reaches)) || !self.holds(target) {
            return false;
        }
        // A reference is typed: one to what isn't of its type, an item or a
        // slice of a `Vec`, `&mut [T]` of `deref_mut`, changes its items, not
        // its length, but where it's a cell.
        let mut writes = writes.clone();
        if let Target::Whole(local) = target
            && let ty::Ref(_, to, _) = *self.body.local_decls[local].ty.kind()
            && !known_by_bounds(to)
            && !to.is_trait()
        {
            for (t, cap) in writes.places.iter_mut() {
                let of = self.type_of(*t);
                let cell = of.is_none_or(|ty| is_a_cell(self.tcx, self.typing_env, ty));
                if !cell && of.is_some_and(|ty| ty != to) {
                    *cap = (*cap).min(ITEMS);
                }
            }
        }
        self.writes.entry(target).or_default().extend(&writes) | self.reaches.entry(target).or_default().extend(reaches)
    }

    /// `value` stored into `place`: into what its derefs reach, its
    /// local's storage of none.
    fn store(&mut self, place: &Place<'tcx>, value: &(Targets, Targets)) -> bool {
        let into = through(&self.writes, None, place);
        let mut changed = false;
        for t in into.places.into_keys() {
            changed |= self.store_into(t, value);
        }
        changed
    }

    /// One pass over the body: whether anything changed.
    fn pass(&mut self) -> bool {
        let mut changed = false;
        for data in self.body.basic_blocks.iter() {
            for statement in &data.statements {
                match &statement.kind {
                    StatementKind::Assign(assign) => {
                        let value = self.rvalue(&assign.1);
                        changed |= self.store(&assign.0, &value);
                    }
                    StatementKind::Intrinsic(intrinsic) => {
                        if let mir::NonDivergingIntrinsic::CopyNonOverlapping(copy) = &**intrinsic
                            && let Some(dst) = copy.dst.place()
                        {
                            // What `src` points to, read into what `dst` does.
                            let src = self.read_through(&self.operands([&copy.src]));
                            changed |= self.store(&dst.project_deeper(&[mir::PlaceElem::Deref], self.tcx), &src);
                        }
                    }
                    _ => {}
                }
            }
            if let TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
            {
                // What it's given, however many pointers down, its result may
                // hold, `deref_mut(&mut guard)`'s, and it may write into what
                // else it's given, `v.push(&mut x)`. Of code not std's, a
                // pointer to what's escaped, which it may have kept.
                let mut value = self.deep(self.operands(args.iter().map(|a| &a.node)));
                if call_runs_code(
                    self.tcx,
                    func.const_fn_def(),
                    args.iter().map(|a| a.node.ty(self.body, self.tcx)),
                ) {
                    let held = pointers_in(self.tcx, self.typing_env, destination.ty(self.body, self.tcx).ty);
                    value.0.unknown |= held.writes;
                    value.1.unknown |= held.holds;
                }
                changed |= self.store(destination, &value);
                for arg in args.iter() {
                    if let Some(place) = arg.node.place()
                        && place.ty(self.body, self.tcx).ty.builtin_deref(true).is_some()
                    {
                        let deref = place.project_deeper(&[mir::PlaceElem::Deref], self.tcx);
                        changed |= self.store(&deref, &value);
                    }
                }
            }
        }
        changed
    }

    /// `value` and each target reachable from its pointers, however many
    /// down, each replaced where each pointer to it may.
    fn deep(&self, (writes, reaches): (Targets, Targets)) -> (Targets, Targets) {
        let mut out = (writes.clone(), reaches.clone());
        for (&t, &cap) in &writes.places {
            let reached = self.reachable(t, true);
            out.0.extend(&if cap >= ITEMS { reached } else { reached.inside() });
        }
        for &t in reaches.places.keys() {
            out.1.extend(&self.reachable(t, false));
        }
        out
    }

    /// What the pointers `pointers` point to hold: a read through them.
    fn read_through(&self, (writes, reaches): &(Targets, Targets)) -> (Targets, Targets) {
        let mut out = (Targets::default(), Targets::default());
        out.0.unknown = writes.unknown;
        out.1.unknown = reaches.unknown;
        for t in reaches.places.keys() {
            if let Some(held) = self.writes.get(t) {
                out.0.extend(held);
            }
            if let Some(held) = self.reaches.get(t) {
                out.1.extend(held);
            }
        }
        out
    }

    /// What `rvalue` makes may write, and may reach.
    fn rvalue(&self, rvalue: &Rvalue<'tcx>) -> (Targets, Targets) {
        match rvalue {
            Rvalue::Ref(_, mir::BorrowKind::Fake(_), _) | Rvalue::RawPtr(mir::RawPtrKind::FakeForPtrMetadata, _) => {
                Default::default()
            }
            Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, place) | Rvalue::RawPtr(_, place) => {
                self.borrow(place, Some(WHOLE))
            }
            Rvalue::Reborrow(_, mutability, place) if mutability.is_mut() => self.borrow(place, Some(WHOLE)),
            Rvalue::Ref(_, _, place) | Rvalue::Reborrow(_, _, place) => {
                let changes = changes_through_shared(self.tcx, self.typing_env, place.ty(self.body, self.tcx).ty);
                self.borrow(place, changes.then_some(INSIDE))
            }
            Rvalue::Cast(mir::CastKind::PointerWithExposedProvenance, ..) | Rvalue::ThreadLocalRef(_) => {
                let unknown = Targets {
                    places: IndexMap::default(),
                    unknown: true,
                };
                (unknown.clone(), unknown)
            }
            Rvalue::Use(operand, _) | Rvalue::Cast(_, operand, _) | Rvalue::Repeat(operand, _) => {
                self.operands([operand])
            }
            Rvalue::WrapUnsafeBinder(operand, _) => self.operands([operand]),
            Rvalue::CopyForDeref(place) => self.operands([&Operand::Copy(*place)]),
            Rvalue::Aggregate(_, fields) => self.operands(fields.iter()),
            _ => Default::default(),
        }
    }

    /// What a borrow of `place` points into: what its derefs reach, its
    /// local's storage of none; with what it may do to it, of one that may
    /// write.
    fn borrow(&self, place: &Place<'tcx>, writes: Option<Cap>) -> (Targets, Targets) {
        let mut reaches = through(&self.reaches, None, place).inside();
        reaches.extend(&through(&self.writes, None, place).inside());
        let writes = match writes {
            Some(cap) => through(&self.writes, None, place).capped(cap),
            None => Targets::default(),
        };
        (writes, reaches)
    }

    /// What reading `operands` gives: the pointers each place holds, read
    /// through a deref or not; of a constant that holds a pointer, a
    /// static's address, what the body can't see.
    fn operands<'o>(&self, operands: impl IntoIterator<Item = &'o Operand<'tcx>>) -> (Targets, Targets)
    where
        'tcx: 'o,
    {
        let (mut writes, mut reaches) = (Targets::default(), Targets::default());
        for operand in operands {
            if let Some(place) = operand.place() {
                let mut from = through(&self.reaches, None, &place);
                from.extend(&through(&self.writes, None, &place));
                for t in from.places.keys() {
                    if let Some(held) = self.writes.get(t) {
                        writes.extend(held);
                    }
                    if let Some(held) = self.reaches.get(t) {
                        reaches.extend(held);
                    }
                }
                writes.unknown |= from.unknown;
                reaches.unknown |= from.unknown;
            } else if let Operand::Constant(constant) = operand {
                let held = pointers_in(self.tcx, self.typing_env, constant.ty());
                writes.unknown |= held.writes;
                reaches.unknown |= held.holds;
            }
        }
        (writes, reaches)
    }

    /// Each target reachable from `target` through the pointers its memory
    /// holds, however many down, and whether one the body can't see:
    /// through those that may write, of `writing`, each replaced where each
    /// pointer to it may, or it's reached through a cell; else any.
    fn reachable(&self, target: Target, writing: bool) -> Targets {
        let map = if writing { &self.writes } else { &self.reaches };
        let mut reached = Targets::default();
        let mut work = vec![(target, WHOLE)];
        let mut seen: IndexSet<(Target, Cap)> = IndexSet::default();
        while let Some((t, cap)) = work.pop() {
            if !seen.insert((t, cap)) {
                continue;
            }
            let Some(held) = map.get(&t) else {
                continue;
            };
            reached.unknown |= held.unknown;
            // What may write its items may write through the pointers they
            // are; what may change only inside a cell, only so, but of a cell.
            let mutable = cap >= ITEMS
                || self
                    .type_of(t)
                    .is_none_or(|ty| is_a_cell(self.tcx, self.typing_env, ty));
            for (&next, &next_cap) in &held.places {
                let cap = if mutable { next_cap } else { INSIDE };
                reached.extend(&Targets {
                    places: IndexMap::from_iter([(next, cap)]),
                    unknown: false,
                });
                work.push((next, cap));
            }
        }
        reached
    }

    /// The targets what's at `location` replaces whatever has escaped, of
    /// `borrowed` there, and whether it also replaces each escaped one: a
    /// call or a drop that runs code not std's, or a write through what the
    /// body can't see.
    fn written(
        &self,
        location: Location,
        borrowed: Option<&DenseBitSet<Local>>,
        cells: &IndexSet<Target>,
    ) -> (IndexMap<Target, Written>, bool) {
        let mut written = Targets::default();
        let data = &self.body.basic_blocks[location.block];
        if let Some(statement) = data.statements.get(location.statement_index) {
            match &statement.kind {
                StatementKind::Assign(assign) => {
                    self.write_place(&assign.0, (cells, borrowed), &mut written);
                    // A scalar written through a pointer to an item, `board[r][c]
                    // = x`, `*n += 1`, changes no length, a row's neither.
                    if assign.0.is_indirect() && scalar(assign.1.ty(self.body, self.tcx)) {
                        written.places.retain(|_, cap| *cap != ITEMS);
                    }
                }
                StatementKind::SetDiscriminant { place, .. } => {
                    self.write_place(place, (cells, borrowed), &mut written)
                }
                StatementKind::Intrinsic(intrinsic) => {
                    if let mir::NonDivergingIntrinsic::CopyNonOverlapping(copy) = &**intrinsic
                        && let Some(dst) = copy.dst.place()
                    {
                        let deref = dst.project_deeper(&[mir::PlaceElem::Deref], self.tcx);
                        self.write_place(&deref, (cells, borrowed), &mut written);
                    }
                }
                _ => {}
            }
            return self.replaced(written, IndexSet::default(), cells, false);
        }
        let mut code = false;
        let mut grows = IndexSet::default();
        match &data.terminator().kind {
            TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => {
                self.write_place(destination, (cells, borrowed), &mut written);
                let callee = func.const_fn_def();
                code = call_runs_code(self.tcx, callee, args.iter().map(|a| a.node.ty(self.body, self.tcx)));
                // A `Vec`'s `push`, std's, grows what its `&mut` points to,
                // what its signature can't say: `x < len` holds past it.
                let pushed = !code
                    && callee.is_some_and(|(callee, generic)| {
                        slice_length(self.tcx, callee, generic) == Some(SliceLength::Push)
                    });
                if pushed && let Some(vec) = args.first().and_then(|a| a.node.place()) {
                    let into = through(
                        &self.writes,
                        Some(cells),
                        &vec.project_deeper(&[mir::PlaceElem::Deref], self.tcx),
                    );
                    grows.extend(
                        only_borrowed(into, borrowed)
                            .places
                            .into_iter()
                            .filter(|&(_, cap)| cap >= ITEMS)
                            .map(|(t, _)| t),
                    );
                } else if code || !callee.is_some_and(|(callee, args)| writes_nothing(self.tcx, callee, args)) {
                    for arg in args.iter() {
                        if let Some(place) = arg.node.place() {
                            written.extend(&only_borrowed(self.reached_by(&place), borrowed));
                        }
                    }
                }
            }
            TerminatorKind::Drop { place, .. } => {
                written.places.insert(Target::Whole(place.local), WHOLE);
                code =
                    crate::lower::analysis::drop_runs_code(self.tcx, self.typing_env, place.ty(self.body, self.tcx).ty);
                if code {
                    written.extend(&only_borrowed(self.reached_by(place), borrowed));
                }
            }
            TerminatorKind::InlineAsm { .. } | TerminatorKind::Yield { .. } => {
                for l in self.body.local_decls.indices() {
                    written.places.insert(Target::Whole(l), WHOLE);
                    written.places.insert(Target::Pointee(l), WHOLE);
                }
                code = true;
            }
            _ => {}
        }
        self.replaced(written, grows, cells, code)
    }

    /// What code given `place` may write: each target a pointer it holds
    /// reaches, however many down, as `reachable` says, not itself.
    fn reached_by(&self, place: &Place<'tcx>) -> Targets {
        let start = through(&self.writes, None, place);
        let whole = (!place.is_indirect()).then_some(Target::Whole(place.local));
        reached_from(&start, whole, |t| self.reachable(t, true))
    }

    /// A write to `place`: its local's storage, and of a write through a
    /// pointer, what the pointer may point into, with what it may do to it:
    /// `*n += 1` of an item's `&mut` its items alone.
    fn write_place(
        &self,
        place: &Place<'tcx>,
        (cells, borrowed): (&IndexSet<Target>, Option<&DenseBitSet<Local>>),
        written: &mut Targets,
    ) {
        // Not the pointer a write goes through, `*n = 1`'s `n`, but what it
        // points to.
        if !place.is_indirect() {
            written.places.insert(Target::Whole(place.local), WHOLE);
            return;
        }
        let targets = through(&self.writes, Some(cells), place);
        written.extend(&only_borrowed(targets, borrowed));
    }

    /// Of `written`, what's done to each target, `extent_of`, and of
    /// `grows`, each grown at least. And whether each escaped target is
    /// written too: of code that runs, or a pointer the body can't see.
    fn replaced(
        &self,
        written: Targets,
        grows: IndexSet<Target>,
        cells: &IndexSet<Target>,
        code: bool,
    ) -> (IndexMap<Target, Written>, bool) {
        let fixed: IndexSet<Target> = (written.places.keys().chain(&grows).copied())
            .filter(|&t| self.type_of(t).is_some_and(fixed_length))
            .collect();
        let mut replaced: IndexMap<Target, Written> = (written.places.into_iter())
            .filter_map(|(t, cap)| extent_of(t, cap, cells, &fixed).map(|extent| (t, extent)))
            .collect();
        for t in grows {
            let extent = if fixed.contains(&t) {
                Written::Items
            } else {
                Written::Grows
            };
            let at = replaced.entry(t).or_insert(extent);
            *at = (*at).max(extent);
        }
        (replaced, code || written.unknown)
    }

    /// Where each target escapes, and whether what escaped may replace it:
    /// a raw pointer made of it; a call that may keep a pointer it's given;
    /// a pointer stored where the body can't see, or into what a parameter
    /// points to; what a drop that runs code, a yield or inline assembly is
    /// given; and each target reachable from what escapes. What a
    /// parameter, or a local the rewrites made, that can be changed through
    /// an alias its caller keeps points to escapes at the body's start.
    fn escapes(&self, made: usize, cells: &IndexSet<Target>) -> IndexMap<Location, IndexMap<Target, Cap>> {
        let mut escapes: IndexMap<Location, IndexMap<Target, Cap>> = IndexMap::default();
        let escape = |escapes: &mut IndexMap<Location, IndexMap<Target, Cap>>, at: Location, targets: &Targets| {
            let entry = escapes.entry(at).or_default();
            for (&t, &cap) in &targets.places {
                let at = entry.entry(t).or_insert(cap);
                *at = (*at).max(cap);
            }
        };
        let start = Location::START;
        for param in (1..=self.body.arg_count).map(Local::from_usize) {
            if changes_through_shared(self.tcx, self.typing_env, self.body.local_decls[param].ty) {
                let mut targets = Targets::default();
                targets.places.insert(Target::Pointee(param), INSIDE);
                escape(&mut escapes, start, &targets);
            }
        }
        for local in (made..self.body.local_decls.len()).map(Local::from_usize) {
            if changes_through_shared(self.tcx, self.typing_env, self.body.local_decls[local].ty) {
                let mut targets = Targets::default();
                targets.places.insert(Target::Whole(local), INSIDE);
                escape(&mut escapes, start, &targets);
            }
        }
        let outside =
            |t: &Target| matches!(t, Target::Pointee(l) if l.as_usize() >= 1 && l.as_usize() <= self.body.arg_count);
        for (block, data) in self.body.basic_blocks.iter_enumerated() {
            for (statement_index, statement) in data.statements.iter().enumerate() {
                let location = Location { block, statement_index };
                if let StatementKind::Assign(assign) = &statement.kind {
                    if let Rvalue::RawPtr(kind, place) = &assign.1
                        && *kind != mir::RawPtrKind::FakeForPtrMetadata
                    {
                        escape(&mut escapes, location, &through(&self.writes, None, place));
                    }
                    let into = through(&self.writes, None, &assign.0);
                    if assign.0.is_indirect() && (into.unknown || into.places.keys().any(outside)) {
                        escape(&mut escapes, location, &self.rvalue(&assign.1).0);
                    }
                }
            }
            let location = self.body.terminator_loc(block);
            let given: Vec<Place<'tcx>> = match &data.terminator().kind {
                TerminatorKind::Drop { place, .. }
                    if crate::lower::analysis::drop_runs_code(
                        self.tcx,
                        self.typing_env,
                        place.ty(self.body, self.tcx).ty,
                    ) =>
                {
                    vec![*place]
                }
                TerminatorKind::Yield { value, .. } => value.place().into_iter().collect(),
                TerminatorKind::InlineAsm { operands, .. } => (operands.iter())
                    .filter_map(|operand| match operand {
                        mir::InlineAsmOperand::In { value, .. }
                        | mir::InlineAsmOperand::InOut { in_value: value, .. } => value.place(),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            for place in given {
                escape(&mut escapes, location, &self.reached_by(&place));
            }
            if let TerminatorKind::Call { func, args, .. } = &data.terminator().kind {
                let callee = func.const_fn_def();
                for arg in args.iter() {
                    if self.call_holds_past(callee, args.iter().map(|a| a.node.ty(self.body, self.tcx)), &arg.node)
                        && let Some(place) = arg.node.place()
                    {
                        escape(&mut escapes, location, &self.reached_by(&place));
                    }
                }
            }
        }
        // What an escaped pointer points into escapes with it.
        for targets in escapes.values_mut() {
            let more: Vec<(Target, Cap)> = (targets.iter())
                .flat_map(|(&t, &cap)| {
                    let reached = self.reachable(t, true);
                    let mutable = cap >= ITEMS || cells.contains(&t);
                    (if mutable { reached } else { reached.inside() }).places
                })
                .collect();
            for (t, cap) in more {
                let at = targets.entry(t).or_insert(cap);
                *at = (*at).max(cap);
            }
        }
        escapes
    }

    /// The targets escaped before each block: what escapes on a way to it,
    /// a forward flow that never forgets.
    fn escaped_before(
        &self,
        escapes: &IndexMap<Location, IndexMap<Target, Cap>>,
    ) -> IndexVec<BasicBlock, IndexMap<Target, Cap>> {
        let blocks = &self.body.basic_blocks;
        let mut before: IndexVec<BasicBlock, IndexMap<Target, Cap>> = IndexVec::from_elem(IndexMap::default(), blocks);
        let add = |into: &mut IndexMap<Target, Cap>, from: &IndexMap<Target, Cap>| {
            let mut changed = false;
            for (&t, &cap) in from {
                match into.get_mut(&t) {
                    Some(at) if cap > *at => {
                        *at = cap;
                        changed = true;
                    }
                    Some(_) => {}
                    None => {
                        into.insert(t, cap);
                        changed = true;
                    }
                }
            }
            changed
        };
        if let Some(start) = escapes.get(&Location::START) {
            add(&mut before[mir::START_BLOCK], start);
        }
        let mut changed = true;
        while changed {
            changed = false;
            for &block in blocks.reverse_postorder() {
                let mut after = before[block].clone();
                for statement_index in 0..=blocks[block].statements.len() {
                    if let Some(at) = escapes.get(&Location { block, statement_index }) {
                        add(&mut after, at);
                    }
                }
                for succ in blocks[block].terminator().successors() {
                    changed |= add(&mut before[succ], &after);
                }
            }
        }
        before
    }

    /// Whether a call of `callee`, given arguments of `given`, may hold
    /// past it a pointer `arg` holds: none of what holds none, nor of a
    /// call that runs no code but std's, whose signature lets one out
    /// alone, which the flow of its result holds; else as the escape owner
    /// says (`pointers::argument_may_keep`), where the walk knows the
    /// libraries; else, may.
    fn call_holds_past(
        &self,
        callee: Option<(DefId, ty::GenericArgsRef<'tcx>)>,
        given: impl Iterator<Item = Ty<'tcx>>,
        arg: &Operand<'tcx>,
    ) -> bool {
        let Some(place) = arg.place() else {
            return false;
        };
        let held = self.reachable(Target::Whole(place.local), true);
        if held.places.keys().all(|&t| t == Target::Whole(place.local)) && !held.unknown {
            return false;
        }
        if !call_runs_code(self.tcx, callee, given) {
            return false;
        }
        let (Some(call), Some(libraries)) = (callee, &self.libraries) else {
            return true;
        };
        argument_may_keep(
            self.tcx,
            self.typing_env,
            libraries,
            call,
            arg.ty(self.body, self.tcx),
            &|t| pointers_in(self.tcx, self.typing_env, t).writes,
        )
    }
}

/// `targets` but each whole local a write reaches through a pointer that
/// rustc's `MaybeBorrowedLocals` says can't be borrowed there, `borrowed`.
fn only_borrowed(mut targets: Targets, borrowed: Option<&DenseBitSet<Local>>) -> Targets {
    targets.places.retain(|t, _| match t {
        Target::Whole(l) => borrowed.is_none_or(|set| set.contains(*l)),
        Target::Pointee(_) => true,
    });
    targets
}

/// What a write with `cap` does to `target`: replaces it, or changes its
/// items, of one whose length is `fixed` only its items; of a cell,
/// replaces it however it's reached; else, through a shared reference,
/// nothing a fact is of.
fn extent_of(target: Target, cap: Cap, cells: &IndexSet<Target>, fixed: &IndexSet<Target>) -> Option<Written> {
    let cap = if cells.contains(&target) { WHOLE } else { cap };
    match cap {
        WHOLE if fixed.contains(&target) => Some(Written::Items),
        WHOLE => Some(Written::Whole),
        ITEMS => Some(Written::Items),
        _ => None,
    }
}

/// Whether a call of `callee` given arguments of `given` may run code that
/// isn't std's: a call it can't resolve, one of the crate's or a
/// library's, or std's given what carries code, a closure, a `dyn`, a
/// type of the crate's or a parameter.
fn call_runs_code<'tcx>(
    tcx: TyCtxt<'tcx>,
    callee: Option<(DefId, ty::GenericArgsRef<'tcx>)>,
    mut given: impl Iterator<Item = Ty<'tcx>>,
) -> bool {
    let Some((callee, args)) = callee else {
        return true;
    };
    !is_std_crate(tcx, callee)
        || args.types().any(|t| crate::lower::analysis::carries_code(tcx, t, true))
        || given.any(|t| crate::lower::analysis::carries_code(tcx, t, true))
}

/// Of each location that may write through a pointer, call or drop, the
/// locals rustc's `MaybeBorrowedLocals` says may be borrowed just before it.
fn borrowed_at_writes<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
) -> crate::collections::IndexMap<Location, DenseBitSet<Local>> {
    let mut cursor = MaybeBorrowedLocals
        .iterate_to_fixpoint(tcx, body, None)
        .into_results_cursor(body);
    let mut at = crate::collections::IndexMap::default();
    for (block, data) in body.basic_blocks.iter_enumerated() {
        for (statement_index, statement) in data.statements.iter().enumerate() {
            let indirect = match &statement.kind {
                StatementKind::Assign(assign) => assign.0.is_indirect(),
                StatementKind::SetDiscriminant { place, .. } => place.is_indirect(),
                StatementKind::Intrinsic(_) => true,
                _ => false,
            };
            if indirect {
                let location = Location { block, statement_index };
                cursor.seek_before_primary_effect(location);
                at.insert(location, cursor.get().clone());
            }
        }
        let location = body.terminator_loc(block);
        if matches!(
            data.terminator().kind,
            TerminatorKind::Call { .. } | TerminatorKind::Drop { .. }
        ) {
            cursor.seek_before_primary_effect(location);
            at.insert(location, cursor.get().clone());
        }
    }
    at
}

#[cfg(test)]
mod tests {
    use super::{INSIDE, IndexMap, Local, Target, Targets, WHOLE, reached_from};

    fn targets(places: &[(Target, u8)]) -> Targets {
        Targets {
            places: places.iter().copied().collect::<IndexMap<_, _>>(),
            unknown: false,
        }
    }

    // `b` reaches what `a` points to whole; `a` itself is given only
    // inside. Whichever comes first, `a` is reached whole: the join keeps
    // the more of the two ways to it, as the fixed point's monotone
    // transfer must (#2112 B 3b).
    #[test]
    fn what_code_reaches_is_the_targets_never_their_order() {
        let (a, b) = (Target::Pointee(Local::from_u32(1)), Target::Pointee(Local::from_u32(2)));
        let reachable = |t: Target| match t == b {
            true => targets(&[(a, WHOLE)]),
            false => Targets::default(),
        };
        let forward = reached_from(&targets(&[(a, INSIDE), (b, WHOLE)]), None, reachable);
        let backward = reached_from(&targets(&[(b, WHOLE), (a, INSIDE)]), None, reachable);
        assert_eq!(forward.places.get(&a), Some(&WHOLE));
        assert_eq!(backward.places.get(&a), Some(&WHOLE));
    }
}
