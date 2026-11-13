//! Integer intervals of a body's locals, a forward dataflow over the MIR
//! the emitter reads (#1671 change 1, #1672 M2): which bounds checks and
//! wraps that MIR proves never fail, of what it says alone, each order
//! rustc's. Run in shadow first: where THIR's ranges leave out a check this
//! doesn't prove, a line is logged (`--intervals-shadow`), a wrong-answer
//! candidate or a gap of this analysis's precision.
//!
//! Each integer local holds an interval, its type's range where nothing is
//! known; a `bool` a comparison made holds that comparison, which narrows
//! its operands on each side of a switch of it, and past an `assert` of it.
//! A local whose place is lent `&mut` or as a raw pointer is never known.
//! At a loop's head, a bound still moving after a few visits widens to its
//! type's.

use crate::collections::{IndexMap, IndexSet};
use crate::lower::recognition::{SliceLength, slice_length};
use rustc_index::IndexVec;
use rustc_middle::mir::{
    self, AssertKind, BasicBlock, BinOp, Body, CastKind, Local, Location, Operand, Rvalue, StatementKind,
    TerminatorKind,
};
use rustc_middle::ty::consts::ConstExt as _;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;

/// How many times a block is visited before what still moves widens.
const VISITS_BEFORE_WIDENING: u32 = 3;

/// What MIR proves of a body: each bounds check that never fails, by the
/// block its `assert` ends, and each `+`, `-` and `*` that never leaves its
/// type's range, where it's written.
#[derive(Default)]
pub(in crate::lower) struct Proofs {
    pub(in crate::lower) bounds: IndexSet<BasicBlock>,
    pub(in crate::lower) exact: IndexSet<Location>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Interval {
    lo: i128,
    hi: i128,
}

impl Interval {
    fn join(self, other: Interval) -> Interval {
        Interval {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
        }
    }
}

/// A comparison a `bool` local holds: `a op b`.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Comparison {
    op: BinOp,
    a: Value,
    b: Value,
}

/// What a comparison compares: a local or a constant.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Value {
    Local(Local),
    Constant(i128),
}

/// What a relation's greater side is: a slice's length, by the local the
/// slice is (`Len`), or an integer local.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Term {
    Len(Local),
    Int(Local),
}

/// What's known at a point: each integer local's interval, none where it
/// isn't; each `bool` local's comparison; and each local that's a copy of
/// another neither has been written since, `_7 = copy _3`, which MIR
/// compares in its place, so a comparison is of what it copies, or a
/// reborrow of it, `_9 = &_1`, whose length is the same. And the small
/// relational part (ADR 1137, M2b): each local that holds a slice's
/// length, by the slice; each `x < y` and `x <= y` known, strict or not;
/// the least each slice's length is; and each `bool` that's a slice's
/// `is_empty()`.
#[derive(Clone, PartialEq)]
struct State {
    intervals: IndexVec<Local, Option<Interval>>,
    comparisons: IndexMap<Local, Comparison>,
    copies: IndexMap<Local, Local>,
    lens: IndexMap<Local, Local>,
    below: IndexSet<(Local, Term, bool)>,
    least: IndexMap<Local, i128>,
    empties: IndexMap<Local, Local>,
}

impl State {
    fn join(&self, other: &State) -> State {
        let intervals = (self.intervals.iter().zip(other.intervals.iter()))
            .map(|(a, b)| match (a, b) {
                (Some(a), Some(b)) => Some(a.join(*b)),
                _ => None,
            })
            .collect();
        let comparisons = (self.comparisons.iter())
            .filter(|&(local, comparison)| other.comparisons.get(local) == Some(comparison))
            .map(|(&local, &comparison)| (local, comparison))
            .collect();
        let both = |a: &IndexMap<Local, Local>, b: &IndexMap<Local, Local>| -> IndexMap<Local, Local> {
            (a.iter())
                .filter(|&(k, v)| b.get(k) == Some(v))
                .map(|(&k, &v)| (k, v))
                .collect()
        };
        let least = (self.least.iter())
            .filter_map(|(&k, &n)| other.least.get(&k).map(|&m| (k, n.min(m))))
            .collect();
        State {
            intervals,
            comparisons,
            copies: both(&self.copies, &other.copies),
            lens: both(&self.lens, &other.lens),
            below: self.below.intersection(&other.below).copied().collect(),
            least,
            empties: both(&self.empties, &other.empties),
        }
    }

