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
mod trailing;

use self::trailing::{Inside, Trailing, with_inside_comments};

pub(crate) use self::expressions::*;

use std::path::PathBuf;

use oxc_allocator::{Allocator, ArenaBox, ArenaVec};
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, ArrowFunctionBody, AssignmentTarget, AssignmentTargetMaybeDefault,
    BindingIdentifier, BindingPattern, BindingProperty, BindingRestElement, BlockStatement, CallExpression,
    CatchClause, CatchParameter, ChainElement, ComputedMemberExpression, Declaration, Directive, ExportFromDeclaration,
    ExportSpecifier, Expression, ForStatementInit, ForStatementLeft, FormalParameter, FormalParameterKind,
    FormalParameterRest, FormalParameters, FunctionBody, FunctionType, IdentifierName, ImportDeclaration,
    ImportOrExportKind, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXChild, JSXClosingElement,
    JSXClosingFragment, JSXElementName, JSXExpression, JSXIdentifier, JSXMemberExpressionObject, JSXOpeningElement,
    JSXOpeningFragment, LabelIdentifier, ModuleExportName, ObjectExpression, ObjectPropertyKind, Program, PropertyKey,
    PropertyKind, SimpleAssignmentTarget, Statement, StaticMemberExpression, StringLiteral, SwitchCase,
    TemplateElement, TemplateElementValue, TemplateLiteral, VariableDeclarationKind, VariableDeclarator,
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
    let Commented {
        text,
        comments,
        trailing,
        arms,
        tops,
    } = comments_of(module, sources);
    let cx = Cx {
        b: AstBuilder::new(&allocator),
        allocator: &allocator,
        source: rust_source,
        across_lines: Default::default(),
        interpolations: Default::default(),
        commented_branches: arms.iter().map(|&(lo, ..)| lo).collect(),
        // Where a comment of the Rust, not a blank line's, is before what starts there.
        commented: (comments.iter())
            .filter(|c| c.span.end > c.span.start)
            .map(|c| c.attached_to)
            .collect(),
    };
    let b = &cx.b;

    let mut body = ArenaVec::new_in(b);
    // What it re-exports, its `pub use`s, first, after what it imports, each
    // where it's written, its comments before it (ADR 0240, amended).
    body.extend(module.reexports.iter().map(|reexport| cx.reexport(reexport)));
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
    // A blank line between two statements where the Rust has one between
    // their code, where it tells (ADR 0444, amended).
    let (comments, told) = apart(&body, comments, &tops, sources, text.len() as u32);
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
    let mut written = crate::format::Written {
        told,
        ..across_lines(&program, &cx)
    };

    // The header, imports and runtime helpers, plain text above the
    // generated code, none of which maps to Rust.
    let mut code = header(module);

    // The generated code, each line after the header: where each ends up,
    // to fix the map.
    let header_lines = code.matches('\n').count() as u32;
    let mut lines: Vec<u32> = Vec::new();
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
        for (lo, hi, operator, texts) in &arms {
            // The branch's first JS the map places, and back from it, on its
            // line, to the operator it's after, `?`, `:`, `+` or `&&`: an
            // arrow's `(` isn't placed.
            let first = (map.get_tokens())
                .filter(|t| offset(t.get_src_line(), t.get_src_col()).is_some_and(|at| *lo <= at && at < *hi))
                .map(|t| (t.get_dst_line(), t.get_dst_col()))
                .min();
            let Some((line, column)) = first else { continue };
            // On its line, or one before it, an arrow's body's first JS
            // being the first placed.
            let found = (0..=line).rev().take(3).find_map(|at| {
                let written = generated_lines.get(at as usize)?;
                // An `if`'s test's: its `if (` too, where the `if` is placed
                // with its test.
                let opening = match *operator {
                    "if ("
                        if written
                            .get(column as usize..)
                            .is_some_and(|rest| rest.starts_with(*operator)) =>
                    {
                        operator.len()
                    }
                    _ => 0,
                };
                let before = if at == line {
                    written.get(..column as usize + opening)?
                } else {
                    written
                };
                before
                    .rfind(operator)
                    .map(|found| (at, written, found + operator.len()))
            });
            if let Some((line, written, after)) = found {
                let start = after + written[after..].len() - written[after..].trim_start().len();
                splits.entry(line).or_default().push((start as u32, texts.as_slice()));
            }
        }
    }
    for list in splits.values_mut() {
        list.sort_by_key(|&(column, _)| column);
    }
    // Where each comment after code on its line goes in the JS's lines.
    let trailing::Placed { after, inside, ends } = match &generated.map {
        Some(map) => trailing::placed(&trailing, map, &generated_lines),
        None => trailing::Placed::default(),
    };
    // Where each split line's pieces are: each from its column, on its line.
    let mut pieces: std::collections::HashMap<u32, Vec<(u32, u32)>> = std::collections::HashMap::new();
    // Where each line has a comment inside it: the piece it's in, where, and
    // how wide, what the JS after it there is moved by (ADR 0630, amended).
    let mut widened: std::collections::HashMap<u32, Vec<(u32, u32, u32)>> = std::collections::HashMap::new();
    let mut added = 0;
    for (i, line) in generated.code.lines().enumerate() {
        let i = i as u32;
        let out_line = header_lines + i + added;
        let mut at = 0;
        let mut placed = vec![(0, out_line)];
        let inside_of = |from: u32, to: u32| {
            let comments: Vec<(u32, Inside, &str)> = (inside.get(&i).into_iter().flatten())
                .filter(|&&(column, ..)| column >= from && column < to)
                .map(|&(column, inside, text)| (column - from, inside, text))
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
            let (head, moved) = with_inside_comments(head.trim_end(), &inside_of(at, column));
            code.push_str(&head);
            widened
                .entry(i)
                .or_default()
                .extend(moved.into_iter().map(|(end, width)| (at, at + end, width)));
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
        let (rest, moved) = with_inside_comments(&line[at as usize..], &inside_of(at, u32::MAX));
        code.push_str(&rest);
        widened
            .entry(i)
            .or_default()
            .extend(moved.into_iter().map(|(end, width)| (at, at + end, width)));
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
    // After everything it maps, so the map's lines stay where they are. Each
    // is apart already, which `told` says of it: what it tells of the code's
    // statements is of the ones before (ADR 0865, amended).
    if let Some(function) = &module.default_export
        && defaulted(module).is_none()
    {
        code.push_str(&format!("\nexport default {function};\n"));
        written.told.push(true);
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
        written.told.push(true);
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
    // Moved by the comments inside its piece of its line before it.
    let wider = |line: u32, from: u32, column: u32| -> u32 {
        (widened.get(&line).into_iter().flatten())
            .filter(|&&(piece, at, _)| piece == from && at <= column)
            .map(|&(.., width)| width)
            .sum()
    };
    let to = |line: u32, column: u32| match pieces.get(&line) {
        Some(placed) => {
            let &(from, to) = placed
                .iter()
                .rev()
                .find(|&&(from, _)| column >= from)
                .expect("the first is at 0");
            (to, column - from + wider(line, from, column))
        }
        None => (
            lines.get(line as usize).copied().unwrap_or(line + last_shift),
            column + wider(line, 0, column),
        ),
    };
    let map = crate::format::moved_positions(&map, to, js_file_name);
    // Laid out as oxfmt lays it out, rust-js's own layout.
    let (code, map) = crate::format::laid_out(code, map, js_file_name, &written);
    Output { code, map }
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
type ArmComments = Vec<(u32, u32, &'static str, Vec<String>)>;

/// A module's comments, `comments_of`'s: the text they're of, the program's
/// source and each doc comment's JSDoc after it; each before what it's
/// before; each after code on its line; each branch's; and where the first
/// of each statement's starts in the Rust, by where the statement does,
/// what a blank line before it is before (`apart`).
struct Commented {
    text: String,
    comments: Vec<oxc_ast::Comment>,
    trailing: Vec<Trailing>,
    arms: ArmComments,
    tops: std::collections::HashMap<u32, u32>,
}

fn comments_of(module: &Module, sources: &crate::program::Sources) -> Commented {
    use oxc_ast::{CommentKind, CommentPosition};
    let mut text = sources.text.clone();
    let mut found = Vec::new();
    let mut tops = std::collections::HashMap::new();
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
    // Where each `if` that's an `else`'s only statement starts, and ends.
    let else_ifs = else_if_starts(module);
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
                // One right after an `=`, an initializer's or an assignment's,
                // not `==`'s, `<=`'s or `+=`'s, is before the value it gives,
                // `= /* .. */ 0b0000000` (ADR 0630, amended).
                let before_value = (sources.text[line_start..c.lo as usize].trim_end())
                    .strip_suffix('=')
                    .is_some_and(|before| !before.ends_with(|c: char| "=!<>+-*/%&|^".contains(c)));
                trailing.push(Trailing {
                    line,
                    column: (c.lo as usize - line_start) as u32,
                    inside,
                    before_value,
                    text: written.to_string(),
                });
            }
        }
        // A block comment on a line of its own between an `=` and the value
        // on the lines after it, as rustfmt writes `= /* .. */ value` too
        // long for its line, is before the value, as one on the value's line
        // is: placed as if right before the value's first code (ADR 0630,
        // amended).
        // Placed so, not before a statement too.
        let before_value =
            |c: &crate::program::Comment| own_line_before_value(&sources.text, &file.comments, c).is_some();
        for c in file.comments.iter().filter(|c| c.lo >= past) {
            if let Some((line, column)) = own_line_before_value(&sources.text, &file.comments, c) {
                trailing.push(Trailing {
                    line,
                    column,
                    inside: true,
                    before_value: true,
                    text: sources.text[c.lo as usize..c.hi as usize].to_string(),
                });
            }
        }
        // Where the `if` is, the keyword, of a comment the code before which,
        // past other comments, is it: the comment is between it and its test.
        let if_keyword = |c: &crate::program::Comment| {
            let mut end = c.lo as usize;
            loop {
                let code = sources.text[file_start..end].trim_end();
                match file
                    .comments
                    .iter()
                    .find(|other| other.hi as usize == file_start + code.len())
                {
                    Some(other) if (other.lo as usize) < end => end = other.lo as usize,
                    _ => {
                        break code
                            .strip_suffix("if")
                            .filter(|before| !before.ends_with(|c: char| c.is_alphanumeric() || c == '_'))
                            .map(|before| (file_start + before.len()) as u32);
                    }
                }
            }
        };
        let after_if = |c: &crate::program::Comment| if_keyword(c).is_some();
        let mut attached: Vec<(crate::program::Comment, u32)> = (file.comments.iter())
            .filter(|c| c.lo >= past && !after_code(c) && !before_value(c))
            .filter_map(|c| {
                // Before a match arm, `IMMEDIATE_PRIORITY => -1.0`, its body's: a start whose text to the next is only a pattern and
                // its `=>` is the arm's test, which an `else if` isn't a
                // statement of (ADR 0590).
                // Of a run of comments before it, each line's: what's between
                // is only comments (ADR 0590, amended).
                let only_comments = |lo: u32, hi: u32| {
                    let mut code = lo;
                    (file.comments.iter())
                        .filter(|other| other.lo >= lo && other.hi <= hi)
                        .chain([&crate::program::Comment { lo: hi, hi, doc: false }])
                        .all(|other| {
                            let between = sources.text.get(code as usize..other.lo as usize);
                            code = other.hi;
                            between.is_some_and(|gap| gap.trim().is_empty())
                        })
                };
                // Of an operator and what opens its operand, `+ &(`, a
                // let-chain's `&& let` (ADR 0444, amended), and comments.
                let only_operators = |lo: u32, hi: u32| {
                    let mut code = lo;
                    (file.comments.iter())
                        .filter(|other| other.lo >= lo && other.hi <= hi)
                        .chain([&crate::program::Comment { lo: hi, hi, doc: false }])
                        .all(|other| {
                            let between = sources.text.get(code as usize..other.lo as usize);
                            code = other.hi;
                            between.is_some_and(|gap| {
                                (gap.split_whitespace())
                                    .all(|word| word == "let" || word.chars().all(|c| "+-*/%&|^!=<>(".contains(c)))
                            })
                        })
                };
                let mut next = starts.partition_point(|&s| s < c.hi);
                while let [at, after, ..] = starts[next.min(starts.len())..]
                    && only_comments(c.hi, at)
                    && let Some(pattern) = sources.text.get(at as usize..after as usize)
                    && pattern.trim_end().ends_with("=>")
                    && !pattern.contains([';', '{', '}', '\n'])
                {
                    next += 1;
                }
                // Past each start it's none of, each rule asked again of the
                // next, so one past an operand isn't then a call's part it's
                // outside of (ADR 0708, amended).
                while let Some(&at) = starts.get(next)
                    && !spans.iter().any(|&(lo, _)| lo == at)
                    && (
                        // Not a part of what starts after it, a call the
                        // comment is before, not inside: `// c\n f(x);`'s is
                        // the statement's.
                        parts.get(&at).is_some_and(|&(whole, _)| whole > c.lo)
                        // A binary's right operand's only where only its
                        // operator is between, as rustfmt writes it first,
                        // `// c\n + &(x)`: not a match arm's, `None => given`
                        // of one read as `n ?? given` (ADR 0444, amended).
                        || arms.get(&at).is_some_and(|arm| arm.operand) && !only_operators(c.hi, at)
                        // An `if`'s test's only where it's written after the
                        // `if`, `if\n // c\n a || b {`.
                        || arms.get(&at).is_some_and(|arm| arm.opening) && !after_if(c)
                    )
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
                        // Past `let`s with no JS of their own, a variable read
                        // in place, `let timer = Rc::clone(timer);` (ADR
                        // 0603), a block's too, `let arg = unsafe {
                        // js::cast(arg) };`, in the same block (ADR 0444,
                        // amended).
                        let (lets, rest) = leading_statements(between);
                        // A closure's opening, `with_borrow_mut(|queue| {`,
                        // whose body is in its place (ADR 0581), is a start;
                        // and a macro's on its own line, `thread_local! {`,
                        // whose code is (ADR 0621); and an `unsafe` block's,
                        // which has no JS but its code's (ADR 0444, amended).
                        let opens = (rest.match_indices('{')).all(|(i, _)| {
                            let head = rest[..i].trim_end();
                            head.ends_with('|') || macro_opening(&rest[..=i]) || unsafe_opening(head)
                        });
                        lets.iter().all(|s| s.starts_with("let ")) && !rest.contains([';', '}']) && opens
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
            // Of a method call rustfmt writes on the line after its receiver,
            // `root.lanes\n    .set(..)`, whose span starts at `.set`: the
            // receiver's line, where the statement starts (ADR 0444, amended).
            while sources.text[file_start + before.len()..lo as usize]
                .trim_start()
                .starts_with('.')
                && let Some(body) = before.strip_suffix('\n')
            {
                before = &before[..body.rfind('\n').map_or(0, |i| i + 1)];
            }
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
            // And past `let`s with no JS of their own, as a comment is, an
            // erased cast's, `let f: &dyn Fn() = unsafe { js::cast(f) };`
            // (ADR 0865): only they, which no statement starts in, are after
            // a blank line.
            let past_lets = before.trim_end().rfind("\n\n").is_some_and(|blank| {
                let (lets, rest) = leading_statements(&before[blank..]);
                let at = |offset: usize| (file_start + offset) as u32;
                let printed = spans.iter().any(|&(s, _)| at(blank) <= s && s < at(before.len()));
                !lets.is_empty() && lets.iter().all(|s| s.starts_with("let ")) && rest.trim().is_empty() && !printed
            });
            gap.matches('\n').count() >= 2 || past_lets
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
        // One inside a statement that's before nothing printed of it, a
        // match arm's of a `match` read as `prev?.types`: before the
        // statement it's in, the innermost, never lost (ADR 0444, amended);
        // not a module's item, a function's, far above where it's written.
        let items: std::collections::HashSet<(u32, u32)> = (module.items.iter())
            .flat_map(|item| match item {
                js::Item::Function(f) => vec![f.span],
                js::Item::Const(c) => vec![c.span],
                js::Item::Namespace(n) => n.methods.iter().map(|m| m.span).collect(),
                js::Item::Statements(_) => Vec::new(),
            })
            .map(|span| (span.lo, span.hi))
            .collect();
        let unattached: Vec<(crate::program::Comment, u32)> = (file.comments.iter())
            .filter(|c| c.lo >= past && !after_code(c) && !before_value(c) && !ends.iter().any(|e| e.lo == c.lo))
            .filter(|c| !attached.iter().any(|(a, _)| a.lo == c.lo))
            .filter_map(|c| {
                let (lo, _) = (spans.iter())
                    .filter(|&&(lo, hi)| lo < c.lo && c.hi <= hi && in_file(lo) && !items.contains(&(lo, hi)))
                    .min_by_key(|&&(lo, hi)| hi - lo)?;
                Some((*c, *lo))
            })
            .collect();
        attached.extend(unattached);
        let mut starts_and_end: Vec<(u32, Vec<crate::program::Comment>)> = (starts.iter())
            .filter(|&&at| in_file(at))
            .map(|&at| (at, attached.iter().filter(|(_, a)| *a == at).map(|(c, _)| *c).collect()))
            .collect();
        if !ends.is_empty() {
            starts_and_end.push((sources.text.len() as u32, ends));
        }
        // The constructors a class's struct's comments are before.
        let mut documented = std::collections::HashSet::new();
        for (at, mut own) in starts_and_end {
            // Where its Rust starts, what a blank line is before: an `if`'s
            // keyword, before the comments its test's are (ADR 0883).
            let mut written = at;
            // An `if`'s test's, those written after its `if`, after its `if
            // (`; the rest its statement's.
            if let Some(&Arm {
                hi,
                operator,
                opening: true,
                ..
            }) = arms.get(&at)
            {
                let (inside, before): (Vec<_>, Vec<_>) = own.into_iter().partition(|c| after_if(c));
                if let Some(keyword) = inside.first().and_then(if_keyword) {
                    written = keyword;
                }
                if !inside.is_empty() {
                    let texts = inside
                        .iter()
                        .map(|c| sources.text[c.lo as usize..c.hi as usize].to_string());
                    arm_comments.push((at, hi, operator, texts.collect()));
                }
                own = before;
                if !spans.iter().any(|&(lo, _)| lo == at) {
                    continue;
                }
            }
            // An `else if`'s, written before its `else`, `}\n// c\nelse if`:
            // before the `else`, after the `}`, on lines of their own, as
            // Prettier keeps them, the `else if` one still; those written in
            // an `else`'s block before its only `if` keep the block, where
            // they are (`Cx::commented`, ADR 0444, amended).
            if let Some(&hi) = else_ifs.get(&at)
                && let Some(last) = own.last()
                && let Some(rest) = sources.text[last.hi as usize..at as usize]
                    .trim_start()
                    .strip_prefix("else")
                && rest.starts_with(char::is_whitespace)
                && !rest.contains(['{', '}', ';'])
            {
                let texts = own
                    .iter()
                    .map(|c| sources.text[c.lo as usize..c.hi as usize].to_string());
                arm_comments.push((at, hi, "}", texts.collect()));
                continue;
            }
            // A branch's, its text as written, each a line of its own.
            if let Some(&Arm { hi, operator, .. }) = arms.get(&at)
                && !spans.iter().any(|&(lo, _)| lo == at)
            {
                if !own.is_empty() {
                    let texts = own
                        .iter()
                        .map(|c| sources.text[c.lo as usize..c.hi as usize].to_string());
                    arm_comments.push((at, hi, operator, texts.collect()));
                }
                continue;
            }
            // A part's, each as written, no blank line before a call's
            // argument, which Prettier keeps none between; an object's field
            // has one where its struct literal's has (ADR 0444, amended), as
            // Prettier keeps between properties, none before the first.
            let part = (parts.get(&at)).is_some_and(|&(_, field)| !field) && !spans.iter().any(|&(lo, _)| lo == at);
            let mut pieces: Vec<(u32, u32)> = Vec::new();
            // A function kept out of line, `#[inline(never)]`, says so in its
            // JSDoc, the last line of its own, or one of its own (ADR 0649).
            let noinline = noinline.contains(&at);
            let mut said = false;
            let mut i = 0;
            while i < own.len() {
                let comment = own[i];
                // One after an `if`, its test's that the test's statement
                // took, `return c ? a : b`'s: before the keyword.
                if blank_before(if_keyword(&comment).unwrap_or(comment.lo)) && !part {
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
            } else if (own.is_empty() && first_on_line && blank_before(written))
                || (!own.is_empty() && last <= at && {
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
            let attached_to = classes.get(&at).copied().unwrap_or_else(|| printed_first(at));
            if let Some(first) = own.first() {
                let top = tops.entry(attached_to).or_insert(first.lo);
                *top = (*top).min(first.lo);
            }
            for (lo, hi) in pieces {
                let written = &text[lo as usize..hi as usize];
                let kind = match (written.starts_with("/*"), written.contains('\n')) {
                    (false, _) => CommentKind::Line,
                    (true, false) => CommentKind::SingleLineBlock,
                    (true, true) => CommentKind::MultiLineBlock,
                };
                let mut made = oxc_ast::Comment::new(lo, hi, kind);
                made.attached_to = attached_to;
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
    // Before each case a case falls through to, `// Fallthrough` (ADR 0905).
    for at in fallthrough_starts(module) {
        let lo = text.len() as u32;
        text.push_str(FALLTHROUGH);
        let mut made = oxc_ast::Comment::new(lo, text.len() as u32, CommentKind::Line);
        made.attached_to = at;
        made.position = CommentPosition::Leading;
        made.set_preceded_by_newline(true);
        made.set_followed_by_newline(true);
        found.push(made);
    }
    Commented {
        text,
        comments: found,
        trailing,
        arms: arm_comments,
        tops,
    }
}

/// `comments`, each of `body`'s statements but the first with a blank line
/// before it where the Rust has one between the code of the one before
/// and its own, and its comments, `tops` (`comments_of`), and none where
/// it has none (ADR 0444, amended); and which the Rust told so, of two in
/// one file, the code of the first ending before the second's starts,
/// which `format::spaced` leaves as they are. Of the rest, a statement
/// rust-js makes of no code of its own, or of another's, `spaced` sets
/// them apart as a person does (ADR 0303). `end` is where the text of the
/// comments ends, where a blank line's empty one is.
fn apart(
    body: &[Statement<'_>],
    mut comments: Vec<oxc_ast::Comment>,
    tops: &std::collections::HashMap<u32, u32>,
    sources: &crate::program::Sources,
    end: u32,
) -> (Vec<oxc_ast::Comment>, Vec<bool>) {
    use oxc_span::GetSpan;
    // Where each statement's code ends in the Rust: the last its nodes are.
    struct Reach {
        end: u32,
        limit: u32,
    }
    impl<'a> Visit<'a> for Reach {
        fn enter_node(&mut self, kind: oxc_ast::AstKind<'a>) {
            let span = kind.span();
            if span.start < span.end && span.end <= self.limit {
                self.end = self.end.max(span.end);
            }
        }
    }
    let limit = sources.text.len() as u32;
    let ends: Vec<u32> = (body.iter())
        .map(|statement| {
            let mut reach = Reach { end: 0, limit };
            reach.visit_statement(statement);
            reach.end
        })
        .collect();
    // The file each place is in, by where each starts.
    let mut files = Vec::new();
    let mut start = 0;
    for file in &sources.files {
        files.push((start, start + file.text.len() as u32));
        start += file.text.len() as u32 + 1;
    }
    let file_of = |at: u32| files.iter().position(|&(lo, hi)| lo <= at && at <= hi);
    let mut told = vec![false; body.len()];
    for i in 1..body.len() {
        let (code_end, start) = (ends[i - 1], body[i].span().start);
        let top = tops.get(&start).map_or(start, |&first| first.min(start));
        if code_end == 0
            || start == 0
            || code_end > top
            || file_of(code_end).is_none()
            || file_of(code_end) != file_of(top)
        {
            continue;
        }
        // Of one line, `#[derive(Debug, Serialize)]`'s functions, the Rust
        // tells nothing of lines between them.
        let lines: Vec<&str> = sources.text[code_end as usize..top as usize].split('\n').collect();
        if lines.len() < 2 {
            continue;
        }
        told[i] = true;
        let blank = lines.len() > 2 && lines[1..lines.len() - 1].iter().any(|line| line.trim().is_empty());
        // Its pieces' first, a blank line if it's one, before its comments:
        // one `comments_of` saw right before it, or else one written here,
        // where what's between has no JS, a type, an `impl`'s opening.
        let first = comments.iter().position(|c| c.attached_to == start);
        let had = first.is_some_and(|at| comments[at].span.is_empty());
        if blank && !had {
            let mut made = oxc_ast::Comment::new(end, end, oxc_ast::CommentKind::Line);
            made.attached_to = start;
            made.position = oxc_ast::CommentPosition::Leading;
            made.set_preceded_by_newline(true);
            made.set_followed_by_newline(true);
            comments.insert(first.unwrap_or(comments.len()), made);
        }
    }
    (comments, told)
}

/// Where each conditional's branch is, `c ? a : b`'s `a` and `b`, of those
/// that say, by where it starts: what a comment before one is before (ADR
/// 0658); each of a `concat!`'s pieces past its first, after its `+`
/// (ADR 0641); and a binary's right operand, after its operator, as a
/// comment before it is, between the two or in the closure it's of,
/// `is_some_and(|v| { // c\n v > 3 })` (ADR 0444, amended).
fn arm_starts(module: &Module) -> std::collections::HashMap<u32, Arm> {
    let mut starts = std::collections::HashMap::new();
    let mut of = |mut stmts: Vec<js::Stmt>| {
        js::each_expr_mut(&mut stmts, &mut |e| {
            let spans: Vec<(js::Span, &'static str, bool)> = match &e.kind {
                ExprKind::Cond(_, yes, no) => vec![(yes.span, "?", false), (no.span, ":", false)],
                // An arrow's value, after its `=>`, as Prettier writes a
                // comment before it (ADR 0883).
                ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => expressions::arrow_value(body)
                    .map(|value| (value.span, "=>", false))
                    .into_iter()
                    .collect(),
                ExprKind::Written(pieces) => pieces.iter().skip(1).map(|piece| (piece.span, "+", false)).collect(),
                // One placed nowhere, an `&&` tidy made of branches, `|| (a
                // && b)`'s, from where its parts are (ADR 0444, amended).
                ExprKind::Binary(op, _, r) => {
                    let span = match r.span.is_none() {
                        true => written_extent(r).map_or(js::Span::NONE, |(lo, hi)| js::Span { lo, hi }),
                        false => r.span,
                    };
                    vec![(span, expressions::operator(*op), true)]
                }
                _ => Vec::new(),
            };
            let arms = (spans.into_iter().filter(|(s, ..)| !s.is_none())).map(|(s, operator, operand)| {
                let arm = Arm {
                    hi: s.hi,
                    operator,
                    operand,
                    opening: false,
                };
                (s.lo, arm)
            });
            starts.extend(arms);
        });
        // An `if`'s test, after its `if (`, which a comment between `if`
        // and its first operand is before (`opening`).
        js::each_block_mut(&mut stmts, &mut |block| {
            for stmt in block.iter() {
                if let StmtKind::If(test, ..) = &stmt.kind
                    && let Some((lo, hi)) = written_extent(test)
                {
                    let arm = Arm {
                        hi,
                        operator: "if (",
                        operand: false,
                        opening: true,
                    };
                    starts.entry(lo).or_insert(arm);
                }
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

/// Where each `if` that's an `else`'s only statement starts, by where it
/// ends: what's printed `else if` where no comment is before it in the
/// block (ADR 0444, amended).
fn else_if_starts(module: &Module) -> std::collections::HashMap<u32, u32> {
    let mut starts = std::collections::HashMap::new();
    let mut of = |mut stmts: Vec<js::Stmt>| {
        js::each_block_mut(&mut stmts, &mut |block| {
            for stmt in block.iter() {
                if let StmtKind::If(_, _, Some(els)) = &stmt.kind
                    && let [only] = els.as_slice()
                    && matches!(only.kind, StmtKind::If(..))
                    && !only.span.is_none()
                {
                    starts.insert(only.span.lo, only.span.hi);
                }
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

/// Where the Rust of `e` is, from the first of its parts placed to the end
/// of the last: an `||` tidy made of guards is placed nowhere, its
/// operands where they're written.
fn written_extent(e: &js::Expr) -> Option<(u32, u32)> {
    let mut extent: Option<(u32, u32)> = None;
    e.clone().each_mut(&mut |part| {
        if !part.span.is_none() {
            let (lo, hi) = extent.unwrap_or((part.span.lo, part.span.hi));
            extent = Some((lo.min(part.span.lo), hi.max(part.span.hi)));
        }
    });
    extent
}

/// A start `arm_starts` found: where it ends, the operator it's after in
/// the JS, whether it's a binary's operand, and whether it's an `if`'s
/// test, whose comments are only those written after its `if`.
#[derive(Clone, Copy)]
struct Arm {
    hi: u32,
    operator: &'static str,
    operand: bool,
    opening: bool,
}

/// Where each call's and `new`'s argument starts, and each object literal's
/// field's value, of those that say, and where what they're parts of
/// starts: what a comment before one in the Rust, inside what it's part
/// of, is before in the JS, as oxc writes it (ADR 0708); and whether it's a field.
fn part_starts(module: &Module) -> std::collections::HashMap<u32, (u32, bool)> {
    let mut starts = std::collections::HashMap::new();
    let mut of = |mut stmts: Vec<js::Stmt>| {
        js::each_expr_mut(&mut stmts, &mut |e| {
            let (spans, field): (Vec<js::Span>, bool) = match &e.kind {
                ExprKind::Call(_, args) | ExprKind::New(_, args) => (args.iter().map(|arg| arg.span).collect(), false),
                ExprKind::Object(props, ..) => {
                    let values = props.iter().filter_map(|prop| match prop {
                        // A method's too, its arrow's (ADR 0882, amended).
                        js::Prop::Field(_, value) | js::Prop::Method(_, value) => Some(value.span),
                        _ => None,
                    });
                    (values.collect(), true)
                }
                _ => (Vec::new(), false),
            };
            if !e.span.is_none() {
                let spans = spans.into_iter().filter(|s| !s.is_none());
                starts.extend(spans.map(|s| (s.lo, (e.span.lo, field))));
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

/// The whole statements `code` starts with, each trimmed, each ending in a
/// `;` outside its brackets, and what's after them: the next's start, or
/// from where a bracket closes one it didn't open, its block's `}`. A
/// bracket or `;` in a string is counted as code's, which only ever leaves
/// fewer of them `let`s, or what's after them with a `;` or `}`.
fn leading_statements(code: &str) -> (Vec<&str>, &str) {
    let (mut statements, mut depth, mut from) = (Vec::new(), 0u32, 0);
    for (at, c) in code.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => match depth.checked_sub(1) {
                Some(inside) => depth = inside,
                None => break,
            },
            ';' if depth == 0 => {
                statements.push(code[from..at].trim());
                from = at + 1;
            }
            _ => {}
        }
    }
    (statements, &code[from..])
}

/// Where a block comment `c` alone on its lines, between an `=` and the
/// value on a line after it, `=\n /* .. */\n value`, goes: the line and
/// the column right before the value's first code, as if written there
/// (ADR 0630, amended). Not one after `==`, `<=` or a compound `+=`, nor
/// one of more than a line; what's between it and the `=` and the value
/// may be other comments only.
fn own_line_before_value(
    text: &str,
    comments: &[crate::program::Comment],
    c: &crate::program::Comment,
) -> Option<(u32, u32)> {
    let written = text.get(c.lo as usize..c.hi as usize)?;
    if !written.starts_with("/*") || written.contains('\n') {
        return None;
    }
    let line_start = text[..c.lo as usize].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[c.hi as usize..]
        .find('\n')
        .map_or(text.len(), |i| c.hi as usize + i);
    if !text[line_start..c.lo as usize].trim().is_empty() || !text[c.hi as usize..line_end].trim().is_empty() {
        return None;
    }
    // Past whitespace and other comments, each way.
    let mut before = c.lo as usize;
    loop {
        let code = text[..before].trim_end();
        match comments.iter().find(|o| o.hi as usize == code.len()) {
            Some(o) => before = o.lo as usize,
            None => break,
        }
    }
    let code = text[..before].trim_end();
    let equals = code
        .strip_suffix('=')
        .is_some_and(|before| !before.ends_with(|c: char| "=!<>+-*/%&|^".contains(c)));
    let mut after = c.hi as usize;
    loop {
        let skipped = after + (text[after..].len() - text[after..].trim_start().len());
        match comments.iter().find(|o| o.lo as usize == skipped) {
            Some(o) => after = o.hi as usize,
            None => {
                after = skipped;
                break;
            }
        }
    }
    if !equals || after >= text.len() {
        return None;
    }
    let value_line_start = text[..after].rfind('\n').map_or(0, |i| i + 1);
    let line = text[..after].matches('\n').count() as u32;
    // Right before the value, so the map's tokens after it are the value's.
    let column = (after - value_line_start) as u32;
    Some((line, column.saturating_sub(1)))
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

/// Where each of `module`'s functions marked `#[inline(never)]` starts: an
/// item's, a method's, and one written in a body, `function inner() {
/// .. }`, however deep (ADR 0649, amended).
fn noinline_starts(module: &Module) -> std::collections::HashSet<u32> {
    let mut starts = std::collections::HashSet::new();
    let mut note = |f: &js::Function| {
        if f.noinline && !f.span.is_none() {
            starts.insert(f.span.lo);
        }
    };
    for item in module.items.iter() {
        match item {
            js::Item::Function(f) => note(f),
            js::Item::Namespace(n) => n.methods.iter().for_each(&mut note),
            js::Item::Statements(_) | js::Item::Const(_) => {}
        }
    }
    for mut body in bodies(module) {
        js::each_block_mut(&mut body, &mut |stmts| {
            for stmt in stmts.iter() {
                if let StmtKind::Function(f) = &stmt.kind {
                    note(f);
                }
            }
        });
    }
    starts
}

/// Each body of `module`'s statements: its functions', its methods', its
/// statements', and of a constant, its value's, as a statement.
fn bodies(module: &Module) -> Vec<Vec<js::Stmt>> {
    (module.items.iter())
        .flat_map(|item| match item {
            js::Item::Function(f) => vec![f.body.clone()],
            js::Item::Namespace(n) => n.methods.iter().map(|method| method.body.clone()).collect(),
            js::Item::Statements(stmts) => vec![stmts.clone()],
            js::Item::Const(c) => vec![vec![StmtKind::Expr(c.value.clone()).at(js::Span::NONE)]],
        })
        .collect()
}

/// What a case that falls through to the next says, before it, as React
/// writes it, and ESLint's `no-fallthrough` asks (ADR 0905).
const FALLTHROUGH: &str = "// Fallthrough";

/// Where each case a case falls through to starts, of `module`'s
/// `switch`es: its first value's, which the case is printed at
/// (`fallen_to`), its comment `FALLTHROUGH` before it.
fn fallthrough_starts(module: &Module) -> Vec<u32> {
    let mut starts = Vec::new();
    for mut body in bodies(module) {
        js::each_block_mut(&mut body, &mut |stmts| {
            for stmt in stmts.iter() {
                if let StmtKind::Switch { cases, .. } = &stmt.kind {
                    starts.extend((0..cases.len()).filter_map(|k| fallen_to(cases, k)));
                }
            }
        });
    }
    starts
}

/// Where the case `cases[k]` is printed, where the one before falls
/// through to it: its first value's start, `FALLTHROUGH` before it.
pub(crate) fn fallen_to(cases: &[js::Case], k: usize) -> Option<u32> {
    let before = cases.get(k.checked_sub(1)?)?;
    let first = cases.get(k)?.values.first()?;
    (before.falls && !first.span.is_none()).then_some(first.span.lo)
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
            // One whose body is its value, `() => f()`, has no statement:
            // what's before the value is its arm's (`arm_starts`).
            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) if expressions::arrow_value(body).is_some() => {}
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
    // A `pub use`'s re-export, which its comments are before.
    for reexport in &module.reexports {
        if !reexport.span.is_none() {
            starts.0.push((reexport.span.lo, reexport.span.hi));
        }
    }
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
    code.push('\n');
    code
}

struct Cx<'a> {
    b: AstBuilder<'a>,
    allocator: &'a Allocator,
    /// The program's text, where a number is written as it was (ADR 0627).
    source: &'a str,
    /// The objects made of those the source writes across lines, by where
    /// each is in the arena (ADR 0835): which a visit of the program tells
    /// the formatter of, in its order (`across_lines`).
    across_lines: std::cell::RefCell<std::collections::HashSet<*const ObjectExpression<'a>>>,
    /// The template literals some of whose values the source writes across
    /// lines, by where each is in the arena, and which (ADR 0641, amended).
    interpolations: std::cell::RefCell<std::collections::HashMap<*const TemplateLiteral<'a>, Vec<bool>>>,
    /// Where a conditional's branch its comments are before starts (ADR
    /// 0658), whose first JS is placed there (`Cx::branch`).
    commented_branches: std::collections::HashSet<u32>,
    /// Where a statement that comments are before starts: an `if` that's an
    /// `else`'s only statement is in its block, not an `else if`, so they're
    /// there (ADR 0444, amended).
    commented: std::collections::HashSet<u32>,
}

/// Of each object of `program`, in the order a visit meets them, as its
/// JS's parse does, whether it's one of `cx`'s `across_lines`, which the
/// source writes across lines (ADR 0835); and so of each template literal's
/// values, `cx`'s `interpolations` (ADR 0641, amended).
fn across_lines<'a>(program: &Program<'a>, cx: &Cx<'a>) -> crate::format::Written {
    struct Found<'s, 'a> {
        cx: &'s Cx<'a>,
        found: crate::format::Written,
    }
    impl<'a> Visit<'a> for Found<'_, 'a> {
        fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
            (self.found.objects).push(self.cx.across_lines.borrow().contains(&(it as *const _)));
            oxc_ast_visit::walk::walk_object_expression(self, it);
        }
        fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
            let interpolations = self.cx.interpolations.borrow();
            let across = interpolations.get(&(it as *const _));
            (self.found.interpolations)
                .extend((0..it.expressions.len()).map(|i| across.is_some_and(|across| across.get(i) == Some(&true))));
            drop(interpolations);
            oxc_ast_visit::walk::walk_template_literal(self, it);
        }
    }
    let mut found = Found {
        cx,
        found: Default::default(),
    };
    found.visit_program(program);
    found.found
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
