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
use crate::lower::recognition::{RangeStep, SliceLength, range_step, slice_length};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_index::IndexVec;
use rustc_middle::mir::{
    self, AssertKind, BasicBlock, BinOp, Body, CastKind, Local, Location, Operand, Rvalue, StatementKind,
    TerminatorKind,
};
use rustc_middle::ty::consts::ConstExt as _;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;
use std::collections::BTreeSet;

mod callers;
mod ranges;
mod session;
mod state;

use callers::parameter_intervals;
use ranges::stepped_ranges;
pub(in crate::lower) use session::*;
use state::*;

/// How many times a block is visited before what still moves widens.
const VISITS_BEFORE_WIDENING: u32 = 3;

/// How many times any block is visited before what still moves widens: a
/// loop no head dominates, of an irreducible flow a transformed body may
/// have, still ends.
const VISITS_BEFORE_WIDENING_ANYWHERE: u32 = 12;

/// What MIR proves of a body: each bounds check that never fails, by the
/// block its `assert` ends, and each `+`, `-` and `*` that never leaves its
/// type's range, where it's written.
#[derive(Default)]
pub(in crate::lower) struct Proofs {
    pub(in crate::lower) bounds: IndexSet<BasicBlock>,
    pub(in crate::lower) exact: IndexSet<Location>,
}

struct Walk<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &'a Body<'tcx>,
    /// The locals whose place is lent `&mut` or as a raw pointer.
    lent: IndexSet<Local>,
    /// The ranges of integers only their `next()` steps (`stepped_ranges`).
    stepped: IndexSet<Local>,
    /// What its callers give each of its parameters, of a function only
    /// calls reach (`parameter_intervals`); none where it's anything.
    from_callers: Vec<Option<Interval>>,
}

/// What MIR proves of `body`, the one the emitter reads.
fn proofs<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &Body<'tcx>,
    given: Vec<Option<Interval>>,
) -> Proofs {
    walk(tcx, typing_env, body, given).run()
}

fn walk<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &'a Body<'tcx>,
    given: Vec<Option<Interval>>,
) -> Walk<'a, 'tcx> {
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
    let stepped = stepped_ranges(tcx, body);
    Walk {
        tcx,
        typing_env,
        body,
        lent,
        stepped,
        from_callers: given,
    }
}

