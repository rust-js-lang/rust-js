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

    /// Where a module's code is, what a comment after its last code ends
    /// (ADR 0621): an inline `mod`'s item, up to its `}`; else its file, whole.
    pub fn module_code(&self, tcx: TyCtxt<'_>, module: rustc_span::def_id::LocalModId) -> js::Span {
        let file = super::module_file(tcx, module);
        let item = tcx.source_span(module.to_local_def_id());
        let inline = !module.to_local_def_id().is_top_level_module()
            && tcx.sess.source_map().lookup_source_file(item.lo()).start_pos == file.start_pos;
        match inline {
            true => self.span(item.with_hi(item.hi() - rustc_span::BytePos(1))),
            false => self.span(Span::with_root_ctxt(file.start_pos, file.end_position())),
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

/// Where a block that ends with comments ends, `{ // why }` of `if c { //
/// why }`, or `{ let _ = a; // Nothing to do. }` of a function's, in the
/// arena: at its `}`, which its comments are printed before (ADRs 0753,
/// 0862). Not a block whose code is last, or of nothing, or one a macro
/// wrote.
pub(super) fn comments_end(tcx: TyCtxt<'_>, sources: &CapturedSources, span: Span) -> Option<js::Span> {
    use rustc_lexer::TokenKind;
    if span.from_expansion() {
        return None;
    }
    let text = tcx.sess.source_map().span_to_snippet(span).ok()?;
    let inner = text.strip_prefix('{')?.strip_suffix('}')?;
    // Those after its last code.
    let mut comments = 0;
    for token in rustc_lexer::tokenize(inner, rustc_lexer::FrontmatterAllowed::No) {
        match token.kind {
            TokenKind::Whitespace => {}
            TokenKind::LineComment { .. } | TokenKind::BlockComment { terminated: true, .. } => comments += 1,
            _ => comments = 0,
        }
    }
    let at = sources.span(span);
    (comments > 0 && !at.is_none()).then(|| js::Span {
        lo: at.hi - 1,
        hi: at.hi,
    })
}

/// Where a loop's body that ends with comments ends, `// TODO ..` before
/// the `}` of `for x in v { if c { .. } else { .. } // TODO .. }`, of the
/// loop's span, `for`'s to its body's `}`: as `comments_end`'s, at that
/// `}` (rd219). Not a loop a macro wrote.
pub(super) fn loop_comments_end(tcx: TyCtxt<'_>, sources: &CapturedSources, span: Span) -> Option<js::Span> {
    use rustc_lexer::TokenKind;
    let span = span.source_callsite();
    if span.from_expansion() {
        return None;
    }
    let text = tcx.sess.source_map().span_to_snippet(span).ok()?;
    let inner = text.strip_suffix('}')?;
    // Those after its last code, which isn't the body's `{`.
    let (mut comments, mut opened) = (0, false);
    for token in rustc_lexer::tokenize(inner, rustc_lexer::FrontmatterAllowed::No) {
        match token.kind {
            TokenKind::Whitespace => {}
            TokenKind::LineComment { .. } | TokenKind::BlockComment { terminated: true, .. } => comments += 1,
            kind => {
                comments = 0;
                opened = kind == TokenKind::OpenBrace;
            }
        }
    }
    let at = sources.span(span);
    (comments > 0 && !opened && !at.is_none()).then(|| js::Span {
        lo: at.hi - 1,
        hi: at.hi,
    })
}

/// Where what a function gives at its end is, `None` of `// Don't return
/// anything\n None`, where a comment is right before it, nothing but
/// whitespace and other comments between: the comment's place, once the
/// `return;` of nothing it is goes (ADR 0862).
pub(super) fn commented(sources: &CapturedSources, span: Span) -> Option<js::Span> {
    if span.from_expansion() {
        return None;
    }
    let at = sources.span(span);
    if at.is_none() {
        return None;
    }
    // Its file, each one's text after the one before's and a newline.
    let mut start = 0;
    let file = (sources.output.files.iter()).find(|f| {
        let end = start + f.text.len() as u32;
        let found = (start..end).contains(&at.lo);
        start = end + 1;
        found
    })?;
    let text = &sources.output.text;
    let mut code = at.lo;
    for c in file.comments.iter().rev().filter(|c| c.hi <= at.lo) {
        if !text[c.hi as usize..code as usize].trim().is_empty() {
            break;
        }
        code = c.lo;
    }
    (code < at.lo).then_some(at)
}

/// `text`'s comments, as rustc's lexer reads them, where they are in the
/// arena, its first at `start` (ADR 0444): an outer doc comment, `///` or
/// `/** */`, its item's; an inner one, `//!`, a plain one, as JS has none.
/// Where the `break` or `continue` the program wrote right after `done`,
/// what a branch did last, is, if comments are right before it: `done` the
/// jump itself, as MIR places a `break`'s `()`, or what's before it, `i +=
/// 3;` of `i += 3; // Skip it.\n continue;`, nothing but whitespace, a `;`
/// and the comments between (rd109). The jump the branch is, as nothing
/// else runs between.
pub(super) fn commented_jump(sources: &CapturedSources, done: js::Span) -> Option<js::Span> {
    if done.is_none() {
        return None;
    }
    let text = &sources.output.text;
    // Its file's, each one's text after the one before's and a newline.
    let mut start = 0;
    let file = (sources.output.files.iter()).find(|f| {
        let end = start + f.text.len() as u32;
        let found = (start..end).contains(&done.lo);
        start = end + 1;
        found
    })?;
    let comments = &file.comments;
    let keyword = |at: u32| {
        let rest = text.get(at as usize..)?;
        ["break", "continue"].into_iter().find_map(|k| {
            let after = rest.strip_prefix(k)?;
            (!after.starts_with(|c: char| c.is_alphanumeric() || c == '_')).then(|| js::Span {
                lo: at,
                hi: at + k.len() as u32,
            })
        })
    };
    // Comments right before `at`, whitespace between.
    let commented_before = |at: u32| {
        let mut code = at;
        for c in comments.iter().rev().filter(|c| c.hi <= at) {
            if !text[c.hi as usize..code as usize].trim().is_empty() {
                break;
            }
            code = c.lo;
        }
        code < at
    };
    let jump = match keyword(done.lo) {
        Some(jump) => jump,
        None => {
            let mut at = done.hi;
            loop {
                let rest = text.get(at as usize..)?;
                let blank = rest.len() - rest.trim_start().len();
                at += blank as u32;
                if text[at as usize..].starts_with(';') {
                    at += 1;
                } else if let Some(c) = comments.iter().find(|c| c.lo == at) {
                    at = c.hi;
                } else if blank == 0 {
                    break;
                }
            }
            keyword(at)?
        }
    };
    commented_before(jump.lo).then_some(jump)
}

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
