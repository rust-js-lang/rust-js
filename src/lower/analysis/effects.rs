//! What a statement may write: the one answer the MIR intervals ask of
//! each fact they keep (#2112 B, ADR 1137 amended). A fact survives a
//! statement only where no local it's of may be written there; what can't
//! be seen may write, so it fails closed.
//!
//! What a pointer may point into is a target: a local's own storage
//! (`Whole`), or what a local that holds a pointer points to (`Pointee`),
//! so a fact of a reference's length is of the reference, and a pointer
//! assigned anew changes nothing it pointed to. Which pointers point where
//! is a flow-insensitive map of what each target's memory holds: a `&mut`,
//! a raw pointer, or a shared reference to what can be changed through one,
//! `&RefCell<_>` (`changes_through_shared`, rustc's `Freeze` refined to what
//! it holds); each copy, each value holding one, each call given one and
//! each place written through one, of what can hold a pointer.
//!
//! A target is written where a statement assigns its local; where a write
//! goes through a pointer to it, of a whole local only one rustc's
//! `MaybeBorrowedLocals` says may be borrowed there; where a call or a drop
//! is given a pointer to it; or where code that isn't std's runs past
//! where a pointer to it escaped. A pointer escapes where it's made raw,
//! given a call that may keep it (`pointers::pointer_may_keep` and
//! `closure_may_keep`, asked, not re-derived), or written into what's
//! escaped or the body can't see: from there on, a forward flow. What a
//! parameter that can be changed through an alias its caller keeps points
//! to has escaped where the body starts. A dead local isn't written: it's
//! read again only once it's assigned.

use crate::collections::{IndexMap, IndexSet};
use crate::lower::pointers::{Libraries, argument_may_keep, raw_pointee};
use crate::lower::recognition::is_std_crate;
use rustc_index::IndexVec;
use rustc_index::bit_set::DenseBitSet;
use rustc_middle::mir::{
    self, BasicBlock, Body, Local, Location, Operand, Place, Rvalue, StatementKind, TerminatorKind,
};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_mir_dataflow::Analysis;
use rustc_mir_dataflow::impls::MaybeBorrowedLocals;
use rustc_span::def_id::DefId;

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

/// The targets a pointer may point into, and whether it may point where
/// the body can't see, into any target that has escaped.
#[derive(Clone, Default, PartialEq, Debug)]
struct Targets {
    places: IndexSet<Target>,
    unknown: bool,
}

impl Targets {
    /// `other`'s targets added: whether any was new.
    fn extend(&mut self, other: &Targets) -> bool {
        let before = (self.places.len(), self.unknown);
        self.places.extend(other.places.iter().copied());
        self.unknown |= other.unknown;
        before != (self.places.len(), self.unknown)
    }
}

