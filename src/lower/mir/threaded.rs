//! An enum a `match` makes in each arm and another `match` takes apart at
//! once (rd433, ADR 1054):
//!
//! ```rust,ignore
//! let settled = match &*chunk.state.borrow() {
//!     State::Done(value) => Ok(*value),
//!     State::Pending(..) => return,
//!     State::Failed(reason) => Err(*reason),
//! };
//! match settled {
//!     Ok(value) => resolve(value),
//!     Err(reason) => reject(reason),
//! }
//! ```
//!
//! Each arm knows which variant it made, so it goes on to that variant's
//! case, past what only ends the first `match`, a borrow's drop, copied to
//! each, and the case reads each field where the arm wrote it: one
//! dispatch, as a person writes it, `if (..) { resolve(chunk.value) } ..`,
//! never an object of a tag made only to be tested again.
//!
//! Rust's MIR says it all: the arms' writes of the local, the blocks between
//! them and the switch on its discriminant, and each read of a field of a
//! variant. The rewrite is MIR's, so what it gives runs as the Rust does:
//! each arm reaches the case its variant's discriminant would, through the
//! same drops, and each field read is of the value its variant's last write
//! gave, as no read of a variant's field is of a value of another variant.

use std::collections::{HashMap, HashSet};

use rustc_abi::{FieldIdx, VariantIdx};
use rustc_middle::mir::visit::{MutVisitor, PlaceContext, Visitor};
use rustc_middle::mir::{
    self, AggregateKind, BasicBlock, Local, Operand, Place, PlaceElem, Rvalue, StatementKind, TerminatorKind,
};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

use super::Mir;
use crate::lower::FnCx;
use crate::lower::drops::Drops;

/// The most blocks that may end the first `match` before the second, each
/// copied to each arm: a borrow's drop, its temporaries', a `goto`.
const MOST_BETWEEN: usize = 8;

