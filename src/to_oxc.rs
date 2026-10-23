//! Our JS AST ──► oxc's AST ──► JS text + source map.
//!
//! This module, its parts under `to_oxc/`, and `format.rs` are the only
//! code that uses oxc. oxc is pre-1.0 and its API changes often, so keeping
//! it here means an oxc upgrade touches these alone.
//!
//! ```text
//!   js::Module ──convert──► oxc Program ──oxc_codegen──► code + map
//!                           (source_text = the .rs file)
//!
//!   final .js  =  header, imports, helpers  (plain text, no mappings)
//!              +  code                        (map shifted to match)
//!   ──format.rs──► as oxfmt formats it, the map moved to match
//!              +  a blank line between statements that span lines
//!              +  //# sourceMappingURL=...
//! ```
//!
//! The trick that makes the map point at Rust: oxc computes line/column for
//! each node from `program.source_text` and the node's span. We hand it the
//! *Rust* file as `source_text`, and spans that are byte offsets into it.

mod expressions;
mod statements;

pub(crate) use self::expressions::*;

use std::path::PathBuf;

use oxc_allocator::{Allocator, ArenaBox, ArenaVec};
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, ArrowFunctionBody, AssignmentTarget, AssignmentTargetMaybeDefault,
    BindingIdentifier, BindingPattern, BindingProperty, BindingRestElement, BlockStatement, CallExpression,
    CatchClause, CatchParameter, ChainElement, ComputedMemberExpression, Declaration, Directive, ExportFromDeclaration,
    Expression, ForStatementInit, ForStatementLeft, FormalParameter, FormalParameterKind, FormalParameterRest,
    FormalParameters, FunctionBody, FunctionType, IdentifierName, ImportDeclaration, JSXAttributeItem,
    JSXAttributeName, JSXAttributeValue, JSXChild, JSXClosingElement, JSXClosingFragment, JSXElementName,
    JSXExpression, JSXIdentifier, JSXMemberExpressionObject, JSXOpeningElement, JSXOpeningFragment, LabelIdentifier,
    ObjectPropertyKind, Program, PropertyKey, PropertyKind, SimpleAssignmentTarget, Statement, StaticMemberExpression,
    StringLiteral, SwitchCase, TemplateElement, TemplateElementValue, VariableDeclarationKind, VariableDeclarator,
};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::Visit;
use oxc_codegen::{Codegen, CodegenOptions, IndentChar};
use oxc_parser::Parser;
use oxc_regular_expression::{LiteralParser, Options};
use oxc_sourcemap::{SourceMap, SourceMapBuilder};
use oxc_span::{SPAN, SourceType, Span};
use oxc_syntax::number::NumberBase;
use oxc_syntax::operator::{AssignmentOperator, BinaryOperator, LogicalOperator, UnaryOperator, UpdateOperator};

use crate::js::{self, ExprKind, JsxTag, Module, Op, Prop, StmtKind, UnaryOp};

pub struct Output {
    pub code: String,
    /// The source map, as JSON.
    pub map: String,
}

/// The function of `module` declared as its default, `export default
/// function noop() {}` (ADR 0611): `default_declared`'s.
fn defaulted(module: &Module) -> Option<&str> {
    module.default_export.as_deref().filter(|_| module.default_declared)
}