/// What a body's statements may write (`Effects::of`).
pub(in crate::lower) struct Effects {
    /// Of each target, the targets of the pointers its memory may hold
    /// that may write: `writes`; and of any pointer: `reaches`.
    writes: IndexMap<Target, Targets>,
    /// Of each local, each target a pointer reachable from it points into,
    /// however many pointers down, and whether one the body can't see.
    reached: IndexVec<Local, Targets>,
    /// The targets escaped before each block, and where each escapes.
    escaped_before: IndexVec<BasicBlock, IndexSet<Target>>,
    escapes: IndexMap<Location, IndexSet<Target>>,
    /// Of each location that writes through a pointer, or calls, or drops,
    /// the locals rustc's `MaybeBorrowedLocals` says may be borrowed there.
    borrowed: IndexMap<Location, DenseBitSet<Local>>,
    /// The calls and drops that may run code that isn't std's, by their
    /// block.
    runs_code: IndexSet<BasicBlock>,
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
    ) -> Effects {
        let mut flow = Flow {
            tcx,
            typing_env,
            body,
            libraries,
            writes: IndexMap::default(),
            reaches: IndexMap::default(),
        };
        flow.seed();
        while flow.pass() {}
        let escapes = flow.escapes(made);
        let escaped_before = flow.escaped_before(&escapes);
        let reached = (body.local_decls.indices())
            .map(|local| flow.reachable(Target::Whole(local), false))
            .collect();
        let runs_code = (body.basic_blocks.iter_enumerated())
            .filter(|(_, data)| match &data.terminator().kind {
                TerminatorKind::Call { func, args, .. } => {
                    call_runs_code(tcx, func.const_fn_def(), args.iter().map(|a| a.node.ty(body, tcx)))
                }
                TerminatorKind::Drop { place, .. } => {
                    crate::lower::analysis::drop_runs_code(tcx, typing_env, place.ty(body, tcx).ty)
                }
                TerminatorKind::InlineAsm { .. } | TerminatorKind::Yield { .. } => true,
                _ => false,
            })
            .map(|(block, _)| block)
            .collect();
        Effects {
            writes: flow.writes,
            reached,
            escaped_before,
            escapes,
            borrowed: borrowed_at_writes(tcx, body),
            runs_code,
        }
    }

    /// Each local what's at `location` of `body` may write, or whose
    /// pointers reach a target it may write: what nothing known of may
    /// survive it.
    pub(in crate::lower) fn stale(&self, body: &Body<'_>, location: Location) -> IndexSet<Local> {
        let escaped = self.escaped_at(location);
        let written = self.written(body, location, &escaped);
        let written_escaped = written.iter().any(|t| escaped.contains(t));
        let mut stale: IndexSet<Local> = written.iter().map(|t| t.local()).collect();
        stale.extend((self.reached.iter_enumerated()).filter_map(|(local, reached)| {
            (reached.places.iter().any(|t| written.contains(t)) || (reached.unknown && written_escaped))
                .then_some(local)
        }));
        stale
    }

    /// The targets escaped just before `location`.
    fn escaped_at(&self, location: Location) -> IndexSet<Target> {
        let mut escaped = self.escaped_before[location.block].clone();
        for statement_index in 0..location.statement_index {
            let at = Location {
                block: location.block,
                statement_index,
            };
            escaped.extend(self.escapes.get(&at).into_iter().flatten().copied());
        }
        escaped
    }

    /// Each target what's at `location` may write, `escaped` those
    /// escaped there.
    fn written(&self, body: &Body<'_>, location: Location, escaped: &IndexSet<Target>) -> IndexSet<Target> {
        let mut written = IndexSet::default();
        let data = &body.basic_blocks[location.block];
        if let Some(statement) = data.statements.get(location.statement_index) {
            match &statement.kind {
                StatementKind::Assign(assign) => self.write_place(location, &assign.0, escaped, &mut written),
                StatementKind::SetDiscriminant { place, .. } => {
                    self.write_place(location, place, escaped, &mut written)
                }
                StatementKind::Intrinsic(intrinsic) => {
                    if let mir::NonDivergingIntrinsic::CopyNonOverlapping(copy) = &**intrinsic
                        && let Some(dst) = copy.dst.place()
                    {
                        let targets = self.through(&dst, true);
                        self.write_targets(location, &targets, escaped, &mut written);
                    }
                }
                _ => {}
            }
            return written;
        }
        let code = self.runs_code.contains(&location.block);
        match &data.terminator().kind {
            TerminatorKind::Call { args, destination, .. } => {
                self.write_place(location, destination, escaped, &mut written);
                for arg in args.iter() {
                    if let Some(place) = arg.node.place() {
                        let targets = self.reachable_from(&place, true);
                        self.write_targets(location, &targets, escaped, &mut written);
                    }
                }
            }
            TerminatorKind::Drop { place, .. } => {
                written.insert(Target::Whole(place.local));
                if code {
                    let targets = self.reachable_from(place, true);
                    self.write_targets(location, &targets, escaped, &mut written);
                }
            }
            TerminatorKind::InlineAsm { .. } | TerminatorKind::Yield { .. } => {
                written.extend(
                    body.local_decls
                        .indices()
                        .flat_map(|l| [Target::Whole(l), Target::Pointee(l)]),
                );
            }
            _ => {}
        }
        if code {
            written.extend(escaped.iter().copied());
        }
        written
    }

    /// What a write to `place` at `location` writes: its local's storage,
    /// and of a write through a pointer, what the pointer may point into.
    fn write_place(
        &self,
        location: Location,
        place: &Place<'_>,
        escaped: &IndexSet<Target>,
        written: &mut IndexSet<Target>,
    ) {
        written.insert(Target::Whole(place.local));
        if place.is_indirect() {
            let targets = self.through(place, true);
            self.write_targets(location, &targets, escaped, written);
        }
    }

    /// `targets` written at `location`: of a whole local, only one rustc's
    /// `MaybeBorrowedLocals` says may be borrowed there; of what the body
    /// can't see, each target escaped.
    fn write_targets(
        &self,
        location: Location,
        targets: &Targets,
        escaped: &IndexSet<Target>,
        written: &mut IndexSet<Target>,
    ) {
        let borrowed = self.borrowed.get(&location);
        written.extend(targets.places.iter().copied().filter(|t| match t {
            Target::Whole(l) => borrowed.is_none_or(|set| set.contains(*l)),
            Target::Pointee(_) => true,
        }));
        if targets.unknown {
            written.extend(escaped.iter().copied());
        }
    }

    /// The targets `place`'s pointers point into, each deref it reads
    /// through followed: what a write to it, or a borrow of it, reaches.
    fn through(&self, place: &Place<'_>, writing: bool) -> Targets {
        through(&self.writes, place, writing)
    }

    /// Each target a pointer reachable from `place` points into, however
    /// many pointers down: what code given it may write.
    fn reachable_from(&self, place: &Place<'_>, writing: bool) -> Targets {
        let start = match place.is_indirect() {
            true => self.through(place, writing),
            false => Targets {
                places: IndexSet::from_iter([Target::Whole(place.local)]),
                unknown: false,
            },
        };
        let mut reached = start.clone();
        let mut work: Vec<Target> = start.places.iter().copied().collect();
        while let Some(t) = work.pop() {
            if let Some(held) = self.writes.get(&t) {
                reached.unknown |= held.unknown;
                for &next in &held.places {
                    if reached.places.insert(next) {
                        work.push(next);
                    }
                }
            }
        }
        // Not what it is, which the call reads, but what it points to.
        if !place.is_indirect() {
            reached.places.shift_remove(&Target::Whole(place.local));
        }
        reached
    }
}