impl<'tcx> FnCx<'_, 'tcx> {
    /// `mir`, each enum one `match` makes and the next takes apart at once
    /// threaded: each arm going on to its variant's case. `None` where
    /// there's none.
    pub(super) fn mir_threaded(&self, mir: &Mir<'tcx>) -> Option<Mir<'tcx>> {
        let mut body: Option<mir::Body<'tcx>> = None;
        let mut cases = mir.cases.clone();
        // Each rewrite leaves the others' locals and blocks as they were.
        for _ in 0..16 {
            let current = body.as_ref().unwrap_or(&mir.body);
            let Some(threading) = self.threading(current) else {
                break;
            };
            let mut next = current.clone();
            threading.apply(self.tcx, &mut next, &mut cases);
            body = Some(next);
        }
        Some(Mir {
            body: body?,
            promoted: mir.promoted.clone(),
            awaits: mir.awaits.clone(),
            cases,
        })
    }

    /// The first enum of `body` that can be threaded, and how.
    fn threading(&self, body: &mir::Body<'tcx>) -> Option<Threading<'tcx>> {
        (body.basic_blocks.iter_enumerated())
            .filter(|(_, data)| !data.is_cleanup)
            .find_map(|(switch, _)| self.threading_at(body, switch))
    }

    /// Of the block `switch`, a switch on the discriminant of a local enum:
    /// how each of its writes goes on to its case.
    fn threading_at(&self, body: &mir::Body<'tcx>, switch: BasicBlock) -> Option<Threading<'tcx>> {
        let tcx = self.tcx;
        let data = &body.basic_blocks[switch];
        let TerminatorKind::SwitchInt {
            discr: Operand::Move(tested),
            targets,
        } = &data.terminator().kind
        else {
            return None;
        };
        let tested = tested.as_local()?;
        let (read_at, enum_local) = data.statements.iter().enumerate().find_map(|(i, s)| match &s.kind {
            StatementKind::Assign(assign) if assign.0.as_local() == Some(tested) => match assign.1 {
                Rvalue::Discriminant(place) => Some((i, place.as_local()?)),
                _ => None,
            },
            _ => None,
        })?;
        if enum_local.as_usize() <= body.arg_count {
            return None;
        }
        let ty = body.local_decls[enum_local].ty;
        let ty::Adt(adt, _) = *ty.kind() else {
            return None;
        };
        // What JS drops of it is nothing, so no drop of it is there to keep.
        if !adt.is_enum() || self.drops(ty) != Drops::Nothing {
            return None;
        }
        // An `Option` that's its value or `undefined` is no object of a tag
        // made to be tested again: where the first `match` holds no borrow
        // the second must be past, it's the value the program named and
        // tests, upstream's `instance` an `if` chain makes and `if
        // (!instance) throw` after (rd531).
        let unboxed = self.option_of(ty).is_some_and(|inner| !self.boxed_payload(inner));
        // The switch's block is only the read and the switch.
        if !(data.statements.iter().enumerate()).all(|(i, s)| i == read_at || quiet(s)) {
            return None;
        }
        let uses = Uses::of(body, enum_local, tested);
        let Uses {
            writes,
            fields,
            other,
            tested_reads,
        } = uses;
        if other || tested_reads != 2 || writes.is_empty() {
            return None;
        }
        // Each write: a whole variant, of at most one field, which its
        // block goes on from to the switch through blocks that only end the
        // first `match`; each variant made once, so each case is its arm's.
        let mut seen = HashSet::new();
        let mut between: Vec<BasicBlock> = Vec::new();
        let mut made = Vec::new();
        for &(block, index) in &writes {
            let StatementKind::Assign(assign) = &body.basic_blocks[block].statements[index].kind else {
                return None;
            };
            let Rvalue::Aggregate(kind, operands) = &assign.1 else {
                return None;
            };
            let AggregateKind::Adt(_, variant, _, _, None) = **kind else {
                return None;
            };
            seen.insert(variant);
            if operands.len() > 1 {
                return None;
            }
            let rest = &body.basic_blocks[block].statements[index + 1..];
            if rest.iter().any(|s| mentions(s, enum_local)) || block == switch {
                return None;
            }
            let mut at = only_successor(&body.basic_blocks[block], enum_local)?;
            let mut path = Vec::new();
            while at != switch {
                let data = &body.basic_blocks[at];
                if data.is_cleanup || path.len() >= MOST_BETWEEN || !data.statements.iter().all(quiet) {
                    return None;
                }
                path.push(at);
                at = only_successor(data, enum_local)?;
            }
            for &block in &path {
                if !between.contains(&block) {
                    between.push(block);
                }
            }
            let value = ty.discriminant_for_variant(tcx, variant)?.val;
            made.push(Made {
                block,
                path,
                case: targets.target_for_value(value),
            });
        }
        // A variant two arms make has its case reached from both, after the
        // first `match`'s chain, where the others' cases, in their arms,
        // can't go on to what follows without a label: each ends the
        // function apart, through nothing but drops, as Flight's `then`
        // does, or it's left whole.
        let mut makers: HashMap<BasicBlock, usize> = HashMap::new();
        for m in &made {
            *makers.entry(m.case).or_default() += 1;
        }
        if makers.values().any(|&n| n > 1) && !ends_apart(body, &makers.keys().copied().collect::<Vec<_>>()) {
            return None;
        }
        let ends_a_borrow = || {
            between.iter().any(|&b| {
                matches!(&body.basic_blocks[b].terminator().kind, TerminatorKind::Drop { place, .. }
                    if self.recognition().is_guard(place.ty(&body.local_decls, tcx).ty))
            })
        };
        // A `loop`'s value, `let next = loop { .. break Some(next); .. }`,
        // has no expression of its own in JS: its breaks write it, and the
        // case goes on there as before (ADR 1055).
        let in_a_loop = || {
            let predecessors = body.basic_blocks.predecessors();
            (made.iter()).any(|m| predecessors[m.block].iter().any(|&p| loops_back(body, p, switch)))
        };
        if unboxed && !ends_a_borrow() && !in_a_loop() {
            return None;
        }
        // Nothing else reaches the switch, or what's between.
        let predecessors = body.basic_blocks.predecessors();
        let entered = |block: BasicBlock| {
            predecessors[block]
                .iter()
                .all(|p| between.contains(p) || made.iter().any(|m| m.block == *p))
        };
        if !entered(switch) || !between.iter().all(|&b| entered(b)) {
            return None;
        }
        // Each field read is of a variant one arm makes, at a type it says;
        // one read only by a variable it's given to, `Err(reason) => ..`,
        // which is then written where the arm writes the field.
        let mut parts: HashMap<(VariantIdx, FieldIdx), Part<'tcx>> = HashMap::new();
        for (variant, field, field_ty, at) in fields {
            if !seen.contains(&variant) {
                return None;
            }
            let part = parts.entry((variant, field)).or_insert(Part {
                ty: field_ty,
                reads: Vec::new(),
            });
            part.reads.push(at);
        }
        for &(block, index) in &writes {
            let StatementKind::Assign(assign) = &body.basic_blocks[block].statements[index].kind else {
                return None;
            };
            if let Rvalue::Aggregate(kind, operands) = &assign.1
                && let AggregateKind::Adt(_, variant, ..) = **kind
                && let Some(operand) = operands.iter().next()
            {
                (parts.entry((variant, FieldIdx::ZERO))).or_insert_with(|| Part {
                    ty: operand.ty(&body.local_decls, tcx),
                    reads: Vec::new(),
                });
            }
        }
        let taken_by = (parts.iter())
            .filter_map(|(&key, part)| Some((key, given_to(body, part)?)))
            .collect();
        Some(Threading {
            local: enum_local,
            switch,
            read_at,
            between,
            made,
            writes,
            parts: parts.into_iter().map(|(key, part)| (key, part.ty)).collect(),
            taken_by,
        })
    }
}

/// How one enum local is threaded.
struct Threading<'tcx> {
    local: Local,
    switch: BasicBlock,
    /// The switch's block's read of its discriminant.
    read_at: usize,
    /// The blocks between the writes and the switch.
    between: Vec<BasicBlock>,
    /// Each write's block, its way to the switch and its case.
    made: Vec<Made>,
    /// Each write, a statement.
    writes: Vec<(BasicBlock, usize)>,
    /// Each variant's field read or written, and its type.
    parts: HashMap<(VariantIdx, FieldIdx), Ty<'tcx>>,
    /// Each field read once, by a variable it's given to whole: that
    /// variable, and where it's given it.
    taken_by: HashMap<(VariantIdx, FieldIdx), (Local, mir::Location)>,
}

/// A write of the enum: its block, the blocks it goes on through to the
/// switch, and the case the switch would go on to.
struct Made {
    block: BasicBlock,
    path: Vec<BasicBlock>,
    case: BasicBlock,
}

/// A variant's field: its type, and where it's read.
struct Part<'tcx> {
    ty: Ty<'tcx>,
    reads: Vec<mir::Location>,
}

