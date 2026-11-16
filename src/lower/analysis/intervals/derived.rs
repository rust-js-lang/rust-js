//! The relations an assignment's arithmetic gives what it assigns (ADR
//! 1137's M2b-1, ADR 1168): `t = i - c` below what `i` is, `t = x / c` or
//! `x >> c` at most `x`, `t = i + c` past `i + (c - 1)` strictly below, and
//! the sum `t = x + c` is, which `x + c` again reads.

use super::*;

/// What `t = rvalue`'s arithmetic tells of `t` (`Walk::derived`): each
/// relation `t` has, and the sum it is.
pub(super) struct Derived {
    pub(super) below_as: Vec<(Term, bool)>,
    pub(super) sum: Option<(Local, i128, bool)>,
}

impl<'tcx> Walk<'_, 'tcx> {
    /// What `t = rvalue` tells of `t`, of what's known before it, `state`;
    /// `range`, `t`'s type's integers, says what's computed exactly.
    pub(super) fn derived(&self, rvalue: &Rvalue<'tcx>, state: &State, range: Option<Interval>) -> Derived {
        let fits = |r: &Interval| range.is_some_and(|t| t.lo <= r.lo && r.hi <= t.hi);
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
        {
            let slice = state.length_of(i);
            // Computed exactly: of `usize`, `i` at least `c`; of `isize`,
            // `-1` as it is, below any length.
            let exact = self
                .arithmetic(BinOp::Sub, &operands.0, &operands.1, state)
                .is_some_and(|d| fits(&d));
            if exact {
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
        // `t = x / c` or `t = x >> c` of what's never negative: at most `x`,
        // below what it's below, `heap[(index - 1) >> 1]`.
        if let Rvalue::BinaryOp(BinOp::Div | BinOp::Shr, operands) = rvalue
            && let Some(x) = operands.0.place().and_then(|p| p.as_local())
            && self.operand(&operands.0, state).is_some_and(|x| x.lo >= 0)
            && let Some(c) = self.operand(&operands.1, state)
            && c.lo == c.hi
            && (c.lo >= 1 || matches!(rvalue, Rvalue::BinaryOp(BinOp::Shr, _)) && c.lo >= 0)
        {
            let x = state.root(x);
            below_as.extend(
                (state.below.iter())
                    .filter(|&&(of, _, _)| of == x)
                    .map(|&(_, y, strict)| (y, strict)),
            );
        }
        // `t = x + c` of a constant at least one: a sum known again, `x + c`
        // computed before, is that one, the same value though it wraps.
        let mut sum: Option<(Local, i128, bool)> = None;
        if let Rvalue::BinaryOp(BinOp::Add, operands) = rvalue
            && let Some(x) = operands.0.place().and_then(|p| p.as_local())
            && !self.lent.contains(&x)
            && let Some(c) = self.operand(&operands.1, state).filter(|c| c.lo == c.hi && c.lo >= 1)
        {
            let exact = (self.arithmetic(BinOp::Add, &operands.0, &operands.1, state)).is_some_and(|i| fits(&i));
            sum = Some((state.root(x), c.lo, exact));
        }
        // `t = i + c` of `i + (c - 1)` strictly below, `i` itself where `c` is
        // one: at most what that's below, `codes[i - 1]` after `codes[i]`'s
        // check and `i += 1`; `i += 2` past `i + 1 < len`.
        if let Rvalue::BinaryOp(BinOp::Add, operands) = rvalue
            && let Some(i) = operands.0.place().and_then(|p| p.as_local())
            && let Some(c) = self.operand(&operands.1, state).filter(|c| c.lo == c.hi && c.lo >= 1)
        {
            let i = state.root(i);
            let before: Vec<Local> = match c.lo {
                1 => vec![i],
                c => (state.sums.iter())
                    .filter(|&(_, &(of, k, exact))| of == i && k == c - 1 && exact)
                    .map(|(&t, _)| t)
                    .collect(),
            };
            below_as.extend(
                (state.below.iter())
                    .filter(|&&(x, _, strict)| before.contains(&x) && strict)
                    .map(|&(_, y, _)| (y, false)),
            );
        }
        Derived { below_as, sum }
    }
}
