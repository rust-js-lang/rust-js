//! The process a program runs in (ADR 0391): its arguments, `env::args()`,
//! its environment, `env::var(key)`, `env::vars()` and their `_os`
//! forms, its working directory, `env::current_dir()`, its temporary and
//! home directories, `env::temp_dir()` and `env::home_dir()`, and its
//! end, `process::exit(code)`. Node has one, and a browser hasn't:
//! each is a helper of the runtime's `process.js`, which in a browser
//! panics, saying which, and recognition's `NODE_ONLY` is the one list of
//! them. And the status a `main` gives it to end with, `Termination`'s
//! `report` of the types std writes it for (#2292), which any JS has.

use crate::lower::calls::Call;
use crate::lower::recognition::{Std, StdItem};
use crate::lower::temporary_names::Role;
use crate::lower::{FnCx, R};
use rust_js_program::js::{self, Expr, Stmt, StmtKind};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

/// What's asked of the process.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ProcessOp {
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
    /// `Termination::report(value)` of a type std writes it for
    /// (`reported_by_std`): the status a `main` of it ends with, an
    /// `ExitCode`'s `u8` (#2292).
    Report,
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
            ProcessOp::Report => unreachable!("written where it's called, `reported`"),
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of the process's: its helper, given what it's given, or a
    /// `report`'s status. `None` if `known` is another.
    pub(in crate::lower) fn process_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Std::Process(op) = known else {
            return Ok(None);
        };
        if op == ProcessOp::Report {
            let value = values.next().expect("`report` takes its value");
            return self.reported(value, call.tys[0], call.span, out).map(Some);
        }
        Ok(Some(Expr::call(Expr::helper(op.helper()), values.collect())))
    }

    /// The status `Termination::report` of `value`, a `ty`, gives, as std
    /// writes it (`reported_by_std`): an `ExitCode`'s its own `u8`, `()`'s
    /// `0`, `!`'s none; a `Result`'s, of an `Ok` its value's, of an `Err`
    /// `1`, `Error: {err:?}` written to stderr first, as std's impl writes
    /// it.
    fn reported(&mut self, value: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Expr> {
        match ty.kind() {
            ty::Tuple(_) => {
                if value.has_effects() {
                    out.push(StmtKind::Expr(value).at(self.js_span(span)));
                }
                Ok(Expr::num(0.0))
            }
            ty::Adt(_, args) if self.recognition().is_result(ty) => {
                let result = self.spill(Role::Result, value, out);
                let mut ok = Vec::new();
                let ok_status = self.reported(js::result::held(result.clone()), args.type_at(0), span, &mut ok)?;
                let mut err = Vec::new();
                let shown = self.debug_string(js::result::held(result.clone()), args.type_at(1), span)?;
                let text = Expr::template(vec!["Error: ".to_string(), "\n".to_string()], vec![shown]);
                let js_span = self.js_span(span);
                err.push(StmtKind::Expr(self.stream_printed(text, true)).at(js_span));
                let status = self.fresh(Role::Status);
                ok.push(StmtKind::Assign(Expr::var(&status), ok_status).at(js_span));
                err.push(StmtKind::Assign(Expr::var(&status), Expr::num(1.0)).at(js_span));
                out.push(StmtKind::Let(status.clone(), None).at(js_span));
                out.push(StmtKind::If(js::result::is(result, true), ok, Some(err)).at(js_span));
                Ok(Expr::var(&status))
            }
            _ if ty.is_never() || self.recognition().is_std_type(ty, StdItem::ExitCode) => Ok(value),
            _ => Err(self.unsupported(span, "a `report` of a type std doesn't write it for")),
        }
    }
}