    /// `local` written: what it held, and each comparison of it, forgotten.
    fn forget(&mut self, local: Local) {
        self.intervals[local] = None;
        self.comparisons.shift_remove(&local);
        self.comparisons
            .retain(|_, c| c.a != Value::Local(local) && c.b != Value::Local(local));
        self.copies.retain(|&copy, &mut of| copy != local && of != local);
        self.lens.retain(|&holder, &mut of| holder != local && of != local);
        self.below
            .retain(|&(x, y, _)| x != local && y != Term::Len(local) && y != Term::Int(local));
        self.least.shift_remove(&local);
        self.empties.retain(|&holder, &mut of| holder != local && of != local);
    }

    /// What `local` is a copy or a reborrow of, else itself.
    fn root(&self, local: Local) -> Local {
        self.copies.get(&local).copied().unwrap_or(local)
    }

    /// The slice whose length `local` holds, itself or what it copies.
    fn length_of(&self, local: Local) -> Option<Local> {
        self.lens
            .get(&local)
            .or_else(|| self.lens.get(&self.root(local)))
            .copied()
    }
}

struct Walk<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &'a Body<'tcx>,
    /// The locals whose place is lent `&mut` or as a raw pointer.
    lent: IndexSet<Local>,
}

/// What MIR proves of `body`, the one the emitter reads.
pub(in crate::lower) fn proofs<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, body: &Body<'tcx>) -> Proofs {
    let lent = (body.basic_blocks.iter())
        .flat_map(|data| &data.statements)
        .filter_map(|statement| match &statement.kind {
            StatementKind::Assign(assign) => match &assign.1 {
                Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, place) | Rvalue::RawPtr(_, place) => Some(place.local),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let walk = Walk {
        tcx,
        typing_env,
        body,
        lent,
    };
    walk.run()
}

impl<'tcx> Walk<'_, 'tcx> {
    fn run(&self) -> Proofs {
        let blocks = self.body.basic_blocks.len();
        let mut entry: IndexVec<BasicBlock, Option<State>> = IndexVec::from_elem_n(None, blocks);
        let mut visits: IndexVec<BasicBlock, u32> = IndexVec::from_elem_n(0, blocks);
        entry[mir::START_BLOCK] = Some(State {
            intervals: IndexVec::from_elem_n(None, self.body.local_decls.len()),
            comparisons: IndexMap::default(),
            copies: IndexMap::default(),
            lens: IndexMap::default(),
            below: IndexSet::default(),
            least: IndexMap::default(),
            empties: IndexMap::default(),
        });
        // A loop's heads, each the target of an edge back from a block it
        // dominates: where what still moves widens, and nowhere else, so
        // what a switch narrows past the head stays narrowed.
        let dominators = self.body.basic_blocks.dominators();
        let heads: IndexSet<BasicBlock> = (self.body.basic_blocks.iter_enumerated())
            .flat_map(|(from, data)| data.terminator().successors().map(move |to| (from, to)))
            .filter(|&(from, to)| dominators.dominates(to, from))
            .map(|(_, to)| to)
            .collect();
        let mut work: Vec<BasicBlock> = vec![mir::START_BLOCK];
        while let Some(block) = work.pop() {
            let Some(mut state) = entry[block].clone() else {
                continue;
            };
            visits[block] += 1;
            let data = &self.body.basic_blocks[block];
            for statement in &data.statements {
                self.statement(statement, &mut state);
            }
            for (target, out) in self.edges(block, &state) {
                let joined = match &entry[target] {
                    Some(old) => {
                        let mut joined = old.join(&out);
                        if heads.contains(&target) && visits[target] >= VISITS_BEFORE_WIDENING {
                            self.widen(old, &mut joined);
                        }
                        joined
                    }
                    None => out,
                };
                if entry[target].as_ref() != Some(&joined) {
                    entry[target] = Some(joined);
                    work.push(target);
                }
            }
        }
        self.proved(&entry)
    }

    /// Each bound of `joined` that moved from `old`, its type's.
    fn widen(&self, old: &State, joined: &mut State) {
        for (local, interval) in joined.intervals.iter_enumerated_mut() {
            let (Some(now), Some(was)) = (interval.as_mut(), old.intervals[local]) else {
                continue;
            };
            let Some(range) = self.range(self.body.local_decls[local].ty) else {
                continue;
            };
            if now.lo < was.lo {
                now.lo = range.lo;
            }
            if now.hi > was.hi {
                now.hi = range.hi;
            }
        }
    }

    /// The checks and operators proved, each at the state where it runs.
    fn proved(&self, entry: &IndexVec<BasicBlock, Option<State>>) -> Proofs {
        let mut proofs = Proofs::default();
        for (block, data) in self.body.basic_blocks.iter_enumerated() {
            let Some(mut state) = entry[block].clone() else {
                continue;
            };
            for (at, statement) in data.statements.iter().enumerate() {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let Rvalue::BinaryOp(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul), operands) = &assign.1
                    && let Some(range) = self.range(assign.0.ty(&self.body.local_decls, self.tcx).ty)
                    && let Some(result) = self.arithmetic(*op, &operands.0, &operands.1, &state)
                    && range.lo <= result.lo
                    && result.hi <= range.hi
                {
                    proofs.exact.insert(Location {
                        block,
                        statement_index: at,
                    });
                }
                self.statement(statement, &mut state);
            }
            if let TerminatorKind::Assert { msg, .. } = &data.terminator().kind
                && let AssertKind::BoundsCheck { len, index } = &**msg
                && (self.below_length(&state, len, index)
                    || (self.operand(len, &state))
                        .zip(self.operand(index, &state))
                        .is_some_and(|(len, index)| 0 <= index.lo && index.hi < len.lo))
            {
                proofs.bounds.insert(block);
            }
        }
        proofs
    }

    /// The integer range of `ty`, wasm32's `usize` 32 bits; none of what
    /// isn't an integer.
    fn range(&self, ty: Ty<'tcx>) -> Option<Interval> {
        let bits = |size: rustc_abi::Size| size.bits() as u32;
        let pointer = bits(self.tcx.data_layout.pointer_size());
        let (signed, width) = match *ty.kind() {
            ty::Uint(u) => (false, u.bit_width().map_or(pointer, |w| w as u32)),
            ty::Int(i) => (true, i.bit_width().map_or(pointer, |w| w as u32)),
            _ => return None,
        };
        Some(match signed {
            true => Interval {
                lo: -(1i128 << (width - 1)),
                hi: (1i128 << (width - 1)) - 1,
            },
            false => Interval {
                lo: 0,
                hi: (1i128 << width) - 1,
            },
        })
    }

    /// What `operand` holds: its interval, else its type's range; none of
    /// what isn't an integer.
    fn operand(&self, operand: &Operand<'tcx>, state: &State) -> Option<Interval> {
        match operand {
            Operand::Constant(c) => {
                let n = c.const_.try_eval_scalar_int(self.tcx, self.typing_env)?;
                let value = match c.ty().is_signed() {
                    true => n.to_int(n.size()),
                    false => n.to_uint(n.size()) as i128,
                };
                Some(Interval { lo: value, hi: value })
            }
            Operand::Copy(place) | Operand::Move(place) => {
                let range = self.range(place.ty(&self.body.local_decls, self.tcx).ty)?;
                Some(match place.as_local() {
                    Some(local) if !self.lent.contains(&local) => state.intervals[local].unwrap_or(range),
                    _ => range,
                })
            }
            Operand::RuntimeChecks(_) => None,
        }
    }

    /// What `a op b` is, where it stays in the integers it's of; none where
    /// it may wrap, which is then its type's range.
    fn arithmetic(&self, op: BinOp, a: &Operand<'tcx>, b: &Operand<'tcx>, state: &State) -> Option<Interval> {
        // Of `a - b` where `b <= a` is known, or `b < a`: at least none,
        // or one, `n - i` under `i < n`, or `len - i` under `i < len`.
        let local = |o: &Operand<'tcx>| o.place().and_then(|p| p.as_local()).map(|l| state.root(l));
        let at_least = match (op, local(a), local(b)) {
            (BinOp::Sub, Some(a), Some(b)) => {
                let greater = |strict: bool| {
                    state.below.contains(&(b, Term::Int(a), strict))
                        || state
                            .lens
                            .get(&a)
                            .is_some_and(|&slice| state.below.contains(&(b, Term::Len(slice), strict)))
                };
                if greater(true) {
                    Some(1)
                } else if greater(false) {
                    Some(0)
                } else {
                    None
                }
            }
            _ => None,
        };
        let (a, b) = (self.operand(a, state)?, self.operand(b, state)?);
        let mut result = Self::bare(op, a, b)?;
        if let Some(least) = at_least {
            result.lo = result.lo.max(least);
        }
        Some(result)
    }

    /// `a op b` of intervals alone.
    fn bare(op: BinOp, a: Interval, b: Interval) -> Option<Interval> {
        Some(match op {
            BinOp::Add => Interval {
                lo: a.lo.checked_add(b.lo)?,
                hi: a.hi.checked_add(b.hi)?,
            },
            BinOp::Sub => Interval {
                lo: a.lo.checked_sub(b.hi)?,
                hi: a.hi.checked_sub(b.lo)?,
            },
            BinOp::Mul => {
                let products = [
                    a.lo.checked_mul(b.lo)?,
                    a.lo.checked_mul(b.hi)?,
                    a.hi.checked_mul(b.lo)?,
                    a.hi.checked_mul(b.hi)?,
                ];
                Interval {
                    lo: *products.iter().min()?,
                    hi: *products.iter().max()?,
                }
            }
            _ => return None,
        })
    }

    /// What a comparison compares of `operand`: a constant, or a local, the
    /// one it's a copy of where it's one.
    fn value(&self, operand: &Operand<'tcx>, state: &State) -> Option<Value> {
        match operand {
            Operand::Constant(_) => self.operand(operand, &self.empty()).map(|i| Value::Constant(i.lo)),
            Operand::Copy(place) | Operand::Move(place) => place
                .as_local()
                .filter(|local| !self.lent.contains(local))
                .map(|local| Value::Local(state.copies.get(&local).copied().unwrap_or(local))),
            Operand::RuntimeChecks(_) => None,
        }
    }

    fn empty(&self) -> State {
        State {
            intervals: IndexVec::from_elem_n(None, self.body.local_decls.len()),
            comparisons: IndexMap::default(),
            copies: IndexMap::default(),
            lens: IndexMap::default(),
            below: IndexSet::default(),
            least: IndexMap::default(),
            empties: IndexMap::default(),
        }
    }

    /// `statement`'s write, what it writes worked out first, of what's known
    /// before it, then the local written forgotten and set.
    fn statement(&self, statement: &mir::Statement<'tcx>, state: &mut State) {
        let StatementKind::Assign(assign) = &statement.kind else {
            return;
        };
        let (place, rvalue) = &**assign;
        let Some(local) = place.as_local() else {
            return;
        };
        if self.lent.contains(&local) {
            state.forget(local);
            return;
        }
        let range = self.range(place.ty(&self.body.local_decls, self.tcx).ty);
        let fits = |r: &Interval| range.is_some_and(|t| t.lo <= r.lo && r.hi <= t.hi);
        let unlent = |of: &mir::Place<'tcx>| of.as_local().filter(|of| !self.lent.contains(of));
        let (mut comparison, mut copy_of, mut length_of) = (None, None, None);
        // `t = i - c` of a constant at least none: below what `i` is, and
        // strictly where `c` is one or more, `path[i - 1]` under `i < len`.
        let mut below_as: Vec<(Term, bool)> = Vec::new();
        if let Rvalue::BinaryOp(BinOp::Sub, operands) = rvalue
            && let Some(i) = operands.0.place().and_then(|p| p.as_local())
            && let Some(c) = self.operand(&operands.1, state).filter(|c| c.lo >= 0)
        {
            let i = state.root(i);
            below_as = (state.below.iter())
                .filter(|&&(x, _, _)| x == i)
                .map(|&(_, y, strict)| (y, strict || c.lo >= 1))
                .collect();
        }
        let interval = match rvalue {
            Rvalue::Use(operand, _) => {
                if let Operand::Copy(of) | Operand::Move(of) = operand {
                    copy_of = unlent(of).map(|of| state.root(of));
                }
                self.operand(operand, state)
            }
            // A reborrow, `&_1` or `&(*_1)`: of the same slice, its length.
            Rvalue::Ref(_, _, of) if matches!(of.projection.as_slice(), [] | [mir::PlaceElem::Deref]) => {
                copy_of = Some(of.local)
                    .filter(|of| !self.lent.contains(of))
                    .map(|of| state.root(of));
                None
            }
            Rvalue::BinaryOp(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul), operands) => {
                self.arithmetic(*op, &operands.0, &operands.1, state).filter(fits)
            }
            Rvalue::BinaryOp(
                op @ (BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne),
                operands,
            ) => {
                if let (Some(a), Some(b)) = (self.value(&operands.0, state), self.value(&operands.1, state))
                    && a != Value::Local(local)
                    && b != Value::Local(local)
                {
                    comparison = Some(Comparison { op: *op, a, b });
                }
                None
            }
            Rvalue::Cast(CastKind::IntToInt, operand, _) => self.operand(operand, state).filter(fits),
            Rvalue::UnaryOp(mir::UnOp::PtrMetadata, operand) => {
                if let Operand::Copy(of) | Operand::Move(of) = operand {
                    length_of = unlent(of).map(|of| state.root(of));
                }
                self.length(operand.ty(self.body, self.tcx))
            }
            _ => None,
        };
        state.forget(local);
        state.intervals[local] = interval.or(range).filter(|_| range.is_some());
        if let Some(c) = comparison {
            state.comparisons.insert(local, c);
        }
        if let Some(of) = copy_of.filter(|&of| of != local) {
            state.copies.insert(local, of);
        }
        if let Some(of) = length_of.filter(|&of| of != local) {
            state.lens.insert(local, of);
        }
        for (y, strict) in below_as {
            if y != Term::Len(local) && y != Term::Int(local) {
                state.below.insert((local, y, strict));
            }
        }
    }

    /// Whether `index` is below `len`'s slice's length, as the relations
    /// known say: a local below it, strictly, or one below its least.
    fn below_length(&self, state: &State, len: &Operand<'tcx>, index: &Operand<'tcx>) -> bool {
        let Some(holder) = len.place().and_then(|p| p.as_local()) else {
            return false;
        };
        let Some(slice) = state.length_of(holder) else {
            return false;
        };
        let least = state.least.get(&slice).copied().unwrap_or(0);
        let in_least = self.operand(index, state).is_some_and(|i| 0 <= i.lo && i.hi < least);
        let related = (index.place().and_then(|p| p.as_local()))
            .is_some_and(|i| state.below.contains(&(state.root(i), Term::Len(slice), true)));
        in_least || related
    }

    /// An array's length, of a pointer or reference to one, as its type
    /// says; of another, a slice's, at least none.
    fn length(&self, of: Ty<'tcx>) -> Option<Interval> {
        let pointee = of.builtin_deref(true).unwrap_or(of);
        let range = self.range(self.tcx.types.usize)?;
        Some(match *pointee.kind() {
            ty::Array(_, n) => {
                let n = i128::from(n.try_to_target_usize(self.tcx)?);
                Interval { lo: n, hi: n }
            }
            _ => range,
        })
    }

    /// Each edge out of `block`, and what's known along it: a switch of a
    /// comparison narrows its operands on each side, an `assert` of one on
    /// the side that goes on.
    fn edges(&self, block: BasicBlock, state: &State) -> Vec<(BasicBlock, State)> {
        let terminator = self.body.basic_blocks[block].terminator();
        match &terminator.kind {
            TerminatorKind::SwitchInt { discr, targets } => {
                let switched = discr.place().and_then(|p| p.as_local());
                let tested = switched.and_then(|l| state.comparisons.get(&l).copied());
                // A slice's `is_empty()` switched on: of its `false`, a length
                // of one at least.
                let emptied = switched.and_then(|l| state.empties.get(&l).copied());
                // An integer switched on: each value on its way, and of the
                // rest, its least and most passed over each listed.
                let integer = switched.filter(|&l| self.range(self.body.local_decls[l].ty).is_some());
                let values: Vec<u128> = targets.iter().map(|(value, _)| value).collect();
                let mut out = Vec::new();
                for (value, target) in targets.iter() {
                    let mut narrowed = state.clone();
                    if let Some(c) = tested {
                        self.narrow(c, value != 0, &mut narrowed);
                    }
                    if let Some(slice) = emptied
                        && value == 0
                    {
                        let least = narrowed.least.entry(slice).or_insert(0);
                        *least = (*least).max(1);
                    }
                    if let Some(l) = integer
                        && let Ok(v) = i128::try_from(value)
                    {
                        narrowed.intervals[l] = Some(Interval { lo: v, hi: v });
                        // Of a slice's length, `match list.len() { 1 => .. }`.
                        if let Some(slice) = narrowed.length_of(l) {
                            let least = narrowed.least.entry(slice).or_insert(0);
                            *least = (*least).max(v);
                        }
                    }
                    out.push((target, narrowed));
                }
                let mut otherwise = state.clone();
                if let Some(c) = tested
                    && values == [0]
                {
                    self.narrow(c, true, &mut otherwise);
                }
                if let Some(l) = integer
                    && let Some(mut known) = otherwise.intervals[l].or_else(|| self.range(self.body.local_decls[l].ty))
                {
                    let listed = |n: i128| u128::try_from(n).is_ok_and(|n| values.contains(&n));
                    while known.lo < known.hi && listed(known.lo) {
                        known.lo += 1;
                    }
                    while known.lo < known.hi && listed(known.hi) {
                        known.hi -= 1;
                    }
                    otherwise.intervals[l] = Some(known);
                    if let Some(slice) = otherwise.length_of(l) {
                        let least = otherwise.least.entry(slice).or_insert(0);
                        *least = (*least).max(known.lo);
                    }
                }
                out.push((targets.otherwise(), otherwise));
                out
            }
            TerminatorKind::Assert {
                cond, expected, target, ..
            } => {
                let mut on = state.clone();
                if let Some(c) = cond
                    .place()
                    .and_then(|p| p.as_local())
                    .and_then(|l| state.comparisons.get(&l).copied())
                {
                    self.narrow(c, *expected, &mut on);
                }
                vec![(*target, on)]
            }
            TerminatorKind::Call {
                func,
                args,
                destination,
                target,
                ..
            } => {
                let mut after = state.clone();
                if let Some(local) = destination.as_local() {
                    after.forget(local);
                    // std's `len()` and `is_empty()` of a slice, a `Vec` or
                    // text, by what it's given, `&v` the `v` it lends.
                    let of = (args.first())
                        .and_then(|arg| arg.node.place())
                        .and_then(|p| p.as_local())
                        .filter(|of| !self.lent.contains(of))
                        .map(|of| state.root(of));
                    // As recognition takes them (`slice_length`), never by name.
                    if let Some((callee, callee_args)) = func.const_fn_def()
                        && let Some(of) = of
                    {
                        match slice_length(self.tcx, callee, callee_args) {
                            Some(SliceLength::Len) => {
                                after.lens.insert(local, of);
                            }
                            Some(SliceLength::IsEmpty) => {
                                after.empties.insert(local, of);
                            }
                            _ => {}
                        }
                    }
                }
                target.map(|t| vec![(t, after)]).unwrap_or_default()
            }
            // A match candidate's and a loop's marks, rustc's: their real way
            // alone, the imaginary one no run takes.
            TerminatorKind::FalseEdge { real_target, .. } | TerminatorKind::FalseUnwind { real_target, .. } => {
                vec![(*real_target, state.clone())]
            }
            _ => terminator.successors().map(|t| (t, state.clone())).collect(),
        }
    }

    /// `c` known to be `holds`: each of its local operands narrowed.
    fn narrow(&self, c: Comparison, holds: bool, state: &mut State) {
        let op = match (c.op, holds) {
            (op, true) => op,
            (BinOp::Lt, false) => BinOp::Ge,
            (BinOp::Le, false) => BinOp::Gt,
            (BinOp::Gt, false) => BinOp::Le,
            (BinOp::Ge, false) => BinOp::Lt,
            (BinOp::Eq, false) => BinOp::Ne,
            (BinOp::Ne, false) => BinOp::Eq,
            _ => return,
        };
        let get = |v: Value, state: &State| match v {
            Value::Constant(n) => Some(Interval { lo: n, hi: n }),
            Value::Local(l) => state.intervals[l].or_else(|| self.range(self.body.local_decls[l].ty)),
        };
        // `x < y` or `x <= y` known, `y` a slice's length where a local
        // holds one, else an integer local.
        let term = |v: Value, state: &State| match v {
            Value::Local(l) => Some(state.length_of(l).map_or(Term::Int(l), Term::Len)),
            Value::Constant(_) => None,
        };
        let related = match op {
            BinOp::Lt | BinOp::Le => Some((c.a, c.b, op == BinOp::Lt)),
            BinOp::Gt | BinOp::Ge => Some((c.b, c.a, op == BinOp::Gt)),
            _ => None,
        };
        if let Some((Value::Local(x), y, strict)) = related
            && let Some(y) = term(y, state)
        {
            state.below.insert((x, y, strict));
        }
        // A constant below a slice's length, `b.len() > 4`: its least.
        if let Some((Value::Constant(c), Value::Local(holder), strict)) = related
            && let Some(slice) = state.length_of(holder)
        {
            let least = state.least.entry(slice).or_insert(0);
            *least = (*least).max(c + i128::from(strict));
        }
        let (Some(a), Some(b)) = (get(c.a, state), get(c.b, state)) else {
            return;
        };
        let (a2, b2) = match op {
            BinOp::Lt => (
                Interval {
                    lo: a.lo,
                    hi: a.hi.min(b.hi - 1),
                },
                Interval {
                    lo: b.lo.max(a.lo + 1),
                    hi: b.hi,
                },
            ),
            BinOp::Le => (
                Interval {
                    lo: a.lo,
                    hi: a.hi.min(b.hi),
                },
                Interval {
                    lo: b.lo.max(a.lo),
                    hi: b.hi,
                },
            ),
            BinOp::Gt => (
                Interval {
                    lo: a.lo.max(b.lo + 1),
                    hi: a.hi,
                },
                Interval {
                    lo: b.lo,
                    hi: b.hi.min(a.hi - 1),
                },
            ),
            BinOp::Ge => (
                Interval {
                    lo: a.lo.max(b.lo),
                    hi: a.hi,
                },
                Interval {
                    lo: b.lo,
                    hi: b.hi.min(a.hi),
                },
            ),
            BinOp::Eq => {
                let both = Interval {
                    lo: a.lo.max(b.lo),
                    hi: a.hi.min(b.hi),
                };
                (both, both)
            }
            _ => return,
        };
        for (v, narrowed) in [(c.a, a2), (c.b, b2)] {
            if let Value::Local(l) = v
                && narrowed.lo <= narrowed.hi
            {
                state.intervals[l] = Some(narrowed);
            }
        }
    }
}

