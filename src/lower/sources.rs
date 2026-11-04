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

/// Of each block of `body`'s that ends with comments after an `if`, its
/// last, which gives no `()` of its own that marks them (ADR 0753): where
/// the `if` is written, its keyword to its test's end, as its JS `if` is
/// spanned, and where the comments end, its block's `}`, which tidy puts
/// them at, after the `if` (`commented_jumps`): `// TODO: ..` after the
/// `if`/`else` that ends a loop's body or an `if`'s block (rd219, #428).
pub(super) fn tail_if_comments(
    tcx: TyCtxt<'_>,
    sources: &CapturedSources,
    body: rustc_span::def_id::DefId,
) -> Vec<(js::Span, js::Span)> {
    use rustc_hir::intravisit::{self, Visitor};
    struct Tails<'a, 'tcx> {
        tcx: TyCtxt<'tcx>,
        sources: &'a CapturedSources,
        found: Vec<(js::Span, js::Span)>,
    }
    impl<'tcx> Visitor<'tcx> for Tails<'_, 'tcx> {
        type NestedFilter = rustc_middle::hir::nested_filter::OnlyBodies;
        fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
            self.tcx
        }
        fn visit_block(&mut self, block: &'tcx rustc_hir::Block<'tcx>) {
            if let Some(tail) = block.expr
                && let rustc_hir::ExprKind::If(test, ..) = tail.kind
                && !tail.span.from_expansion()
                && let Some(end) = comments_end(self.tcx, self.sources, block.span)
            {
                let written = self.sources.span(tail.span.with_hi(test.span.hi()));
                if !written.is_none() {
                    self.found.push((written, end));
                }
            }
            intravisit::walk_block(self, block);
        }
    }
    let Some(local) = body.as_local() else {
        return Vec::new();
    };
    let Some(owned) = tcx.hir_maybe_body_owned_by(local) else {
        return Vec::new();
    };
    let mut tails = Tails {
        tcx,
        sources,
        found: Vec::new(),
    };
    tails.visit_expr(owned.value);
    tails.found
}

/// Of each arm of `body`'s `match`es with comments above it, what they
/// say of: of one past its first that does nothing, `{}`, `// Fine.\n
/// Ok(()) => {}`, where its block is, and where it ends, its `}`, which
/// those comments are printed before, the arm's own, as if written in it
/// (ADR 0753, amended, #508); not the first arm's: the `if` MIR makes of a
/// `match` is where its first arm is, and a comment above that arm is
/// above the `if`. And of one that's only a `continue`, `// Skipped.\n 1
/// => continue,`, where that `continue` is, which tidy keeps where it's
/// written, those comments before it, as of an arm whose block holds them
/// (#610, the user's decision); and of a block that's only a `continue`,
/// `_ => { // c\n continue; }`, its comments above the arm or in the block
/// before it, as a bare one's (#666).
pub(super) fn commented_arms(
    tcx: TyCtxt<'_>,
    sources: &CapturedSources,
    body: rustc_span::def_id::DefId,
) -> (Vec<(js::Span, js::Span)>, Vec<js::Span>) {
    use rustc_hir::intravisit::{self, Visitor};
    use rustc_lexer::TokenKind;
    struct Arms<'a, 'tcx> {
        tcx: TyCtxt<'tcx>,
        sources: &'a CapturedSources,
        found: Vec<(js::Span, js::Span)>,
        continues: Vec<js::Span>,
    }
    impl<'tcx> Visitor<'tcx> for Arms<'_, 'tcx> {
        type NestedFilter = rustc_middle::hir::nested_filter::OnlyBodies;
        fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
            self.tcx
        }
        fn visit_expr(&mut self, e: &'tcx rustc_hir::Expr<'tcx>) {
            if let rustc_hir::ExprKind::Match(scrutinee, arms, rustc_hir::MatchSource::Normal) = e.kind
                && !e.span.from_expansion()
            {
                for (i, arm) in arms.iter().enumerate() {
                    // Not a guarded one's, whose `case` they're above
                    // (rd619).
                    if arm.guard.is_some() || arm.body.span.from_expansion() {
                        continue;
                    }
                    // What's before it: the arm before, or the `match`'s
                    // scrutinee, and its `{`.
                    let before = i.checked_sub(1).map_or(scrutinee.span, |i| arms[i].span);
                    let map = self.tcx.sess.source_map();
                    let Ok(between) = map.span_to_snippet(before.between(arm.span)) else {
                        continue;
                    };
                    // Only comments, past the arm before's `,` or the
                    // `match`'s `{`, above it; and nothing in its `{}`.
                    let only = |text: &str, past: bool| {
                        let mut comments = 0;
                        for token in rustc_lexer::tokenize(text, rustc_lexer::FrontmatterAllowed::No) {
                            match token.kind {
                                TokenKind::Whitespace => {}
                                TokenKind::Comma if past && i > 0 => {}
                                TokenKind::OpenBrace if past && i == 0 => {}
                                TokenKind::LineComment { doc_style: None }
                                | TokenKind::BlockComment {
                                    doc_style: None,
                                    terminated: true,
                                } => comments += 1,
                                _ => return None,
                            }
                        }
                        Some(comments)
                    };
                    let (at, arm_at) = (self.sources.span(arm.body.span), self.sources.span(arm.span));
                    let Some(above) = only(&between, true) else {
                        continue;
                    };
                    if at.is_none() || arm_at.is_none() {
                        continue;
                    }
                    // A block's `continue` alone, `{ // c\n continue; }`:
                    // where it is, of comments above the arm or before it
                    // in its block, nothing else before it there (#666).
                    let alone = |block: &rustc_hir::Block<'_>| {
                        let jump = match (block.stmts, block.expr) {
                            ([], Some(e)) => e,
                            (
                                [
                                    rustc_hir::Stmt {
                                        kind: rustc_hir::StmtKind::Semi(e),
                                        ..
                                    },
                                ],
                                None,
                            ) => e,
                            _ => return None,
                        };
                        if !matches!(jump.kind, rustc_hir::ExprKind::Continue(_)) || jump.span.from_expansion() {
                            return None;
                        }
                        let before = map.span_to_snippet(block.span.with_hi(jump.span.lo())).ok()?;
                        let inside = only(before.strip_prefix('{')?, false)?;
                        let at = self.sources.span(jump.span);
                        (above + inside > 0 && !at.is_none()).then_some(at)
                    };
                    match arm.body.kind {
                        rustc_hir::ExprKind::Continue(_) if above > 0 => self.continues.push(at),
                        rustc_hir::ExprKind::Block(block, None) if let Some(jump) = alone(block) => {
                            self.continues.push(jump)
                        }
                        rustc_hir::ExprKind::Block(block, None)
                            if above > 0
                                && block.stmts.is_empty()
                                && block.expr.is_none()
                                && let Ok(inside) = map.span_to_snippet(block.span)
                                && let Some(inner) = inside.strip_prefix('{').and_then(|t| t.strip_suffix('}'))
                                && only(inner, false) == Some(0) =>
                        {
                            self.found.push((
                                at,
                                js::Span {
                                    lo: arm_at.lo,
                                    hi: arm_at.lo + 1,
                                },
                            ));
                        }
                        _ => {}
                    }
                }
            }
            intravisit::walk_expr(self, e);
        }
    }
    let Some(local) = body.as_local() else {
        return (Vec::new(), Vec::new());
    };
    let Some(owned) = tcx.hir_maybe_body_owned_by(local) else {
        return (Vec::new(), Vec::new());
    };
    let mut arms = Arms {
        tcx,
        sources,
        found: Vec::new(),
        continues: Vec::new(),
    };
    arms.visit_expr(owned.value);
    (arms.found, arms.continues)
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

