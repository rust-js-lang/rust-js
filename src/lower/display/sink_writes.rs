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
use crate::lower::representation::Num;

/// One write into a writer's sink, `f` (`$writerSink`), as std makes it.
#[derive(Clone)]
pub(in crate::lower) enum SinkWrite {
    /// `f.write(text)`: one `write_str`.
    Str(Expr),
    /// `f.writeChar(c)`: one `write_char`.
    Char(Expr),
    /// A std function that writes its pieces into the sink, given it first,
    /// `$writeInteger(f, n, options)`: `pad_integral`'s or `pad`'s; a
    /// dictionary's, `$writeBy(f, TDisplay, t, options)` (`fmt_into`).
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
    /// split isn't followed yet (#1652): a composite's, a generic or `dyn`
    /// value's.
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
        let float = shown_num(ty).filter(|num| num.float());
        // `{:x?}` of an integer is its `{:x}` (ADR 0559); of another leaf,
        // a float, a string, a `char` or a `bool`, nothing, as its `Debug`
        // reads no hex flag; a composite, and a writer of the crate's own, is
        // given it among its options, which each integer it shows reads.
        let kind = match (kind, spec.debug_hex) {
            (Std::FmtDebug, Some(radix)) if int.is_some() => Std::FmtRadix(radix),
            _ => kind,
        };
        let options = width.is_some()
            || precision.is_some()
            || spec.debug_hex.is_some()
            || spec.plus
            || spec.alternate
            || spec.zero
            || spec.align.is_some()
            || spec.fill != ' ';
        let options_given = || options_object(spec, &width, &precision);
        // A `T`'s or a `dyn`'s, its dictionary's `fmtInto`.
        let given = match options {
            true => options_given(),
            false => Expr::js_undefined(),
        };
        if let Some(write) = self.written_by_dictionary(value.clone(), (kind, ty), given, span)? {
            return Ok(Some(vec![write]));
        }
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
            // An integer's `{:e}`, core's `exp_u64`'s text or parts.
            Std::FmtExp(upper) if int.is_some() => {
                let mut args = vec![value, Expr::bool(upper)];
                args.extend(options.then(options_given));
                SinkWrite::Helper("$writeIntExp", args)
            }
            // A float's, `flt2dec`'s parts as `pad_formatted_parts` writes
            // them.
            Std::FmtDisplay | Std::FmtDebug | Std::FmtExp(_) if let Some(num) = float => {
                let mut args = vec![value, Expr::str(float_trait(kind)), Expr::bool(num == Num::F32)];
                args.extend(options.then(options_given));
                SinkWrite::Helper("$writeFloat", args)
            }
            // A `char`'s case mapping's, `c.to_lowercase()`'s, as core's
            // `CaseMappingIter` writes it: each `char` a `write_char`, its
            // options unread.
            Std::FmtDisplay if crate::lower::recognition::is_case_mapping(self.tcx, ty.peel_refs()) => {
                let text = self.format_value(value, (kind, ty), Spec::plain(), (None, None), span)?;
                SinkWrite::Helper("$writeChars", vec![text])
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
            // A string's and a `char`'s `Debug`: its escapes, each a write
            // of its own, its options unread.
            Std::FmtDebug if ty.is_char() => SinkWrite::Helper("$writeDebugChar", vec![value]),
            Std::FmtDebug if self.is_string_like(ty) => {
                let text = self.format_value(value, (Std::FmtDisplay, ty), Spec::plain(), (None, None), span)?;
                SinkWrite::Helper("$writeDebugStr", vec![text])
            }
            // A writer of the crate's own, which writes through a sink:
            // called with the writer's sink armed, so each piece it writes is
            // a call of the writer's (`$writingInto`), given the
            // placeholder's options.
            _ if self.writes_into_a_writer(kind, ty) => {
                let (given, options) = crate::lower::format_spec::placeholder_given(spec, (width, precision));
                match self.with_options(options, |cx| cx.written_into_a_writer(kind, ty, value, &given, span))? {
                    Some(write) => write,
                    None => return Ok(None),
                }
            }
            // A composite's `Debug`, by core's builders (step 4).
            Std::FmtDebug => {
                let given = match options {
                    true => options_given(),
                    false => Expr::js_undefined(),
                };
                return self.debug_sink_writes(value, ty, (&given, &Expr::bool(spec.alternate)), span);
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
        // What it points at, read as `{}` reads it (#1755).
        let (value, ty) = self.shown_through(value, ty);
        if let Some(write) = self.written_by_dictionary(value.clone(), (kind, ty), options.clone(), span)? {
            return Ok(Some(vec![write]));
        }
        let int = shown_num(ty).filter(|num| !num.float());
        let debug_hex = self.debug_hex();
        // A std parse error's `Display`, `pad` of its message, its value
        // (ADR 0063), as a `dyn Error` of it writes it (`$parseErrorDyn`).
        if kind == Std::FmtDisplay && self.recognition().is_parse_error(ty.peel_refs()) {
            return Ok(Some(vec![SinkWrite::Helper("$writePadded", vec![value, options])]));
        }
        let given = |value: Expr| vec![value, options.clone()];
        Ok(Some(vec![match kind {
            Std::FmtDisplay if int.is_some() => SinkWrite::Helper("$writeInteger", given(value)),
            // `pad_integral(true, "0x", digits)`, by the options as it runs.
            Std::FmtRadix(radix) if let Some(num) = int => {
                if value.has_effects() {
                    return Ok(None);
                }
                let digits = radix.digits(value, num);
                SinkWrite::Helper(
                    "$writeIntegral",
                    vec![Expr::bool(true), Expr::str(radix.prefix()), digits, options],
                )
            }
            // An integer's `{:e}`, core's `exp_u64`'s.
            Std::FmtExp(upper) if int.is_some() => {
                SinkWrite::Helper("$writeIntExp", vec![value, Expr::bool(upper), options.clone()])
            }
            Std::FmtDisplay | Std::FmtDebug | Std::FmtExp(_)
                if let Some(num) = shown_num(ty).filter(|num| num.float()) =>
            {
                SinkWrite::Helper(
                    "$writeFloat",
                    vec![
                        value,
                        Expr::str(float_trait(kind)),
                        Expr::bool(num == Num::F32),
                        options.clone(),
                    ],
                )
            }
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
            Std::FmtDisplay if crate::lower::recognition::is_case_mapping(self.tcx, ty.peel_refs()) => {
                let text = self.format_value(value, (kind, ty), Spec::plain(), (None, None), span)?;
                SinkWrite::Helper("$writeChars", vec![text])
            }
            // An `Arguments`' `Display`, of one made a `dyn Display`, which
            // is the function that writes into the sink it claims (step 6,
            // `given_to_a_write`): armed with this one, as std's
            // `fmt::write` of it writes each piece (#1712). One generic code
            // writes is refused where it's given (`written_params`).
            Std::FmtDisplay if self.is_lang_adt(ty.peel_refs(), LangItem::FormatArguments) => {
                SinkWrite::Helper("$writingInto", vec![value])
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
            Std::FmtDebug if ty.is_char() => SinkWrite::Helper("$writeDebugChar", vec![value]),
            Std::FmtDebug if self.is_string_like(ty) => {
                let text = self.format_value(value, (Std::FmtDisplay, ty), Spec::plain(), (None, None), span)?;
                SinkWrite::Helper("$writeDebugStr", vec![text])
            }
            _ if self.writes_into_a_writer(kind, ty) => {
                // Given the `Formatter`'s options, as std's hands them on.
                let pretty = match options.kind {
                    js::ExprKind::Undefined => Pretty::Plain,
                    _ => Pretty::When(options.clone()),
                };
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
    /// compiles; or a std type's `@rust-js/std` compiles and frames, its
    /// contract says, `fuseDebug_fmt` (#1712). Another crate's writer makes
    /// its text whole.
    pub(in crate::lower) fn writes_into_a_writer(&self, kind: Std, ty: Ty<'tcx>) -> bool {
        let Some(fmt_trait) = self.fmt_trait_of(kind) else {
            return false;
        };
        let Some(adt) = ty.ty_adt_def().map(|adt| adt.did()) else {
            return false;
        };
        // Compiled into `@rust-js/std` for this build: of the plain one, not
        // a `Cycle`, which the runtime holds (`held_cycle`).
        let std_framed = |instance: DefId| {
            crate::lower::recognition::is_compiled_std(self.tcx, instance) && self.krate.foreign.lists_fmt_into(adt)
        };
        if !adt.is_local() && crate::lower::recognition::std_crate(self.tcx, adt.krate).is_none() {
            return false;
        }
        let fmt = crate::lower::recognition::trait_method(self.tcx, fmt_trait, "fmt");
        let args = self.tcx.mk_args(&[ty.into()]);
        matches!(self.resolve_self_instance(fmt, args), Ok(Some(instance))
            if (instance.def_id().is_local() && self.writes_through_sink(instance.def_id())) || std_framed(instance.def_id()))
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
        let fmt_trait = self.fmt_trait_of(kind).expect("a formatting trait");
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

/// The trait `$writeFloat` writes a float by, `kind`'s: `"D"` `Display`,
/// `"G"` `Debug`, `"e"` `LowerExp`, `"E"` `UpperExp`.
fn float_trait(kind: Std) -> &'static str {
    match kind {
        Std::FmtDisplay => "D",
        Std::FmtExp(true) => "E",
        Std::FmtExp(false) => "e",
        _ => "G",
    }
}
