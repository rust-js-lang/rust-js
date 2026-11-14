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
    fn index_below(&self, state: &State, slice: Local, index: &Operand<'tcx>, strict: bool) -> bool {
        let least = state.least.get(&slice).copied().unwrap_or(0);
        let in_least = (self.operand(index, state))
            .is_some_and(|i| 0 <= i.lo && if strict { i.hi < least } else { i.hi <= least });
        let local = index.place().and_then(|p| p.as_local()).map(|i| state.root(i));
        let related = local.is_some_and(|i| {
            state.below.contains(&(i, Term::Len(slice), true))
                || !strict && (state.below.contains(&(i, Term::Len(slice), false)) || state.length_of(i) == Some(slice))
        });
        in_least || related
    }
}