/// `text`'s comments, as rustc's lexer reads them, where they are in the
/// arena, its first at `start` (ADR 0444): an outer doc comment, `///` or
/// `/** */`, its item's; an inner one, `//!`, a plain one, as JS has none,
/// where what it's in ends (#486).
fn comments(text: &str, start: u32) -> Vec<Comment> {
    use rustc_lexer::{DocStyle, TokenKind};
    let mut found: Vec<Comment> = Vec::new();
    let mut at = start;
    // How deep in braces each token is, and each inner doc comment not yet
    // closed, by where it is in `found` and how deep: the `}` that ends what
    // it's in is where it ends.
    let mut depth = 0usize;
    let mut open: Vec<(usize, usize)> = Vec::new();
    for token in rustc_lexer::tokenize(text, rustc_lexer::FrontmatterAllowed::Yes) {
        match token.kind {
            TokenKind::OpenBrace => depth += 1,
            TokenKind::CloseBrace => {
                depth = depth.saturating_sub(1);
                while let Some(&(i, d)) = open.last()
                    && d > depth
                {
                    found[i].inner = Some(at);
                    open.pop();
                }
            }
            _ => {}
        }
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
                inner: None,
                written: true,
            });
            if matches!(doc_style, Some(DocStyle::Inner)) {
                open.push((found.len() - 1, depth));
            }
        }
        at += token.len;
    }
    for (i, _) in open {
        found[i].inner = Some(at);
    }
    // Those the JS leaves out, a `// SAFETY:` one's (ADR 0444, amended).
    let spans: Vec<(usize, usize)> = (found.iter())
        .map(|c| ((c.lo - start) as usize, (c.hi - start) as usize))
        .collect();
    for (c, left_out) in found.iter_mut().zip(crate::comments::left_out(text, &spans)) {
        c.written = !left_out;
    }
    found
}

impl<'tcx> super::FnCx<'_, 'tcx> {
    /// What `body` says of what does nothing, its sources this crate's.
    pub(super) fn commented(&self, body: rustc_span::def_id::DefId) -> Commented {
        let (arms, continues) = commented_arms(self.tcx, self.krate.sources, body);
        Commented {
            jumps: tail_if_comments(self.tcx, self.krate.sources, body),
            arms,
            continues,
        }
    }
}

/// What a body says of what does nothing: each `if` its block ends with
/// before comments (`tail_if_comments`, rd219, #428), and its structure's
/// jumps with comments before them, found as it's made (rd109); and each
/// arm that does nothing with comments above it (`commented_arms`, #508),
/// and each `continue` that's an arm with comments above it (#610).
pub(super) struct Commented {
    pub(super) jumps: Vec<(js::Span, js::Span)>,
    pub(super) arms: Vec<(js::Span, js::Span)>,
    pub(super) continues: Vec<js::Span>,
}

/// Where each of `module`'s type declarations is, the whole item: a struct,
/// an enum, a union, a trait or a type alias, which writes no JS where it's
/// written but where a statement of its own starts in it, a class's (#643).
pub(super) fn declarations(tcx: TyCtxt<'_>, module: rustc_span::def_id::LocalModId) -> impl Iterator<Item = Span> + '_ {
    use rustc_hir::def::DefKind;
    (tcx.hir_module_free_items(module))
        .map(|item| item.owner_id.def_id)
        .filter(move |&id| {
            matches!(
                tcx.def_kind(id),
                DefKind::Struct | DefKind::Enum | DefKind::Union | DefKind::Trait | DefKind::TyAlias
            )
        })
        .map(move |id| tcx.source_span(id))
}
