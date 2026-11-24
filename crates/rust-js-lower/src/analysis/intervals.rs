//! Integer intervals of a body's locals, a forward dataflow over the MIR
//! the emitter reads (#1671 change 1, #1672 M2): which bounds checks and
//! wraps that MIR proves never fail, of what it says alone, each order
//! rustc's. Run in shadow first: where THIR's ranges leave out a check this
//! doesn't prove, a line is logged (`--shadow intervals=<file>`), a wrong-answer
//! candidate or a gap of this analysis's precision.
//!
//! Each integer local holds an interval, its type's range where nothing is
//! known; a `bool` a comparison made holds that comparison, which narrows
//! its operands on each side of a switch of it, and past an `assert` of it.
//! A local whose place is lent `&mut` or as a raw pointer is never known.
//! At a loop's head, a bound still moving after a few visits widens to the
//! next constant the body compares an integer with, or past them all to its
//! type's.

use crate::bindings::{is_rest_value, reads_string_length};
use crate::recognition::{
    RangeStep, SliceLength, call_indexes, counts_utf16_units, integer_from_bool, integer_min_max, local_key_with,
    option_cloned_kind, range_index, range_index_bounds, range_of_integers, range_step, rc_deref, ref_deref,
    refcell_borrow, slice_get, slice_length, text_length,
};
use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_index::IndexVec;
use rustc_middle::mir::{
    self, AssertKind, BasicBlock, BinOp, Body, CastKind, Local, Location, Operand, Rvalue, StatementKind,
    TerminatorKind,
};
use rustc_middle::ty::consts::ConstExt as _;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Span;
use rustc_span::def_id::DefId;
use std::collections::BTreeSet;

mod arithmetic;
mod called_on;
mod shadowed;
mod transfer;
pub(crate) use called_on::carries_code;
mod callers;
mod calls;
mod captures;
mod cells;
mod derived;
mod fields;
mod fixed_points;
mod guards;
mod keys;
mod proved;
mod pushed;
mod ranges;
mod session;
mod shared;
mod state;
mod tables;

use crate::pointers::Libraries;
use callers::parameter_intervals;
use captures::{capture_intervals, closure_results};
use fixed_points::FixedPoints;
use ranges::stepped_ranges;
pub use session::*;
use state::*;

/// What a function only calls reach is given at every call: each
/// parameter's interval, and each relation between them, `i < s.len()`
/// of `item(items, i)` under `i < items.len()` (#1672 M3), the one
/// parameter's index below the other's length, strictly or not.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Given {
    intervals: Vec<Option<Interval>>,
    below: Vec<(usize, usize, bool)>,
    /// Of a closure, what each of its captures is where it's made, by
    /// field (`capture_intervals`, #1701).
    captures: Vec<Option<Interval>>,
}

/// The most a JS string's `length` is: `2^31 - 1`, as every engine's
/// string is at most, so two add without passing `u32`'s most (ADRs 0632,
/// 0682).
const STRING_LENGTH_MOST: i128 = (1 << 31) - 1;

/// How many passes narrow the fixed point widening reached.
const NARROWING_PASSES: u32 = 2;

/// How many times a block is visited before what still moves widens, at
/// any block: narrowing after gives back what a loop's test bounds, as
/// widening at loops' heads alone had kept, so one rule, which ends of an
/// irreducible flow too.
const VISITS_BEFORE_WIDENING: u32 = 3;

/// What MIR proves of a body: each bounds check that never fails, by the
/// block its `assert` ends, and each `+`, `-` and `*` that never leaves its
/// type's range, where it's written.
#[derive(Default)]
pub(crate) struct Proofs {
    pub(crate) bounds: IndexSet<BasicBlock>,
    pub(crate) exact: IndexSet<Location>,
    /// Each integer `as` another whose value is in its target's range, the
    /// same number, `s as usize` of `s >= 0` (ADR 0701, #1672 M3).
    pub(crate) casts: IndexSet<Location>,
    /// The blocks whose terminator is a `OnceCell`'s `set` of a cell known
    /// empty there (`empty_sets`, ADR 0847, #1672 M3).
    pub(crate) empty_sets: IndexSet<BasicBlock>,
    /// The blocks whose terminator is a call that checks its indexes
    /// against its receiver's length, each known in bounds there: a
    /// `Vec`'s `v[i]`, `swap`, `remove` (`call_indexes`, #1672 M3, step 5).
    pub(crate) call_bounds: IndexSet<BasicBlock>,
    /// The blocks whose terminator is a slice's `first()` or `last()` of
    /// one known not empty there, each `Some` (#1860).
    pub(crate) present: IndexSet<BasicBlock>,
}

