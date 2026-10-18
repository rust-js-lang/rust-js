//! Capture normalized frontend files and translate spans into an owned arena.

use crate::js;
use crate::program::{Comment, Source, Sources};
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;
use rustc_span::def_id::LocalDefId;

pub(super) struct CapturedSources {
    offsets: Vec<(u32, u32, u32)>,
    pub output: Sources,
}

impl CapturedSources {
    pub fn new(tcx: TyCtxt<'_>) -> Self {
        let mut output = Sources {
            text: String::new(),
            files: Vec::new(),
        };
        let mut offsets = Vec::new();
        let mut line = 0;
        for file in tcx.sess.source_map().files().iter() {
            let Some(text) = file.src.as_deref() else { continue };
            let start = output.text.len() as u32;
            offsets.push((file.start_pos.0, file.end_position().0, start));
            output.files.push(Source {
                path: file.name.clone().into_local_path(),
                text: text.to_string(),
                line,
                comments: comments(text, start),
            });
            output.text.push_str(text);
            output.text.push('\n');
            line += text.bytes().filter(|&byte| byte == b'\n').count() as u32 + 1;
        }
        Self { offsets, output }
    }

    /// Where an item is: its own span, or, one a macro made of the program's
    /// text, `thread_local!`'s `static`, its name's, which is that text, so
    /// its comments are before it (ADR 0444); else the macro's call.
    pub fn item_span(&self, tcx: TyCtxt<'_>, item: LocalDefId) -> js::Span {
        let span = tcx.def_span(item);
        match tcx.def_ident_span(item) {
            Some(name) if span.from_expansion() && !name.from_expansion() => self.span(name),
            _ => self.span(span),
        }
    }

    pub fn span(&self, span: Span) -> js::Span {
        let span = span.source_callsite();
        let index = self.offsets.partition_point(|&(start, _, _)| start <= span.lo().0);
        let Some(&(start, end, offset)) = index.checked_sub(1).map(|i| &self.offsets[i]) else {
            return js::Span::NONE;
        };
        if span.hi().0 > end {
            return js::Span::NONE;
        }
        js::Span {
            lo: offset + span.lo().0 - start,
            hi: offset + span.hi().0 - start,
        }
    }
}

/// `text`'s comments, as rustc's lexer reads them, where they are in the
/// arena, its first at `start` (ADR 0444): an outer doc comment, `///` or
/// `/** */`, its item's; an inner one, `//!`, a plain one, as JS has none.
fn comments(text: &str, start: u32) -> Vec<Comment> {
    use rustc_lexer::{DocStyle, TokenKind};
    let mut found = Vec::new();
    let mut at = start;
    for token in rustc_lexer::tokenize(text, rustc_lexer::FrontmatterAllowed::Yes) {
        let doc = match token.kind {
            TokenKind::LineComment { doc_style } => Some(doc_style),
            TokenKind::BlockComment {
                doc_style,
                terminated: true,
            } => Some(doc_style),
            _ => None,
        };
        // A CRLF's `\r` isn't the comment's; one JS would read otherwise
        // than Rust isn't written (`is_one_in_js`).
        let written = text[(at - start) as usize..][..token.len as usize].trim_end_matches('\r');
        if let Some(doc_style) = doc
            && crate::comments::is_one_in_js(written)
        {
            found.push(Comment {
                lo: at,
                hi: at + written.len() as u32,
                doc: matches!(doc_style, Some(DocStyle::Outer)),
            });
        }
        at += token.len;
    }
    found
}
