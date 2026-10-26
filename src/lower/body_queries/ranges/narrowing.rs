//! What a condition shows where it holds (ADR 0576): each variable it
//! compares narrowed, `i < n` putting `i` below `n`'s value or a slice's
//! length, and a slice's length at least what it's compared with.

use super::known::{Below, Known, Range, by_value};
use super::reads::pure;
use super::*;

impl<'a, 'tcx> Walk<'a, 'tcx> {
    /// What `cond` being `holds` shows, narrowing what's known: of a
    /// variable compared, `i < half`, and of a slice's length,
    /// `!xs.is_empty()`. Where it can't hold, nothing goes on.
    pub(super) fn refine(&mut self, cond: ExprId, holds: bool, known: &mut Option<Known>) {
        let thir = self.thir;
        let e = strip(thir, cond);
        match thir[e].kind {
            ExprKind::Unary { op: UnOp::Not, arg } => self.refine(arg, !holds, known),
            // Where an item's there, its index is below a length (`present`).
            ExprKind::Let { expr, ref pat } if holds && present::some_pattern(self.tcx, pat) => {
                if let Some(var) = present::present_index(self.tcx, thir, expr, self.bodies) {
                    present::below_most(var, self.volatile.contains(&var), known);
                }
            }
            // A variable that holds a test, the test's (ADR 0806).
            ExprKind::VarRef { id } | ExprKind::UpvarRef { var_hir_id: id, .. }
                if let Some((test, _)) = known.as_ref().and_then(|k| k.tests.get(&id)) =>
            {
                self.refine(*test, holds, known)
            }
            ExprKind::LogicalOp { op, lhs, rhs } if matches!(op, LogicalOp::And) == holds => {
                self.refine(lhs, holds, known);
                self.refine(rhs, holds, known);
            }
            _ if let Some((SliceLength::IsEmpty, items)) = self.asks(e) => {
                if !holds
                    && let Some(slice) = self.slice(items)
                    && let Some(k) = known.as_mut()
                {
                    let least = k.lengths.entry(slice).or_insert(0);
                    *least = (*least).max(1);
                }
            }
            ExprKind::Binary { op, lhs, rhs } if pure(self.tcx, thir, lhs) && pure(self.tcx, thir, rhs) => {
                let op = match (op, holds) {
                    (op, true) => op,
                    (BinOp::Lt, false) => BinOp::Ge,
                    (BinOp::Le, false) => BinOp::Gt,
                    (BinOp::Gt, false) => BinOp::Le,
                    (BinOp::Ge, false) => BinOp::Lt,
                    (BinOp::Eq, false) => BinOp::Ne,
                    (BinOp::Ne, false) => BinOp::Eq,
                    _ => return,
                };
                let flipped = match op {
                    BinOp::Lt => BinOp::Gt,
                    BinOp::Le => BinOp::Ge,
                    BinOp::Gt => BinOp::Lt,
                    BinOp::Ge => BinOp::Le,
                    op => op,
                };
                // Each side's value, which the condition's own walk judged.
                let (Some(l), Some(r)) = (self.value_of(lhs, known), self.value_of(rhs, known)) else {
                    return;
                };
                let (l, r) = (self.bounded_by_itself(l, lhs), self.bounded_by_itself(r, rhs));
                self.narrow(lhs, op, &r, known);
                self.narrow(rhs, flipped, &l, known);
                if op == BinOp::Ne {
                    self.below_length(lhs, rhs, known);
                    self.below_length(rhs, lhs, known);
                }
                // `xs.len() > k`: it's at least so long.
                for (side, op, other) in [(lhs, op, &r), (rhs, flipped, &l)] {
                    if let Some((SliceLength::Len, items)) = self.asks(side)
                        && let Some(slice) = self.slice(items)
                        && let Some(k) = known.as_mut()
                    {
                        let least = match op {
                            BinOp::Gt => other.lo + 1,
                            BinOp::Ge | BinOp::Eq => other.lo,
                            BinOp::Ne if other.constant() == Some(0) => 1,
                            _ => continue,
                        };
                        k.at_least(&(slice, Below::LENGTH), least);
                    }
                }
            }
            _ => {}
        }
    }

    /// The integer `e` is through each `as` that keeps it, `s` of `s as
    /// usize` where `s >= 0`, and `xs.len()` of `xs.len() as isize` of
    /// sized items (ADR 0701): what it is, the integer it's of is.
    pub(super) fn uncast(&self, mut e: ExprId, known: &Option<Known>) -> ExprId {
        let thir = self.thir;
        loop {
            e = strip(thir, e);
            let ExprKind::Cast { source } = thir[e].kind else {
                return e;
            };
            let (Some((lo, hi)), Some(of)) = (type_range(thir[e].ty), self.cast_of(source, known)) else {
                return e;
            };
            if !(lo <= of.0 && of.1 <= hi) {
                return e;
            }
            e = source;
        }
    }