struct Walk<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    /// The body, of a closure's its captures each a local (`as_locals`).
    body: std::borrow::Cow<'a, Body<'tcx>>,
    /// The locals whose place is lent `&mut` or as a raw pointer.
    lent: IndexSet<Local>,
    /// The ranges of integers only their `next()` steps (`stepped_ranges`).
    stepped: IndexSet<Local>,
    /// What its callers give each of its parameters, of a function only
    /// calls reach (`parameter_intervals`); none where it's anything.
    from_callers: Given,
    /// The locals a raw pointer is made of, which a push's length is never
    /// known of (`pushes`).
    pointed: IndexSet<Local>,
    /// The locals that hold or lend a `RefCell`, whose lengths a call may
    /// write (`cells`, #1741).
    cells: IndexSet<Local>,
    /// Of a closure's body, each capture's local given what it was made
    /// with, by field (`as_locals`, #1701).
    captured: Vec<(usize, Local)>,
    /// Each `&mut` of a `Vec` or an integer a local holds, lent only to the
    /// calls it's given to, by the local holding it (`called_on`).
    called_on: called_on::Holders,
    /// Of each such holder that is or holds a closure, that closure's code
    /// (`called_on`, #1947).
    called_on_code: IndexMap<Local, (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>)>,
    /// The `borrow_mut()` guards held while they live (`guards`).
    guards: guards::Guards,
    /// The row types of tables no `&mut` resizes (`tables::rows_kept`).
    kept_rows: IndexSet<Ty<'tcx>>,
    /// Whether the body's formatting may run code (`formats_code`).
    formats_code: bool,
    /// The crate's libraries, which `pointer_may_keep` asks, where the
    /// walk lowers a body; none of a walk for what callers give (#1947).
    libraries: Option<Libraries<'a, 'tcx>>,
    /// Where the walk's fixed point is kept for another walk of the same
    /// body and `Given` (`FixedPoints`), of a walk for what callers give.
    fixed_points: Option<&'a FixedPoints>,
    /// What each statement may write, where the session records the
    /// shadow: the query the rules are compared with (`shadowed`).
    effects: Option<crate::analysis::Effects>,
    /// Each statement's location, by its address, which the transfer asks
    /// the query of (`transfer`).
    located: IndexMap<usize, Location>,
}

/// Whether `ty` is an integer a JS number holds as it is: of 32 bits at
/// most, `usize` and `isize` rust-js's 32 (ADR 0090); a 64-bit one is a
/// `BigInt`.
fn held_by_a_number(ty: Ty<'_>) -> bool {
    matches!(
        ty.kind(),
        ty::Uint(ty::UintTy::U8 | ty::UintTy::U16 | ty::UintTy::U32 | ty::UintTy::Usize)
            | ty::Int(ty::IntTy::I8 | ty::IntTy::I16 | ty::IntTy::I32 | ty::IntTy::Isize)
    )
}

/// What MIR proves of `body`, the one the emitter reads.
fn proofs<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &'a Body<'tcx>,
    given: Given,
    libraries: Option<Libraries<'a, 'tcx>>,
    cells: Option<&crate::analysis::CellBorrows<'tcx>>,
) -> Proofs {
    let mut walk = walk(tcx, typing_env, body, given);
    walk.libraries = libraries;
    let caches = crate::session::caches(tcx);
    if crate::session::shadowing(tcx, crate::session::ShadowKind::Intervals) || caches.intervals_by_effects {
        let made = body.local_decls.len();
        walk.effects = Some(crate::analysis::Effects::of(
            tcx,
            typing_env,
            &walk.body,
            libraries,
            made,
            cells
                .map(|cells| move |ty| cells.unnamed_may_borrow(tcx, ty))
                .as_ref()
                .map(|f| f as &dyn Fn(Ty<'tcx>) -> bool),
        ));
        walk.located = (walk.body.basic_blocks.iter_enumerated())
            .flat_map(|(block, data)| {
                (data.statements.iter().enumerate()).map(move |(statement_index, s)| {
                    (std::ptr::from_ref(s).addr(), Location { block, statement_index })
                })
            })
            .collect();
    }
    walk.run()
}

/// Whether `place`, lent `&mut`, is an integer local the walk forgets
/// where it's lent, not in the whole body: borrowck holds that it's never
/// assigned while the borrow lives, so once it's assigned again it's what
/// that gave, `k` of `k += 1` before a later `&mut k`. Not one a raw pointer
/// is made of, which may write it after.
fn lent_where_borrowed(body: &Body<'_>, place: &mir::Place<'_>) -> bool {
    let pointed = (body.basic_blocks.iter())
        .flat_map(|data| &data.statements)
        .any(|statement| {
            matches!(&statement.kind, StatementKind::Assign(assign)
            if matches!(&assign.1, Rvalue::RawPtr(_, of) if of.local == place.local))
        });
    place.projection.is_empty() && body.local_decls[place.local].ty.is_integral() && !pointed
}

fn walk<'a, 'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    body: &'a Body<'tcx>,
    given: Given,
) -> Walk<'a, 'tcx> {
    let (body, captured) = match captures::as_locals(tcx, body) {
        Some((rewritten, captured)) => (std::borrow::Cow::Owned(rewritten), captured),
        None => (std::borrow::Cow::Borrowed(body), Vec::new()),
    };
    // A struct's fields a reference parameter is read through, locals too.
    let body = match fields::fields_as_locals(tcx, &body) {
        Some(rewritten) => std::borrow::Cow::Owned(rewritten),
        None => body,
    };
    // And a struct's fields a `RefCell` guard lends, while it lives (#1815).
    let body = match fields::guard_fields_as_locals(tcx, &body) {
        Some(rewritten) => std::borrow::Cow::Owned(rewritten),
        None => body,
    };
    let called_on::CalledOn {
        holders: called_on,
        closures: called_on_code,
    } = called_on::called_on(tcx, &body);
    let guards = guards::guards(tcx, &body);
    let kept_rows = tables::rows_kept(tcx, &body);
    let (lent, stepped, pointed, cells) = facts(tcx, &body, &called_on, (&guards, &kept_rows));
    let formats_code = called_on::formats_code(tcx, &body);
    Walk {
        tcx,
        typing_env,
        body,
        lent,
        stepped,
        from_callers: given,
        pointed,
        cells,
        captured,
        called_on,
        called_on_code,
        guards,
        kept_rows,
        formats_code,
        libraries: None,
        fixed_points: None,
        effects: None,
        located: IndexMap::default(),
    }
}

/// What a walk of `body` knows before it starts: the locals lent, the
/// ranges only their `next()` steps, the locals a raw pointer is made of,
/// and those that hold or lend a `RefCell`.
fn facts<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    called_on: &called_on::Holders,
    (guards, kept_rows): (&guards::Guards, &IndexSet<Ty<'tcx>>),
) -> (IndexSet<Local>, IndexSet<Local>, IndexSet<Local>, IndexSet<Local>) {
    // A local whose own place is lent; not a reference to a slice, an
    // array or text whose pointee is, `&mut (*heap)` of `heap.swap(..)`,
    // which no write through changes the length of.
    let fixed_length = |local: Local| {
        let ty = body.local_decls[local].ty;
        ty.builtin_deref(true)
            .is_some_and(|pointee| matches!(pointee.kind(), ty::Slice(_) | ty::Str | ty::Array(..)))
    };
    let lent = (body.basic_blocks.iter())
        .flat_map(|data| &data.statements)
        .filter_map(|statement| match &statement.kind {
            StatementKind::Assign(assign) => match &assign.1 {
                Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, place) | Rvalue::RawPtr(_, place) => Some(place),
                _ => None,
            },
            _ => None,
        })
        .filter(|place| !(matches!(place.projection.first(), Some(mir::PlaceElem::Deref)) && fixed_length(place.local)))
        .filter(|place| !lent_where_borrowed(body, place))
        // A `Vec` lent only to the calls it's given to, and those `&mut`s.
        .filter(|place| !called_on.contains_key(&place.local) && !called_on.values().flatten().any(|&of| of == place.local))
        // A held guard, lent only to its `deref_mut`, and what that lends.
        .filter(|place| !guards.held.contains(&place.local) && !guards.lends.contains(&place.local))
        // A row's `&mut` of a table whose rows nothing resizes (`rows_kept`).
        .filter(|place| !matches!(*body.local_decls[place.local].ty.kind(), ty::Ref(_, row, ty::Mutability::Mut) if kept_rows.contains(&row)))
        .map(|place| place.local)
        .collect();
    let stepped = stepped_ranges(tcx, body);
    let pointed = (body.basic_blocks.iter())
        .flat_map(|data| &data.statements)
        .filter_map(|statement| match &statement.kind {
            StatementKind::Assign(assign) => match &assign.1 {
                Rvalue::RawPtr(_, place) => Some(place.local),
                _ => None,
            },
            _ => None,
        })
        .collect();
    (lent, stepped, pointed, cells::cells(tcx, body))
}

