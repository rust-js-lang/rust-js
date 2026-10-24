//! `format_args!`, as `format!`, `println!` and `write!` hand it over: its
//! template decoded, and the values it shows written into a JS template
//! literal, in Rust's order (ADRs 0034, 0058, 0066).

use super::calls::Call;
use super::display::{append_written, join};

use super::format_spec::Spec;
use super::recognition::StreamOp;
use super::{FnCx, R, Std};
use crate::js;
use crate::js::StmtKind;
use crate::js::{Expr, Stmt};

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Span;

/// A piece of a `format_args!` template.
#[derive(Clone)]
pub(super) enum Piece {
    Text(String),
    /// A placeholder: which of the arguments goes there, and its options.
    Argument(usize, Spec),
}

/// The template rustc made of the format string of the macro call at
/// `span`, as its source writes it: each of its pieces' part of the
/// template, and whether it's written across lines. A `concat!`'s pieces
/// where each placeholder and each `{{` or `}}` is in one of them, and
/// their text is the template's (ADR 0641); otherwise one, the whole, as
/// a placeholder across pieces, `concat!("{", "}")`, is only one of the
/// string they make, and a value rustc writes in the template, a literal's
/// or a nested `format_args!`'s, is in none.
pub(super) fn written_pieces(tcx: TyCtxt<'_>, span: Span, template: Vec<Piece>) -> Vec<(Vec<Piece>, bool)> {
    let written = super::literals::format_string(tcx, span).unwrap_or_default();
    let segments: Option<Vec<Vec<Segment>>> = written.iter().map(|piece| segments(&piece.text)).collect();
    if written.len() > 1
        && let Some(split) = segments.and_then(|segments| split(template.clone(), &segments))
    {
        return split
            .into_iter()
            .zip(written)
            .map(|(split, piece)| (split, piece.lines))
            .collect();
    }
    // Written across lines, its line breaks as they are (ADR 0066).
    let breaks = template
        .iter()
        .any(|piece| matches!(piece, Piece::Text(text) if text.contains('\n')));
    let lines = breaks && written.first().is_some_and(|piece| piece.lines);
    vec![(template, lines)]
}

/// A part of a format string, as its source writes it.
enum Segment {
    Text(String),
    Placeholder,
}

/// The parts of `text`, one of a format string's pieces, `{{` and `}}` each
/// the brace it is: `None` where a placeholder doesn't end in it. A `{{`
/// or `}}` that doesn't, a brace of each piece, is two where the template
/// has one, which `split` finds.
fn segments(text: &str) -> Option<Vec<Segment>> {
    let (mut segments, mut literal) = (Vec::new(), String::new());
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match c {
            '{' | '}' if chars.next_if(|&(_, next)| next == c).is_some() => literal.push(c),
            '{' => {
                let end = at + text[at..].find('}')?;
                while chars.next_if(|&(i, _)| i <= end).is_some() {}
                segments.push(Segment::Text(std::mem::take(&mut literal)));
                segments.push(Segment::Placeholder);
            }
            c => literal.push(c),
        }
    }
    segments.push(Segment::Text(literal));
    Some(segments)
}

/// `template` split as `written`, each piece's parts in turn: `None` where
/// they aren't its text and placeholders. `println!`'s line break, which
/// it writes after them, is the last piece's.
fn split(template: Vec<Piece>, written: &[Vec<Segment>]) -> Option<Vec<Vec<Piece>>> {
    let mut rest = std::collections::VecDeque::from(template);
    let mut split = Vec::new();
    for segments in written {
        let mut pieces = Vec::new();
        for segment in segments {
            match segment {
                Segment::Text(text) => {
                    let mut text = text.as_str();
                    while !text.is_empty() {
                        let Some(Piece::Text(head)) = rest.front_mut() else {
                            return None;
                        };
                        if let Some(after) = text.strip_prefix(head.as_str()) {
                            text = after;
                            pieces.extend(rest.pop_front());
                        } else {
                            // The template's text goes on into the next piece.
                            let after = head.strip_prefix(text)?;
                            pieces.push(Piece::Text(std::mem::take(&mut text).to_owned()));
                            *head = after.to_owned();
                        }
                    }
                }
                Segment::Placeholder => match rest.pop_front() {
                    Some(argument @ Piece::Argument(..)) => pieces.push(argument),
                    _ => return None,
                },
            }
        }
        split.push(pieces);
    }
    match rest.pop_front() {
        None => {}
        Some(Piece::Text(text)) if text == "\n" && rest.is_empty() => split.last_mut()?.push(Piece::Text(text)),
        Some(_) => return None,
    }
    Some(split)
}

