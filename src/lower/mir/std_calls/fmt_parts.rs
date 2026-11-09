//! The parts of a `format_args!` as its template writes them, and whether
//! formatting them runs the program's code out of Rust's order (ADRs 0054,
//! 0641): `mir_fmt_call`'s, apart.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether the template `bytes` formats `items` would evaluate one that
    /// does something after formatting that may run the program's code, a
    /// `Debug` of the crate's, where Rust evaluates it first. Of its parts
    /// formatted with a stand-in for each argument, what each formatting
    /// runs alone. MIR's folding keeps the arguments in the order given, and
    /// one read twice held.
    pub(super) fn formats_out_of_order(&mut self, items: &[Value<'tcx>], template: &[Piece], span: Span) -> R<bool> {
        let effectful: Vec<bool> = (items.iter())
            .map(|item| matches!(item, Value::Fmt(_, _, value) if value.has_effects()))
            .collect();
        if !effectful.contains(&true) {
            return Ok(false);
        }
        let stand_in = |i: usize| format!("_$argument{i}");
        let stood: Vec<Value<'tcx>> = (items.iter().enumerate())
            .map(|(i, item)| match item {
                Value::Fmt(kind, ty, _) => Value::Fmt(*kind, *ty, Expr::var(&stand_in(i))),
                other => other.clone(),
            })
            .collect();
        let mut formatted = false;
        for (part, _) in self.formatted_parts(&stood, template, &mut std::iter::empty(), span)? {
            let mut reads = false;
            part.visit_vars(&mut |var| reads |= (0..items.len()).any(|i| effectful[i] && var == stand_in(i)));
            if reads && formatted {
                return Ok(true);
            }
            formatted |= part.runs_program_code();
        }
        Ok(false)
    }

    /// Each written piece of a format string, `written`, its parts, as
    /// `formatted_parts`, and whether it's written across lines.
    pub(super) fn written_parts(
        &mut self,
        items: &[Value<'tcx>],
        written: &[(Vec<Piece>, bool)],
        arguments: &[Option<Span>],
        span: Span,
    ) -> R<crate::lower::display::WrittenParts> {
        let mut arguments = arguments.iter().copied();
        (written.iter())
            .map(|(template, lines)| Ok((self.formatted_parts(items, template, &mut arguments, span)?, *lines)))
            .collect()
    }

    /// Each part of a format string's template, its text and each argument
    /// of `items` formatted as its spec says, and whether its code, of
    /// `arguments`, each placeholder's in turn, is written across lines, as
    /// its JS may be too (ADR 0641, amended).
    pub(super) fn formatted_parts(
        &mut self,
        items: &[Value<'tcx>],
        template: &[Piece],
        arguments: &mut dyn Iterator<Item = Option<Span>>,
        span: Span,
    ) -> R<Vec<(Expr, bool)>> {
        let mut parts = Vec::new();
        for piece in template {
            parts.push(match *piece {
                Piece::Text(ref text) => (Expr::str(text), false),
                Piece::Argument(i, spec) => {
                    let bad = || self.unsupported(span, "this format argument, from its MIR");
                    let item = |i: usize| match items.get(i) {
                        Some(Value::Fmt(kind, ty, value)) => Some((*kind, *ty, value.clone())),
                        _ => None,
                    };
                    let (kind, ty, value) = item(i).ok_or_else(bad)?;
                    // `{:#?}` of a `&dyn Debug` made elsewhere, the string it
                    // showed there, not pretty (ADR 0137).
                    if spec.alternate && kind == Std::FmtDebug && self.is_dyn_debug(ty) {
                        return Err(self.unsupported(span, "`{:#?}` of a `&dyn Debug` made elsewhere"));
                    }
                    // `{:1$}`'s width, or `{:.*}`'s precision, another argument.
                    let width = match spec.width_from {
                        Some(from) => Some(item(from).ok_or_else(bad)?.2),
                        None => spec.width.map(|w| Expr::int(w.into())),
                    };
                    let precision = match spec.precision_from {
                        Some(from) => Some(item(from).ok_or_else(bad)?.2),
                        None => spec.precision.map(|p| Expr::int(p.into())),
                    };
                    let value = self.format_value(value, (kind, ty), spec, (width, precision), span)?;
                    let code = arguments.next().flatten();
                    let code = code.and_then(|at| self.tcx.sess.source_map().span_to_snippet(at).ok());
                    (value, code.is_some_and(|code| code.contains('\n')))
                }
            });
        }
        Ok(parts)
    }

    /// Whether the `Arguments` the call ending block `at` is given a `&` of, `as_str(&_8)`,
    /// was made of a literal alone, `Arguments::from_str("plain")` or a
    /// constant, `Some(true)`, or with arguments, `Arguments::new(..)`,
    /// `Some(false)`, by the one assignment of each local in `body`; `None` of
    /// one made elsewhere, a parameter's, or assigned more than once (#1091).
    pub(super) fn arguments_made(
        &self,
        body: &rustc_middle::mir::Body<'tcx>,
        at: rustc_middle::mir::BasicBlock,
    ) -> Option<bool> {
        use rustc_middle::mir::{Operand, Rvalue, StatementKind, TerminatorKind};
        let local_of = |operand: &Operand<'tcx>| operand.place().filter(|p| p.projection.is_empty()).map(|p| p.local);
        let TerminatorKind::Call { args, .. } = &body.basic_blocks[at].terminator().kind else {
            return None;
        };
        let lent = local_of(&args.first()?.node)?;
        // Each assignment of `local`: a statement's, or a call's result.
        let assigned = |local: rustc_middle::mir::Local| {
            let statements = (body.basic_blocks.iter())
                .flat_map(|data| data.statements.iter())
                .filter_map(|s| match &s.kind {
                    StatementKind::Assign(assign) if assign.0.local == local => Some(Err(&assign.1)),
                    _ => None,
                });
            let calls = (body.basic_blocks.iter()).filter_map(|data| match &data.terminator().kind {
                TerminatorKind::Call { func, destination, .. } if destination.local == local => Some(Ok(func)),
                _ => None,
            });
            let all: Vec<_> = statements.chain(calls).collect();
            (all.len() == 1).then(|| all[0])
        };
        let Err(Rvalue::Ref(_, _, made)) = assigned(lent)? else {
            return None;
        };
        if !made.projection.is_empty() || body.local_kind(made.local) == rustc_middle::mir::LocalKind::Arg {
            return None;
        }
        match assigned(made.local)? {
            Ok(func) => {
                let (def_id, args) = func.const_fn_def()?;
                match self.recognition().classify(def_id, args)? {
                    Std::FmtStr => Some(true),
                    Std::FmtNew => Some(false),
                    _ => None,
                }
            }
            Err(Rvalue::Use(Operand::Constant(_), _)) => Some(true),
            Err(_) => None,
        }
    }
}