/// Print `module` as JS, with a source map pointing into `rust_source`,
/// laid out as the formatter lays it out (`format::laid_out`). What the
/// crate's transforms make of it, and the `sourceMappingURL`, are the
/// artifact planner's.
///
/// `source_path` is how the map names the Rust file (relative to the map),
/// and `js_file_name` is the output's file name. `path_of` names a source
/// the map points into, asked only for those it does; `primary` is the
/// module's own, which the map names first.
pub fn emit(
    module: &Module,
    sources: &crate::program::Sources,
    path_of: &dyn Fn(usize) -> String,
    primary: Option<usize>,
    source_path: &str,
    js_file_name: &str,
) -> Output {
    let rust_source = sources.text.as_str();
    let allocator = Allocator::default();
    // The module's comments, each where its code is (ADR 0444), whose text
    // the program's source has, a doc comment's JSDoc after the Rust.
    let (text, comments, trailing) = comments_of(module, sources);
    let cx = Cx {
        b: AstBuilder::new(&allocator),
        allocator: &allocator,
        source: rust_source,
        // A blank line's piece is no comment (`comments_of`).
        commented: (comments.iter())
            .filter(|c| !c.span.is_empty())
            .map(|c| c.attached_to)
            .collect(),
    };
    let b = &cx.b;

    let mut body = ArenaVec::new_in(b);
    let mut items = module.items.as_slice();
    while let [item, rest @ ..] = items {
        match item {
            js::Item::Namespace(n) => body.push(cx.namespace(n)),
            // A constant declared as the default is its value (ADR 0625).
            js::Item::Const(c) if defaulted(module) == Some(c.name.as_str()) => body.push(cx.default_value(c)),
            js::Item::Const(c) => body.push(cx.constant(c)),
            js::Item::Statements(stmts) => body.extend(cx.stmts(stmts)),
            js::Item::Function(f) => body.push(cx.function(f, defaulted(module) == Some(f.name.as_str()))),
        }
        items = rest;
    }
    let program = Program::new(
        Span::new(0, rust_source.len() as u32),
        SourceType::mjs(),
        allocator.alloc_str(&text),
        ArenaVec::from_iter_in(comments, b),
        None,                // hashbang
        ArenaVec::new_in(b), // directives
        body,
        b,
    );
    let options = CodegenOptions {
        indent_char: IndentChar::Space,
        indent_width: 2,
        source_map_path: Some(PathBuf::from(source_path)),
        ..CodegenOptions::default()
    };
    let generated = Codegen::new().with_options(options).build(&program);

    // The header, imports and runtime helpers, plain text above the
    // generated code, none of which maps to Rust.
    let mut code = header(module);

    // The generated code, each line after the header: where each ends up,
    // to fix the map.
    let header_lines = code.matches('\n').count() as u32;
    let mut lines: Vec<u32> = Vec::new();
    // A comment after code is after the last JS of its line (ADR 0444); of
    // a line no JS is of, a `}`'s, it's nobody's.
    let mut after: std::collections::HashMap<u32, Vec<&str>> = std::collections::HashMap::new();
    // One inside an expression, after the JS of the code right before it,
    // its line's and column's (ADR 0630).
    let mut inside: std::collections::HashMap<u32, Vec<(u32, &str)>> = std::collections::HashMap::new();
    let generated_lines: Vec<&str> = generated.code.lines().collect();
    if let Some(map) = &generated.map {
        for comment in &trailing {
            // The nearest before it whose JS is a name, a number or a string.
            let before = comment.column.and_then(|column| {
                let mut tokens: Vec<_> = (map.get_tokens())
                    .filter(|t| t.get_src_line() == comment.line && t.get_src_col() < column)
                    .collect();
                tokens.sort_by_key(|t| std::cmp::Reverse(t.get_src_col()));
                tokens
                    .into_iter()
                    .map(|t| (t.get_dst_line(), t.get_dst_col()))
                    .find(|&(line, column)| {
                        (generated_lines.get(line as usize)).is_some_and(|text| word_end(text, column).is_some())
                    })
            });
            match before {
                Some((line, column)) => inside.entry(line).or_default().push((column, &comment.text)),
                None => {
                    let last = (map.get_tokens())
                        .filter(|t| t.get_src_line() == comment.line)
                        .map(|t| t.get_dst_line())
                        .max();
                    if let Some(last) = last {
                        after.entry(last).or_default().push(&comment.text);
                    }
                }
            }
        }
    }
    for (i, (out_line, line)) in (header_lines..).zip(generated.code.lines()).enumerate() {
        let line = match inside.get(&(i as u32)) {
            Some(comments) => with_inside_comments(line, comments),
            None => line.to_string(),
        };
        code.push_str(&line);
        for comment in after.get(&(i as u32)).into_iter().flatten() {
            code.push(' ');
            code.push_str(comment);
        }
        code.push('\n');
        lines.push(out_line);
    }
    // After everything it maps, so the map's lines stay where they are.
    if let Some(function) = &module.default_export
        && defaulted(module).is_none()
    {
        code.push_str(&format!("\nexport default {function};\n"));
    }
    // Its own items by other names, `pub use`'s (ADR 0596).
    if !module.aliases.is_empty() {
        let named: Vec<String> = (module.aliases.iter())
            .map(|(local, exported)| match local == exported {
                true => local.clone(),
                false => format!("{local} as {exported}"),
            })
            .collect();
        code.push_str(&format!("\nexport {{ {} }};\n", named.join(", ")));
    }
    let map = generated.map.expect("a source map, since source_map_path is set");
    // The sources the map points into, and only those, named from here.
    let source_of = |line: u32| sources.file_at(line);
    let used: std::collections::BTreeSet<usize> = primary
        .into_iter()
        .chain(
            map.get_tokens()
                .filter(|t| t.get_source_id().is_some())
                .filter_map(|t| source_of(t.get_src_line())),
        )
        .collect();
    let paths: std::collections::HashMap<usize, String> = used.into_iter().map(|i| (i, path_of(i))).collect();
    let map = restore_sources(&map, sources, &paths, primary);
    let map = crate::format::moved_lines(&map, &lines, js_file_name);
    // Laid out as oxfmt lays it out, rust-js's own layout.
    let (code, map) = crate::format::laid_out(code, map, js_file_name);
    Output { code, map }
}

