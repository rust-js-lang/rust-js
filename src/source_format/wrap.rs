//! Each `jsx!` body laid out at rustfmt's width (#1901, ADR 1166): its
//! JSX by oxc's rules, Prettier's, each piece of Rust in it by rustfmt.
//!
//! ```text
//!   body ──shape──► JSX of placeholders ──oxc──► laid out ──splice──► body
//!                     │                                      ▲
//!                     └── each piece of Rust ──rustfmt───────┘
//! ```
//!
//! A piece is a placeholder, `Ω3`, as wide as rustfmt lays it out on one
//! line, or wider than any line where that's several, so its parent breaks
//! as Prettier breaks one of a multi-line expression; each comes back as it
//! is, each line after its first at its line's indent, or the layout fails.
//! A piece on a line still too long is laid out again at the width its
//! column leaves, once: two rounds at most. A body with a comment no JSX
//! slot holds, between its props or its children, keeps its lines, only
//! its indentation laid out, as a note says.

use std::ops::Range;

use super::Job;
use crate::jsx_syntax::formatting::Body;
use crate::jsx_syntax::formatting::shape::{Attr, Child, Element, Piece};

/// `text`, its `jsx!` bodies indented already, with each laid out.
pub(super) fn laid_out(job: &Job, text: &str) -> Result<String, String> {
    // In the order they're written, as the visitor meets them in another:
    // each replaced from the last, so those before keep where they are.
    let mut bodies = super::shapes(job, text)?;
    bodies.sort_by_key(|body| body.open);
    let mut out = text.to_string();
    for body in bodies.iter().rev() {
        let shape = body
            .shape
            .as_ref()
            .map_err(|why| format!("{}: {why}", job.name.display()))?;
        if let Some(line) = loose_comment(text, body, shape) {
            job.note(format!(
                "{}:{line}: a `jsx!` with a comment between its props or children keeps its lines; only its indentation is laid out",
                job.name.display()
            ));
            continue;
        }
        let laid = body_laid_out(job, text, body, shape)?;
        // Its literals are the body's, each as it's written: a layout that
        // changed one, a multi-line string's line, is an error, never
        // written. oxc's own tokens are held by `layout`; rustfmt's of a
        // piece of Rust are rustfmt's.
        if literals(&laid) != literals(&text[body.open..body.close + 1]) {
            let line = text[..body.open].matches('\n').count() + 1;
            return Err(format!(
                "{}:{line}: laying out a `jsx!` changed a literal in it, so it's left as it is: please report it",
                job.name.display()
            ));
        }
        out.replace_range(body.open..body.close + 1, &laid);
    }
    Ok(out)
}

/// The tokens of `text` but its whitespace, each as it's written.
fn tokens(text: &str) -> Vec<&str> {
    let mut found: Vec<&str> = Vec::new();
    let mut at = 0;
    for token in rustc_lexer::tokenize(text, rustc_lexer::FrontmatterAllowed::No) {
        let len = token.len as usize;
        if !matches!(token.kind, rustc_lexer::TokenKind::Whitespace) {
            found.push(&text[at..at + len]);
        }
        at += len;
    }
    found
}

/// The literals of `text`, each as it's written, in order.
fn literals(text: &str) -> Vec<&str> {
    let mut found: Vec<&str> = Vec::new();
    let mut at = 0;
    for token in rustc_lexer::tokenize(text, rustc_lexer::FrontmatterAllowed::No) {
        let len = token.len as usize;
        if matches!(token.kind, rustc_lexer::TokenKind::Literal { .. }) {
            found.push(&text[at..at + len]);
        }
        at += len;
    }
    found
}

/// The line of a comment of `body` outside its pieces of Rust and its
/// `{/* .. */}` children, which no JSX slot holds; none where it has none.
fn loose_comment(text: &str, body: &Body, shape: &Element) -> Option<usize> {
    let mut held = Vec::new();
    held_ranges(shape, &mut held);
    let inner = &text[body.open + 1..body.close];
    let mut at = body.open + 1;
    for token in rustc_lexer::tokenize(inner, rustc_lexer::FrontmatterAllowed::No) {
        let len = token.len as usize;
        let comment = matches!(
            token.kind,
            rustc_lexer::TokenKind::LineComment { .. } | rustc_lexer::TokenKind::BlockComment { .. }
        );
        if comment && !held.iter().any(|range| range.contains(&at)) {
            return Some(text[..at].matches('\n').count() + 1);
        }
        at += len;
    }
    None
}

