//! What the walk proves of a body at its fixed point (ADR 1137): each
//! `+`, `-` and `*` that stays in its type's range, each integer cast of
//! a value in its target's, and each bounds check that never fails, each at
//! the state where it runs.

use super::*;

impl<'tcx> Walk<'_, 'tcx> {
    /// The checks and operators proved, each at the state where it runs.
    pub(super) fn proved(&self, entry: &IndexVec<BasicBlock, Option<State>>) -> Proofs {
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
                // Of two a JS number holds, 32 bits at most: a 64-bit one is a
                // `BigInt`, its value the same but not the JS value.
                if let StatementKind::Assign(assign) = &statement.kind
                    && let Rvalue::Cast(CastKind::IntToInt, operand, to) = &assign.1
                    && held_by_a_number(operand.ty(&*self.body, self.tcx))
                    && held_by_a_number(*to)
                    && let Some(range) = self.range(*to)
                    && let Some(value) = self.operand(operand, &state)
                    && range.lo <= value.lo
                    && value.hi <= range.hi
                {
                    proofs.casts.insert(Location {
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
            if let TerminatorKind::Call { func, args, .. } = &data.terminator().kind
                && let Some((callee, callee_args)) = func.const_fn_def()
                && let Some(indexes) = call_indexes(self.tcx, callee, callee_args)
                && let Some(holder) = (args.first())
                    .and_then(|arg| arg.node.place())
                    .and_then(|p| p.as_local())
                    .filter(|l| !self.lent.contains(l))
                    .map(|l| state.root(l))
                && (indexes.iter()).all(|&(at, strict)| {
                    args.get(at)
                        .is_some_and(|i| self.index_below(&state, holder, &i.node, strict))
                })
            {
                proofs.call_bounds.insert(block);
            }
            if let TerminatorKind::Call { func, args, .. } = &data.terminator().kind
                && self.key_held(func, args, &state)
            {
                proofs.call_bounds.insert(block);
            }
            if let TerminatorKind::Call { func, args, .. } = &data.terminator().kind
                && let Some((callee, callee_args)) = func.const_fn_def()
                && let Some(at) = range_index(self.tcx, callee, callee_args)
                && let Some(holder) = (args.first())
                    .and_then(|arg| arg.node.place())
                    .and_then(|p| p.as_local())
                    .filter(|l| !self.lent.contains(l))
                    .map(|l| state.root(l))
                && args
                    .get(at)
                    .is_some_and(|range| self.range_within(&state, holder, &range.node))
            {
                proofs.call_bounds.insert(block);
            }
        }
        proofs
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
        self.index_below(state, slice, index, true)
    }

    /// Whether `index` is below `slice`'s length, or, not `strict`, at
    /// most it: below the least it is, or below it as a relation knows; or
    /// its length itself, `insert(v.len(), x)`.
    /// Whether `range`, `a..b` or one of its kinds, is within `slice`'s
    /// length: its start at most its end, its end at most the length, or
    /// below it of `..=b`; `..` always.
    fn range_within(&self, state: &State, slice: Local, range: &Operand<'tcx>) -> bool {
        let ty::Adt(adt, _) = *range.ty(&*self.body, self.tcx).kind() else {
            return false;
        };
        let Some((start, end)) = range_index_bounds(self.tcx, adt.did()) else {
            return false;
        };
        if start.is_none() && end.is_none() {
            return true;
        }
        let Some((_, Rvalue::Aggregate(_, fields))) =
            range.place().and_then(|p| p.as_local()).and_then(|r| self.made(r))
        else {
            return false;
        };
        let field = |at: usize| fields.iter().nth(at);
        let end_within = match end {
            Some((at, inclusive)) => field(at).is_some_and(|end| self.index_below(state, slice, end, inclusive)),
            None => true,
        };
        let start_within = match (start, end) {
            (Some(at), None) => field(at).is_some_and(|start| self.index_below(state, slice, start, false)),
            (Some(at), Some((to, _))) => field(at)
                .zip(field(to))
                .is_some_and(|(start, end)| self.at_most(state, start, end)),
            (None, _) => true,
        };
        end_within && start_within
    }

    /// Whether `a` is at most `b`: by their intervals, or a relation known.
    fn at_most(&self, state: &State, a: &Operand<'tcx>, b: &Operand<'tcx>) -> bool {
        let by_intervals = (self.operand(a, state))
            .zip(self.operand(b, state))
            .is_some_and(|(a, b)| a.hi <= b.lo);
        let local = |o: &Operand<'tcx>| o.place().and_then(|p| p.as_local()).map(|l| state.root(l));
        let related = local(a).zip(local(b)).is_some_and(|(a, b)| {
            a == b || state.below.contains(&(a, Term::Int(b), true)) || state.below.contains(&(a, Term::Int(b), false))
        });
        by_intervals || related
    }

    fn index_below(&self, state: &State, slice: Local, index: &Operand<'tcx>, strict: bool) -> bool {
        let slices = state.as_long_as(slice);
        // At least what's known of it, or an array's own, `[i32; 11]`'s 11.
        let typed = (self.length(self.body.local_decls[slice].ty)).map_or(0, |n| n.lo);
        // Of each as long, its least, and the least of what holds its
        // length, `height` of `vec![row; height]`.
        let held = (state.lens.iter())
            .filter(|(_, s)| slices.contains(s))
            .filter_map(|(&w, _)| state.intervals[w].map(|n| n.lo));
        let known = (slices.iter())
            .filter_map(|s| state.least.get(s).copied())
            .chain(held)
            .max()
            .unwrap_or(0);
        let least = known.max(typed);
        let in_least = (self.operand(index, state))
            .is_some_and(|i| 0 <= i.lo && if strict { i.hi < least } else { i.hi <= least });
        let local = index.place().and_then(|p| p.as_local()).map(|i| state.root(i));
        // Below an integer that holds its length, `i < count` of
        // `vec![0; count]` (`tables`), as below its length.
        let below = |i: Local, s: Local, strict: bool| {
            state.below.contains(&(i, Term::Len(s), strict))
                || (state.below.iter()).any(|&(x, y, st)| {
                    x == i && st == strict && matches!(y, Term::Int(w) if state.length_of(w) == Some(s))
                })
        };
        let related = local.is_some_and(|i| {
            slices.iter().any(|&s| {
                below(i, s, true)
                    || (state.below_past.iter()).any(|&(x, sl, c)| (x, sl) == (i, s) && state.least_of(s) > c)
                    || !strict && (below(i, s, false) || state.length_of(i) == Some(s))
            })
        });
        in_least || related
    }
}