/// Of a field read once, `x = move (settled as Err).0`, by a variable the
/// program names: that variable, and where. One the arm writes again,
/// `Err(mut reason) => { reason *= 2; .. }`, too: each place the variant is
/// made writes it first, before the switch, and the arm, after (#481).
fn given_to(body: &mir::Body<'_>, part: &Part<'_>) -> Option<(Local, mir::Location)> {
    let [at] = part.reads[..] else {
        return None;
    };
    let statement = body.basic_blocks[at.block].statements.get(at.statement_index)?;
    let StatementKind::Assign(assign) = &statement.kind else {
        return None;
    };
    let (Rvalue::Use(Operand::Copy(from) | Operand::Move(from), _), Some(to)) = (&assign.1, assign.0.as_local()) else {
        return None;
    };
    if from.projection.len() != 2 || to.as_usize() <= body.arg_count {
        return None;
    }
    let named = (body.var_debug_info.iter())
        .any(|info| matches!(info.value, mir::VarDebugInfoContents::Place(p) if p.as_local() == Some(to)));
    named.then_some((to, at))
}

impl<'tcx> Threading<'tcx> {
    /// Of `body`, each block an arm goes on to its case through, and
    /// where the `match` is written, put in `cases`.
    fn apply(self, tcx: ty::TyCtxt<'tcx>, body: &mut mir::Body<'tcx>, cases: &mut HashMap<BasicBlock, Span>) {
        let written = body.basic_blocks[self.switch].terminator().source_info.span;
        let span = body.local_decls[self.local].source_info.span;
        // A local of each field, in place of the variant's: the variable
        // it's given to, its write gone, or one of its own.
        let mut parts: Vec<_> = self.parts.iter().map(|(&key, &ty)| (key, ty)).collect();
        parts.sort_by_key(|(key, _)| *key);
        let mut locals: HashMap<(VariantIdx, FieldIdx), Local> = HashMap::new();
        for (key, ty) in parts {
            let local = match self.taken_by.get(&key) {
                Some(&(to, at)) => {
                    body.basic_blocks.as_mut()[at.block].statements[at.statement_index].kind = StatementKind::Nop;
                    for data in body.basic_blocks.as_mut().iter_mut() {
                        for statement in &mut data.statements {
                            if matches!(statement.kind, StatementKind::StorageLive(l) if l == to) {
                                statement.kind = StatementKind::Nop;
                            }
                        }
                    }
                    to
                }
                None => body.local_decls.push(mir::LocalDecl::new(ty, span)),
            };
            locals.insert(key, local);
        }
        // Each write, of its one field's local or of nothing.
        for &(block, index) in &self.writes {
            let statement = &mut body.basic_blocks.as_mut()[block].statements[index];
            let StatementKind::Assign(assign) = &mut statement.kind else {
                unreachable!("a write")
            };
            let Rvalue::Aggregate(kind, operands) = &mut assign.1 else {
                unreachable!("a variant made")
            };
            let AggregateKind::Adt(_, variant, ..) = **kind else {
                unreachable!("a variant made")
            };
            match std::mem::take(operands).into_iter().next() {
                Some(operand) => {
                    assign.0 = Place::from(locals[&(variant, FieldIdx::ZERO)]);
                    assign.1 = Rvalue::Use(operand, mir::WithRetag::Yes);
                }
                None => statement.kind = StatementKind::Nop,
            }
        }
        // Each arm's way to its case: what's between, copied.
        let blocks = body.basic_blocks.as_mut();
        for &Made { block, ref path, case } in &self.made {
            let mut copies: HashMap<BasicBlock, BasicBlock> = HashMap::new();
            let mut ending = blocks[self.switch].clone();
            ending.statements[self.read_at].kind = StatementKind::Nop;
            ending.terminator_mut().kind = TerminatorKind::Goto { target: case };
            let ended = blocks.push(ending);
            cases.insert(ended, written);
            for &b in path {
                let copy = blocks.push(blocks[b].clone());
                copies.insert(b, copy);
            }
            let next = |b: BasicBlock| match b == self.switch {
                true => ended,
                false => copies.get(&b).copied().unwrap_or(b),
            };
            for &copy in copies.values() {
                retarget(&mut blocks[copy], self.local, &next);
            }
            retarget(&mut blocks[block], self.local, &next);
        }
        // The blocks it replaced are reached no more.
        for &b in self.between.iter().chain([&self.switch]) {
            let data = &mut blocks[b];
            for statement in &mut data.statements {
                statement.kind = StatementKind::Nop;
            }
            data.terminator_mut().kind = TerminatorKind::Unreachable;
        }
        // Each mention of it otherwise: a field's its local's; a drop of it,
        // which drops nothing, gone; the rest nothing.
        let mut rewrite = Rewrite {
            tcx,
            local: self.local,
            locals: &locals,
        };
        let block_ids: Vec<BasicBlock> = body.basic_blocks.indices().collect();
        for block in block_ids {
            let data = &mut body.basic_blocks.as_mut()[block];
            for statement in &mut data.statements {
                if quiet_of(statement, self.local) {
                    statement.kind = StatementKind::Nop;
                }
            }
            if let TerminatorKind::Drop { place, target, .. } = &data.terminator().kind
                && place.as_local() == Some(self.local)
            {
                let target = *target;
                data.terminator_mut().kind = TerminatorKind::Goto { target };
            }
        }
        rewrite.visit_body(body);
        body.var_debug_info.retain(|info| match &info.value {
            mir::VarDebugInfoContents::Place(place) => place.local != self.local,
            mir::VarDebugInfoContents::Const(_) => true,
        });
    }
}