/// Where `element`'s pieces of Rust and comment children are, which hold
/// their comments.
fn held_ranges(element: &Element, held: &mut Vec<Range<usize>>) {
    for attr in &element.attrs {
        match attr {
            Attr::Named {
                value: Some(Piece::Braced { whole, .. }),
                ..
            }
            | Attr::Spread { whole, .. } => held.push(whole.clone()),
            _ => {}
        }
    }
    for child in element.children.iter().flatten() {
        match child {
            Child::Element(inner) => held_ranges(inner, held),
            Child::Piece(Piece::Braced { whole, .. }) | Child::Comment(whole) => held.push(whole.clone()),
            Child::Piece(Piece::Literal(_)) => {}
        }
    }
}

/// Of each line of `text` but the first, whether it starts inside a token,
/// a multi-line string's or a block comment's, whose text it is: a line
/// that does is copied as it is, never indented or unindented, which would
/// change the string.
fn inside_tokens(text: &str) -> Vec<bool> {
    let mut inside = Vec::new();
    let mut at = 0;
    let mut spans = Vec::new();
    for token in rustc_lexer::tokenize(text, rustc_lexer::FrontmatterAllowed::No) {
        let len = token.len as usize;
        if !matches!(token.kind, rustc_lexer::TokenKind::Whitespace) {
            spans.push(at..at + len);
        }
        at += len;
    }
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        start += line.len();
        if start < text.len() {
            inside.push(spans.iter().any(|span| span.start < start && start < span.end));
        }
    }
    inside
}

/// A piece of Rust as it's laid out: its lines, rustfmt's, the first where
/// its placeholder is, each other at that line's indent.
struct Laid {
    lines: Vec<String>,
    /// Of each line but the first, whether it's inside a token, as it is.
    verbatim: Vec<bool>,
}

impl Laid {
    fn width(&self) -> usize {
        self.lines.first().map_or(0, |line| line.chars().count())
    }
}

/// How a piece is written in the JSX oxc lays out, and back.
enum Form {
    /// A tag's name, a path, `Ctx::Provider`, or generics, `Card::<T>`:
    /// once of one that closes itself, twice of one with its closing tag.
    Tag { times: usize },
    /// `{expr}`'s expression, its braces the JSX's; a child's or a prop's.
    Expression { child: bool },
    /// A literal, `"Save"`, back as it is: a prop's written `"Ω"`, a
    /// string oxc never breaks, a child's `{Ω}`.
    Literal { prop: bool },
    /// A spread, `{...props}` or `{..base}`, its dots the source's.
    Spread(String),
}

/// `body` laid out, its delimiters with it.
fn body_laid_out(job: &Job, text: &str, body: &Body, shape: &Element) -> Result<String, String> {
    let open = &text[body.open..body.open + 1];
    let close = &text[body.close..body.close + 1];
    let base = body.indent + job.tab;
    let width = job.max_width.saturating_sub(base).max(20);
    // Each piece's text, the JSX's Rust in order, and what it's written as.
    // Its placeholders' stem, one the body doesn't hold: `Ω`, else `ΩΩ`.
    let written = &text[body.open..body.close + 1];
    let stem = (1..)
        .map(|n| "Ω".repeat(n))
        .find(|stem| !written.contains(stem.as_str()))
        .expect("a stem");
    let mut pieces: Vec<(String, Form)> = Vec::new();
    let mut jsx = String::new();
    write_element(text, shape, &mut jsx, &mut pieces, &stem);

    let rust: Vec<usize> = (0..pieces.len())
        .filter(|&i| matches!(pieces[i].1, Form::Expression { .. } | Form::Spread(_)))
        .collect();
    let mut laid = laid_pieces(job, &pieces, &rust, width)?;
    let mut lines = layout(&jsx, &pieces, &laid, width, job, &stem)?;
    // A second round: a piece on a line still too long, laid out again at
    // what its column leaves.
    let mut again: Vec<(usize, usize)> = Vec::new();
    for line in &lines {
        if line.verbatim || base + line.text.chars().count() <= job.max_width {
            continue;
        }
        // A piece of one line on it, at what its column leaves; or the piece
        // of several whose line it is, at what its indent leaves.
        for &(piece, column) in &line.pieces {
            if laid[piece].lines.len() == 1 && rust.contains(&piece) {
                again.push((piece, job.max_width.saturating_sub(base + column).max(20)));
            }
        }
        if let Some((piece, indent)) = line.of
            && rust.contains(&piece)
            && !again.iter().any(|&(p, _)| p == piece)
        {
            again.push((piece, job.max_width.saturating_sub(base + indent).max(20)));
        }
    }
    if !again.is_empty() {
        for (piece, width) in again {
            let narrower = laid_pieces(job, &pieces, &[piece], width)?;
            laid[piece] = narrower.into_iter().nth(piece).expect("the piece laid out again");
        }
        lines = layout(&jsx, &pieces, &laid, width, job, &stem)?;
    }
    let one_line = !text[body.open..body.close].contains('\n') && lines.len() == 1;
    let fits = body.indent + "jsx! ".len() + 2 + lines[0].text.chars().count() + 2 <= job.max_width;
    if one_line && fits {
        return Ok(format!("{open} {} {close}", lines[0].text));
    }
    let mut out = String::from(open);
    out.push('\n');
    for line in &lines {
        if !line.text.is_empty() && !line.verbatim {
            out.push_str(&" ".repeat(base));
        }
        out.push_str(&line.text);
        out.push('\n');
    }
    out.push_str(&" ".repeat(body.indent));
    out.push_str(close);
    Ok(out)
}