/// What `--intervals-shadow` records: each line, and the proofs of each
/// body being lowered, the innermost last.
#[derive(Default)]
pub(crate) struct Shadow {
    lines: IndexSet<String>,
    bodies: Vec<(Proofs, IndexMap<Span, bool>)>,
}

/// A body's lowering begun, of a session that records the shadow: its
/// proofs, and of each span its operators are written at whether each is
/// proved, until the guard it gives drops.
pub(in crate::lower) fn shadow_body<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &Body<'tcx>,
) -> Option<ShadowScope<'tcx>> {
    let caches = crate::lower::session::caches(tcx);
    let shadow = caches.intervals_shadow.as_ref()?;
    let proofs = proofs(tcx, typing_env, body);
    let mut exact: IndexMap<Span, bool> = IndexMap::default();
    for (block, data) in body.basic_blocks.iter_enumerated() {
        for (statement_index, statement) in data.statements.iter().enumerate() {
            if let StatementKind::Assign(assign) = &statement.kind
                && let Rvalue::BinaryOp(BinOp::Add | BinOp::Sub | BinOp::Mul, _) = &assign.1
            {
                let proved = proofs.exact.contains(&Location { block, statement_index });
                *exact.entry(statement.source_info.span).or_insert(true) &= proved;
            }
        }
    }
    shadow.borrow_mut().bodies.push((proofs, exact));
    Some(ShadowScope { tcx })
}