/// `data`'s way on, `next` of where it went.
fn retarget(data: &mut mir::BasicBlockData<'_>, local: Local, next: &dyn Fn(BasicBlock) -> BasicBlock) {
    match &mut data.terminator_mut().kind {
        TerminatorKind::Goto { target } => *target = next(*target),
        TerminatorKind::Drop { place, target, .. } if place.local != local => *target = next(*target),
        _ => unreachable!("checked: a goto or a drop"),
    }
}

/// Whether what each of `cases` runs reaches nothing another reaches but
/// the function's end: blocks of storage and drops alone, and its return.
fn ends_apart(body: &mir::Body<'_>, cases: &[BasicBlock]) -> bool {
    let mut reached: HashMap<BasicBlock, BasicBlock> = HashMap::new();
    for &case in cases {
        let mut seen = HashSet::from([case]);
        let mut stack = vec![case];
        while let Some(block) = stack.pop() {
            match reached.insert(block, case) {
                Some(other) if other != case => {
                    let data = &body.basic_blocks[block];
                    let ends = matches!(
                        data.terminator().kind,
                        TerminatorKind::Goto { .. } | TerminatorKind::Drop { .. } | TerminatorKind::Return
                    );
                    if !ends || !data.statements.iter().all(quiet) {
                        return false;
                    }
                }
                _ => {}
            }
            for next in super::cfg::successors(body, block) {
                if seen.insert(next) {
                    stack.push(next);
                }
            }
        }
    }
    true
}