/// The targets `place` reaches through its derefs, of the map `held`: of
/// none, its local's storage.
fn through(held: &IndexMap<Target, Targets>, place: &Place<'_>, _: bool) -> Targets {
    let mut at = Targets {
        places: IndexSet::from_iter([Target::Whole(place.local)]),
        unknown: false,
    };
    for elem in place.projection.iter() {
        if elem == mir::PlaceElem::Deref {
            let mut next = Targets {
                places: IndexSet::default(),
                unknown: at.unknown,
            };
            for t in &at.places {
                if let Some(targets) = held.get(t) {
                    next.extend(targets);
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
    writes: IndexMap<Target, Targets>,
    reaches: IndexMap<Target, Targets>,
}

impl<'tcx> Flow<'_, 'tcx> {
    /// Each local that holds a pointer points to what it points to.
    fn seed(&mut self) {
        for local in self.body.local_decls.indices() {
            let (holds, writes) = pointers_in(self.tcx, self.typing_env, self.body.local_decls[local].ty);
            if holds {
                self.reaches
                    .entry(Target::Whole(local))
                    .or_default()
                    .places
                    .insert(Target::Pointee(local));
            }
            if writes {
                self.writes
                    .entry(Target::Whole(local))
                    .or_default()
                    .places
                    .insert(Target::Pointee(local));
            }
        }
    }

    /// Whether `target`'s memory may hold a pointer: a local's, as its type
    /// says; what a pointer points to, as what it points to says, or may.
    fn holds(&self, target: Target) -> bool {
        let ty = self.body.local_decls[target.local()].ty;
        match target {
            Target::Whole(_) => pointers_in(self.tcx, self.typing_env, ty).0,
            Target::Pointee(_) => ty
                .builtin_deref(true)
                .is_none_or(|to| pointers_in(self.tcx, self.typing_env, to).0),
        }
    }

    /// `(writes, reaches)` stored into `target`'s memory, where it may hold
    /// a pointer: whether anything changed.
    fn store_into(&mut self, target: Target, (writes, reaches): &(Targets, Targets)) -> bool {
        if (writes.places.is_empty() && !writes.unknown && reaches.places.is_empty() && !reaches.unknown)
            || !self.holds(target)
        {
            return false;
        }
        self.writes.entry(target).or_default().extend(writes) | self.reaches.entry(target).or_default().extend(reaches)
    }

    /// `value` stored into `place`: into what its derefs reach, its
    /// local's storage of none.
    fn store(&mut self, place: &Place<'tcx>, value: &(Targets, Targets)) -> bool {
        let into = through(&self.writes, place, true);
        let mut changed = false;
        for t in into.places {
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
                            let mut src = self.operands([&copy.src]);
                            src = self.read_through(&src);
                            changed |= self.store(&dst.project_deeper(&[mir::PlaceElem::Deref], self.tcx), &src);
                        }
                    }
                    _ => {}
                }
            }
            if let TerminatorKind::Call { args, destination, .. } = &data.terminator().kind {
                // What it's given, however many pointers down, its result may
                // hold, `deref_mut(&mut guard)`'s, and it may write into what
                // else it's given, `v.push(&mut x)`.
                let value = self.deep(self.operands(args.iter().map(|a| &a.node)));
                changed |= self.store(destination, &value);
                for arg in args.iter() {
                    if let Some(place) = arg.node.place() {
                        let deref = place.project_deeper(&[mir::PlaceElem::Deref], self.tcx);
                        if place.ty(self.body, self.tcx).ty.builtin_deref(true).is_some() {
                            changed |= self.store(&deref, &value);
                        }
                    }
                }
            }
        }
        changed
    }

    /// `value` and each target reachable from its pointers, however many
    /// down.
    fn deep(&self, (writes, reaches): (Targets, Targets)) -> (Targets, Targets) {
        let mut out = (writes.clone(), reaches.clone());
        for t in &writes.places {
            out.0.extend(&self.reachable(*t, true));
        }
        for t in &reaches.places {
            out.1.extend(&self.reachable(*t, false));
        }
        out
    }

    /// What the pointers `pointers` point to hold: a read through them.
    fn read_through(&self, (writes, reaches): &(Targets, Targets)) -> (Targets, Targets) {
        let mut out = (Targets::default(), Targets::default());
        out.0.unknown = writes.unknown;
        out.1.unknown = reaches.unknown;
        for t in &reaches.places {
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
            Rvalue::Ref(_, kind, place) => {
                let writable = matches!(kind, mir::BorrowKind::Mut { .. })
                    || changes_through_shared(self.tcx, self.typing_env, place.ty(self.body, self.tcx).ty);
                self.borrow(place, writable)
            }
            Rvalue::Reborrow(_, mutability, place) => {
                let writable = mutability.is_mut()
                    || changes_through_shared(self.tcx, self.typing_env, place.ty(self.body, self.tcx).ty);
                self.borrow(place, writable)
            }
            Rvalue::RawPtr(_, place) => self.borrow(place, true),
            Rvalue::Cast(mir::CastKind::PointerWithExposedProvenance, ..) | Rvalue::ThreadLocalRef(_) => {
                let unknown = Targets {
                    places: IndexSet::default(),
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
    /// local's storage of none.
    fn borrow(&self, place: &Place<'tcx>, writable: bool) -> (Targets, Targets) {
        let reaches = through(&self.writes, place, false);
        let reaches = match place.is_indirect() {
            // Through a shared pointer too, which `writes` doesn't hold.
            true => {
                let mut all = through(&self.reaches, place, false);
                all.extend(&reaches);
                all
            }
            false => reaches,
        };
        let writes = match writable {
            true => through(&self.writes, place, true),
            false => Targets::default(),
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
                for t in through(&self.reaches, &place, false)
                    .places
                    .iter()
                    .chain(&through(&self.writes, &place, false).places)
                {
                    if let Some(held) = self.writes.get(t) {
                        writes.extend(held);
                    }
                    if let Some(held) = self.reaches.get(t) {
                        reaches.extend(held);
                    }
                }
                let from = through(&self.reaches, &place, false);
                writes.unknown |= from.unknown;
                reaches.unknown |= from.unknown;
            } else if let Operand::Constant(constant) = operand {
                let (holds, write) = pointers_in(self.tcx, self.typing_env, constant.ty());
                writes.unknown |= write;
                reaches.unknown |= holds;
            }
        }
        (writes, reaches)
    }

    /// Each target reachable from `target` through the pointers its memory
    /// holds, however many down, and whether one the body can't see:
    /// through those that may write, of `writing`, else any.
    fn reachable(&self, target: Target, writing: bool) -> Targets {
        let map = if writing { &self.writes } else { &self.reaches };
        let mut reached = Targets::default();
        let mut work = vec![target];
        while let Some(t) = work.pop() {
            if let Some(held) = map.get(&t) {
                reached.unknown |= held.unknown;
                for &next in &held.places {
                    if reached.places.insert(next) {
                        work.push(next);
                    }
                }
            }
        }
        reached
    }

    /// Where each target escapes: a raw pointer made of it; a call that may
    /// keep a pointer it's given; a pointer stored where the body can't see,
    /// or into what a parameter points to; and each target reachable from
    /// what escapes. What a parameter that can be changed through an alias
    /// its caller keeps points to escapes at the body's start.
    fn escapes(&self, made: usize) -> IndexMap<Location, IndexSet<Target>> {
        let mut escapes: IndexMap<Location, IndexSet<Target>> = IndexMap::default();
        let start = Location::START;
        for param in (1..=self.body.arg_count).map(Local::from_usize) {
            if changes_through_shared(self.tcx, self.typing_env, self.body.local_decls[param].ty) {
                escapes.entry(start).or_default().insert(Target::Pointee(param));
            }
        }
        for local in (made..self.body.local_decls.len()).map(Local::from_usize) {
            if changes_through_shared(self.tcx, self.typing_env, self.body.local_decls[local].ty) {
                escapes.entry(start).or_default().insert(Target::Whole(local));
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
                        escapes
                            .entry(location)
                            .or_default()
                            .extend(through(&self.writes, place, true).places);
                    }
                    let into = through(&self.writes, &assign.0, true);
                    if assign.0.is_indirect() && (into.unknown || into.places.iter().any(outside)) {
                        let (writes, _) = self.rvalue(&assign.1);
                        escapes.entry(location).or_default().extend(writes.places);
                    }
                }
            }
            let location = self.body.terminator_loc(block);
            // What a drop that runs code, a yield or inline assembly is given
            // it may keep.
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
                escapes
                    .entry(location)
                    .or_default()
                    .extend(self.reachable(Target::Whole(place.local), true).places);
            }
            if let TerminatorKind::Call { func, args, .. } = &data.terminator().kind {
                let callee = func.const_fn_def();
                for arg in args.iter() {
                    if self.call_holds_past(callee, args.iter().map(|a| a.node.ty(self.body, self.tcx)), &arg.node)
                        && let Some(place) = arg.node.place()
                    {
                        let mut kept = self.reachable(Target::Whole(place.local), true);
                        kept.places.shift_remove(&Target::Whole(place.local));
                        escapes.entry(location).or_default().extend(kept.places);
                    }
                }
            }
        }
        // What an escaped pointer points into escapes with it.
        for targets in escapes.values_mut() {
            let more: Vec<Target> = targets.iter().flat_map(|&t| self.reachable(t, true).places).collect();
            targets.extend(more);
        }
        escapes
    }

    /// The targets escaped before each block: what escapes on a way to it,
    /// a forward flow that never forgets.
    fn escaped_before(&self, escapes: &IndexMap<Location, IndexSet<Target>>) -> IndexVec<BasicBlock, IndexSet<Target>> {
        let blocks = &self.body.basic_blocks;
        let mut before: IndexVec<BasicBlock, IndexSet<Target>> = IndexVec::from_elem(IndexSet::default(), blocks);
        before[mir::START_BLOCK].extend(escapes.get(&Location::START).into_iter().flatten().copied());
        let mut changed = true;
        while changed {
            changed = false;
            for &block in blocks.reverse_postorder() {
                let mut after = before[block].clone();
                for statement_index in 0..=blocks[block].statements.len() {
                    after.extend(
                        escapes
                            .get(&Location { block, statement_index })
                            .into_iter()
                            .flatten()
                            .copied(),
                    );
                }
                for succ in blocks[block].terminator().successors() {
                    let len = before[succ].len();
                    before[succ].extend(after.iter().copied());
                    changed |= before[succ].len() != len;
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
        if held.places.iter().all(|&t| t == Target::Whole(place.local)) && !held.unknown {
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
            &|t| pointers_in(self.tcx, self.typing_env, t).1,
        )
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

/// Of the pointers `ty` holds, whether there's any, and whether any may
/// write what it points to: a `&mut`, a raw pointer, a shared reference
/// to what can be changed through one (`changes_through_shared`), or what
/// can't be seen, a `dyn`, a type parameter, an opaque type. Of std's
/// types, what their type arguments hold, a `Vec<&mut u32>`'s; of the
/// crate's, what their fields do; a closure's, its captures'.
fn pointers_in<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> (bool, bool) {
    fn walk<'tcx>(
        tcx: TyCtxt<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
        ty: Ty<'tcx>,
        seen: &mut Vec<Ty<'tcx>>,
        found: &mut (bool, bool),
    ) {
        if seen.contains(&ty) {
            return;
        }
        seen.push(ty);
        match *ty.kind() {
            ty::Bool
            | ty::Char
            | ty::Int(_)
            | ty::Uint(_)
            | ty::Float(_)
            | ty::Str
            | ty::Never
            | ty::FnDef(..)
            | ty::FnPtr(..) => {}
            ty::Ref(_, pointee, mutability) => {
                found.0 = true;
                found.1 |= mutability.is_mut() || changes_through_shared(tcx, typing_env, pointee);
                walk(tcx, typing_env, pointee, seen, found);
            }
            ty::Slice(item) | ty::Array(item, _) => walk(tcx, typing_env, item, seen, found),
            ty::Tuple(items) => items.iter().for_each(|t| walk(tcx, typing_env, t, seen, found)),
            ty::Closure(_, args) => {
                (args.as_closure().upvar_tys().iter()).for_each(|t| walk(tcx, typing_env, t, seen, found))
            }
            ty::Adt(adt, args) if is_std_crate(tcx, adt.did()) => {
                // A borrow it holds by a lifetime, a guard's or an iterator's:
                // one that may write where what it holds is invariant,
                // `RefMut<'_, T>`, `IterMut<'_, T>`, as a `&mut T` is.
                // A raw pointer std's type is, `NonNull<T>`, whose type
                // arguments say nothing of it; and a `Box`, whose deref is
                // to what it points to, `&(*items)` a closure captures.
                if raw_pointee(tcx, ty).is_some() || ty.is_box() {
                    *found = (true, true);
                }
                if args.regions().next().is_some() {
                    found.0 = true;
                    let variances = tcx.variances_of(adt.did());
                    found.1 |= (args.iter().zip(variances.iter()))
                        .any(|(arg, variance)| arg.as_type().is_some() && *variance == ty::Variance::Invariant);
                }
                args.types().for_each(|t| walk(tcx, typing_env, t, seen, found))
            }
            ty::Adt(adt, args) => (adt.all_fields())
                .for_each(|field| walk(tcx, typing_env, field.ty(tcx, args).skip_normalization(), seen, found)),
            _ => *found = (true, true),
        }
    }
    let mut found = (false, false);
    walk(tcx, typing_env, ty, &mut Vec::new(), &mut found);
    found
}

/// Whether a shared reference to `ty` may change what it refers to:
/// what's not `Freeze`, a `Cell` or a `RefCell`, or holds one, through a
/// pointer too, `Rc<RefCell<_>>`; a raw pointer; or what can't be seen, a
/// type parameter, a `dyn`, an opaque type. Of std's types, what their
/// type arguments are; of the crate's, what their fields are.
pub(in crate::lower) fn changes_through_shared<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: Ty<'tcx>,
) -> bool {
    fn walk<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>, seen: &mut Vec<Ty<'tcx>>) -> bool {
        if seen.contains(&ty) {
            return false;
        }
        seen.push(ty);
        match *ty.kind() {
            ty::Bool
            | ty::Char
            | ty::Int(_)
            | ty::Uint(_)
            | ty::Float(_)
            | ty::Str
            | ty::Never
            | ty::FnDef(..)
            | ty::FnPtr(..) => false,
            ty::Ref(_, pointee, _) | ty::Slice(pointee) | ty::Array(pointee, _) => walk(tcx, typing_env, pointee, seen),
            ty::Tuple(items) => items.iter().any(|t| walk(tcx, typing_env, t, seen)),
            ty::Closure(_, args) => (args.as_closure().upvar_tys().iter()).any(|t| walk(tcx, typing_env, t, seen)),
            ty::Adt(adt, args) => {
                if !ty.is_freeze(tcx, typing_env) || raw_pointee(tcx, ty).is_some() {
                    return true;
                }
                if is_std_crate(tcx, adt.did()) {
                    args.types().any(|t| walk(tcx, typing_env, t, seen))
                } else {
                    (adt.all_fields())
                        .any(|field| walk(tcx, typing_env, field.ty(tcx, args).skip_normalization(), seen))
                }
            }
            _ => true,
        }
    }
    walk(tcx, typing_env, ty, &mut Vec::new())
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