/// A comment after code on its line (ADR 0444): its line, its column where
/// code follows it there, inside an expression (ADR 0630), and its text.
struct Trailing {
    line: u32,
    column: Option<u32>,
    text: String,
}

/// Where the name, number or string at `column` of `line` ends: none of
/// another token, `(` or `{`.
fn word_end(line: &str, column: u32) -> Option<usize> {
    let start = column as usize;
    let rest = line.get(start..)?;
    let end = match rest.chars().next()? {
        quote @ ('"' | '\'' | '`') => rest[1..].find(quote).map(|i| i + 2)?,
        _ => (rest.find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$' || c == '.'))).unwrap_or(rest.len()),
    };
    (end > 0).then_some(start + end)
}

/// `line` of the JS with each comment after the code that starts at its
/// column: a name, a number or a string, `case 79 /* "O" */:` (ADR 0630).
fn with_inside_comments(line: &str, comments: &[(u32, &str)]) -> String {
    let mut out = line.to_string();
    let mut at: Vec<(usize, &str)> = (comments.iter())
        .filter_map(|&(column, text)| Some((word_end(line, column)?, text)))
        .collect();
    // From the last, so each column is still where it was.
    at.sort_by_key(|&(end, _)| std::cmp::Reverse(end));
    for (end, text) in at {
        out.insert_str(end, &format!(" {text}"));
    }
    out
}

/// The comments of the files `module`'s code is of, past the ones each
/// starts with, a header's (ADR 0383), each before the statement or item it's
/// before: where only the start of that is between them, no `;`, `{` or
/// `}`, so not one of what has no JS, a struct's (ADR 0444); `let`s that
/// have none, read in place (ADR 0603); or a
/// closure's opening, `|queue| {`, whose body is in its place (ADR 0581). Each is as
/// written; a doc comment, `///` lines or `/** */`, a JSDoc one, `/** .. */`,
/// whose text is added after the arena's, where its span points. The text
/// the program's source is, and the comments.
fn comments_of(module: &Module, sources: &crate::program::Sources) -> (String, Vec<oxc_ast::Comment>, Vec<Trailing>) {
    use oxc_ast::{CommentKind, CommentPosition};
    let mut text = sources.text.clone();
    let mut found = Vec::new();
    let mut trailing = Vec::new();
    let spans = statement_spans(module);
    let noinline = noinline_starts(module);
    let mut starts: Vec<u32> = spans.iter().map(|&(lo, _)| lo).collect();
    starts.sort_unstable();
    starts.dedup();
    // What's before the Rust statement at `start` is before the first JS
    // statement printed of it, a temporary rust-js made first, `const value
    // = ..` of `let v = V { patch: .. }`: the first of its spans.
    let printed_first = |start: u32| {
        let end = (spans.iter())
            .filter(|&&(lo, _)| lo == start)
            .map(|&(_, hi)| hi)
            .max()
            .unwrap_or(start);
        (spans.iter())
            .find(|&&(lo, _)| lo >= start && (lo < end || lo == start))
            .map_or(start, |&(lo, _)| lo)
    };
    let mut file_start = 0;
    for file in &sources.files {
        let start = file_start;
        file_start += file.text.len() + 1;
        let end = start + file.text.len();
        // Past what it starts with, a license's, its header's (ADR 0383).
        let past = (start + crate::comments::leading_comments(&file.text).len()) as u32;
        let file_start = start;
        let in_file = |at: u32| at >= past && (at as usize) < end;
        if !starts.iter().any(|&at| in_file(at)) {
            continue;
        }
        // Each comment, past the leading ones, with what it's before.
        // One after code on its line is that code's: by its line, which the
        // map gives the JS of.
        let after_code = |c: &crate::program::Comment| {
            let line_start = sources.text[..c.lo as usize].rfind('\n').map_or(0, |i| i + 1);
            !sources.text[line_start..c.lo as usize].trim().is_empty()
        };
        for c in file.comments.iter().filter(|c| c.lo >= past && after_code(c)) {
            let written = &sources.text[c.lo as usize..c.hi as usize];
            if !written.contains('\n') {
                let line = sources.text[..c.lo as usize].matches('\n').count() as u32;
                let line_start = sources.text[..c.lo as usize].rfind('\n').map_or(0, |i| i + 1);
                let line_end = sources.text[c.hi as usize..]
                    .find('\n')
                    .map_or(sources.text.len(), |i| c.hi as usize + i);
                // A block comment with code after it on its line is inside
                // an expression: after the code right before it (ADR 0630).
                let inside = written.starts_with("/*") && !sources.text[c.hi as usize..line_end].trim().is_empty();
                let column = inside.then(|| (c.lo as usize - line_start) as u32);
                trailing.push(Trailing {
                    line,
                    column,
                    text: written.to_string(),
                });
            }
        }
        let attached: Vec<(crate::program::Comment, u32)> = (file.comments.iter())
            .filter(|c| c.lo >= past && !after_code(c))
            .filter_map(|c| {
                // Before a match arm, `IMMEDIATE_PRIORITY => -1.0`, its body's: a start whose text to the next is only a pattern and
                // its `=>` is the arm's test, which an `else if` isn't a
                // statement of (ADR 0590).
                let mut next = starts.partition_point(|&s| s < c.hi);
                while let [at, after, ..] = starts[next.min(starts.len())..]
                    && sources
                        .text
                        .get(c.hi as usize..at as usize)
                        .is_some_and(|gap| gap.trim().is_empty())
                    && let Some(pattern) = sources.text.get(at as usize..after as usize)
                    && pattern.trim_end().ends_with("=>")
                    && !pattern.contains([';', '{', '}', '\n'])
                {
                    next += 1;
                }
                let at = *starts.get(next).filter(|&&at| in_file(at))?;
                // What's between, but other comments, whose text is no code.
                let mut code = c.hi;
                let only_starts = (file.comments.iter())
                    .filter(|other| other.lo >= c.hi && other.hi <= at)
                    .chain([&crate::program::Comment {
                        lo: at,
                        hi: at,
                        doc: false,
                    }])
                    .all(|other| {
                        let between = &sources.text[code as usize..other.lo as usize];
                        code = other.hi;
                        // A closure's opening, `with_borrow_mut(|queue| {`,
                        // whose body is in its place (ADR 0581), is a start;
                        // and a macro's on its own line, `thread_local! {`,
                        // whose code is (ADR 0621).
                        let opens = (between.match_indices('{'))
                            .all(|(i, _)| between[..i].trim_end().ends_with('|') || macro_opening(&between[..=i]));
                        // `let`s with no JS of their own, a variable read in
                        // place, `let timer = Rc::clone(timer);` (ADR 0603).
                        let unprinted_lets = between.trim_end().ends_with(';')
                            && !between.contains(['{', '}'])
                            && (between.split(';').map(str::trim))
                                .filter(|s| !s.is_empty())
                                .all(|s| s.starts_with("let "));
                        (!between.contains([';', '}']) && opens) || unprinted_lets
                    });
                only_starts.then_some((*c, at))
            })
            .collect();
        // What's before each statement: its comments, and a blank line where
        // the Rust has one, before or between them (ADR 0444).
        // Before the line it's on: a statement's span may start inside its
        // Rust's, `if` before its test, a `let .. else` before its pattern.
        let blank_before = |lo: u32| {
            let before = &sources.text[file_start..lo as usize];
            let mut before = &before[..before.rfind('\n').map_or(0, |i| i + 1)];
            // Its attributes' lines, `#[cfg_attr(..)]`, are its own: what's
            // before them (ADR 0606); and a macro's opening, `thread_local! {`,
            // its code's (ADR 0621).
            while let Some(body) = before.strip_suffix('\n')
                && let last = &body[body.rfind('\n').map_or(0, |i| i + 1)..]
                && (last.trim_start().starts_with("#[") || macro_opening(last))
            {
                before = &body[..body.len() - last.len()];
            }
            let gap = &before[before.trim_end_matches(char::is_whitespace).len()..];
            gap.matches('\n').count() >= 2
        };
        // A blank line: a line comment of nothing, which prints as one.
        let blank = |text: &String| {
            let at = text.len() as u32;
            (at, at)
        };
        // After the module's last code, to where its code ends, its file's
        // end or an inline `mod`'s `}`, only comments: its JS's end (ADR 0621).
        let code = module.code;
        let ends: Vec<crate::program::Comment> = (file.comments.iter())
            .filter(|c| c.lo >= past && !after_code(c) && !code.is_none() && code.lo <= c.lo && c.hi <= code.hi)
            .filter(|c| {
                let mut past = c.hi as usize;
                for other in file.comments.iter().filter(|o| o.lo >= c.hi && o.hi <= code.hi) {
                    if !sources.text[past..other.lo as usize].trim().is_empty() {
                        return false;
                    }
                    past = other.hi as usize;
                }
                sources.text[past..code.hi as usize].trim().is_empty()
            })
            .copied()
            .collect();
        let mut starts_and_end: Vec<(u32, Vec<crate::program::Comment>)> = (starts.iter())
            .filter(|&&at| in_file(at))
            .map(|&at| (at, attached.iter().filter(|(_, a)| *a == at).map(|(c, _)| *c).collect()))
            .collect();
        if !ends.is_empty() {
            starts_and_end.push((sources.text.len() as u32, ends));
        }
        for (at, own) in starts_and_end {
            let mut pieces: Vec<(u32, u32)> = Vec::new();
            // A function kept out of line, `#[inline(never)]`, says so in its
            // JSDoc, the last line of its own, or one of its own (ADR 0649).
            let noinline = noinline.contains(&at);
            let mut said = false;
            let mut i = 0;
            while i < own.len() {
                let comment = own[i];
                if blank_before(comment.lo) {
                    pieces.push(blank(&text));
                }
                let is_doc_line =
                    |c: &crate::program::Comment| c.doc && sources.text[c.lo as usize..].starts_with("///");
                let run = own[i..].iter().take_while(|c| is_doc_line(c)).count();
                let lines: Vec<&str> = (own[i..i + run].iter())
                    .map(|c| &sources.text[c.lo as usize..c.hi as usize])
                    .collect();
                // A doc comment's lines, before one item: one JSDoc of them,
                // or each as written where JS can't hold one (`jsdoc`).
                let mut lines: Vec<&str> = lines.iter().map(|l| l.trim_start_matches("///")).collect();
                if noinline && run > 0 && i + run == own.len() {
                    lines.push(NOINLINE);
                    said = true;
                }
                match crate::comments::jsdoc(&lines).filter(|_| run > 0) {
                    Some(jsdoc) => {
                        let lo = text.len() as u32;
                        text.push_str(&jsdoc);
                        pieces.push((lo, text.len() as u32));
                        i += run - 1;
                    }
                    None => pieces.push((comment.lo, comment.hi)),
                }
                i += 1;
            }
            let last = own.last().map_or(at, |c| c.hi);
            // Of the first statement on its line only: not a closure's in
            // the `let` it's written in.
            let line_start = sources.text[..at as usize].rfind('\n').map_or(0, |i| i + 1) as u32;
            let first_on_line = !starts.iter().any(|&s| s >= line_start && s < at);
            if (own.is_empty() && first_on_line && blank_before(at))
                || (!own.is_empty() && {
                    // A line of nothing between: not a closure's opening's.
                    let gap: Vec<&str> = sources.text[last as usize..at as usize].split('\n').collect();
                    gap.len() > 2 && gap[1..gap.len() - 1].iter().any(|line| line.trim().is_empty())
                })
            {
                pieces.push(blank(&text));
            }
            if noinline && !said {
                let lo = text.len() as u32;
                text.push_str(&format!("/** {NOINLINE} */"));
                pieces.push((lo, text.len() as u32));
            }
            for (lo, hi) in pieces {
                let written = &text[lo as usize..hi as usize];
                let kind = match (written.starts_with("/*"), written.contains('\n')) {
                    (false, _) => CommentKind::Line,
                    (true, false) => CommentKind::SingleLineBlock,
                    (true, true) => CommentKind::MultiLineBlock,
                };
                let mut made = oxc_ast::Comment::new(lo, hi, kind);
                made.attached_to = printed_first(at);
                made.position = CommentPosition::Leading;
                made.set_preceded_by_newline(true);
                made.set_followed_by_newline(true);
                found.push(made);
            }
        }
    }
    (text, found, trailing)
}

/// Whether `line`, to its end, opens a macro's body on a line of its own,
/// `thread_local! {`, `jsx! {`: a path, `!` and `{`, whose code is the
/// program's text (ADR 0621).
fn macro_opening(line: &str) -> bool {
    let line = line[line.rfind('\n').map_or(0, |i| i + 1)..].trim();
    line.strip_suffix('{')
        .and_then(|head| head.trim_end().strip_suffix('!'))
        .is_some_and(|path| !path.is_empty() && path.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ':'))
}