impl<'tcx> Walk<'_, 'tcx> {
    fn run(&self) -> Proofs {
        let entry = self.entries();
        self.proved(&entry)
    }

    /// What's known at each block's terminator, its statements run.
    fn ends(&self) -> IndexMap<BasicBlock, State> {
        let entry = self.entries();
        (self.body.basic_blocks.iter_enumerated())
            .filter_map(|(block, data)| {
                let mut state = entry[block].clone()?;
                for statement in &data.statements {
                    self.statement(statement, &mut state);
                }
                Some((block, state))
            })
            .collect()
    }

    /// What's known as each block begins, at its fixed point.
    fn entries(&self) -> IndexVec<BasicBlock, Option<State>> {
        let blocks = self.body.basic_blocks.len();
        let mut entry: IndexVec<BasicBlock, Option<State>> = IndexVec::from_elem_n(None, blocks);
        let mut visits: IndexVec<BasicBlock, u32> = IndexVec::from_elem_n(0, blocks);
        let mut intervals: IndexVec<Local, Option<Interval>> = IndexVec::from_elem_n(None, self.body.local_decls.len());
        for (at, &interval) in self.from_callers.iter().enumerate() {
            let param = Local::from_usize(at + 1);
            if at < self.body.arg_count && !self.lent.contains(&param) {
                intervals[param] = interval;
            }
        }
        entry[mir::START_BLOCK] = Some(State {
            intervals,
            comparisons: IndexMap::default(),
            copies: IndexMap::default(),
            lens: IndexMap::default(),
            below: IndexSet::default(),
            least: IndexMap::default(),
            empties: IndexMap::default(),
            ranges: IndexMap::default(),
        });
        // A loop's heads, each the target of an edge back from a block it
        // dominates: where what still moves widens, and nowhere else, so
        // what a switch narrows past the head stays narrowed. Of a block
        // the start reaches, which a transformed body's may not.
        let dominators = self.body.basic_blocks.dominators();
        let heads: IndexSet<BasicBlock> = (self.body.basic_blocks.iter_enumerated())
            .flat_map(|(from, data)| data.terminator().successors().map(move |to| (from, to)))
            .filter(|&(from, to)| dominators.is_reachable(from) && dominators.dominates(to, from))
            .map(|(_, to)| to)
            .collect();
        // Each block waiting, once, by its place in reverse postorder: what
        // flows into a block is visited before it, so each is visited
        // fewer times than a stack would.
        let mut rank: IndexVec<BasicBlock, usize> = IndexVec::from_elem_n(usize::MAX, blocks);
        for (at, &block) in self.body.basic_blocks.reverse_postorder().iter().enumerate() {
            rank[block] = at;
        }
        let mut work: BTreeSet<(usize, BasicBlock)> = BTreeSet::from([(rank[mir::START_BLOCK], mir::START_BLOCK)]);
        while let Some((_, block)) = work.pop_first() {
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
                        let widens = heads.contains(&target) && visits[target] >= VISITS_BEFORE_WIDENING
                            || visits[target] >= VISITS_BEFORE_WIDENING_ANYWHERE;
                        if widens {
                            self.widen(old, &mut joined);
                        }
                        joined
                    }
                    None => out,
                };
                if entry[target].as_ref() != Some(&joined) {
                    entry[target] = Some(joined);
                    work.insert((rank[target], target));
                }
            }
        }
        entry
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
                    && !(*op == BinOp::Mul && self.may_make_negative_zero(&operands.0, &operands.1, &state))
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
        // A 128-bit integer is never known: a `u128`'s range is past an
        // `i128`'s, and an `i128`'s ends are where `x + 1` or `-x` of them
        // would wrap here.
        if width >= 128 {
            return None;
        }
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

    /// Whether `a * b` can be JS's `-0`, which an integer never is (ADR
    /// 0064): one of them 0, the other below it, `-4 * 0`. JS's own `*` is
    /// then no integer's product, though it stays in range.
    fn may_make_negative_zero(&self, a: &Operand<'tcx>, b: &Operand<'tcx>, state: &State) -> bool {
        let (Some(a), Some(b)) = (self.operand(a, state), self.operand(b, state)) else {
            return true;
        };
        let zero = |x: Interval| x.lo <= 0 && 0 <= x.hi;
        zero(a) && b.lo < 0 || zero(b) && a.lo < 0
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
            // Of a divisor all of one sign, the quotient, truncated toward
            // zero, is at a corner: monotone in each operand.
            BinOp::Div if b.lo > 0 || b.hi < 0 => {
                let quotients = [a.lo / b.lo, a.lo / b.hi, a.hi / b.lo, a.hi / b.hi];
                Interval {
                    lo: *quotients.iter().min()?,
                    hi: *quotients.iter().max()?,
                }
            }
            // Of a positive divisor, the remainder has the dividend's sign,
            // and is less than the divisor and no more than the dividend.
            BinOp::Rem if b.lo > 0 => Interval {
                lo: if a.lo >= 0 { 0 } else { a.lo.max(1 - b.hi) },
                hi: if a.hi <= 0 { 0 } else { a.hi.min(b.hi - 1) },
            },
            // Of two never negative, at most the lesser.
            BinOp::BitAnd if a.lo >= 0 && b.lo >= 0 => Interval {
                lo: 0,
                hi: a.hi.min(b.hi),
            },
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
            ranges: IndexMap::default(),
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
        let ranged = self.ranged(rvalue, state);
        if self.lent.contains(&local) {
            state.forget(local);
            state.ranges.extend(
                ranged
                    .filter(|r| r.held != Held::Range || self.stepped.contains(&local))
                    .map(|ranged| (local, ranged)),
            );
            return;
        }
        let range = self.range(place.ty(&self.body.local_decls, self.tcx).ty);
        let fits = |r: &Interval| range.is_some_and(|t| t.lo <= r.lo && r.hi <= t.hi);
        let unlent = |of: &mir::Place<'tcx>| of.as_local().filter(|of| !self.lent.contains(of));
        let (mut comparison, mut copy_of, mut length_of) = (None, None, None);
        // `t = i - c` of a constant at least none, where `i` is known at
        // least `c`, so it never wraps: below what `i` is, and strictly
        // where `c` is one or more, `path[i - 1]` under `1 <= i < len`.
        // Of `i = 0`, `i - 1` wraps, and is past every length. And where
        // `i` holds a slice's length, at least `c`, strictly below it,
        // `list[list.len() - 1]` of a list not empty.
        let mut below_as: Vec<(Term, bool)> = Vec::new();
        if let Rvalue::BinaryOp(BinOp::Sub, operands) = rvalue
            && let Some(i) = operands.0.place().and_then(|p| p.as_local())
            && let Some(c) = self.operand(&operands.1, state).filter(|c| c.lo >= 0)
            && let Some(at_least) = self.operand(&operands.0, state).map(|i| i.lo)
        {
            let slice = state.length_of(i);
            let least = slice.and_then(|s| state.least.get(&s).copied()).unwrap_or(0);
            if at_least.max(least) >= c.hi {
                let i = state.root(i);
                below_as = (state.below.iter())
                    .filter(|&&(x, _, _)| x == i)
                    .map(|&(_, y, strict)| (y, strict || c.lo >= 1))
                    .collect();
                if let Some(slice) = slice.filter(|_| c.lo >= 1) {
                    below_as.push((Term::Len(slice), true));
                }
            }
        }
        // `t = i + 1` of `i` strictly below: at most what it's below,
        // `codes[i - 1]` after `codes[i]`'s check and `i += 1`.
        if let Rvalue::BinaryOp(BinOp::Add, operands) = rvalue
            && let Some(i) = operands.0.place().and_then(|p| p.as_local())
            && self.operand(&operands.1, state) == Some(Interval { lo: 1, hi: 1 })
        {
            let i = state.root(i);
            below_as.extend(
                (state.below.iter())
                    .filter(|&&(x, _, strict)| x == i && strict)
                    .map(|&(_, y, _)| (y, false)),
            );
        }
        let interval = match rvalue {
            // A value a range's `next()` gave, `i` of `for i in a..b`: at
            // least `a`, below `b`.
            Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _) | Rvalue::CopyForDeref(of)
                if let [mir::PlaceElem::Downcast(_, variant), mir::PlaceElem::Field(field, _)] =
                    of.projection.as_slice()
                    && variant.as_u32() == 1
                    && field.as_u32() == 0
                    && let Some(&Ranged {
                        held: Held::Next,
                        least,
                        end,
                    }) = state.ranges.get(&of.local) =>
            {
                let end_most = match end {
                    Value::Constant(n) => Some(n),
                    Value::Local(l) => state.intervals[l]
                        .or_else(|| self.range(self.body.local_decls[l].ty))
                        .map(|i| i.hi),
                };
                if let Value::Local(l) = end {
                    below_as.push((state.length_of(l).map_or(Term::Int(l), Term::Len), true));
                }
                end_most
                    .map(|most| Interval {
                        lo: least,
                        hi: most - 1,
                    })
                    .filter(|i| i.lo <= i.hi)
                    .filter(fits)
            }
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
            Rvalue::BinaryOp(
                op @ (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem | BinOp::BitAnd),
                operands,
            ) => self.arithmetic(*op, &operands.0, &operands.1, state).filter(fits),
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
        state.ranges.extend(
            ranged
                .filter(|r| r.held != Held::Range || self.stepped.contains(&local))
                .map(|ranged| (local, ranged)),
        );
        for (y, strict) in below_as {
            if y != Term::Len(local) && y != Term::Int(local) {
                state.below.insert((local, y, strict));
            }
        }
    }

    /// What `rvalue` makes of a range of integers: one built, `a..b`, or
    /// one moved or copied, or a `&mut` of one `next()` alone steps.
    fn ranged(&self, rvalue: &Rvalue<'tcx>, state: &State) -> Option<Ranged> {
        match rvalue {
            Rvalue::Aggregate(kind, fields)
                if let mir::AggregateKind::Adt(adt, ..) = **kind
                    && self.tcx.is_lang_item(adt, LangItem::Range)
                    && let [start, end] = fields.raw.as_slice() =>
            {
                Some(Ranged {
                    held: Held::Range,
                    least: self.operand(start, state)?.lo,
                    end: self.value(end, state)?,
                })
            }
            Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _) => {
                let ranged = *state.ranges.get(&of.as_local()?)?;
                (ranged.held == Held::Range).then_some(ranged)
            }
            Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of) => {
                let ranged = *state.ranges.get(&of.local)?;
                match (of.projection.as_slice(), ranged.held) {
                    ([], Held::Range) if self.stepped.contains(&of.local) => Some(Ranged {
                        held: Held::Lent,
                        ..ranged
                    }),
                    // A reborrow of one lent, `&mut (*lent)`.
                    ([mir::PlaceElem::Deref], Held::Lent) => Some(ranged),
                    _ => None,
                }
            }
            _ => None,
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
    /// An array's length, of a pointer or reference to one, as its type
    /// says; of a slice's or text's, at most what an allocation holds of
    /// its items, `isize::MAX` bytes over each one's size, as Rust's every
    /// allocation is, so a sum of two lengths of `u8`s never wraps.
    fn length(&self, of: Ty<'tcx>) -> Option<Interval> {
        let pointee = of.builtin_deref(true).unwrap_or(of);
        let range = self.range(self.tcx.types.usize)?;
        let most_bytes = self.range(self.tcx.types.isize)?.hi;
        let item = match *pointee.kind() {
            ty::Array(_, n) => {
                let n = i128::from(n.try_to_target_usize(self.tcx)?);
                return Some(Interval { lo: n, hi: n });
            }
            ty::Slice(item) => Some(item),
            ty::Str => Some(self.tcx.types.u8),
            _ => None,
        };
        let size = item
            .and_then(|item| self.tcx.layout_of(self.typing_env.as_query_input(item)).ok())
            .map(|layout| layout.size.bytes() as i128);
        Some(match size {
            Some(size) if size > 0 => Interval {
                lo: 0,
                hi: most_bytes / size,
            },
            _ => range,
        })
    }

    /// A collection's length, a slice's, text's, or a `Vec`'s or a
    /// `VecDeque`'s of its first type argument, at most what an allocation
    /// holds of its items (`length`).
    fn length_of_collection(&self, of: Ty<'tcx>) -> Option<Interval> {
        let held = of.peel_refs();
        match *held.kind() {
            ty::Slice(_) | ty::Str | ty::Array(..) => self.length(held),
            ty::Adt(_, args) => {
                let item = args.types().next()?;
                self.length(Ty::new_slice(self.tcx, item))
            }
            _ => None,
        }
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
                                // At most what an allocation holds of its items.
                                let receiver = args.first().map(|arg| arg.node.ty(self.body, self.tcx));
                                if let Some(most) = receiver.and_then(|ty| self.length_of_collection(ty)) {
                                    after.intervals[local] = Some(most);
                                }
                            }
                            Some(SliceLength::IsEmpty) => {
                                after.empties.insert(local, of);
                            }
                            _ => {}
                        }
                    }
                    // A range's `into_iter()`, itself, and its `next()`, what
                    // it gives.
                    let given = (args.first())
                        .and_then(|arg| arg.node.place())
                        .and_then(|p| p.as_local())
                        .and_then(|of| state.ranges.get(&of).copied());
                    if let Some((callee, callee_args)) = func.const_fn_def()
                        && let Some(given) = given
                    {
                        match (range_step(self.tcx, callee, callee_args), given.held) {
                            (Some(RangeStep::IntoIter), Held::Range) if self.stepped.contains(&local) => {
                                after.ranges.insert(local, given);
                            }
                            (Some(RangeStep::Next), Held::Lent) => {
                                after.ranges.insert(
                                    local,
                                    Ranged {
                                        held: Held::Next,
                                        ..given
                                    },
                                );
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
            // Unequal to a value known exactly, at either end: past it,
            // `steps != 0`.
            BinOp::Ne => {
                let past = |x: Interval, y: Interval| match y.lo == y.hi {
                    true if x.lo == y.lo => Interval { lo: x.lo + 1, hi: x.hi },
                    true if x.hi == y.lo => Interval { lo: x.lo, hi: x.hi - 1 },
                    _ => x,
                };
                (past(a, b), past(b, a))
            }
            _ => return,
        };
        for (v, narrowed) in [(c.a, a2), (c.b, b2)] {
            if let Value::Local(l) = v
                && narrowed.lo <= narrowed.hi
            {
                state.narrowed_to(l, narrowed);
            }
        }
    }
}