/// Where `data` goes on to, a `goto` or a drop of something else, alone.
fn only_successor(data: &mir::BasicBlockData<'_>, local: Local) -> Option<BasicBlock> {
    match &data.terminator().kind {
        TerminatorKind::Goto { target } => Some(*target),
        TerminatorKind::Drop { place, target, .. } if place.local != local => Some(*target),
        _ => None,
    }
}

/// Whether `statement` does nothing JS sees: storage, a fake read, a
/// mention.
fn quiet(statement: &mir::Statement<'_>) -> bool {
    matches!(
        &statement.kind,
        StatementKind::StorageLive(_)
            | StatementKind::StorageDead(_)
            | StatementKind::Nop
            | StatementKind::AscribeUserType(..)
            | StatementKind::FakeRead(_)
            | StatementKind::PlaceMention(_)
    )
}

/// Whether `statement` is one of `local`'s mentions that's nothing.
fn quiet_of(statement: &mir::Statement<'_>, local: Local) -> bool {
    match &statement.kind {
        StatementKind::StorageLive(l) | StatementKind::StorageDead(l) => *l == local,
        StatementKind::FakeRead(read) => read.1.local == local,
        StatementKind::PlaceMention(place) => place.local == local,
        _ => false,
    }
}

/// Whether `statement` mentions `local` other than as nothing.
fn mentions(statement: &mir::Statement<'_>, local: Local) -> bool {
    struct Seen(Local, bool);
    impl<'tcx> Visitor<'tcx> for Seen {
        fn visit_local(&mut self, local: Local, _: PlaceContext, _: mir::Location) {
            self.1 |= local == self.0;
        }
    }
    if quiet_of(statement, local) {
        return false;
    }
    let mut seen = Seen(local, false);
    seen.visit_statement(statement, mir::Location::START);
    seen.1
}

/// Each mention of an enum local, and of what holds its discriminant.
struct Uses<'tcx> {
    /// Each whole write of it, a statement.
    writes: Vec<(BasicBlock, usize)>,
    /// Each field of a variant read or written, its type, and where.
    fields: Vec<(VariantIdx, FieldIdx, Ty<'tcx>, mir::Location)>,
    /// Whether it's mentioned otherwise: whole, or its discriminant read
    /// elsewhere.
    other: bool,
    /// The mentions of what holds its discriminant.
    tested_reads: usize,
}