/// Holds a body's proofs in the shadow while it's lowered.
pub(in crate::lower) struct ShadowScope<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl Drop for ShadowScope<'_> {
    fn drop(&mut self) {
        if let Some(shadow) = &crate::lower::session::caches(self.tcx).intervals_shadow {
            shadow.borrow_mut().bodies.pop();
        }
    }
}

/// What THIR's facts leave out: a bounds check, by the block its `assert`
/// ends, or a wrap.
pub(in crate::lower) enum LeftOut {
    Bounds(BasicBlock),
    Wrap,
}

/// THIR's facts leave out a check at `span` of `owner`'s body, `left`'s:
/// a line logged where MIR's proofs, of the body being lowered, don't cover
/// it. Nothing asked of a session that records no shadow.
pub(in crate::lower) fn shadow_left_out(
    tcx: TyCtxt<'_>,
    owner: rustc_span::def_id::DefId,
    span: Span,
    left: impl FnOnce() -> Option<LeftOut>,
) {
    let caches = crate::lower::session::caches(tcx);
    let Some(shadow) = &caches.intervals_shadow else {
        return;
    };
    let Some(left) = left() else {
        return;
    };
    let mut shadow = shadow.borrow_mut();
    let Some((proofs, exact)) = shadow.bodies.last() else {
        return;
    };
    let (kind, proved) = match left {
        LeftOut::Bounds(block) => ("bounds", proofs.bounds.contains(&block)),
        LeftOut::Wrap => ("wrap", exact.get(&span).copied().unwrap_or(false)),
    };
    if !proved {
        let at = tcx.sess.source_map().span_to_diagnostic_string(span);
        let line = format!("{kind}\t{}\t{at}\n", tcx.def_path_str(owner));
        shadow.lines.insert(line);
    }
}

/// Each check THIR's facts leave out that MIR doesn't prove, a line each,
/// `kind<TAB>function<TAB>where`: what `--intervals-shadow` writes.
pub(crate) fn shadow_report(tcx: TyCtxt<'_>) -> String {
    let caches = crate::lower::session::caches(tcx);
    caches
        .intervals_shadow
        .as_ref()
        .map(|s| s.borrow().lines.iter().map(String::as_str).collect())
        .unwrap_or_default()
}
