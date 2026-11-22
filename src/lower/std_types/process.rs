//! The process a program runs in (ADR 0391): its arguments, `env::args()`,
//! its environment, `env::var(key)`, `env::vars()` and their `_os`
//! forms, its working directory, `env::current_dir()`, its temporary and
//! home directories, `env::temp_dir()` and `env::home_dir()`, and its
//! end, `process::exit(code)`. Node has one, and a browser hasn't:
//! each is a helper of the runtime's `process.js`, which in a browser
//! panics, saying which, and recognition's `NODE_ONLY` is the one list of
//! them.

use crate::lower::FnCx;
use crate::lower::recognition::Std;
use rust_js_program::js::Expr;

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
    /// `env::current_dir()`: `Ok` of its working directory, a path its text
    /// (ADR 0173), or `Err` of the OS's error, quickcheck's (#1420).
    CurrentDir,
    /// `env::temp_dir()`: `TMPDIR`, else std's unix body's by the OS, of
    /// macOS `confstr(_CS_DARWIN_USER_TEMP_DIR)`'s, `/tmp` where that
    /// fails, of another `/tmp` (#1639).
    TempDir,
    /// `env::home_dir()`: `HOME` where it isn't empty, else the password
    /// database's entry of the process's user, or `None` (#1639).
    HomeDir,
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
            ProcessOp::CurrentDir => "$currentDir",
            ProcessOp::TempDir => "$tempDir",
            ProcessOp::HomeDir => "$homeDir",
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
