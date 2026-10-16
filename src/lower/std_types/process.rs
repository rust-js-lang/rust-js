//! The process a program runs in (ADR 0391): its arguments, `env::args()`,
//! its environment, `env::var(key)`, `env::vars()` and their `_os`
//! forms, and its end,
//! `process::exit(code)`. Node has one, and a browser hasn't:
//! each is a helper of the runtime's `process.js`, which in a browser
//! panics, saying which, and recognition's `NODE_ONLY` is the one list of
//! them.

use crate::js::Expr;
use crate::lower::FnCx;
use crate::lower::recognition::Std;

/// What's asked of the process.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum ProcessOp {
    /// `env::args()`: its arguments, its program's path first, an array.
    Args,
    /// `env::var(key)`: `Ok` of its environment's variable, or `Err` of
    /// `VarError::NotPresent`. JS's are text, so never `NotUnicode`.
    Var,
    /// `env::var_os(key)`: its environment's variable, or `None`. An
    /// `OsString` is its text (ADR 0173).
    VarOs,
    /// `env::vars()`: its environment's pairs, an array.
    Vars,
    /// `env::vars_os()`: the same pairs, an `OsString` its text (ADR 0173).
    VarsOs,
    /// `process::exit(code)`: it ends, with that code.
    Exit,
}

impl ProcessOp {
    /// The runtime's helper that does it, in `process.js`.
    pub(in crate::lower) fn helper(self) -> &'static str {
        match self {
            ProcessOp::Args => "$args",
            ProcessOp::Var => "$var",
            ProcessOp::VarOs => "$varOs",
            ProcessOp::Vars => "$vars",
            ProcessOp::VarsOs => "$varsOs",
            ProcessOp::Exit => "$exit",
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of the process's: its helper, given what it's given. `None`
    /// if `known` is another.
    pub(in crate::lower) fn process_call(&mut self, known: Std, values: &mut std::vec::IntoIter<Expr>) -> Option<Expr> {
        let Std::Process(op) = known else {
            return None;
        };
        Some(Expr::call(Expr::helper(op.helper()), values.collect()))
    }
}