    /// What `source`, an `as`'s, is without walking it: a variable's range
    /// as known, or of its type; a slice's `len()`, from 0 to its most
    /// (ADR 0709).
    fn cast_of(&self, source: ExprId, known: &Option<Known>) -> Option<(i128, i128)> {
        let thir = self.thir;
        if let Some(var) = var_of(thir, source) {
            return (known.as_ref())
                .and_then(|k| k.vars.get(&var).map(|r| (r.lo, r.hi)))
                .or_else(|| type_range(thir[source].ty));
        }
        match thir[strip(thir, source)].kind {
            ExprKind::Call { fun, .. } if matches!(self.asks(source), Some((SliceLength::Len, _))) => {
                Some((0, self.length_most_of(fun)))
            }
            _ => None,
        }
    }

    /// `side != xs.len()` of a `side` at most the length: below it, and
    /// so through each `as` that keeps either, `s != xs.len() as isize`.
    fn below_length(&self, side: ExprId, length: ExprId, known: &mut Option<Known>) {
        let Some((SliceLength::Len, items)) = self.asks(self.uncast(length, known)) else {
            return;
        };
        let (Some(slice), Some(var)) = (self.slice(items), var_of(self.thir, self.uncast(side, known))) else {
            return;
        };
        if let Some(range) = known.as_mut().and_then(|k| k.vars.get_mut(&var))
            && range.below == Some((slice.clone(), Below::LENGTH))
        {
            range.below = Some((slice, Below::LENGTH.offset(-1)));
        }
    }

    /// A variable `side` that's `op` `other`, narrowed; and of `i + k`
    /// that can't have wrapped, `i` that's `op` `other - k` (ADR 0632).
    fn narrow(&self, side: ExprId, op: BinOp, other: &Range, known: &mut Option<Known>) {
        let thir = self.thir;
        if let ExprKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } = thir[strip(thir, side)].kind
        {
            for (term, k) in [(lhs, rhs), (rhs, lhs)] {
                let (Some(k), Some((_, most)), Some(var)) =
                    (literal(thir, k), type_range(thir[term].ty), var_of(thir, term))
                else {
                    continue;
                };
                let hi = (known.as_ref())
                    .and_then(|known| known.vars.get(&var))
                    .map_or(most, |r| r.hi);
                if k >= 0 && hi + k <= most {
                    let less = Range {
                        lo: other.lo - k,
                        hi: other.hi - k,
                        below: (other.below.as_ref()).map(|(s, b)| (s.clone(), b.offset(-k))),
                    };
                    self.narrow(term, op, &less, known);
                }
            }
            return;
        }
        let side = self.uncast(side, known);
        let Some(var) = var_of(thir, side) else { return };
        let Some((lo, hi)) = type_range(thir[side].ty) else {
            return;
        };
        if self.volatile.contains(&var) {
            return;
        }
        let Some(k) = known.as_mut() else { return };
        let current = k.vars.get(&var).cloned().unwrap_or(Range::of(lo, hi));
        // A slice's length first, which an index is judged by; never its
        // own value, `i < i + 1`.
        let below = |by: i128| {
            let theirs = (other.below.as_ref())
                .filter(|((_, v), _)| *v != var)
                .map(|(s, x)| (s.clone(), x.offset(by)));
            match (theirs, &current.below) {
                (Some(theirs), _) if !by_value(&theirs.0) => Some(theirs),
                (_, Some(own)) if !by_value(&own.0) => Some(own.clone()),
                (theirs, own) => theirs.or(own.clone()),
            }
        };
        let narrowed = match op {
            BinOp::Lt => Range {
                hi: current.hi.min(other.hi - 1),
                below: below(-1),
                ..current
            },
            BinOp::Le => Range {
                hi: current.hi.min(other.hi),
                below: below(0),
                ..current
            },
            BinOp::Eq => Range {
                lo: current.lo.max(other.lo),
                hi: current.hi.min(other.hi),
                below: below(0),
            },
            BinOp::Gt => Range {
                lo: current.lo.max(other.lo + 1),
                ..current
            },
            BinOp::Ge => Range {
                lo: current.lo.max(other.lo),
                ..current
            },
            BinOp::Ne if other.constant() == Some(current.lo) => Range {
                lo: current.lo + 1,
                ..current
            },
            BinOp::Ne if other.constant() == Some(current.hi) => Range {
                hi: current.hi - 1,
                ..current
            },
            _ => return,
        };
        if narrowed.lo > narrowed.hi {
            *known = None;
            return;
        }
        // At least so much, and at most a length: the slice at least so long.
        if let Some(below) = &narrowed.below {
            k.at_least(below, narrowed.lo);
        }
        k.vars.insert(var, narrowed);
    }
}
