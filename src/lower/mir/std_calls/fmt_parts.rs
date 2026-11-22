//! The parts of a `format_args!` as its template writes them, and whether
//! formatting them runs the program's code out of Rust's order (ADRs 0054,
//! 0641): `mir_fmt_call`'s, apart.

use super::*;

impl<'mcx, 'a, 'tcx> MirCx<'mcx, 'a, 'tcx> {
    /// Whether the template `bytes` formats `items` would evaluate one that
    /// does something after formatting that may run the program's code, a
    /// `Debug` of the crate's, where Rust evaluates it first. Of its parts
    /// formatted with a stand-in for each argument, what each formatting
    /// runs alone. MIR's folding keeps the arguments in the order given, and
    /// one read twice held. Where its pieces are `written_apart`, each
    /// written as it's made, through stdout's line or into a text that may
    /// fail (#1490, ADR 0187), writing a piece is something done too, so an
    /// argument that does something after one is evaluated out of order.
    pub(super) fn formats_out_of_order(
        &mut self,
        items: &[Value<'tcx>],
        template: &[Piece],
        written_apart: bool,
        span: Span,
    ) -> R<bool> {
        let effectful: Vec<bool> = (items.iter())
            .map(|item| matches!(item, Value::Fmt(_, _, value) if value.has_effects()))
            .collect();
        if !effectful.contains(&true) {
            return Ok(false);
        }
        let stand_ins: Vec<js::Name> = (0..items.len())
            .map(|i| js::Name::declared(format!("_$argument{i}")))
            .collect();

        let stood: Vec<Value<'tcx>> = (items.iter().enumerate())
            .map(|(i, item)| match item {
                Value::Fmt(kind, ty, _) => Value::Fmt(*kind, *ty, Expr::var(&stand_ins[i])),
                other => other.clone(),
            })
            .collect();
        let mut formatted = false;
        for (part, _) in self.formatted_parts(&stood, template, &mut std::iter::empty(), span)? {
            let mut reads = false;
            part.visit_names(&mut |var| reads |= (0..items.len()).any(|i| effectful[i] && *var == stand_ins[i]));
            if reads && formatted {
                return Ok(true);
            }
            formatted |= written_apart || part.runs_program_code();
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
    ) -> Option<(bool, rustc_middle::mir::Local)> {
        use rustc_middle::mir::Rvalue;
        let lent = called_with(body, at)?;
        let Err(Rvalue::Ref(_, _, made)) = the_one_write(body, lent)? else {
            return None;
        };
        if !made.projection.is_empty() {
            return None;
        }
        Some((self.made_of_a_literal(body, made.local)?, made.local))
    }

    /// The payload of the `panic_fmt(args)` ending block `at`, as Rust's
    /// panic handler makes it: a `&str` where `args.as_str()` is `Some`, of
    /// an `Arguments` made of a literal alone, a `String` of one made with
    /// arguments, as `arguments_made` tells `as_str()` (ADR 0660, amended);
    /// `None` of one made elsewhere (#1091).
    pub(in crate::lower) fn panic_fmt_payload(
        &self,
        body: &rustc_middle::mir::Body<'tcx>,
        at: rustc_middle::mir::BasicBlock,
    ) -> Option<js::Payload> {
        Some(match self.made_of_a_literal(body, called_with(body, at)?)? {
            true => js::Payload::Str,
            false => js::Payload::String,
        })
    }

    /// `panic_fmt(args)` ending block `at`, thrown with its payload's kind,
    /// a `&str` where `args.as_str()` is `Some`, else a `String`, as Rust's
    /// handler makes it (`panic_fmt_payload`); of an `Arguments` made in
    /// another function, which nothing here says, refused, as `as_str()` is.
    pub(super) fn panic_fmt_lowering<'c>(
        &self,
        body: &rustc_middle::mir::Body<'tcx>,
        at: rustc_middle::mir::BasicBlock,
        span: Span,
    ) -> StdLowering<'c, 'a, 'tcx> {
        match self.panic_fmt_payload(body, at) {
            Some(payload) => lower(move |this, state, values, out| {
                this.flush(state, out)?;
                let message = values.next().expect("the panic's `Arguments`");
                let message = this.value_expr(message, span)?;
                out.push(StmtKind::Throw(Expr::panic(message, payload)).at(this.js_span(span)));
                Ok(Value::Expr(Expr::js_undefined()))
            }),
            None => lower(move |this, _, _, _| {
                Err(this.unsupported(
                    span,
                    "a panic of a `fmt::Arguments` made in another function: whether it was made with arguments, which makes its payload a `String`, not a `&str`, isn't known here (#1091)",
                ))
            }),
        }
    }

    /// Whether the `Arguments` `made` holds was made of a literal alone,
    /// `from_str("plain")`'s or a constant, or with arguments, `new(..)`'s,
    /// by its one write in `body`; `None` of a parameter, or another write.
    fn made_of_a_literal(&self, body: &rustc_middle::mir::Body<'tcx>, made: rustc_middle::mir::Local) -> Option<bool> {
        use rustc_middle::mir::{Operand, Rvalue};
        if body.local_kind(made) == rustc_middle::mir::LocalKind::Arg {
            return None;
        }
        match the_one_write(body, made)? {
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

/// The local the call ending block `at` of `body` is given first, itself.
fn called_with(
    body: &rustc_middle::mir::Body<'_>,
    at: rustc_middle::mir::BasicBlock,
) -> Option<rustc_middle::mir::Local> {
    let rustc_middle::mir::TerminatorKind::Call { args, .. } = &body.basic_blocks[at].terminator().kind else {
        return None;
    };
    args.first()?
        .node
        .place()
        .filter(|p| p.projection.is_empty())
        .map(|p| p.local)
}

/// The one write of `local` in `body`, a statement's rvalue or a call's
/// function (`the_write`), a shared borrow of it no write (#2211).
fn the_one_write<'b, 'tcx>(
    body: &'b rustc_middle::mir::Body<'tcx>,
    local: rustc_middle::mir::Local,
) -> Option<Result<&'b rustc_middle::mir::Operand<'tcx>, &'b rustc_middle::mir::Rvalue<'tcx>>> {
    crate::lower::body_queries::the_write(body, local, crate::lower::body_queries::Borrowed::Mutably).map(|write| {
        match write {
            crate::lower::body_queries::Write::Statement(rvalue) => Err(rvalue),
            crate::lower::body_queries::Write::Call(func, _) => Ok(func),
        }
    })
}