/// The span of each of `list`'s statements that has one: a temporary
/// rust-js made has the next's start when it's printed (`push_stmts`).
fn list_spans(list: &[js::Stmt]) -> impl Iterator<Item = (u32, u32)> + '_ {
    (list.iter())
        .filter(|stmt| !stmt.span.is_none())
        .map(|stmt| (stmt.span.lo, stmt.span.hi))
}

/// Closure Compiler's annotation of a function kept out of line, Rust's
/// `#[inline(never)]` (ADR 0649).
const NOINLINE: &str = "@noinline";

/// Where each of `module`'s functions marked `#[inline(never)]` starts.
fn noinline_starts(module: &Module) -> std::collections::HashSet<u32> {
    let functions = (module.items.iter()).flat_map(|item| match item {
        js::Item::Function(f) => vec![f],
        js::Item::Namespace(n) => n.methods.iter().collect(),
        _ => Vec::new(),
    });
    functions
        .filter(|f| f.noinline && !f.span.is_none())
        .map(|f| f.span.lo)
        .collect()
}

/// The span of each statement and item of `module`, closures' and nested
/// functions' too, each list's in the order it's printed: what a comment is
/// attached to (ADR 0444).
fn statement_spans(module: &Module) -> Vec<(u32, u32)> {
    fn of_stmts(stmts: &[js::Stmt], starts: &mut Vec<(u32, u32)>) {
        let mut copy = stmts.to_vec();
        let mut functions = Vec::new();
        js::statement_lists(&mut copy, &mut |list| {
            starts.extend(list_spans(list));
            for stmt in list.iter() {
                if let StmtKind::Function(f) = &stmt.kind {
                    functions.push(f.body.clone());
                }
            }
        });
        js::each_expr_mut(&mut copy, &mut |e| match &mut e.kind {
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => {
                js::statement_lists(body, &mut |list| starts.extend(list_spans(list)));
            }
            ExprKind::Function(f) => functions.push(f.body.clone()),
            _ => {}
        });
        for body in functions {
            of_stmts(&body, starts);
        }
    }
    let mut starts = Vec::new();
    let function = |f: &js::Function, starts: &mut Vec<(u32, u32)>| {
        if !f.span.is_none() {
            starts.push((f.span.lo, f.span.hi));
        }
        of_stmts(&f.body, starts);
    };
    for item in &module.items {
        match item {
            js::Item::Const(c) => {
                if !c.span.is_none() {
                    starts.push((c.span.lo, c.span.hi));
                }
                of_stmts(&[StmtKind::Expr(c.value.clone()).at(js::Span::NONE)], &mut starts);
            }
            js::Item::Function(f) => function(f, &mut starts),
            js::Item::Namespace(n) => {
                for method in &n.methods {
                    function(method, &mut starts);
                }
            }
            js::Item::Statements(stmts) => of_stmts(stmts, &mut starts),
        }
    }
    starts
}

