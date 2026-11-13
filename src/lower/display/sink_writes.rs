//! What a `write!` into a writer of the crate's own, or a `W: fmt::Write`'s,
//! writes, a call at a time, the one owner of std's split of each value
//! (#1652, ADR 1135): std's `write_fmt` calls the writer's `write_str` once
//! for each literal piece, and once for each piece a value's `fmt` writes,
//! and its `write_char` where std's does, a sign and each fill char. So a
//! writer that counts its calls, or fails at one, sees what it sees in Rust.
//! A value whose split isn't followed here yet has none, and its `write!`
//! is refused (#1652).

use super::*;
use crate::lower::format_spec::{Spec, options_object};

/// One write into a writer's sink, `f` (`$writerSink`), as std makes it.
#[derive(Clone)]
pub(in crate::lower) enum SinkWrite {
    /// `f.write(text)`: one `write_str`.
    Str(Expr),
    /// `f.writeChar(c)`: one `write_char`.
    Char(Expr),
    /// A std function that writes its pieces into the sink, given it first,
    /// `$writeInteger(f, n, options)`: `pad_integral`'s or `pad`'s.
    Helper(&'static str, Vec<Expr>),
}

impl SinkWrite {
    /// Its statement, written into `sink`.
    pub(in crate::lower) fn written_into(self, sink: &Expr, js_span: js::Span) -> Stmt {
        let call = match self {
            SinkWrite::Str(text) => Expr::call(Expr::member(sink.clone(), "write"), vec![text]),
            SinkWrite::Char(c) => Expr::call(Expr::member(sink.clone(), "writeChar"), vec![c]),
            SinkWrite::Helper(name, mut args) => {
                args.insert(0, sink.clone());
                Expr::call_helper(name, args)
            }
        };
        StmtKind::Expr(call).at(js_span)
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What one placeholder writes into a writer's sink: `value`, shown as
    /// `kind` says, with `spec`'s options and its `width` and `precision`,
    /// each piece as std's `fmt` of it writes it. `None` of a value whose
    /// split isn't followed yet (#1652): a float's, a `Debug` of text, a
    /// composite's or a `fmt` of the crate's own.
    pub(in crate::lower) fn sink_writes(
        &mut self,
        value: Expr,
        (kind, ty): (Std, Ty<'tcx>),
        spec: Spec,
        (width, precision): (Option<Expr>, Option<Expr>),
        span: Span,
    ) -> R<Option<Vec<SinkWrite>>> {
        let (value, ty) = match kind {
            Std::FmtDisplay => self.display_handed_on(value, ty),
            _ => (value, ty),
        };
        let ty = ty.peel_refs();
        let int = shown_num(ty).filter(|num| !num.float());
        // `{:x?}` of an integer is its `{:x}` (ADR 0559).
        let kind = match (kind, spec.debug_hex) {
            (Std::FmtDebug, Some(radix)) if int.is_some() => Std::FmtRadix(radix),
            (_, Some(_)) => return Ok(None),
            _ => kind,
        };
        let options = width.is_some()
            || precision.is_some()
            || spec.plus
            || spec.alternate
            || spec.zero
            || spec.align.is_some()
            || spec.fill != ' ';
        let options_given = || options_object(spec, &width, &precision);
        // `pad`'s fast path: no width and no precision, its one `write_str`.
        let padded = |text: Expr| match width.is_none() && precision.is_none() {
            true => SinkWrite::Str(text),
            false => SinkWrite::Helper("$writePadded", vec![text, options_given()]),
        };
        Ok(Some(vec![match kind {
            // `pad_integral` of its digits: a negative one's sign is a
            // `write_char`; an unsigned one's, given no options, its digits,
            // one `write_str`.
            Std::FmtDisplay | Std::FmtDebug if let Some(num) = int => match options || num.signed() {
                false => SinkWrite::Str(self.format_value(value, (kind, ty), spec, (width, precision), span)?),
                true => {
                    let mut args = vec![value];
                    args.extend(options.then(options_given));
                    SinkWrite::Helper("$writeInteger", args)
                }
            },
            // `pad_integral(true, "0x", digits)`: a prefix where `{:#x}` asks.
            Std::FmtRadix(radix) if let Some(num) = int => {
                let digits = radix.digits(value, num);
                match options {
                    false => SinkWrite::Str(digits),
                    true => SinkWrite::Helper(
                        "$writeIntegral",
                        vec![Expr::bool(true), Expr::str(radix.prefix()), digits, options_given()],
                    ),
                }
            }
            // A `char`'s `Display`, given no width nor precision, is its
            // `write_char`; else `pad` of its text.
            Std::FmtDisplay if ty.is_char() => match width.is_none() && precision.is_none() {
                true => SinkWrite::Char(value),
                false => padded(value),
            },
            // A `bool`'s `Display`, and its `Debug`, which is its `Display`:
            // `pad("true")`.
            Std::FmtDisplay | Std::FmtDebug if ty.is_bool() => {
                padded(Expr::cond(value, Expr::str("true"), Expr::str("false")))
            }
            // A string's `Display`: `pad` of it.
            Std::FmtDisplay if self.is_string_like(ty) => {
                let text = self.format_value(value, (kind, ty), Spec::plain(), (None, None), span)?;
                padded(text)
            }
            _ => return Ok(None),
        }]))
    }
}