/// The placeholder of piece `i`, its body's `stem` and its number, an
/// identifier the body holds none of.
fn placeholder(i: usize, width: usize, stem: &str) -> String {
    let name = format!("{stem}{i}");
    let pad = width.saturating_sub(name.chars().count());
    format!("{name}{}", "_".repeat(pad))
}

/// `element` as JSX, each piece of Rust a placeholder, pushed to `pieces`.
fn write_element(text: &str, element: &Element, out: &mut String, pieces: &mut Vec<(String, Form)>, stem: &str) {
    out.push('<');
    // A path, `Ctx::Provider`, or generics, `Card::<T>`, as a name of its
    // width, back as it's written; a JSX name as it is.
    let name = element.name.as_ref().map(|name| &text[name.clone()]);
    let tag = name.filter(|written| !written.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.'));
    let tag = tag.map(|written| {
        let times = 1 + usize::from(element.children.is_some());
        pieces.push((written.to_string(), Form::Tag { times }));
        tag_placeholder(pieces.len() - 1, written.chars().count(), stem)
    });
    let name = tag.clone().or(name.map(str::to_string)).unwrap_or_default();
    out.push_str(&name);
    for attr in &element.attrs {
        out.push(' ');
        match attr {
            Attr::Named { name, value } => {
                out.push_str(&text[name.clone()]);
                if let Some(value) = value {
                    out.push('=');
                    write_piece(text, value, out, pieces, true);
                }
            }
            Attr::Spread { whole, dots, inner } => {
                let dots = &text[whole.start + 1..whole.start + 1 + dots];
                pieces.push((relative(text, inner.clone()), Form::Spread(dots.to_string())));
                out.push_str(&format!("{{...{}}}", marker(pieces.len() - 1)));
            }
        }
    }
    match &element.children {
        None => out.push_str(" />"),
        Some(children) => {
            out.push('>');
            for child in children {
                match child {
                    Child::Element(inner) => write_element(text, inner, out, pieces, stem),
                    Child::Piece(piece) => write_piece(text, piece, out, pieces, false),
                    Child::Comment(whole) => out.push_str(&text[whole.clone()]),
                }
            }
            // The opening tag's name, which oxc requires the closing's be.
            out.push_str("</");
            out.push_str(&name);
            out.push('>');
        }
    }
}

/// A tag name's placeholder: a capitalized one, a component's.
fn tag_placeholder(i: usize, width: usize, stem: &str) -> String {
    let name = format!("{stem}{i}T");
    let pad = width.saturating_sub(name.chars().count());
    format!("{name}{}", "_".repeat(pad))
}

/// A piece's marker, which `layout` makes as wide as the piece is laid out.
fn marker(i: usize) -> String {
    format!("\u{1}{i}\u{1}")
}

/// The Rust at `range` of `text`, its lines after its first relative to its
/// first's line, as rustfmt lays out a block, but a line inside a token,
/// which is as it is: laid out again where it lands, its indent that line's.
fn relative(text: &str, range: Range<usize>) -> String {
    let piece = text[range.clone()].trim();
    let line_start = text[..range.start].rfind('\n').map_or(0, |at| at + 1);
    let indent = text[line_start..].len() - text[line_start..].trim_start().len();
    let inside = inside_tokens(piece);
    let mut out = String::new();
    for (n, line) in piece.split('\n').enumerate() {
        if n > 0 {
            out.push('\n');
        }
        let keep = n == 0 || inside.get(n - 1).copied().unwrap_or(false);
        let spaces = line.len() - line.trim_start_matches(' ').len();
        out.push_str(if keep { line } else { &line[spaces.min(indent)..] });
    }
    out
}

fn write_piece(text: &str, piece: &Piece, out: &mut String, pieces: &mut Vec<(String, Form)>, prop: bool) {
    match piece {
        Piece::Braced { inner, .. } => {
            pieces.push((relative(text, inner.clone()), Form::Expression { child: !prop }));
            out.push_str(&format!("{{{}}}", marker(pieces.len() - 1)));
        }
        Piece::Literal(range) => {
            pieces.push((text[range.clone()].to_string(), Form::Literal { prop }));
            match prop {
                true => out.push_str(&format!("\"{}\"", marker(pieces.len() - 1))),
                false => out.push_str(&format!("{{{}}}", marker(pieces.len() - 1))),
            }
        }
    }
}

/// Each piece laid out: of `which`, rustfmt's at `width`, all of them in
/// one run; of the others, a literal as it's written, or what it is.
fn laid_pieces(job: &Job, pieces: &[(String, Form)], which: &[usize], width: usize) -> Result<Vec<Laid>, String> {
    let mut synthetic = String::new();
    for &i in which {
        synthetic.push_str(&format!("fn __rust_js_piece_{i}() {{\n{}\n}}\n", pieces[i].0));
    }
    let formatted = match which.is_empty() {
        true => String::new(),
        false => super::formatted_rust(job, &synthetic, Some(width + job.tab))?,
    };
    let mut laid: Vec<Laid> = (pieces.iter())
        .map(|(piece, _)| Laid {
            lines: piece.split('\n').map(str::to_string).collect(),
            verbatim: inside_tokens(piece),
        })
        .collect();
    for &i in which {
        let head = format!("fn __rust_js_piece_{i}() {{\n");
        let start = formatted
            .find(&head)
            .ok_or_else(|| format!("rustfmt lost a `jsx!` piece of Rust, {}", pieces[i].0))?
            + head.len();
        let end = start
            + formatted[start..]
                .find("\n}\n")
                .ok_or("rustfmt's piece of Rust has no end")?;
        let body = &formatted[start..end];
        let inside = inside_tokens(body);
        let lines: Vec<String> = (body.lines().enumerate())
            .map(|(n, line)| match n > 0 && inside[n - 1] {
                true => line.to_string(),
                false => line.strip_prefix(&" ".repeat(job.tab)).unwrap_or(line).to_string(),
            })
            .collect();
        // A piece that ends a statement, `x;`, is a block's: as it's written.
        let verbatim = inside_tokens(&lines.join("\n"));
        laid[i] = match lines.is_empty() {
            true => Laid {
                lines: vec![String::new()],
                verbatim: Vec::new(),
            },
            false => Laid { lines, verbatim },
        };
    }
    Ok(laid)
}

/// A line of a laid-out body: its text, and where each piece's first line
/// is in it, by its column.
struct Line {
    text: String,
    pieces: Vec<(usize, usize)>,
    /// Inside a token, a string's: as it is, never indented.
    verbatim: bool,
    /// Of a line of a piece's after its first, the piece and the indent its
    /// lines are at.
    of: Option<(usize, usize)>,
}

/// `jsx` laid out at `width`, each piece's marker made a placeholder of its
/// width, then spliced back.
fn layout(
    jsx: &str,
    pieces: &[(String, Form)],
    laid: &[Laid],
    width: usize,
    job: &Job,
    stem: &str,
) -> Result<Vec<Line>, String> {
    let mut code = jsx.to_string();
    for i in (0..pieces.len()).rev() {
        let wide = match laid[i].lines.len() {
            1 => match pieces[i].1 {
                // A tag's is written already, of its name's width.
                Form::Tag { .. } => continue,
                // `{Ω}` or `"Ω"` stands for the literal, braces and all.
                Form::Literal { .. } => laid[i].width().saturating_sub(2).max(1),
                Form::Spread(ref dots) => (laid[i].width() + dots.len()).saturating_sub(3).max(1),
                Form::Expression { .. } => laid[i].width().max(1),
            },
            // Several lines: a template of two, which oxc keeps as it is,
            // hugged in its braces, its parent broken, Prettier's; a child's
            // first line too wide for one, so its parent puts it on a line
            // of its own, as Prettier puts a child of several lines.
            // A child's, wider than any line, so its parent puts it on one of
            // its own; oxc breaks its braces, which the splice joins again.
            _ if matches!(pieces[i].1, Form::Expression { child: true }) => width + 1,
            _ => {
                code = code.replace(&marker(i), &format!("`{}\n`", placeholder(i, 1, stem)));
                continue;
            }
        };
        code = code.replace(&marker(i), &placeholder(i, wide, stem));
    }
    let out = crate::format::jsx_expression(&code, width as u16, job.tab as u8)
        .map_err(|why| format!("{}: oxc couldn't lay out a `jsx!`: {why}", job.name.display()))?;
    // oxc moves its tokens, never changes one: a `{" "}` it adds, or any
    // other, is an error.
    if tokens(&out) != tokens(&code) {
        return Err(format!(
            "{}: oxc changed a `jsx!`'s tokens as it laid it out, so it's left as it is: please report it",
            job.name.display()
        ));
    }
    splice(&out, pieces, laid, job, stem)
}

/// `out`, oxc's, each placeholder its piece, once: one missing or repeated
/// is an error, never a guess. A piece of several lines is a template
/// literal of two, `` `Ω3 `` and `` ` ``, which oxc keeps as it is: its
/// first line where the first is, each other at that line's indent, and
/// what follows the second backtick after its last.
fn splice(out: &str, pieces: &[(String, Form)], laid: &[Laid], job: &Job, stem: &str) -> Result<Vec<Line>, String> {
    let fail = |what: &str| format!("{}: {what} in a `jsx!`'s layout", job.name.display());
    let mut seen = vec![0usize; pieces.len()];
    let mut lines: Vec<Line> = Vec::new();
    let source: Vec<&str> = out.lines().collect();
    // Of each line but the first, whether it starts inside a token, a
    // comment's, which is as it is.
    let inside = inside_tokens(out);
    let mut n = 0;
    while n < source.len() {
        let line = source[n];
        n += 1;
        if n > 1 && inside.get(n - 2).copied().unwrap_or(false) {
            lines_push(&mut lines, line.to_string(), Vec::new(), true);
            continue;
        }
        let mut indent = line.len() - line.trim_start().len();
        let mut text = String::new();
        let mut placed = Vec::new();
        let mut rest = line.to_string();
        // Whether the line's start is a piece's, inside a token, as it is.
        let mut verbatim_tail = false;
        while let Some(at) = rest.find(stem) {
            let after_mark = &rest[at + stem.len()..];
            let digits: String = after_mark.chars().take_while(char::is_ascii_digit).collect();
            let i: usize = digits.parse().map_err(|_| fail("a placeholder oxc changed"))?;
            let mut end = at + stem.len() + digits.len();
            let tag = rest[end..].starts_with('T');
            end += usize::from(tag);
            end += rest[end..].chars().take_while(|&c| c == '_').count();
            let piece = pieces.get(i).ok_or_else(|| fail("a placeholder of no piece"))?;
            seen[i] += 1;
            let mut before = rest[..at].to_string();
            let mut after = rest[end..].to_string();
            // A piece of several lines: its template's second line, the
            // next, starts with its closing backtick.
            let several = before.ends_with('`');
            if several {
                before.pop();
                if !after.is_empty() {
                    return Err(fail("a template's first line with more after it"));
                }
                let next = source.get(n).ok_or_else(|| fail("a template's second line"))?;
                after = next
                    .strip_prefix('`')
                    .ok_or_else(|| fail("a template's second line"))?
                    .to_string();
                n += 1;
            }
            let own = &laid[i].lines;
            let written: Vec<String> = match &piece.1 {
                Form::Tag { .. } if tag => vec![piece.0.clone()],
                Form::Tag { .. } => return Err(fail("a tag's placeholder oxc changed")),
                Form::Literal { prop: true } => {
                    // `"Ω"` back to the literal, its quotes with it.
                    before = before
                        .strip_suffix('"')
                        .ok_or_else(|| fail("a prop literal's quotes"))?
                        .to_string();
                    after = after
                        .strip_prefix('"')
                        .ok_or_else(|| fail("a prop literal's quotes"))?
                        .to_string();
                    own.clone()
                }
                Form::Literal { prop: false } => {
                    // `{Ω}` back to the literal, its braces with it, which oxc
                    // may have broken onto lines of their own, a long one's.
                    match before.strip_suffix('{') {
                        Some(stripped) => {
                            before = stripped.to_string();
                            after = after
                                .strip_prefix('}')
                                .ok_or_else(|| fail("a literal's braces"))?
                                .to_string();
                        }
                        None => {
                            let open = lines
                                .last_mut()
                                .filter(|line| before.trim().is_empty() && line.text.ends_with('{'));
                            let open = open.ok_or_else(|| fail("a literal's braces"))?;
                            open.text.pop();
                            if open.text.trim().is_empty() {
                                lines.pop();
                            }
                            let close = source
                                .get(n)
                                .map(|line| line.trim_start())
                                .filter(|line| line.starts_with('}'));
                            if !after.is_empty() || close.is_none() {
                                return Err(fail("a literal's braces"));
                            }
                            before = " ".repeat(indent.saturating_sub(job.tab));
                            after = close.expect("its closing brace")[1..].to_string();
                            n += 1;
                        }
                    }
                    own.clone()
                }
                Form::Spread(dots) => {
                    before = before
                        .strip_suffix("...")
                        .ok_or_else(|| fail("a spread's dots"))?
                        .to_string();
                    let mut own = own.clone();
                    own[0] = format!("{dots}{}", own[0]);
                    own
                }
                Form::Expression { .. } => {
                    // Braces oxc broke onto lines of their own, around a long
                    // one, joined again: `{expr}`, Rust's.
                    let open = (before.trim().is_empty() && !several)
                        .then(|| lines.last().filter(|line| line.text.ends_with('{')))
                        .flatten();
                    let close = source
                        .get(n)
                        .map(|line| line.trim_start())
                        .filter(|line| line.starts_with('}'));
                    if let (Some(_), Some(close), true) = (open, close, after.is_empty()) {
                        let open = lines.pop().expect("its opening brace's line");
                        before = open.text;
                        indent = before.len() - before.trim_start().len();
                        after = close.to_string();
                        n += 1;
                    }
                    own.clone()
                }
            };
            text.push_str(&before);
            placed.push((i, text.chars().count()));
            if written.len() == 1 {
                text.push_str(&written[0]);
                rest = after;
                continue;
            }
            // Its first line ends this one; each other is a line of its own,
            // at this line's indent, or as it is inside a token; its last
            // is the line then, what follows it read on.
            text.push_str(&written[0]);
            lines_push(
                &mut lines,
                std::mem::take(&mut text),
                std::mem::take(&mut placed),
                false,
            );
            let last = written.len() - 1;
            for (k, piece_line) in written[1..].iter().enumerate() {
                let verbatim = laid[i].verbatim.get(k).copied().unwrap_or(false);
                let own = match (verbatim, piece_line.is_empty()) {
                    (true, _) => piece_line.clone(),
                    (false, true) => String::new(),
                    (false, false) => format!("{}{piece_line}", " ".repeat(indent)),
                };
                match k + 1 == last {
                    false => {
                        lines_push(&mut lines, own, Vec::new(), verbatim);
                        lines.last_mut().expect("the line pushed").of = Some((i, indent));
                    }
                    // A last line inside a token, a string's end, is the
                    // line as it is, what follows after it.
                    true => {
                        text = own;
                        verbatim_tail = verbatim;
                    }
                }
            }
            rest = after;
        }
        if !rest.is_empty() || !text.is_empty() || line.is_empty() {
            text.push_str(&rest);
            lines_push(&mut lines, text, placed, verbatim_tail);
        }
    }
    let times = |i: usize| match pieces[i].1 {
        Form::Tag { times } => times,
        _ => 1,
    };
    if let Some(i) = (0..pieces.len()).find(|&i| seen[i] != times(i)) {
        return Err(format!(
            "{}: a `jsx!` piece came back {} times from its layout, `{}`",
            job.name.display(),
            seen[i],
            pieces[i].0
        ));
    }
    Ok(lines)
}

/// A line, its trailing spaces dropped but of one inside a token, which is
/// as it is.
fn lines_push(lines: &mut Vec<Line>, text: String, pieces: Vec<(usize, usize)>, verbatim: bool) {
    lines.push(Line {
        text: match verbatim {
            true => text,
            false => text.trim_end().to_string(),
        },
        pieces,
        verbatim,
        of: None,
    });
}
