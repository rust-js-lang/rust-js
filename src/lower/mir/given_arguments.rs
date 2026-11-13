//! A `fmt::Arguments` given on, as `arguments_use` says, is a function
//! that makes its text (#1565): made where the `format_args!` is written,
//! its operands evaluated there, and called where std formats it, so its
//! values' `Display` and `Debug` run there, as often as it's formatted.

use super::arguments_use::{ArgumentsUse, arguments_use, holds_text, resolves_to_own};
use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether the call of `known` into `destination` makes an `Arguments`
    /// the body gives on, which is a function of its text.
    pub(super) fn makes_arguments_given_on(
        &self,
        state: &State<'_, 'tcx>,
        known: Option<Std>,
        destination: Place<'tcx>,
    ) -> bool {
        matches!(known, Some(Std::FmtNew | Std::FmtStr))
            && (destination.as_local())
                .is_none_or(|local| arguments_use(&self.recognition(), state.body, local) == ArgumentsUse::GivenOn)
    }

    /// Of an `Arguments` given on, what `format_args!` gives it, `values`:
    /// each operand evaluated now, held where it's anything but a variable
    /// or a constant, so what the function formats is what Rust evaluated
    /// where the macro is written, however late it's called.
    pub(super) fn arguments_operands_held(
        &mut self,
        state: &mut State<'_, 'tcx>,
        values: &mut [Value<'tcx>],
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        self.flush(state, out)?;
        if let Some(Value::List(items, _)) = values.get_mut(1) {
            for item in items.iter_mut() {
                if let Value::Fmt(_, _, value) = item
                    && !matches!(value.kind, js::ExprKind::Var(_))
                    && !value.is_constant()
                {
                    *value = self.spill(Base::Temporary, std::mem::replace(value, Expr::undefined()), out);
                }
            }
        }
        Ok(())
    }

    /// The function of an `Arguments` given on, of what `format_args!`
    /// made of it, `made`: `() => text`.
    /// One that may fail makes its text piece by piece, what it made
    /// before the error given with it, `$fmtWritten`'s, as one formatted
    /// where it's made writes it (ADR 0187).
    pub(super) fn arguments_given_on(&mut self, made: Value<'tcx>, span: Span) -> R<Value<'tcx>> {
        let (text, fails) = match made {
            Value::Printing(text, fails) => (text, fails),
            Value::Fmt(_, _, text) => (text, true),
            made => (self.value_expr(made, span)?, false),
        };
        let body = match fails {
            false => vec![StmtKind::Return(Some(text)).at(js::Span::NONE)],
            true => {
                let name = self.fresh(Role::Text);
                let made = Expr::var(&name);
                let mut written = Vec::new();
                crate::lower::display::append_written(&made, text, true, js::Span::NONE, &mut written);
                let error = self.fresh(Role::Error);
                let thrown = Expr::call_helper("$fmtWritten", vec![Expr::var(&error), made.clone()]);
                vec![
                    StmtKind::Let(name, Some(Expr::str(""))).at(js::Span::NONE),
                    StmtKind::TryCatch(written, Some(error), vec![StmtKind::Throw(thrown).at(js::Span::NONE)])
                        .at(js::Span::NONE),
                    StmtKind::Return(Some(made)).at(js::Span::NONE),
                ]
            }
        };
        Ok(Value::Expr(Expr::arrow(Vec::new(), body)))
    }

    /// Each `Arguments` std's call `callee` is given by value, its
    /// `operands`' `values`, that's a function, of a place not made its
    /// text here (`holds_text`): called, its text made where std formats
    /// it, as one that may print or fail where any writer of the crate's
    /// may, as a `dyn`'s value is.
    pub(super) fn arguments_formatted(
        &mut self,
        state: &State<'_, 'tcx>,
        (callee, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        (operands, arg_tys): (&[&Operand<'tcx>], &[Ty<'tcx>]),
        values: &mut [Value<'tcx>],
        span: Span,
    ) -> R<()> {
        // Not a std trait's method an impl of the crate's defines, which
        // takes the function as any of the crate's does (#1695).
        if !crate::lower::recognition::is_std_crate(self.tcx, callee)
            || resolves_to_own(&self.recognition(), callee, generic_args)
        {
            return Ok(());
        }
        for (i, value) in values.iter_mut().enumerate() {
            let (Some(operand), Some(&ty)) = (operands.get(i), arg_tys.get(i)) else {
                continue;
            };
            if !self.is_lang_adt(ty, LangItem::FormatArguments) {
                continue;
            }
            let text = (operand.place())
                .and_then(|place| place.as_local())
                .is_some_and(|local| holds_text(&self.recognition(), state.body, local));
            if text {
                continue;
            }
            let given = self.value_expr(std::mem::replace(value, Value::Expr(Expr::undefined())), span)?;
            let called = Expr::call(given, Vec::new());
            let fails = self.krate.failing.any;
            *value = match self.krate.prints.any_writer_prints() {
                true => Value::Printing(called, fails),
                false if fails => Value::Fmt(Std::FmtNew, ty, called),
                false => Value::Expr(called),
            };
        }
        Ok(())
    }
}