/// A `format_args!` template, decoded (its encoding is documented in core's
/// `fmt::Arguments`): literal pieces prefixed by their length, and a byte
/// with the top two bits set for each placeholder, which names an argument by
/// its place in the array of them. `None` if it isn't one.
pub(super) fn decode_template(template: &[u8]) -> Option<Vec<Piece>> {
    let byte = |i: usize| template.get(i).copied();
    let u16_at = |i: usize| Some(u16::from_le_bytes([byte(i)?, byte(i + 1)?]) as usize);
    let piece = |from: usize, len: usize| {
        let bytes = template.get(from..from + len)?;
        Some(Piece::Text(String::from_utf8_lossy(bytes).into_owned()))
    };
    let (mut pieces, mut i, mut next) = (Vec::new(), 0, 0);
    loop {
        let b = byte(i)?;
        i += 1;
        match b {
            0 => break,
            1..=0x7f => {
                pieces.push(piece(i, b as usize)?);
                i += b as usize;
            }
            0x80 => {
                let len = u16_at(i)?;
                pieces.push(piece(i + 2, len)?);
                i += 2 + len;
            }
            _ if b & 0xc0 == 0xc0 => {
                // Then, if its bits say so: flags, width, precision, and
                // which argument (ADR 0058).
                let mut spec = Spec::plain();
                if b & 0b1 != 0 {
                    let flags = u32::from_le_bytes([byte(i)?, byte(i + 1)?, byte(i + 2)?, byte(i + 3)?]);
                    spec = Spec::from_flags(flags);
                    i += 4;
                }
                // An indirect one is the index of the argument that holds it.
                if b & 0b10 != 0 {
                    let field = u16_at(i)?;
                    match b & 0b1_0000 != 0 {
                        true => spec.width_from = Some(field),
                        false => spec.width = Some(field as u16),
                    }
                    i += 2;
                }
                if b & 0b100 != 0 {
                    let field = u16_at(i)?;
                    match b & 0b10_0000 != 0 {
                        true => spec.precision_from = Some(field),
                        false => spec.precision = Some(field as u16),
                    }
                    i += 2;
                }
                let index = if b & 0b1000 != 0 {
                    let k = u16_at(i)?;
                    i += 2;
                    k
                } else {
                    next
                };
                next = index + 1;
                pieces.push(Piece::Argument(index, spec));
            }
            _ => return None,
        }
    }
    Some(pieces)
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A standard stream's, of its arguments' values: it holds nothing JS
    /// needs, and a write to one is `print!`'s (ADR 0132).
    pub(in crate::lower) fn stream_values(
        &mut self,
        op: StreamOp,
        values: Vec<Expr>,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> Expr {
        let mut values = values.into_iter();
        if op != StreamOp::Open
            && let Some(on) = values.next()
            && on.has_effects()
        {
            out.push(StmtKind::Expr(on).at(self.js_span(span)));
        }
        match op {
            StreamOp::Write { error } => {
                let text = values.next().expect("a write's text");
                match without_newline(text) {
                    Ok(line) => Expr::call(
                        Expr::member(Expr::var("console"), if error { "error" } else { "log" }),
                        vec![line],
                    ),
                    Err(text) => Expr::call(Expr::helper(if error { "$eprint" } else { "$print" }), vec![text]),
                }
            }
            StreamOp::WriteBytes { error } | StreamOp::Written { error } => {
                let bytes = values.next().expect("a write's bytes");
                let stream = Expr::num(f64::from(u8::from(error)));
                let written = Expr::call(Expr::helper("$printBytes"), vec![bytes, stream]);
                match op {
                    StreamOp::Written { .. } => crate::js::result::ok(written),
                    _ => written,
                }
            }
            _ => Expr::undefined(),
        }
    }

    /// `failing_consumer`'s JS, of `value`: what `format!` or `print!`
    /// writes, or `to_string()`'s string, made in `body` (ADR 0364).
    pub(in crate::lower) fn failing_values(
        &mut self,
        known: Std,
        mut body: Vec<Stmt>,
        value: Expr,
        span: Span,
        out: &mut Vec<Stmt>,
    ) -> Expr {
        let js_span = self.js_span(span);
        if let Std::Print { error } = known {
            let (text, caught) = (self.fresh("text"), self.fresh("error"));
            append_written(&Expr::var(&text), value, true, js_span, &mut body);
            let failed = Expr::call(
                Expr::helper("$fmtPrintFailed"),
                vec![Expr::var(&caught), Expr::var(&text), Expr::bool(error)],
            );
            out.push(StmtKind::Let(text.clone(), Some(Expr::str(""))).at(js_span));
            out.push(StmtKind::TryCatch(body, Some(caught), vec![StmtKind::Expr(failed).at(js_span)]).at(js_span));
            return Expr::call(
                Expr::helper(if error { "$eprint" } else { "$print" }),
                vec![Expr::var(&text)],
            );
        }
        let message = match known {
            Std::ToString => "a Display implementation returned an error unexpectedly: Error",
            _ => "a formatting trait implementation returned an error when the underlying stream did not: Error",
        };
        body.push(StmtKind::Return(Some(value)).at(js_span));
        Expr::call(
            Expr::helper("$fmtOrPanic"),
            vec![Expr::arrow(Vec::new(), body), Expr::str(message)],
        )
    }

    /// What `print!`, `panic!` and `format_args!` hand their values to (ADRs 0026, 0034): `None` if `known` is another.
    pub(super) fn print_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Call { generic_args, span, .. } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        let js_span = self.js_span(span);
        Ok(Some(match known {
            Std::PanicDisplay { unreachable } => {
                let ty = generic_args
                    .types()
                    .next()
                    .expect("`panic_display` has a type argument");
                let shown = self.display_string(arg(), ty, span)?;
                let shown = match unreachable {
                    true => join(vec![Expr::str("internal error: entered unreachable code: "), shown]),
                    false => shown,
                };
                out.push(StmtKind::Throw(Expr::panic(shown)).at(js_span));
                Expr::undefined()
            }
            Std::Panic | Std::PanicFmt => {
                out.push(StmtKind::Throw(Expr::panic(arg())).at(js_span));
                Expr::undefined()
            }
            Std::Unreachable => {
                out.push(self.unreachable_code(span));
                Expr::undefined()
            }
            // A `&str` or a `String` is the panic's message, as Rust's hook
            // shows it; another payload, `panic!(5)`, has none.
            Std::BeginPanic => {
                let payload = generic_args.types().next().expect("`begin_panic` has a type argument");
                let text = matches!(payload.kind(), ty::Ref(_, inner, _) if inner.is_str())
                    || self.is_lang_adt(payload, LangItem::String);
                if !text {
                    return Err(self.unsupported(span, "a panic whose payload isn't text"));
                }
                out.push(StmtKind::Throw(Expr::panic(arg())).at(js_span));
                Expr::undefined()
            }
            // A whole line is `console.log`'s, which ends it; text that may
            // not end one is written as it is (ADR 0087).
            Std::Print { error } => match without_newline(arg()) {
                Ok(line) => Expr::call(
                    Expr::member(Expr::var("console"), if error { "error" } else { "log" }),
                    vec![line],
                ),
                Err(text) => Expr::call(Expr::helper(if error { "$eprint" } else { "$print" }), vec![text]),
            },
            Std::FmtStr => arg(),
            Std::FmtDisplay => {
                let ty = generic_args.types().next().expect("`new_display` has a type argument");
                self.display_string(arg(), ty, span)?
            }
            Std::FmtDebug => {
                let ty = generic_args.types().next().expect("`new_debug` has a type argument");
                self.debug_string(arg(), ty, span)?
            }
            // Only in a `format_args!` it recognizes whole (ADR 0058).
            Std::FmtRadix(_) | Std::FmtExp(_) | Std::FmtPointer | Std::FmtUsize => {
                return Err(self.unsupported(span, "`{:x}` and the like here"));
            }
            _ => return Ok(None),
        }))
    }
}

/// `"a\n"`, `` `a ${x}\n` `` or `"a" + "b\n"`: the line, `"a"`, without the
/// newline `println!` ends it with. Anything else is given back.
pub(super) fn without_newline(mut text: Expr) -> Result<Expr, Expr> {
    let last = match &mut text.kind {
        js::ExprKind::Str(s) => s,
        js::ExprKind::Template(texts, ..) => texts.last_mut().expect("a template has a last text"),
        js::ExprKind::Written(pieces) => &mut pieces.last_mut().expect("a written string's pieces").text,
        _ => return Err(text),
    };
    match last.ends_with('\n') {
        true => {
            last.pop();
            Ok(text)
        }
        false => Err(text),
    }
}