/// `import clause from "from";`, an import's line, the one way it's written:
/// a module's, and the `#[test]` runner's (`output.rs`).
pub fn import_line(clause: &str, from: &str) -> String {
    format!("import {clause} from {from:?};\n")
}

/// What's above a module's code: its header, directives, imports, one block
/// as a person orders it (ADR 0295), and re-exports. Layout, as
/// `format::spaced`'s blank lines are, of what the module says.
fn header(module: &Module) -> String {
    let mut code = format!("{}\n", module.header);
    for directive in &module.directives {
        code.push_str(&format!("\n{directive:?};\n"));
    }
    let packages = &module.packages;
    // Sorted already, and each once, as the artifact planner chose them.
    let helpers = &module.helpers;
    // One block, as a person orders it (ADR 0295): packages, then the
    // module's own relative ones, bindings and the crate's alike, each by
    // path; then what it imports for its effect, as written, its CSS after
    // what it styles; then the runtime's helpers.
    let mut named: Vec<(bool, &str, String)> = Vec::new();
    let mut effects = Vec::new();
    let clause_of = |pairs: &mut dyn Iterator<Item = &(String, String)>| -> Vec<String> {
        pairs
            .map(|(export, local)| {
                if export == local {
                    export.clone()
                } else {
                    format!("{export} as {local}")
                }
            })
            .collect()
    };
    for package in packages {
        let names = clause_of(&mut package.named.iter());
        let names = (!names.is_empty()).then(|| format!("{{ {} }}", names.join(", ")));
        let clause: Vec<String> = package.default.iter().cloned().chain(names).collect();
        let relative = crate::paths::is_relative_specifier(&package.from);
        if !clause.is_empty() {
            let line = import_line(&clause.join(", "), &package.from);
            named.push((relative, &package.from, line));
        }
        if let Some(namespace) = &package.namespace {
            let line = import_line(&format!("* as {namespace}"), &package.from);
            named.push((relative, &package.from, line));
        }
        if clause.is_empty() && package.namespace.is_none() {
            effects.push(format!("import {:?};\n", package.from));
        }
    }
    for import in &module.imports {
        let default = (import.named.iter())
            .find(|(export, _)| export == "default")
            .map(|(_, local)| local);
        let names = clause_of(&mut import.named.iter().filter(|(export, _)| export != "default"));
        let clause = match (default, names.is_empty()) {
            (Some(default), true) => default.clone(),
            (Some(default), false) => format!("{default}, {{ {} }}", names.join(", ")),
            (None, _) => format!("{{ {} }}", names.join(", ")),
        };
        let line = import_line(&clause, &import.from);
        named.push((crate::paths::is_relative_specifier(&import.from), &import.from, line));
    }
    named.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    let runtime =
        (!helpers.is_empty()).then(|| import_line(&format!("{{ {} }}", helpers.join(", ")), crate::runtime::PACKAGE));
    let lines: Vec<String> = named
        .into_iter()
        .map(|(_, _, line)| line)
        .chain(effects)
        .chain(runtime)
        .collect();
    if !lines.is_empty() {
        code.push('\n');
        code.push_str(&lines.concat());
    }
    // What it re-exports, its `pub use` (ADR 0240), after what it imports.
    if !module.reexports.is_empty() {
        code.push('\n');
        for reexport in &module.reexports {
            let named: Vec<_> = (reexport.named.iter())
                .map(|(export, alias)| match export == alias {
                    true => export.clone(),
                    false => format!("{export} as {alias}"),
                })
                .collect();
            code.push_str(&format!(
                "export {{ {} }} from {:?};\n",
                named.join(", "),
                reexport.from
            ));
        }
    }
    code.push('\n');
    code
}

struct Cx<'a> {
    b: AstBuilder<'a>,
    allocator: &'a Allocator,
    /// The program's text, where a number is written as it was (ADR 0627).
    source: &'a str,
    /// Where a statement a comment is printed before starts.
    commented: std::collections::HashSet<u32>,
}

#[cfg(test)]
mod tests {
    // A module or declarations rust-js wrote that don't parse are its bug:
    // which, and the line, so the bug can be found (chrono's `enum Void {}`
    // was `type Void = ;`).
    #[test]
    fn unreadable_output_says_where() {
        let error = super::module_specifiers("export const a = 1;\ntype Void = ;\n", true).unwrap_err();
        assert!(error.contains("the declarations it wrote"), "{error}");
        assert!(error.contains("at line 2: `type Void = ;`"), "{error}");
    }
}
