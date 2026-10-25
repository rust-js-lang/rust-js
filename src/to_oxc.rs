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
    ObjectExpression, ObjectPropertyKind, Program, PropertyKey, PropertyKind, SimpleAssignmentTarget, Statement,
    StaticMemberExpression, StringLiteral, SwitchCase, TemplateElement, TemplateElementValue, VariableDeclarationKind,
    VariableDeclarator,
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
    let (text, comments, trailing, arms) = comments_of(module, sources);
    let cx = Cx {
        b: AstBuilder::new(&allocator),
        allocator: &allocator,
        source: rust_source,
        // A blank line's piece is no comment (`comments_of`).
        commented: (comments.iter())
            .filter(|c| !c.span.is_empty())
            .map(|c| c.attached_to)
            .collect(),
        across_lines: Default::default(),
    };
    let b = &cx.b;

    let mut body = ArenaVec::new_in(b);
    let mut items = module.items.as_slice();
    while let [item, rest @ ..] = items {
        match item {
            js::Item::Namespace(n) => match &n.class {
                Some(class) => body.extend(cx.class(n, class)),
                None => body.push(cx.namespace(n)),
            },
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
    let objects = across_lines(&program, &cx.across_lines.borrow());

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
    // A conditional's branch's comments, before its value on lines of their
    // own, `? // In modern browsers ..`, as oxc writes none before the
    // first (ADR 0658): its line split where its value's JS starts.
    let mut splits: std::collections::HashMap<u32, Vec<(u32, &[String])>> = std::collections::HashMap::new();
    if let Some(map) = &generated.map {
        // Where each line of the Rust starts, to read a token's place in it.
        let line_starts: Vec<u32> = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i as u32 + 1))
            .collect();
        let offset = |line: u32, column: u32| line_starts.get(line as usize).map(|start| start + column);
        for (lo, hi, texts) in &arms {
            // The branch's first JS the map places, and back from it, on its
            // line, to the `?`, `:` or `+` it's after: an arrow's `(` isn't placed.
            let first = (map.get_tokens())
                .filter(|t| offset(t.get_src_line(), t.get_src_col()).is_some_and(|at| *lo <= at && at < *hi))
                .map(|t| (t.get_dst_line(), t.get_dst_col()))
                .min();
            let Some((line, column)) = first else { continue };
            // On its line, or one before it, an arrow's body's first JS
            // being the first placed.
            let operator = (0..=line).rev().take(3).find_map(|at| {
                let written = generated_lines.get(at as usize)?;
                let before = if at == line {
                    written.get(..column as usize)?
                } else {
                    written
                };
                before
                    .rfind(['?', ':', '+'])
                    .map(|operator| (at, written, operator + 1))
            });
            if let Some((line, written, after)) = operator {
                let start = after + written[after..].len() - written[after..].trim_start().len();
                splits.entry(line).or_default().push((start as u32, texts.as_slice()));
            }
        }
    }
    for list in splits.values_mut() {
        list.sort_by_key(|&(column, _)| column);
    }
    // A line comment after code of its line that JS has before the code of
    // a line after it, `? "…" // ellipsis` of a conditional on one line:
    // after the code right before it, the rest of the JS's line on the next
    // (ADR 0733), so it's that code's, not all of it after the line's end.
    let mut ends: std::collections::HashMap<u32, Vec<(u32, &str)>> = std::collections::HashMap::new();
    if let Some(map) = &generated.map {
        for comment in &trailing {
            // The nearest before it whose JS is a name, a number or a string.
            let before = {
                let mut tokens: Vec<_> = (map.get_tokens())
                    .filter(|t| t.get_src_line() == comment.line && t.get_src_col() < comment.column)
                    .collect();
                tokens.sort_by_key(|t| std::cmp::Reverse(t.get_src_col()));
                tokens
                    .into_iter()
                    .map(|t| (t.get_dst_line(), t.get_dst_col()))
                    .find(|&(line, column)| {
                        (generated_lines.get(line as usize)).is_some_and(|text| word_end(text, column).is_some())
                    })
            };
            // Where its JS ends, of a line comment, when a later line's JS
            // follows it on its line: before that, and the `(`, `?`, `:` or
            // `,` it's after.
            let end = before.filter(|_| !comment.inside).and_then(|(line, column)| {
                let text = generated_lines.get(line as usize)?;
                let later = (map.get_tokens())
                    .filter(|t| t.get_dst_line() == line && t.get_dst_col() > column && t.get_src_line() > comment.line)
                    .map(|t| t.get_dst_col())
                    .min()?;
                let head = &text[..in_bytes(text, later)];
                let cut = head
                    .trim_end_matches(|c: char| c.is_whitespace() || "([{?:,".contains(c))
                    .len();
                (cut > in_bytes(text, column)).then_some((line, cut as u32))
            });
            match (before, end) {
                (Some((line, column)), _) if comment.inside => {
                    inside.entry(line).or_default().push((column, &comment.text))
                }
                (_, Some((line, end))) => ends.entry(line).or_default().push((end, &comment.text)),
                _ => {
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
    // Where each split line's pieces are: each from its column, on its line.
    let mut pieces: std::collections::HashMap<u32, Vec<(u32, u32)>> = std::collections::HashMap::new();
    let mut added = 0;
    for (i, line) in generated.code.lines().enumerate() {
        let i = i as u32;
        let out_line = header_lines + i + added;
        let mut at = 0;
        let mut placed = vec![(0, out_line)];
        let inside_of = |from: u32, to: u32| {
            let comments: Vec<(u32, &str)> = (inside.get(&i).into_iter().flatten())
                .filter(|&&(column, _)| column >= from && column < to)
                .map(|&(column, text)| (column - from, text))
                .collect();
            comments
        };
        // Where it's split: before a branch's comments, on lines of their
        // own, or after a line comment, which ends its line.
        let mut cuts: Vec<(u32, &[String], Option<&str>)> = (splits.get(&i).into_iter().flatten())
            .map(|&(column, texts)| (column, texts, None))
            .chain((ends.get(&i).into_iter().flatten()).map(|&(column, text)| (column, &[][..], Some(text))))
            .collect();
        cuts.sort_by_key(|&(column, ..)| column);
        for (column, texts, end) in cuts {
            let Some(head) = line.get(at as usize..column as usize).filter(|_| column > at) else {
                continue;
            };
            code.push_str(&with_inside_comments(head.trim_end(), &inside_of(at, column)));
            if let Some(end) = end {
                code.push(' ');
                code.push_str(end);
            }
            for text in texts {
                code.push('\n');
                code.push_str(text);
            }
            code.push('\n');
            added += 1 + texts.len() as u32;
            at = column;
            placed.push((column, header_lines + i + added));
        }
        code.push_str(&with_inside_comments(&line[at as usize..], &inside_of(at, u32::MAX)));
        for comment in after.get(&i).into_iter().flatten() {
            code.push(' ');
            code.push_str(comment);
        }
        code.push('\n');
        lines.push(out_line);
        if placed.len() > 1 {
            pieces.insert(i, placed);
        }
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
    let last_shift = lines.last().map_or(0, |&last| last + 1 - lines.len() as u32);
    let to = |line: u32, column: u32| match pieces.get(&line) {
        Some(placed) => {
            let &(from, to) = placed
                .iter()
                .rev()
                .find(|&&(from, _)| column >= from)
                .expect("the first is at 0");
            (to, column - from)
        }
        None => (lines.get(line as usize).copied().unwrap_or(line + last_shift), column),
    };
    let map = crate::format::moved_positions(&map, to, js_file_name);
    // Laid out as oxfmt lays it out, rust-js's own layout.
    let (code, map) = crate::format::laid_out(code, map, js_file_name, &objects);
    Output { code, map }
}

/// A comment after code on its line (ADR 0444): its line, its column,
/// whether code follows it there, inside an expression (ADR 0630), and its
/// text.
struct Trailing {
    line: u32,
    column: u32,
    inside: bool,
    text: String,
}

/// Where a map's `column` of `line`, in UTF-16 units, is in its bytes.
fn in_bytes(line: &str, column: u32) -> usize {
    let mut units = 0;
    for (at, c) in line.char_indices() {
        if units >= column {
            return at;
        }
        units += c.len_utf16() as u32;
    }
    line.len()
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
type ArmComments = Vec<(u32, u32, Vec<String>)>;

fn comments_of(
    module: &Module,
    sources: &crate::program::Sources,
) -> (String, Vec<oxc_ast::Comment>, Vec<Trailing>, ArmComments) {
    use oxc_ast::{CommentKind, CommentPosition};
    let mut text = sources.text.clone();
    let mut found = Vec::new();
    let mut trailing = Vec::new();
    let mut arm_comments = Vec::new();
    let (spans, comment_ends) = statement_spans(module);
    // Where each class's struct is, and its constructor function, which
    // what's before the struct is before.
    let classes: std::collections::HashMap<u32, u32> = (module.items.iter())
        .filter_map(|item| match item {
            js::Item::Namespace(n) => declared_class(n),
            _ => None,
        })
        .map(|(declared, constructor)| (declared.lo, constructor.lo))
        .collect();
    let noinline = noinline_starts(module);
    // A conditional's branch is a start too, its value's, that a comment
    // before it in Rust is before (ADR 0658).
    let arms = arm_starts(module);
    // And a call's argument, and an object literal's field, whose comments
    // oxc writes before it, on lines of their own (ADR 0708).
    let parts = part_starts(module);
    let mut starts: Vec<u32> = spans.iter().map(|&(lo, _)| lo).chain(arms.keys().copied()).collect();
    // What's first on its line is of these alone.
    let mut statement_starts = starts.clone();
    statement_starts.sort_unstable();
    starts.extend(parts.keys().copied());
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
                trailing.push(Trailing {
                    line,
                    column: (c.lo as usize - line_start) as u32,
                    inside,
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
                // Not a part of what starts after it, a call the comment
                // is before, not inside: `// c\n f(x);`'s is the statement's.
                while let Some(at) = starts.get(next)
                    && parts.get(at).is_some_and(|&whole| whole > c.lo)
                    && !spans.iter().any(|&(lo, _)| lo == *at)
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
                        // whose code is (ADR 0621); and an `unsafe` block's,
                        // which has no JS but its code's (ADR 0444, amended).
                        let opens = (between.match_indices('{')).all(|(i, _)| {
                            let head = between[..i].trim_end();
                            head.ends_with('|') || macro_opening(&between[..=i]) || unsafe_opening(head)
                        });
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
        // The constructors a class's struct's comments are before.
        let mut documented = std::collections::HashSet::new();
        for (at, own) in starts_and_end {
            // A branch's, its text as written, each a line of its own.
            if let Some(&hi) = arms.get(&at)
                && !spans.iter().any(|&(lo, _)| lo == at)
            {
                if !own.is_empty() {
                    let texts = own
                        .iter()
                        .map(|c| sources.text[c.lo as usize..c.hi as usize].to_string());
                    arm_comments.push((at, hi, texts.collect()));
                }
                continue;
            }
            // A part's, each as written, no blank line: Prettier keeps none
            // between a call's arguments, nor before an object's first field.
            let part = parts.contains_key(&at) && !spans.iter().any(|&(lo, _)| lo == at);
            let mut pieces: Vec<(u32, u32)> = Vec::new();
            // A function kept out of line, `#[inline(never)]`, says so in its
            // JSDoc, the last line of its own, or one of its own (ADR 0649).
            let noinline = noinline.contains(&at);
            let mut said = false;
            let mut i = 0;
            while i < own.len() {
                let comment = own[i];
                if blank_before(comment.lo) && !part {
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
            let first_on_line = !statement_starts.iter().any(|&s| s >= line_start && s < at);
            if part {
            } else if (own.is_empty() && first_on_line && blank_before(at))
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
            // A class's struct's comments, before its constructor: the
            // blank line before them is the function's, not one between
            // them and it, before `new`.
            match classes.get(&at) {
                Some(&constructor) if !own.is_empty() => {
                    documented.insert(constructor);
                }
                _ if documented.contains(&at) => {
                    let blanks = pieces.iter().take_while(|&&(lo, hi)| lo == hi).count();
                    pieces.drain(..blanks);
                }
                _ => {}
            }
            for (lo, hi) in pieces {
                let written = &text[lo as usize..hi as usize];
                let kind = match (written.starts_with("/*"), written.contains('\n')) {
                    (false, _) => CommentKind::Line,
                    (true, false) => CommentKind::SingleLineBlock,
                    (true, true) => CommentKind::MultiLineBlock,
                };
                let mut made = oxc_ast::Comment::new(lo, hi, kind);
                made.attached_to = classes.get(&at).copied().unwrap_or_else(|| printed_first(at));
                // A block's of only comments, where it ends: no node starts
                // there, so oxc prints it as it does a comment whose node is
                // gone, before what follows or the block's `}` (ADR 0753).
                if comment_ends.contains(&at) {
                    made.content = oxc_ast::CommentContent::Legal;
                }
                made.position = CommentPosition::Leading;
                made.set_preceded_by_newline(true);
                made.set_followed_by_newline(true);
                found.push(made);
            }
        }
    }
    (text, found, trailing, arm_comments)
}

/// Where each conditional's branch is, `c ? a : b`'s `a` and `b`, of those
/// that say, by where it starts: what a comment before one is before (ADR
/// 0658); and each of a `concat!`'s pieces past its first, after its `+`
/// (ADR 0641).
fn arm_starts(module: &Module) -> std::collections::HashMap<u32, u32> {
    let mut starts = std::collections::HashMap::new();
    let mut of = |mut stmts: Vec<js::Stmt>| {
        js::each_expr_mut(&mut stmts, &mut |e| {
            let spans: Vec<js::Span> = match &e.kind {
                ExprKind::Cond(_, yes, no) => vec![yes.span, no.span],
                ExprKind::Written(pieces) => pieces.iter().skip(1).map(|piece| piece.span).collect(),
                _ => Vec::new(),
            };
            starts.extend(spans.into_iter().filter(|s| !s.is_none()).map(|s| (s.lo, s.hi)));
        });
    };
    for item in &module.items {
        match item {
            js::Item::Const(c) => of(vec![StmtKind::Expr(c.value.clone()).at(js::Span::NONE)]),
            js::Item::Statements(stmts) => of(stmts.clone()),
            js::Item::Function(f) => of(f.body.clone()),
            js::Item::Namespace(n) => n.methods.iter().for_each(|method| of(method.body.clone())),
        }
    }
    starts
}

/// Where each call's and `new`'s argument starts, and each object literal's
/// field's value, of those that say, and where what they're parts of
/// starts: what a comment before one in the Rust, inside what it's part
/// of, is before in the JS, as oxc writes it (ADR 0708).
fn part_starts(module: &Module) -> std::collections::HashMap<u32, u32> {
    let mut starts = std::collections::HashMap::new();
    let mut of = |mut stmts: Vec<js::Stmt>| {
        js::each_expr_mut(&mut stmts, &mut |e| {
            let spans: Vec<js::Span> = match &e.kind {
                ExprKind::Call(_, args) | ExprKind::New(_, args) => args.iter().map(|arg| arg.span).collect(),
                ExprKind::Object(props, ..) => (props.iter())
                    .filter_map(|prop| match prop {
                        js::Prop::Field(_, value) => Some(value.span),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            if !e.span.is_none() {
                starts.extend(spans.into_iter().filter(|s| !s.is_none()).map(|s| (s.lo, e.span.lo)));
            }
        });
    };
    for item in &module.items {
        match item {
            js::Item::Const(c) => of(vec![StmtKind::Expr(c.value.clone()).at(js::Span::NONE)]),
            js::Item::Statements(stmts) => of(stmts.clone()),
            js::Item::Function(f) => of(f.body.clone()),
            js::Item::Namespace(n) => n.methods.iter().for_each(|method| of(method.body.clone())),
        }
    }
    starts
}

/// Whether `head`, the code before a `{`, ends in `unsafe`, the keyword,
/// whose block is its code alone in JS.
fn unsafe_opening(head: &str) -> bool {
    head.strip_suffix("unsafe")
        .is_some_and(|before| !before.ends_with(|c: char| c.is_alphanumeric() || c == '_'))
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
/// attached to (ADR 0444); and where each block of only comments ends,
/// `StmtKind::Comments`, whose comments oxc prints there (ADR 0753).
fn statement_spans(module: &Module) -> (Vec<(u32, u32)>, std::collections::HashSet<u32>) {
    type Found = (Vec<(u32, u32)>, std::collections::HashSet<u32>);
    fn list(list: &[js::Stmt], (starts, ends): &mut Found) {
        starts.extend(list_spans(list));
        let comments = list
            .iter()
            .filter(|s| matches!(s.kind, StmtKind::Comments) && !s.span.is_none());
        ends.extend(comments.map(|s| s.span.lo));
    }
    fn of_stmts(stmts: &[js::Stmt], starts: &mut Found) {
        let mut copy = stmts.to_vec();
        let mut functions = Vec::new();
        js::statement_lists(&mut copy, &mut |stmts| {
            list(stmts, starts);
            for stmt in stmts.iter() {
                if let StmtKind::Function(f) = &stmt.kind {
                    functions.push(f.body.clone());
                }
            }
        });
        js::each_expr_mut(&mut copy, &mut |e| match &mut e.kind {
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => {
                js::statement_lists(body, &mut |stmts| list(stmts, starts));
            }
            ExprKind::Function(f) => functions.push(f.body.clone()),
            _ => {}
        });
        for body in functions {
            of_stmts(&body, starts);
        }
    }
    let mut starts: Found = (Vec::new(), Default::default());
    let function = |f: &js::Function, starts: &mut Found| {
        if !f.span.is_none() {
            starts.0.push((f.span.lo, f.span.hi));
        }
        of_stmts(&f.body, starts);
    };
    for item in &module.items {
        match item {
            js::Item::Const(c) => {
                if !c.span.is_none() {
                    starts.0.push((c.span.lo, c.span.hi));
                }
                of_stmts(&[StmtKind::Expr(c.value.clone()).at(js::Span::NONE)], &mut starts);
            }
            js::Item::Function(f) => function(f, &mut starts),
            js::Item::Namespace(n) => {
                for method in &n.methods {
                    function(method, &mut starts);
                }
                // A class's struct, whose comments are its constructor's.
                if let Some((declared, _)) = declared_class(n) {
                    starts.0.push((declared.lo, declared.hi));
                }
            }
            js::Item::Statements(stmts) => of_stmts(stmts, &mut starts),
        }
    }
    starts
}

/// A class's struct, where it's declared, and its constructor function,
/// which the comments before the struct, its `///` doc, are before (ADR
/// 0761): `/** Base class helpers .. */ function Component(..)`.
fn declared_class(n: &js::Namespace) -> Option<(js::Span, js::Span)> {
    let class = n.class.as_ref().filter(|class| !class.declared.is_none())?;
    let constructor = n.methods.iter().find(|f| f.name == class.constructor)?;
    Some((class.declared, constructor.span))
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
    /// The objects made of those the source writes across lines, by where
    /// each is in the arena (ADR 0835): which a visit of the program tells
    /// the formatter of, in its order (`across_lines`).
    across_lines: std::cell::RefCell<std::collections::HashSet<*const ObjectExpression<'a>>>,
}

/// Of each object of `program`, in the order a visit meets them, as its
/// JS's parse does, whether it's one of `across_lines`, which the source
/// writes across lines (ADR 0835).
fn across_lines<'a>(
    program: &Program<'a>,
    across_lines: &std::collections::HashSet<*const ObjectExpression<'a>>,
) -> Vec<bool> {
    struct Objects<'s, 'a> {
        across_lines: &'s std::collections::HashSet<*const ObjectExpression<'a>>,
        found: Vec<bool>,
    }
    impl<'a> Visit<'a> for Objects<'_, 'a> {
        fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
            self.found.push(self.across_lines.contains(&(it as *const _)));
            oxc_ast_visit::walk::walk_object_expression(self, it);
        }
    }
    let mut objects = Objects {
        across_lines,
        found: Vec::new(),
    };
    objects.visit_program(program);
    objects.found
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
