//! `std::backtrace::Backtrace` (ADR 0568): a backtrace is disabled unless
//! forced, as Rust's is unless `RUST_BACKTRACE` asks, which JS has no
//! environment to. `capture()` and `disabled()` are `undefined`;
//! `force_capture()` is JS's own stack, `$backtrace()`, a string; `status()`
//! is `Disabled` of `undefined`, else `Captured`, a `BacktraceStatus` being
//! its variant's name, as std's enums are; shown, `disabled backtrace` or
//! `<disabled>`, or the frames.

use crate::js::Expr;
use crate::lower::FnCx;
use crate::lower::calls::Call;
use crate::lower::recognition::Std;

/// What's asked of a `Backtrace`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum BacktraceOp {
    /// `Backtrace::capture()`: disabled, as it is unless the environment asks.
    Capture,
    /// `Backtrace::force_capture()`: JS's stack.
    ForceCapture,
    /// `Backtrace::disabled()`.
    Disabled,
    /// `b.status()`.
    Status,
}

/// Which of `Backtrace`'s functions `path` is, as std names it.
pub(in crate::lower) fn backtrace_op(path: &str) -> Option<BacktraceOp> {
    Some(match path.strip_prefix("std::backtrace::Backtrace::")? {
        "capture" => BacktraceOp::Capture,
        "force_capture" => BacktraceOp::ForceCapture,
        "disabled" => BacktraceOp::Disabled,
        "status" => BacktraceOp::Status,
        _ => return None,
    })
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `Backtrace`'s function: `None` if `known` is another.
    pub(in crate::lower) fn backtrace_call(
        &mut self,
        known: Std,
        _call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
    ) -> Option<Expr> {
        let Std::Backtrace(op) = known else {
            return None;
        };
        Some(match op {
            BacktraceOp::Capture | BacktraceOp::Disabled => Expr::undefined(),
            BacktraceOp::ForceCapture => Expr::call_helper("$backtrace", Vec::new()),
            BacktraceOp::Status => Expr::call_helper("$backtraceStatus", vec![values.next().expect("a backtrace")]),
        })
    }

    /// `{}` or `{:?}` of a `Backtrace`, as std shows it.
    pub(in crate::lower) fn backtrace_shown(&self, value: Expr, debug: bool) -> Expr {
        Expr::call_helper("$backtraceShown", vec![value, Expr::bool(debug)])
    }
}