impl<'tcx> Uses<'tcx> {
    fn of(body: &mir::Body<'tcx>, local: Local, tested: Local) -> Self {
        struct Walk<'tcx> {
            local: Local,
            tested: Local,
            uses: Uses<'tcx>,
            discriminants: usize,
        }
        impl<'tcx> Visitor<'tcx> for Walk<'tcx> {
            fn visit_statement(&mut self, statement: &mir::Statement<'tcx>, location: mir::Location) {
                if quiet_of(statement, self.local) {
                    return;
                }
                if let StatementKind::Assign(assign) = &statement.kind {
                    if assign.0.as_local() == Some(self.local)
                        && let Rvalue::Aggregate(kind, operands) = &assign.1
                        && matches!(**kind, AggregateKind::Adt(..))
                    {
                        self.uses.writes.push((location.block, location.statement_index));
                        for operand in operands {
                            self.visit_operand(operand, location);
                        }
                        return;
                    }
                    if let Rvalue::Discriminant(place) = &assign.1
                        && place.as_local() == Some(self.local)
                        && assign.0.as_local() == Some(self.tested)
                    {
                        self.discriminants += 1;
                        self.uses.tested_reads += 1;
                        return;
                    }
                }
                self.super_statement(statement, location);
            }
            fn visit_terminator(&mut self, terminator: &mir::Terminator<'tcx>, location: mir::Location) {
                if let TerminatorKind::Drop { place, .. } = &terminator.kind
                    && place.as_local() == Some(self.local)
                {
                    return;
                }
                self.super_terminator(terminator, location);
            }
            fn visit_place(&mut self, place: &Place<'tcx>, context: PlaceContext, location: mir::Location) {
                // What debug info says of it goes with it.
                if matches!(context, PlaceContext::NonUse(_)) {
                    return;
                }
                if place.local == self.local {
                    match place.projection[..] {
                        // A field read whole, or through what it points to;
                        // not a tuple's parts apart, whose variant holds them
                        // together, `const [name, root] = first`.
                        [
                            PlaceElem::Downcast(_, variant),
                            PlaceElem::Field(field, ty),
                            ref rest @ ..,
                        ] if !matches!(rest.first(), Some(PlaceElem::Field(..))) => {
                            self.uses.fields.push((variant, field, ty, location));
                        }
                        _ => self.uses.other = true,
                    }
                }
                if place.local == self.tested {
                    self.uses.tested_reads += 1;
                }
                self.super_place(place, context, location);
            }
        }
        let mut walk = Walk {
            local,
            tested,
            uses: Uses {
                writes: Vec::new(),
                fields: Vec::new(),
                other: false,
                tested_reads: 0,
            },
            discriminants: 0,
        };
        walk.visit_body(body);
        if walk.discriminants != 1 {
            walk.uses.other = true;
        }
        walk.uses
    }
}

/// Each field of a variant of `local` read as its own local.
struct Rewrite<'a, 'tcx> {
    tcx: ty::TyCtxt<'tcx>,
    local: Local,
    locals: &'a HashMap<(VariantIdx, FieldIdx), Local>,
}

impl<'tcx> MutVisitor<'tcx> for Rewrite<'_, 'tcx> {
    fn tcx(&self) -> ty::TyCtxt<'tcx> {
        self.tcx
    }
    fn visit_place(&mut self, place: &mut Place<'tcx>, _: PlaceContext, _: mir::Location) {
        if place.local != self.local {
            return;
        }
        if let [
            PlaceElem::Downcast(_, variant),
            PlaceElem::Field(field, _),
            ref rest @ ..,
        ] = place.projection[..]
            && let Some(&part) = self.locals.get(&(variant, field))
        {
            let rest: Vec<PlaceElem<'tcx>> = rest.to_vec();
            *place = Place {
                local: part,
                projection: self.tcx.mk_place_elems(&rest),
            };
        }
    }
}

/// Whether `from` reaches itself again without passing `past`: it's in a
/// loop that `past` isn't in the way of.
fn loops_back(body: &mir::Body<'_>, from: BasicBlock, past: BasicBlock) -> bool {
    let mut seen = HashSet::new();
    let mut pending: Vec<BasicBlock> = body.basic_blocks[from].terminator().successors().collect();
    while let Some(at) = pending.pop() {
        if at == from {
            return true;
        }
        if at == past || body.basic_blocks[at].is_cleanup || !seen.insert(at) {
            continue;
        }
        pending.extend(body.basic_blocks[at].terminator().successors());
    }
    false
}
