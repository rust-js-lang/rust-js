//! Unwinding (ADR 0660): what JS throws unwinds Rust's frames, a panic's
//! `throw new Error(..)` (ADR 0012) or any JS throw, as Rust's unwinding
//! runs each scope's drops, its `finally` (ADR 0098). `catch_unwind(f)` is
//! `$catchUnwind(f)`, `Ok` of `f()` or `Err` of what unwound, its payload
//! `$unwound(e)`, a `dyn Any` (ADR 0331); preparation writes it as JS's
//! `try` where its `Result` is matched, as a throwing call's (ADR 0637).
//! `resume_unwind(payload)` is `throw $unwinding(payload)`. An
//! `AssertUnwindSafe` is the value it holds (`Recognition::transparent`).

use crate::js::{Expr, Stmt, StmtKind};
use crate::lower::calls::Call;
use crate::lower::recognition::Std;
use crate::lower::{FnCx, R};

/// What's asked of unwinding.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum UnwindOp {
    /// `panic::catch_unwind(f)`.
    Catch,
    /// `panic::resume_unwind(payload)`.
    Resume,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of unwinding's: `None` if `known` is another. Refused where
    /// the crate's panics abort, `panic = "abort"`, which rust-js's unwind
    /// (ADR 0090): `catch_unwind` would catch what ends a process.
    pub(in crate::lower) fn unwind_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Std::Unwind(op) = known else {
            return Ok(None);
        };
        if self.krate.panics_abort {
            return Err(self.unsupported(
                call.span,
                "unwinding where a panic aborts, `panic = \"abort\"`: rust-js's panics unwind",
            ));
        }
        let value = values.next().expect("rustc checked the arguments");
        Ok(Some(match op {
            UnwindOp::Catch => Expr::caught_unwind(value),
            UnwindOp::Resume => {
                let thrown = Expr::call_helper("$unwinding", vec![value]);
                out.push(StmtKind::Throw(thrown).at(self.js_span(call.span)));
                Expr::undefined()
            }
        }))
    }
}
