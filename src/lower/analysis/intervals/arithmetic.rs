//! What an integer operation gives, by the MIR intervals (ADR 1137): `a op b`
//! of intervals alone, where it stays in the integers it's of, raised by
//! what's known to be below what, and whether a product can be JS's `-0`.

use super::*;

impl<'tcx> Walk<'_, 'tcx> {
    /// Whether `a * b` can be JS's `-0`, which an integer never is (ADR
    /// 0064): one of them 0, the other below it, `-4 * 0`. JS's own `*` is
    /// then no integer's product, though it stays in range.
    pub(super) fn may_make_negative_zero(&self, a: &Operand<'tcx>, b: &Operand<'tcx>, state: &State) -> bool {
        let (Some(a), Some(b)) = (self.operand(a, state), self.operand(b, state)) else {
            return true;
        };
        let zero = |x: Interval| x.lo <= 0 && 0 <= x.hi;
        zero(a) && b.lo < 0 || zero(b) && a.lo < 0
    }

    /// What `a op b` is, where it stays in the integers it's of; none where
    /// it may wrap, which is then its type's range.
    pub(super) fn arithmetic(
        &self,
        op: BinOp,
        a: &Operand<'tcx>,
        b: &Operand<'tcx>,
        state: &State,
    ) -> Option<Interval> {
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
    pub(super) fn bare(op: BinOp, a: Interval, b: Interval) -> Option<Interval> {
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
            // A shift right of what's never negative by a known amount.
            BinOp::Shr if a.lo >= 0 && b.lo == b.hi && (0..64).contains(&b.lo) => Interval {
                lo: a.lo >> b.lo,
                hi: a.hi >> b.lo,
            },
            _ => return None,
        })
    }
}
