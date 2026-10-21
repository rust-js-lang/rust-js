//! `std::future`'s functions (ADRs 0029, 0571): a future is a JS promise, so
//! `ready(v)`, a future already done, is `Promise.resolve(v)`.

use crate::js::Expr;
use crate::lower::FnCx;
use crate::lower::recognition::Std;

/// What's asked of `std::future`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum FutureOp {
    /// `future::ready(v)`: `Promise.resolve(v)`.
    Ready,
}

/// Which of `std::future`'s functions `path` is, as std names it.
pub(in crate::lower) fn future_op(path: &str) -> Option<FutureOp> {
    match path {
        "std::future::ready" => Some(FutureOp::Ready),
        _ => None,
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A `std::future` function's call: `None` if `known` is another.
    pub(in crate::lower) fn future_call(&mut self, known: Std, values: &mut std::vec::IntoIter<Expr>) -> Option<Expr> {
        let Std::Future(op) = known else {
            return None;
        };
        Some(match op {
            FutureOp::Ready => Expr::call(
                Expr::member(Expr::var("Promise"), "resolve"),
                vec![values.next().expect("its value")],
            ),
        })
    }
}
