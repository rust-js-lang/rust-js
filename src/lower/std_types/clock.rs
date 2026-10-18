//! std's clock (ADR 0447): a `SystemTime` and an `Instant` are their
//! nanoseconds, a BigInt, as a `Duration` is its own (ADR 0188); a
//! `SystemTimeError` is the `Duration` it holds. `now()` reads JS's clocks.

use crate::js::{Expr, Op, Stmt};
use crate::lower::{FnCx, R};

/// A clock's method or operator.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum ClockOp {
    /// `SystemTime::now()`, `Instant::now()`.
    Now,
    /// `a.duration_since(b)`, `a - b` of two `Instant`s: a `SystemTime`'s a
    /// `Result`, its `Err` of the time `b` is after; an `Instant`'s zero
    /// where `b` is after, or `None`, `checked_duration_since`.
    Since { checked: bool },
    /// `t.elapsed()`: the time since `t`, as `Since` is.
    Elapsed,
    /// `t + d`, `t - d` (and so `t += d`), `checked_add` and
    /// `checked_sub`, which a BigInt never overflows: `Some`.
    Shift { sub: bool },
    /// A `SystemTimeError`'s `duration()`: what it holds.
    ErrorDuration,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What `op` of a clock's makes, of its arguments' values, `instant` of
    /// an `Instant`'s, else a `SystemTime`'s.
    pub(in crate::lower) fn clock_values(
        &mut self,
        op: ClockOp,
        instant: bool,
        values: &mut std::vec::IntoIter<Expr>,
        _out: &mut Vec<Stmt>,
    ) -> R<Expr> {
        let mut arg = || values.next().expect("rustc checked the arguments");
        let now = || Expr::call_helper(if instant { "$instantNow" } else { "$systemNow" }, Vec::new());
        // The time from `b` to `a`.
        let since = |a: Expr, b: Expr, checked: bool| {
            let helper = match (instant, checked) {
                (false, _) => "$timeSince",
                (true, false) => "$instantSince",
                (true, true) => "$instantCheckedSince",
            };
            Expr::call_helper(helper, vec![a, b])
        };
        Ok(match op {
            ClockOp::Now => now(),
            ClockOp::Since { checked } => {
                let (a, b) = (arg(), arg());
                since(a, b, checked)
            }
            ClockOp::Elapsed => since(now(), arg(), false),
            ClockOp::Shift { sub } => {
                let (t, d) = (arg(), arg());
                Expr::bin(if sub { Op::Sub } else { Op::Add }, t, d)
            }
            ClockOp::ErrorDuration => arg(),
        })
    }
}
