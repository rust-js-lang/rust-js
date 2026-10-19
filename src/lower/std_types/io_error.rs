//! `io::Error` (ADR 0462): a `$IoError`, its kind, an `ErrorKind`'s name as
//! a fieldless enum's value is, and the error it holds, a `dyn Error` pair
//! (ADR 0141), where it was made of one.

use crate::js::Expr;
use crate::lower::calls::Call;
use crate::lower::recognition::Std;
use crate::lower::{FnCx, R};

/// What's asked of an `io::Error`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum IoErrorOp {
    /// `io::Error::new(kind, error)`: of `error`, a message or an error.
    New,
    /// `io::Error::other(error)`: `New` of `Other`.
    Other,
    /// `io::Error::from(kind)`, and `kind.into()`: of its kind alone.
    FromKind,
    /// `e.kind()`.
    Kind,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// An `io::Error`'s function: `None` if `known` is another.
    pub(in crate::lower) fn io_error_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
    ) -> R<Option<Expr>> {
        let Std::IoError(op) = known else {
            return Ok(None);
        };
        let made = |kind: Expr, error: Option<Expr>| {
            let mut args = vec![kind];
            args.extend(error);
            Expr::new_(Expr::helper("$IoError"), args)
        };
        Ok(Some(match op {
            IoErrorOp::New => {
                let kind = values.next().expect("a kind");
                let error = self.held_error(values.next().expect("an error"), call.tys[1], call)?;
                made(kind, Some(error))
            }
            IoErrorOp::Other => {
                let error = self.held_error(values.next().expect("an error"), call.tys[0], call)?;
                made(Expr::str("Other"), Some(error))
            }
            IoErrorOp::FromKind => made(values.next().expect("a kind"), None),
            IoErrorOp::Kind => Expr::member(values.next().expect("an error"), "kind"),
        }))
    }

    /// What an `io::Error` is made of, `error` of type `from`, as the
    /// `Box<dyn Error + Send + Sync>` std makes of it: itself where it's
    /// one already, else `{ value, impl }`, its `Error`'s dictionary.
    fn held_error(&mut self, error: Expr, from: rustc_middle::ty::Ty<'tcx>, call: Call<'_, 'tcx>) -> R<Expr> {
        if from
            .boxed_ty()
            .is_some_and(|inner| matches!(inner.kind(), rustc_middle::ty::Dynamic(..)))
        {
            return Ok(error);
        }
        let dictionary = self.error_dictionary(from, call.span)?;
        Ok(Expr::object(vec![
            crate::js::Prop::Field(crate::js::HELD.into(), error),
            crate::js::Prop::Field(crate::js::DICTIONARY.into(), dictionary),
        ]))
    }
}