impl<'tcx> Walk<'_, 'tcx> {
    fn run(&self) -> Proofs {
        let entry = self.entries();
        if let Some(effects) = &self.effects
            && !crate::session::caches(self.tcx).intervals_by_effects
        {
            self.shadow_effects(effects, &entry);
        }
        self.proved(&entry)
    }

    /// What's known at each block's terminator, its statements run.
    /// What's known at each of `blocks`' terminators, its statements run:
    /// those alone kept, a body of thousands of blocks the end of each
    /// otherwise (#1672).
    fn ends(&self, blocks: &IndexSet<BasicBlock>) -> IndexMap<BasicBlock, State> {
        let entry = self.entries();
        (self.body.basic_blocks.iter_enumerated())
            .filter(|(block, _)| blocks.contains(block))
            .filter_map(|(block, data)| {
                let mut state = entry[block].clone()?;
                for statement in &data.statements {
                    self.statement(statement, &mut state);
                }
                Some((block, state))
            })
            .collect()
    }

    /// What's known as each block begins, at its fixed point, walked.
    fn fixed_point(&self) -> IndexVec<BasicBlock, Option<State>> {
        let blocks = self.body.basic_blocks.len();
        let mut entry: IndexVec<BasicBlock, Option<State>> = IndexVec::from_elem_n(None, blocks);
        let mut visits: IndexVec<BasicBlock, u32> = IndexVec::from_elem_n(0, blocks);
        let mut intervals = state::LocalIntervals::default();
        for (at, &interval) in self.from_callers.intervals.iter().enumerate() {
            let param = Local::from_usize(at + 1);
            if at < self.body.arg_count && !self.lent.contains(&param) {
                intervals.set(param, interval);
            }
        }
        // Of a closure's, each capture given what it was made with (#1701).
        for &(field, local) in &self.captured {
            if !self.lent.contains(&local) {
                intervals.set(local, self.from_callers.captures.get(field).copied().flatten());
            }
        }
        let param =
            |at: usize| Some(Local::from_usize(at + 1)).filter(|p| at < self.body.arg_count && !self.lent.contains(p));
        let below: shared::SharedSet<(Local, Term, bool)> = (self.from_callers.below.iter())
            .filter_map(|&(index, of, strict)| Some((param(index)?, Term::Len(param(of)?), strict)))
            .collect();
        entry[mir::START_BLOCK] = Some(State {
            intervals,
            comparisons: Default::default(),
            copies: Default::default(),
            lens: Default::default(),
            below,
            below_past: Default::default(),
            least: Default::default(),
            splits: Default::default(),
            empties: Default::default(),
            ranges: Default::default(),
            sums: Default::default(),
            gets: Default::default(),
            positions: Default::default(),
            iterates: Default::default(),
            pushed: Default::default(),
            lending: Default::default(),
            lent_away: Default::default(),
            escaped: Default::default(),
            guarded: Default::default(),
            keys: Default::default(),
            has_keys: Default::default(),
            equal_lens: Default::default(),
            rows: Default::default(),
            halves: Default::default(),
            doubled: Default::default(),
        });
        // Each block waiting, once, by its place in reverse postorder: what
        // flows into a block is visited before it, so each is visited
        // fewer times than a stack would.
        let mut rank: IndexVec<BasicBlock, usize> = IndexVec::from_elem_n(usize::MAX, blocks);
        for (at, &block) in self.body.basic_blocks.reverse_postorder().iter().enumerate() {
            rank[block] = at;
        }
        let thresholds = self.thresholds();
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
                        if visits[target] >= VISITS_BEFORE_WIDENING {
                            self.widen(old, &mut joined, &thresholds);
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
        // Narrowing (ADR 1137): from the fixed point, each block's entry the
        // join of what flows into it, no widening, a few passes in reverse
        // postorder, each block's flow out its new one; what a loop's test
        // bounds, which widening at its head let go, comes back, `i` of
        // `i + 1 < len`. A fixed point's flow is one still, so each pass
        // stays above what runs.
        let flow = |state: &Option<State>, block: BasicBlock| -> Vec<(BasicBlock, State)> {
            let Some(mut state) = state.clone() else {
                return Vec::new();
            };
            for statement in &self.body.basic_blocks[block].statements {
                self.statement(statement, &mut state);
            }
            self.edges(block, &state)
        };
        let mut outs: IndexVec<BasicBlock, Vec<(BasicBlock, State)>> = (self.body.basic_blocks.indices())
            .map(|block| flow(&entry[block], block))
            .collect();
        let predecessors = self.body.basic_blocks.predecessors();
        for _ in 0..NARROWING_PASSES {
            for &block in self.body.basic_blocks.reverse_postorder() {
                if block != mir::START_BLOCK {
                    let into = (predecessors[block].iter())
                        .flat_map(|&from| outs[from].iter().filter(|(to, _)| *to == block).map(|(_, out)| out))
                        .fold(None::<State>, |joined, out| {
                            Some(joined.map_or_else(|| out.clone(), |j| j.join(out)))
                        });
                    entry[block] = into;
                }
                outs[block] = flow(&entry[block], block);
            }
        }
        entry
    }

    /// The constants the body compares an integer with, each one less and one
    /// more too, and its collections' most lengths, in order: where a
    /// widened bound stops before its type's
    /// (ADR 1137). A counter a loop breaks out of past 10, `count > 10`, which
    /// a `continue` brings back to the head as it was, widened to its type's
    /// is a fixed point no narrowing leaves, as the `continue` keeps it; to
    /// 10, the test keeps it there, and `count + 1` is JS's own.
    fn thresholds(&self) -> Vec<i128> {
        let mut thresholds: Vec<i128> = (self.body.basic_blocks.iter())
            .flat_map(|data| &data.statements)
            .filter_map(|statement| match &statement.kind {
                StatementKind::Assign(assign) => match &assign.1 {
                    Rvalue::BinaryOp(
                        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne,
                        operands,
                    ) => Some(operands),
                    _ => None,
                },
                _ => None,
            })
            .flat_map(|operands| [&operands.0, &operands.1])
            .filter_map(|operand| match operand {
                Operand::Constant(c) if self.range(c.ty()).is_some() => self.constant(c),
                _ => None,
            })
            .flat_map(|c| [c - 1, c, c + 1])
            .collect();
        // And each of its collections' most length, and one less, the most
        // an index below it is: `r` of `while let Some(row) = rows.get(r)`,
        // which grows by the outer loop's turns, widened at an inner loop's
        // head to its type's, is a fixed point no narrowing leaves, as the
        // inner loop's back edge keeps it; to the most `get`'s `Some` puts
        // it below, `r + 1` is JS's own (#1742).
        thresholds.extend(
            (self.body.local_decls.iter())
                .filter_map(|decl| self.length_of_collection(decl.ty))
                .flat_map(|most| [most.hi - 1, most.hi]),
        );
        thresholds.sort_unstable();
        thresholds.dedup();
        thresholds
    }

    /// Each bound of `joined` that moved from `old`, to the next of
    /// `thresholds` past it, or its type's.
    fn widen(&self, old: &State, joined: &mut State, thresholds: &[i128]) {
        let known: Vec<(Local, Interval)> = joined.intervals.known().collect();
        for (local, mut now) in known {
            let Some(was) = old.intervals[local] else {
                continue;
            };
            let Some(range) = self.range(self.body.local_decls[local].ty) else {
                continue;
            };
            if now.lo < was.lo {
                now.lo = (thresholds.iter().rev().copied())
                    .find(|&t| t <= now.lo && range.lo <= t)
                    .unwrap_or(range.lo);
            }
            if now.hi > was.hi {
                now.hi = (thresholds.iter().copied())
                    .find(|&t| now.hi <= t && t <= range.hi)
                    .unwrap_or(range.hi);
            }
            if joined.intervals[local] != Some(now) {
                joined.intervals.set(local, Some(now));
            }
        }
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
    /// An integer constant's value.
    fn constant(&self, c: &mir::ConstOperand<'tcx>) -> Option<i128> {
        let n = c.const_.try_eval_scalar_int(self.tcx, self.typing_env)?;
        Some(match c.ty().is_signed() {
            true => n.to_int(n.size()),
            false => n.to_uint(n.size()) as i128,
        })
    }

    fn operand(&self, operand: &Operand<'tcx>, state: &State) -> Option<Interval> {
        match operand {
            Operand::Constant(c) => self.constant(c).map(|value| Interval { lo: value, hi: value }),
            Operand::Copy(place) | Operand::Move(place) => {
                let range = self.range(place.ty(&self.body.local_decls, self.tcx).ty)?;
                Some(match place.as_local() {
                    Some(local) if !self.lent.contains(&local) => {
                        let known = state.intervals[local].unwrap_or(range);
                        // At most what it's below is at most, a length's
                        // what an allocation holds: `i + 1` of `i < len`.
                        let below = (state.below.iter())
                            .filter(|&&(x, _, _)| x == state.root(local))
                            .filter_map(|&(_, y, strict)| {
                                let most = match y {
                                    Term::Len(slice) => self.length_of_collection(self.body.local_decls[slice].ty)?.hi,
                                    Term::Int(l) => {
                                        state.intervals[l]
                                            .or_else(|| self.range(self.body.local_decls[l].ty))?
                                            .hi
                                    }
                                };
                                Some(most - i128::from(strict))
                            })
                            .min();
                        // A length at least its slice's least, `xs.len() - 1` of
                        // a slice not empty.
                        let least = (state.length_of(local))
                            .and_then(|slice| state.least.get(&slice).copied())
                            .filter(|&least| least <= known.hi);
                        let known = least.map_or(known, |least| Interval {
                            lo: known.lo.max(least),
                            hi: known.hi,
                        });
                        match below {
                            Some(most) if known.lo <= most => Interval {
                                lo: known.lo,
                                hi: known.hi.min(most),
                            },
                            _ => known,
                        }
                    }
                    _ => range,
                })
            }
            Operand::RuntimeChecks(_) => None,
        }
    }

    /// What a comparison compares of `operand`: a constant, or a local, the
    /// one it's a copy of where it's one.
    fn value(&self, operand: &Operand<'tcx>, state: &State) -> Option<Value> {
        match operand {
            Operand::Constant(c) => match self.constant(c) {
                Some(n) => Some(Value::Constant(n)),
                None => self.range(c.ty()).map(|range| Value::Range(range.lo, range.hi)),
            },
            Operand::Copy(place) | Operand::Move(place) => place
                .as_local()
                .filter(|local| !self.lent.contains(local))
                .map(|local| Value::Local(state.copies.get(&local).copied().unwrap_or(local))),
            Operand::RuntimeChecks(_) => None,
        }
    }

    /// `statement`'s write, what it writes worked out first, of what's known
    /// before it, then the local written forgotten and set.
    fn statement_rules(&self, statement: &mir::Statement<'tcx>, state: &mut State) {
        self.pushes(statement, state);
        // A write through a raw pointer, which may reach a cell's value with
        // no call (`cells`, #1741), or an escaped `Vec` (#1966).
        if let StatementKind::Assign(assign) = &statement.kind
            && self.written_through_a_pointer(&assign.0)
        {
            self.cells.iter().for_each(|&cell| state.forget_lengths(cell));
            state.forget_escaped();
        }
        // A `&mut` or a raw pointer made through a raw pointer, `&mut
        // (*p)` of `(*STASH).truncate(0)`, which std's code writes through
        // running none of the crate's: an escaped `Vec` it may reach
        // (#1966).
        if let StatementKind::Assign(assign) = &statement.kind
            && let Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of) | Rvalue::RawPtr(_, of) = &assign.1
            && self.written_through_a_pointer(of)
        {
            state.forget_escaped();
        }
        let StatementKind::Assign(assign) = &statement.kind else {
            return;
        };
        let (place, rvalue) = &**assign;
        // A write through a reference, `*v = Vec::new()`: what's known of
        // the length it reaches forgotten (#1877). Not an item's, `v[i] = x`
        // of a `&mut [T]`, which keeps the length; a table's rows are
        // reached by `Vec`'s `IndexMut` alone, never a place's index.
        if let Some(deref) = place.projection.iter().position(|elem| elem == mir::PlaceElem::Deref)
            && !place.projection[deref..].iter().any(|elem| {
                matches!(
                    elem,
                    mir::PlaceElem::Index(_) | mir::PlaceElem::ConstantIndex { .. } | mir::PlaceElem::Subslice { .. }
                )
            })
        {
            state.forget_lengths(state.root(place.local));
        }
        let Some(local) = place.as_local() else {
            return;
        };
        // An integer local lent `&mut` where the walk forgets it, not lent in
        // the whole body (`lent_where_borrowed`): what it held may be
        // written through the borrow, until it's assigned again. Not one
        // lent only to calls, a closure's capture given only to them, which
        // each such call forgets (`called_on_mut`, #1947).
        if let Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of) = rvalue
            && let Some(of) = of.as_local()
            && lent_where_borrowed(&self.body, &mir::Place::from(of))
            && !self.called_on.values().flatten().any(|&lent| lent == of)
        {
            state.forget(of);
            state.lent_away.insert(of);
        }
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
        // What its arithmetic tells of it: each relation, and the sum it is
        // (`derived`).
        let derived::Derived {
            mut below_as,
            sum,
            half_of,
            twice_below,
        } = self.derived(rvalue, state, range);
        let again = sum.and_then(|(of, c, _)| {
            (state.sums.iter())
                .find(|&(&t, &(o, k, _))| (o, k) == (of, c) && t != local)
                .map(|(&t, _)| t)
        });
        let interval = match rvalue {
            // A value a range's `next()` gave, `i` of `for i in a..b`: at
            // least `a`, below `b`.
            Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _) | Rvalue::CopyForDeref(of)
                if let [mir::PlaceElem::Downcast(_, variant), mir::PlaceElem::Field(field, _), rest @ ..] =
                    of.projection.as_slice()
                    && variant.as_u32() == 1
                    && field.as_u32() == 0
                    && let Some(&Ranged {
                        held: Held::Next,
                        least,
                        end,
                    }) = state.ranges.get(&of.local)
                    // A range's value, or an enumerated slice's index, the
                    // pair's first.
                    && match (end, rest) {
                        (End::Below(_) | End::AtMost(_), []) => true,
                        (End::Indexes(_), [mir::PlaceElem::Field(index, _)]) => index.as_u32() == 0,
                        _ => false,
                    } =>
            {
                let most = |v: Value| match v {
                    Value::Constant(n) => Some(n),
                    Value::Range(_, hi) => Some(hi),
                    Value::Local(l) => state.intervals[l]
                        .or_else(|| self.range(self.body.local_decls[l].ty))
                        .map(|i| i.hi),
                };
                // An index of an enumerated slice: below its length. A
                // range's value: below its end, or at most it of `a..=b`.
                let (end_most, by, strict) = match end {
                    End::Indexes(slice) => {
                        below_as.push((Term::Len(slice), true));
                        (
                            self.length_of_collection(self.body.local_decls[slice].ty).map(|l| l.hi),
                            None,
                            true,
                        )
                    }
                    End::Below(v) => (most(v), Some(v), true),
                    End::AtMost(v) => (most(v), Some(v), false),
                    End::Items(_) => (None, None, true),
                };
                // And below what the end is below, `v[i]` of `(0..=n).rev()`
                // under `n < v.len()`.
                if let Some(Value::Local(l)) = by {
                    below_as.push((state.length_of(l).map_or(Term::Int(l), Term::Len), strict));
                    below_as.extend(
                        (state.below.iter())
                            .filter(|&&(x, _, _)| x == state.root(l))
                            .map(|&(_, y, s)| (y, s || strict)),
                    );
                    // Below `y + 1`: at most `y`, `k` of `for k in 1..line + 1`.
                    if strict && let Some(&(of, 1, _)) = state.sums.get(&state.root(l)) {
                        below_as.push((Term::Int(of), false));
                    }
                }
                end_most
                    .map(|most| Interval {
                        lo: least,
                        hi: most - i128::from(strict),
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
            // A reborrow, `&_1` or `&(*_1)`, or a raw pointer of one, `&raw const
            // (fake) (*_1)` of a bounds check: of the same slice, its length.
            Rvalue::Ref(_, _, of) | Rvalue::RawPtr(_, of)
                if matches!(of.projection.as_slice(), [] | [mir::PlaceElem::Deref]) =>
            {
                copy_of = Some(of.local)
                    .filter(|of| !self.lent.contains(of))
                    .map(|of| state.root(of));
                None
            }
            Rvalue::BinaryOp(
                op @ (BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem | BinOp::BitAnd | BinOp::Shr),
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
            // A held test negated, `!short` of `short = len <= LIMIT`.
            Rvalue::UnaryOp(mir::UnOp::Not, Operand::Copy(of) | Operand::Move(of)) => {
                comparison = of
                    .as_local()
                    .and_then(|of| state.comparison_of(of))
                    .and_then(Comparison::negated);
                None
            }
            // A cast that keeps its value, what it casts fitting its type:
            // a copy of it, its relations its own, `s as usize` of `s >= 1`.
            // A `bool`'s cast, `apple as usize`, is 0 or 1 (#1863).
            Rvalue::Cast(CastKind::IntToInt, operand, _) if operand.ty(&*self.body, self.tcx).is_bool() => {
                Some(Interval { lo: 0, hi: 1 }).filter(fits)
            }
            Rvalue::Cast(CastKind::IntToInt, operand, _) => {
                let kept = self.operand(operand, state).filter(fits);
                if kept.is_some()
                    && let Operand::Copy(of) | Operand::Move(of) = operand
                {
                    copy_of = unlent(of).map(|of| state.root(of));
                }
                kept
            }
            Rvalue::UnaryOp(mir::UnOp::PtrMetadata, operand) => {
                if let Operand::Copy(of) | Operand::Move(of) = operand {
                    length_of = unlent(of).map(|of| state.root(of));
                }
                self.length(operand.ty(&*self.body, self.tcx))
            }
            _ => None,
        };
        // A sum known again: that one's value, as narrowed, `i += 1` past
        // `i + 1 < n`.
        let interval = again.and_then(|t| state.intervals[t]).or(interval);
        state.forget(local);
        state
            .intervals
            .set(local, interval.or(range).filter(|_| range.is_some()));
        if let Some(c) = comparison {
            state.comparisons.insert(local, c);
        }
        if let Some(of) = copy_of.or(again).filter(|&of| of != local) {
            state.copies.insert(local, of);
            // Its relations its own too, so a join where what it copies
            // differs keeps them: `index = parent` at a loop's back edge.
            let relations: Vec<(Term, bool)> = (state.below.iter())
                .filter(|&&(x, _, _)| x == of)
                .map(|&(_, y, strict)| (y, strict))
                .collect();
            for (y, strict) in relations {
                if y != Term::Int(local) && y != Term::Len(local) {
                    state.below.insert((local, y, strict));
                }
            }
        }
        // The discriminant of a slice's `get(index)`: its `Some` the index's.
        if let Rvalue::Discriminant(of) = rvalue
            && let Some(&got) = of.as_local().and_then(|of| state.gets.get(&of))
        {
            state.gets.insert(local, got);
        }
        // A slice iterator lent `&mut`, or moved: the slice it steps.
        if let Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, of) | Rvalue::Use(mir::Operand::Move(of), ..) = rvalue
            && of.projection.is_empty()
            && let Some(&slice) = state.iterates.get(&of.local)
        {
            state.iterates.insert(local, slice);
        }
        // A slice iterator's `position()`'s `Some` payload, `(o as Some).0`,
        // read only where it's `Some`, or its `?`'s `Continue`'s: below the
        // length of the slice the iterator stepped, nothing writing it since
        // (#1945).
        if let Rvalue::Use(mir::Operand::Copy(of) | mir::Operand::Move(of), ..) = rvalue
            && let [mir::PlaceElem::Downcast(_, variant), mir::PlaceElem::Field(field, _)] = of.projection.as_slice()
            && field.as_u32() == 0
            && let Some(&(slice, holds)) = state.positions.get(&of.local)
            && variant.as_u32() == holds
        {
            state.below.insert((local, Term::Len(slice), true));
        }
        if let Some(key) = sum.filter(|&(of, _, _)| of != local) {
            state.sums.insert(local, key);
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
        if let Some(of) = half_of.filter(|&of| of != Term::Len(local) && of != Term::Int(local)) {
            state.halves.insert(local, of);
        }
        if let Some(of) = twice_below.filter(|&of| of != Term::Len(local) && of != Term::Int(local)) {
            state.doubled.insert(local, of);
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
                    end: End::Below(self.value(end, state)?),
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
    fn edges_rules(&self, block: BasicBlock, state: &State) -> Vec<(BasicBlock, State)> {
        let terminator = self.body.basic_blocks[block].terminator();
        match &terminator.kind {
            TerminatorKind::SwitchInt { discr, targets } => {
                let switched = discr.place().and_then(|p| p.as_local());
                let tested = switched.and_then(|l| state.comparison_of(l));
                // A slice's `is_empty()` switched on: of its `false`, a length
                // of one at least.
                let emptied = switched.and_then(|l| state.empties.get(&l).copied());
                // An integer switched on: each value on its way, and of the
                // rest, its least and most passed over each listed.
                let integer = switched.filter(|&l| self.range(self.body.local_decls[l].ty).is_some());
                // A slice's `get(index)`'s discriminant: of `Some`, the index
                // below the slice's length.
                let gotten = switched.and_then(|l| state.gets.get(&l).copied());
                // A map's `contains_key(&k)` switched on: of its `true`, the key held.
                let keyed = switched.and_then(|l| state.has_keys.get(&l).copied());
                let values: Vec<u128> = targets.iter().map(|(value, _)| value).collect();
                let mut out = Vec::new();
                for (value, target) in targets.iter() {
                    let mut narrowed = state.clone();
                    // A way no value takes, `i > 0` of `i = 0`: not taken.
                    if let Some(c) = tested
                        && !self.narrow(c, value != 0, &mut narrowed)
                    {
                        continue;
                    }
                    if let Some(held) = keyed.filter(|_| value != 0) {
                        narrowed.keys.insert(held);
                    }
                    if let Some(slice) = emptied
                        && value == 0
                    {
                        let least = narrowed.least.or_insert(slice, 0);
                        *least = (*least).max(1);
                    }
                    if let Some((index, most)) = gotten
                        && value == 1
                        && let Some(known) =
                            (narrowed.intervals[index]).or_else(|| self.range(self.body.local_decls[index].ty))
                        && known.lo <= most
                    {
                        narrowed.narrowed_to(
                            index,
                            Interval {
                                lo: known.lo,
                                hi: known.hi.min(most),
                            },
                        );
                    }
                    if let Some(l) = integer
                        && let Ok(v) = i128::try_from(value)
                    {
                        narrowed.intervals.set(l, Some(Interval { lo: v, hi: v }));
                        // Of a slice's length, `match list.len() { 1 => .. }`.
                        if let Some(slice) = narrowed.length_of(l) {
                            let least = narrowed.least.or_insert(slice, 0);
                            *least = (*least).max(v);
                        }
                    }
                    out.push((target, narrowed));
                }
                let mut otherwise = state.clone();
                if let Some(c) = tested
                    && values == [0]
                    && !self.narrow(c, true, &mut otherwise)
                {
                    return out;
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
                    otherwise.intervals.set(l, Some(known));
                    if let Some(slice) = otherwise.length_of(l) {
                        let least = otherwise.least.or_insert(slice, 0);
                        *least = (*least).max(known.lo);
                    }
                }
                if let Some(held) = keyed.filter(|_| values == [0]) {
                    otherwise.keys.insert(held);
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
                    && !self.narrow(c, *expected, &mut on)
                {
                    return Vec::new();
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
                after.unpush(destination.local);
                // A call that may write a cell: what's known of each cell's
                // length forgotten (`cells`, #1741), but of one a held guard
                // guards, and at none of a held guard's own (`guards`).
                if !self.cells.is_empty() && self.may_write_cells(func) && !self.guard_call(func, args, destination) {
                    (self.cells.iter())
                        .filter(|&&cell| !state.is_guarded(cell))
                        .for_each(|&cell| after.forget_lengths(cell));
                }
                if self.may_write_escaped(func, args) {
                    after.forget_escaped();
                }
                self.called_on_mut(func, args, state, &mut after);
                self.guard_lent(func, args, state, &mut after);
                if let Some(local) = destination.as_local() {
                    after.forget(local);
                    self.called(func, args, local, state, &mut after);
                    self.derefed(func, args, local, state, &mut after);
                    self.guarded(args, local, func, state, &mut after);
                    self.asked_key(func, args, local, state, &mut after);
                    self.tabled(func, args, local, state, &mut after);
                }
                target.map(|t| vec![(t, after)]).unwrap_or_default()
            }
            // A match candidate's and a loop's marks, rustc's: their real way
            // alone, the imaginary one no run takes.
            TerminatorKind::FalseEdge { real_target, .. } | TerminatorKind::FalseUnwind { real_target, .. } => {
                vec![(*real_target, state.clone())]
            }
            // A held guard's drop, which runs none of the crate's code: its
            // cell no longer guarded (`guards`).
            TerminatorKind::Drop { place, .. } if place.as_local().is_some_and(|g| self.guards.held.contains(&g)) => {
                let mut after = state.clone();
                after.guarded.shift_remove(&place.local);
                terminator.successors().map(|t| (t, after.clone())).collect()
            }
            // A drop that may run code: of each cell but one a held guard
            // guards, and of each escaped `Vec`, what's known of its length
            // forgotten.
            TerminatorKind::Drop { place, .. }
                if (!self.cells.is_empty() || !state.escaped.is_empty()) && self.drop_may_write_cells(place) =>
            {
                let mut after = state.clone();
                after.forget_escaped();
                (self.cells.iter())
                    .filter(|&&cell| !state.is_guarded(cell))
                    .for_each(|&cell| after.forget_lengths(cell));
                terminator.successors().map(|t| (t, after.clone())).collect()
            }
            _ => terminator.successors().map(|t| (t, state.clone())).collect(),
        }
    }

    /// `c` known to be `holds`: each of its local operands narrowed.
    /// Whether the way it narrows can be taken: none where what it narrows
    /// is left with no value, `i > 0` of `i = 0`.
    fn narrow(&self, c: Comparison, holds: bool, state: &mut State) -> bool {
        let op = match (c.op, holds) {
            (op, true) => op,
            (BinOp::Lt, false) => BinOp::Ge,
            (BinOp::Le, false) => BinOp::Gt,
            (BinOp::Gt, false) => BinOp::Le,
            (BinOp::Ge, false) => BinOp::Lt,
            (BinOp::Eq, false) => BinOp::Ne,
            (BinOp::Ne, false) => BinOp::Eq,
            _ => return true,
        };
        let get = |v: Value, state: &State| match v {
            Value::Constant(n) => Some(Interval { lo: n, hi: n }),
            Value::Range(lo, hi) => Some(Interval { lo, hi }),
            Value::Local(l) => state.intervals[l].or_else(|| self.range(self.body.local_decls[l].ty)),
        };
        // `x < y` or `x <= y` known, `y` a slice's length where a local
        // holds one, else an integer local.
        let term = |v: Value, state: &State| match v {
            Value::Local(l) => Some(state.length_of(l).map_or(Term::Int(l), Term::Len)),
            Value::Constant(_) | Value::Range(..) => None,
        };
        let related = match op {
            BinOp::Lt | BinOp::Le => Some((c.a, c.b, op == BinOp::Lt)),
            BinOp::Gt | BinOp::Ge => Some((c.b, c.a, op == BinOp::Gt)),
            _ => None,
        };
        if let Some((Value::Local(x), y, strict)) = related
            && let Some(y) = term(y, state)
        {
            state.related(x, y, strict);
            // `x + c < y` of a sum that never wraps: `x < y`.
            if let Some(&(of, _, true)) = state.sums.get(&x) {
                state.related(of, y, true);
            }
            // `x < y + 1`: `x <= y`, `k` of `for k in 1..line + 1`, though `y + 1`
            // wraps, its least then, which nothing is below.
            if strict
                && let Term::Int(t) = y
                && let Some(&(of, 1, _)) = state.sums.get(&t)
            {
                state.related(x, Term::Int(of), false);
            }
        }
        // `x == y`: each at most the other, `prev_length ==
        // properties.len()` (ADR 1168, #1948).
        if op == BinOp::Eq
            && let (Value::Local(a), Value::Local(b)) = (c.a, c.b)
        {
            for (x, y) in [(a, b), (b, a)] {
                if let Some(y) = term(Value::Local(y), state) {
                    state.related(x, y, false);
                }
            }
        }
        // `x <= y` known and `x != y`: `x < y`, `s != len` of `s <= len`.
        if op == BinOp::Ne
            && let (Value::Local(a), Value::Local(b)) = (c.a, c.b)
        {
            for (x, y) in [(a, b), (b, a)] {
                if let Some(y) = term(Value::Local(y), state)
                    && state.below.contains(&(x, y, false))
                {
                    state.related(x, y, true);
                }
            }
        }
        // A constant below a slice's length, `b.len() > 4`: its least.
        if let Some((Value::Constant(c), Value::Local(holder), strict)) = related
            && let Some(slice) = state.length_of(holder)
        {
            let least = state.least.or_insert(slice, 0);
            *least = (*least).max(c + i128::from(strict));
        }
        let (Some(a), Some(b)) = (get(c.a, state), get(c.b, state)) else {
            return true;
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
            _ => return true,
        };
        if a2.lo > a2.hi || b2.lo > b2.hi {
            return false;
        }
        for (v, narrowed) in [(c.a, a2), (c.b, b2)] {
            if let Value::Local(l) = v {
                state.narrowed_to(l, narrowed);
                // A length narrowed, `length == 1` of `let length =
                // keys.len()`: its slice at least so long.
                if let Some(slice) = state.length_of(l) {
                    let least = state.least.or_insert(slice, 0);
                    *least = (*least).max(narrowed.lo);
                }
            }
        }
        // Of two locals compared, the relation their intervals show there,
        // `n < limit` of `n = 1` and `limit = 4`: a loop's invariant a first
        // turn seeds, which the join at its head keeps.
        if let (Value::Local(a), Value::Local(b)) = (c.a, c.b) {
            for (x, y) in [(a, b), (b, a)] {
                if let Some(strict) = state.below_by_intervals(x, Term::Int(y)) {
                    state.related(x, Term::Int(y), strict);
                }
            }
        }
        true
    }
}
