//! What a `write!` into a writer of the crate's own, or a `W: fmt::Write`'s,
//! writes, a call at a time, the one owner of std's split of each value
//! (#1652, ADR 1135): std's `write_fmt` calls the writer's `write_str` once
//! for each literal piece, and once for each piece a value's `fmt` writes,
//! and its `write_char` where std's does, a sign and each fill char. So a
//! writer that counts its calls, or fails at one, sees what it sees in Rust.
//! A value whose split isn't followed here yet has none, and its `write!`
//! is refused (#1652).

use super::*;
use crate::lower::format_spec::{Radix, Spec, options_object};

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
            // A writer of the crate's own is given it among its options.
            (_, Some(_)) if !self.writes_into_a_writer(kind, ty) => return Ok(None),
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
            // A writer of the crate's own, which writes through a sink:
            // called with the writer's sink armed, so each piece it writes is
            // a call of the writer's (`$writingInto`), given the
            // placeholder's options.
            Std::FmtDisplay | Std::FmtDebug if self.writes_into_a_writer(kind, ty) => {
                let (given, options) = crate::lower::format_spec::placeholder_given(spec, (width, precision));
                match self.with_options(options, |cx| cx.written_into_a_writer(kind, ty, value, &given, span))? {
                    Some(write) => write,
                    None => return Ok(None),
                }
            }
            _ => return Ok(None),
        }]))
    }

    /// What a `fmt` of `value`, a `kind` of `ty`, given the `Formatter` of a
    /// writer that writes through a sink, writes into it, by `options`, the
    /// writer's `Formatter`'s, known as it runs: std's `pad_integral` of an
    /// integer, `pad` of a string or a `bool`, a `char`'s `write_char` or
    /// `pad`, and a writer of the crate's own with the sink armed for it.
    /// `None` of a value whose split isn't followed yet (#1652): a float's,
    /// a `Debug` of text, a composite's, an integer's `Debug` where a
    /// placeholder may ask it in hex.
    pub(in crate::lower) fn sink_writes_given(
        &mut self,
        value: Expr,
        (kind, ty): (Std, Ty<'tcx>),
        options: Expr,
        span: Span,
    ) -> R<Option<Vec<SinkWrite>>> {
        let int = shown_num(ty).filter(|num| !num.float());
        let debug_hex = self.debug_hex();
        let given = |value: Expr| vec![value, options.clone()];
        Ok(Some(vec![match kind {
            Std::FmtDisplay if int.is_some() => SinkWrite::Helper("$writeInteger", given(value)),
            Std::FmtDebug if int.is_some() && !debug_hex => SinkWrite::Helper("$writeInteger", given(value)),
            // Where a placeholder may ask it, `{:x?}`, its hex digits, by
            // `LowerHex` or `UpperHex` as its options say (ADR 0559).
            Std::FmtDebug if let Some(num) = int => {
                if value.has_effects() {
                    return Ok(None);
                }
                let lower = Radix::LowerHex.digits(value.clone(), num);
                let upper = Radix::UpperHex.digits(value.clone(), num);
                SinkWrite::Helper("$writeDebugInteger", vec![value, lower, upper, options])
            }
            Std::FmtDisplay if ty.is_char() => SinkWrite::Helper("$writeCharOf", given(value)),
            Std::FmtDisplay | Std::FmtDebug if ty.is_bool() => SinkWrite::Helper(
                "$writePadded",
                given(Expr::cond(value, Expr::str("true"), Expr::str("false"))),
            ),
            Std::FmtDisplay if self.is_string_like(ty) => {
                let text = self.format_value(value, (kind, ty), Spec::plain(), (None, None), span)?;
                SinkWrite::Helper("$writePadded", given(text))
            }
            Std::FmtDisplay | Std::FmtDebug if self.writes_into_a_writer(kind, ty) => {
                let pretty = self.writer_pretty();
                match self.written_into_a_writer(kind, ty, value, &pretty, span)? {
                    Some(write) => write,
                    None => return Ok(None),
                }
            }
            _ => return Ok(None),
        }]))
    }

    /// Does `ty`'s `fmt` of `kind`, `Display` or `Debug`, write through a
    /// sink a `write!` into a writer arms (`analysis::prints`'
    /// `written_into_writers`): a writer of the crate's own type, which it
    /// compiles. Another crate's writer makes its text whole.
    pub(in crate::lower) fn writes_into_a_writer(&self, kind: Std, ty: Ty<'tcx>) -> bool {
        let fmt_trait = match kind {
            Std::FmtDisplay => self.display_trait(),
            Std::FmtDebug => self.debug_trait(),
            _ => return false,
        };
        if !ty.ty_adt_def().is_some_and(|adt| adt.did().is_local()) {
            return false;
        }
        let fmt = crate::lower::recognition::trait_method(self.tcx, fmt_trait, "fmt");
        let args = self.tcx.mk_args(&[ty.into()]);
        matches!(self.resolve_self_instance(fmt, args), Ok(Some(instance))
            if instance.def_id().is_local() && self.writes_through_sink(instance.def_id()))
    }

    /// `ty`'s writer of `kind`, such a writer, called given `value` and
    /// `pretty`'s options with the sink armed for it, `$writingInto(f,
    /// pointDisplay_fmt, p, options)`: its arguments evaluated before it's
    /// armed. `None` of one called through a value, a class's.
    pub(in crate::lower) fn written_into_a_writer(
        &mut self,
        kind: Std,
        ty: Ty<'tcx>,
        value: Expr,
        pretty: &Pretty,
        span: Span,
    ) -> R<Option<SinkWrite>> {
        let fmt_trait = match kind {
            Std::FmtDisplay => self.display_trait(),
            _ => self.debug_trait(),
        };
        let fmt = crate::lower::recognition::trait_method(self.tcx, fmt_trait, "fmt");
        let args = self.tcx.mk_args(&[ty.into()]);
        let (writer, values) = self.writer_given(fmt, args, value, pretty, span)?;
        Ok(self.writer_armed(writer, values))
    }

    /// `writer`, a writer of the crate's own that writes through a sink,
    /// given `values`, called with the sink armed for it. `None` of one
    /// called through a value, a class's.
    pub(in crate::lower) fn writer_armed(&self, writer: DefId, values: Vec<Expr>) -> Option<SinkWrite> {
        let mut given = vec![self.crate_callee(writer)?];
        given.extend(values);
        Some(SinkWrite::Helper("$writingInto", given))
    }
}
