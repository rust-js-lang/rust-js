//! Our JS AST ──► oxc's AST ──► JS text + source map.
//!
//! This and `format.rs` are the only files that use oxc. oxc is pre-1.0 and
//! its API changes often, so keeping it here means an oxc upgrade touches two.
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

use std::path::PathBuf;

use oxc_allocator::{Allocator, ArenaBox, ArenaVec};
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, ArrowFunctionBody, AssignmentTarget, BindingIdentifier, BindingPattern,
    BindingProperty, BindingRestElement, BlockStatement, CallExpression, CatchClause, CatchParameter, ChainElement,
    ComputedMemberExpression, Declaration, Directive, ExportFromDeclaration, Expression, ForStatementInit,
    ForStatementLeft, FormalParameter, FormalParameterKind, FormalParameterRest, FormalParameters, FunctionBody,
    FunctionType, IdentifierName, ImportDeclaration, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXChild,
    JSXClosingElement, JSXClosingFragment, JSXElementName, JSXExpression, JSXIdentifier, JSXMemberExpressionObject,
    JSXOpeningElement, JSXOpeningFragment, LabelIdentifier, ObjectPropertyKind, Program, PropertyKey, PropertyKind,
    SimpleAssignmentTarget, Statement, StaticMemberExpression, StringLiteral, TemplateElement, TemplateElementValue,
    VariableDeclarationKind, VariableDeclarator,
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
            js::Item::Const(c) => body.push(cx.constant(c)),
            js::Item::Statements(stmts) => body.extend(cx.stmts(stmts)),
            js::Item::Function(f) => body.push(cx.function(f)),
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
    if let Some(map) = &generated.map {
        for (line, comment) in &trailing {
            let last = (map.get_tokens())
                .filter(|t| t.get_src_line() == *line)
                .map(|t| t.get_dst_line())
                .max();
            if let Some(last) = last {
                after.entry(last).or_default().push(comment);
            }
        }
    }
    for (i, (out_line, line)) in (header_lines..).zip(generated.code.lines()).enumerate() {
        code.push_str(line);
        for comment in after.get(&(i as u32)).into_iter().flatten() {
            code.push(' ');
            code.push_str(comment);
        }
        code.push('\n');
        lines.push(out_line);
    }
    // After everything it maps, so the map's lines stay where they are.
    if let Some(function) = &module.default_export {
        code.push_str(&format!("\nexport default {function};\n"));
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

/// The comments of the files `module`'s code is of, past the ones each
/// starts with, a header's (ADR 0383), each before the statement or item it's
/// before: where only the start of that is between them, no `;`, `{` or
/// `}`, so not one of what has no JS, a struct's (ADR 0444). Each is as
/// written; a doc comment, `///` lines or `/** */`, a JSDoc one, `/** .. */`,
/// whose text is added after the arena's, where its span points. The text
/// the program's source is, and the comments.
fn comments_of(
    module: &Module,
    sources: &crate::program::Sources,
) -> (String, Vec<oxc_ast::Comment>, Vec<(u32, String)>) {
    use oxc_ast::{CommentKind, CommentPosition};
    let mut text = sources.text.clone();
    let mut found = Vec::new();
    let mut trailing = Vec::new();
    let spans = statement_spans(module);
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
                trailing.push((line, written.to_string()));
            }
        }
        let attached: Vec<(crate::program::Comment, u32)> = (file.comments.iter())
            .filter(|c| c.lo >= past && !after_code(c))
            .filter_map(|c| {
                let at = *starts
                    .get(starts.partition_point(|&s| s < c.hi))
                    .filter(|&&at| in_file(at))?;
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
                        !between.contains([';', '{', '}'])
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
            let before = &before[..before.rfind('\n').map_or(0, |i| i + 1)];
            let gap = &before[before.trim_end_matches(char::is_whitespace).len()..];
            gap.matches('\n').count() >= 2
        };
        // A blank line: a line comment of nothing, which prints as one.
        let blank = |text: &String| {
            let at = text.len() as u32;
            (at, at)
        };
        for &at in starts.iter().filter(|&&at| in_file(at)) {
            let mut pieces: Vec<(u32, u32)> = Vec::new();
            let own: Vec<crate::program::Comment> =
                attached.iter().filter(|(_, a)| *a == at).map(|(c, _)| *c).collect();
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
                let lines: Vec<&str> = lines.iter().map(|l| l.trim_start_matches("///")).collect();
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
                || (!own.is_empty() && sources.text[last as usize..at as usize].matches('\n').count() >= 2)
            {
                pieces.push(blank(&text));
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

/// The span of each of `list`'s statements that has one: a temporary
/// rust-js made has the next's start when it's printed (`push_stmts`).
fn list_spans(list: &[js::Stmt]) -> impl Iterator<Item = (u32, u32)> + '_ {
    (list.iter())
        .filter(|stmt| !stmt.span.is_none())
        .map(|stmt| (stmt.span.lo, stmt.span.hi))
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
/// as a person orders it (ADR 0295), re-exports and caches. Layout, as
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
    if !module.caches.is_empty() {
        code.push_str(&format!("\nvar {};\n", module.caches.join(", ")));
    }
    code.push('\n');
    code
}

struct Cx<'a> {
    b: AstBuilder<'a>,
    allocator: &'a Allocator,
    /// Where a statement a comment is printed before starts.
    commented: std::collections::HashSet<u32>,
}

impl<'a> Cx<'a> {
    fn params(&self, kind: FormalParameterKind, patterns: &[js::Pattern]) -> FormalParameters<'a> {
        let b = &self.b;
        // The last, `...args`, gathers the rest (ADR 0428).
        let (rest, patterns) = match patterns.split_last() {
            Some((js::Pattern::Rest(name), before)) => (Some(name), before),
            _ => (None, patterns),
        };
        let rest = rest.map(|name| {
            let binding = BindingPattern::new_binding_identifier(SPAN, self.name(name), b);
            ArenaBox::new_in(
                FormalParameterRest::new(
                    SPAN,
                    ArenaVec::new_in(b),
                    BindingRestElement::new(SPAN, binding, b),
                    None,
                    b,
                ),
                b,
            )
        });
        let params = patterns.iter().map(|pattern| {
            // `location = { .. }`, where it's given none (ADR 0427).
            let (pattern, default) = match pattern {
                js::Pattern::Defaulted(name, default) => (
                    BindingPattern::new_binding_identifier(SPAN, self.name(name), b),
                    Some(ArenaBox::new_in(self.expr(default), b)),
                ),
                pattern => (self.pattern(pattern), None),
            };
            FormalParameter::new(
                SPAN,
                ArenaVec::new_in(b),
                pattern,
                None,
                default,
                false,
                None,
                false,
                false,
                b,
            )
        });
        FormalParameters::new(SPAN, kind, ArenaVec::from_iter_in(params, b), rest, b)
    }

    /// A function's or an arrow's body: its directives, `"use cache";`, in
    /// its prologue (ADR 0349), then its statements.
    fn function_body(&self, body: &[js::Stmt]) -> FunctionBody<'a> {
        let b = &self.b;
        let directives = body.iter().filter_map(|s| match &s.kind {
            StmtKind::Directive(directive) => {
                let directive = self.name(directive);
                Some(Directive::new(
                    SPAN,
                    StringLiteral::new(SPAN, directive, None, b),
                    directive,
                    b,
                ))
            }
            _ => None,
        });
        let directives = ArenaVec::from_iter_in(directives, b);
        let stmts: Vec<js::Stmt> = (body.iter())
            .filter(|s| !matches!(s.kind, StmtKind::Directive(_)))
            .cloned()
            .collect();
        FunctionBody::new(SPAN, directives, self.stmts(&stmts), b)
    }

    fn function(&self, f: &js::Function) -> Statement<'a> {
        let b = &self.b;
        let params = self.params(FormalParameterKind::FormalParameter, &f.params);
        let body = self.function_body(&f.body);
        let decl = Declaration::new_function_declaration(
            span(f.span),
            FunctionType::FunctionDeclaration,
            Some(BindingIdentifier::new(span(f.name_span), self.name(&f.name), b)),
            false, // generator
            f.is_async,
            false, // declare
            None,  // type parameters
            None,  // this param
            ArenaBox::new_in(params, b),
            None, // return type
            Some(ArenaBox::new_in(body, b)),
            b,
        );
        if f.export {
            Statement::new_export_declaration(span(f.span), decl, b)
        } else {
            decl.into()
        }
    }

    /// `export const Counter = { new(step) { .. }, .. };`: each method in
    /// shorthand, as a hand-written object of functions has them.
    fn namespace(&self, n: &js::Namespace) -> Statement<'a> {
        let b = &self.b;
        let constants = n.constants.iter().map(|c| {
            let key = PropertyKey::new_static_identifier(span(c.span), self.name(&c.name), b);
            ObjectPropertyKind::new_object_property(
                span(c.span),
                PropertyKind::Init,
                key,
                self.expr(&c.value),
                false,
                false,
                false,
                b,
            )
        });
        let methods = {
            ArenaVec::from_iter_in(
                constants.chain(n.methods.iter().map(|f| {
                    let params = self.params(FormalParameterKind::FormalParameter, &f.params);
                    let body = self.function_body(&f.body);
                    let value = Expression::new_function_expression(
                        span(f.span),
                        FunctionType::FunctionExpression,
                        None,
                        false, // generator
                        f.is_async,
                        false, // declare
                        None,  // type parameters
                        None,  // this param
                        ArenaBox::new_in(params, b),
                        None, // return type
                        Some(ArenaBox::new_in(body, b)),
                        b,
                    );
                    let key = PropertyKey::new_static_identifier(span(f.name_span), self.name(&f.name), b);
                    ObjectPropertyKind::new_object_property(
                        span(f.span),
                        PropertyKind::Init,
                        key,
                        value,
                        true,
                        false,
                        false,
                        b,
                    )
                })),
                b,
            )
        };
        let id = BindingPattern::new_binding_identifier(SPAN, self.name(&n.name), b);
        let object = Expression::new_object_expression(SPAN, methods, b);
        let declarator = VariableDeclarator::new(SPAN, id, None, Some(object), false, b);
        let decl = Declaration::new_variable_declaration(
            SPAN,
            VariableDeclarationKind::Const,
            ArenaVec::from_iter_in([declarator], b),
            false,
            b,
        );
        if n.export {
            Statement::new_export_declaration(SPAN, decl, b)
        } else {
            decl.into()
        }
    }

    /// `const { Intro, p: P } = MDXComponents;` of constants each reading a
    /// property of the object, `read_together`'s.
    fn constant(&self, c: &js::Const) -> Statement<'a> {
        let b = &self.b;
        let sp = span(c.span);
        let id = BindingPattern::new_binding_identifier(SPAN, self.name(&c.name), b);
        // `let cached;`: a variable set later starts `undefined` by itself.
        let init = match (&c.value.kind, c.mutable) {
            (ExprKind::Undefined, true) => None,
            _ => Some(self.expr(&c.value)),
        };
        let declarator = VariableDeclarator::new(sp, id, None, init, false, b);
        let kind = if c.mutable {
            VariableDeclarationKind::Let
        } else {
            VariableDeclarationKind::Const
        };
        let decl = Declaration::new_variable_declaration(sp, kind, ArenaVec::from_iter_in([declarator], b), false, b);
        if c.export {
            Statement::new_export_declaration(sp, decl, b)
        } else {
            decl.into()
        }
    }

    fn stmts(&self, stmts: &[js::Stmt]) -> ArenaVec<'a, Statement<'a>> {
        let mut out = ArenaVec::new_in(&self.b);
        self.push_stmts(stmts, &mut out);
        out
    }

    /// `stmts`, each pushed to `out`.
    fn push_stmts(&self, stmts: &[js::Stmt], out: &mut ArenaVec<'a, Statement<'a>>) {
        for (i, s) in stmts.iter().enumerate() {
            let mut statement = self.stmt(s);
            // One rust-js made, a temporary, with no Rust of its own: at the
            // start of the next that has, whose comments it's printed after
            // (ADR 0444), the first of its JS.
            if s.span.is_none()
                && let Some(next) = stmts[i + 1..].iter().find(|n| !n.span.is_none())
            {
                *oxc_span::GetSpanMut::span_mut(&mut statement) = Span::new(next.span.lo, next.span.lo);
            }
            out.push(statement);
        }
    }

    fn block(&self, stmts: &[js::Stmt]) -> Statement<'a> {
        Statement::new_block_statement(SPAN, self.stmts(stmts), &self.b)
    }

    fn stmt(&self, s: &js::Stmt) -> Statement<'a> {
        let b = &self.b;
        let sp = span(s.span);
        match &s.kind {
            StmtKind::Const(name, init) => self.declare(sp, VariableDeclarationKind::Const, name, Some(init)),
            StmtKind::Function(function) => self.function(function),
            StmtKind::Directive(_) => unreachable!("a directive is its function's body's, taken first"),
            StmtKind::Let(name, init) => self.declare(sp, VariableDeclarationKind::Let, name, init.as_ref()),
            StmtKind::Destructure {
                pattern,
                value,
                mutable,
            } => {
                let kind = if *mutable {
                    VariableDeclarationKind::Let
                } else {
                    VariableDeclarationKind::Const
                };
                let declarator =
                    VariableDeclarator::new(sp, self.pattern(pattern), None, Some(self.expr(value)), false, b);
                Statement::new_variable_declaration(sp, kind, ArenaVec::from_iter_in([declarator], b), false, b)
            }
            StmtKind::Assign(target, value) => {
                let assign = Expression::new_assignment_expression(
                    sp,
                    AssignmentOperator::Assign,
                    self.assignment_target(target),
                    self.expr(value),
                    b,
                );
                Statement::new_expression_statement(sp, assign, b)
            }
            StmtKind::Compound(op, target, value) => {
                let operator = match op {
                    Op::Add => AssignmentOperator::Addition,
                    Op::Sub => AssignmentOperator::Subtraction,
                    Op::Mul => AssignmentOperator::Multiplication,
                    Op::Div => AssignmentOperator::Division,
                    Op::Rem => AssignmentOperator::Remainder,
                    Op::Coalesce => AssignmentOperator::LogicalNullish,
                    _ => unreachable!("preparation compounds these alone"),
                };
                let assign = Expression::new_assignment_expression(
                    sp,
                    operator,
                    self.assignment_target(target),
                    self.expr(value),
                    b,
                );
                Statement::new_expression_statement(sp, assign, b)
            }
            StmtKind::Expr(e) => Statement::new_expression_statement(sp, self.expr(e), b),
            StmtKind::If(cond, then, els) => {
                let els = els.as_deref().map(|els| match els {
                    // A lone `if` in the `else` prints as `else if`.
                    [
                        only @ js::Stmt {
                            kind: StmtKind::If(..), ..
                        },
                    ] => self.stmt(only),
                    _ => self.block(els),
                });
                Statement::new_if_statement(sp, self.expr(cond), self.block(then), els, b)
            }
            StmtKind::Labeled(label, body) => {
                let block = Statement::new_block_statement(sp, self.stmts(body), b);
                self.labeled(sp, Some(label), block)
            }
            StmtKind::Try(body, finally) => Statement::new_try_statement(
                sp,
                BlockStatement::boxed(SPAN, self.stmts(body), b),
                None,
                Some(BlockStatement::boxed(SPAN, self.stmts(finally), b)),
                b,
            ),
            StmtKind::TryCatch(body, error, handler) => {
                let param = error.as_ref().map(|error| {
                    let name = BindingPattern::new_binding_identifier(SPAN, self.name(error), b);
                    CatchParameter::new(SPAN, name, None, b)
                });
                let clause = CatchClause::boxed(SPAN, param, BlockStatement::boxed(SPAN, self.stmts(handler), b), b);
                Statement::new_try_statement(
                    sp,
                    BlockStatement::boxed(SPAN, self.stmts(body), b),
                    Some(clause),
                    None,
                    b,
                )
            }
            StmtKind::While { label, cond, body } => {
                let w = Statement::new_while_statement(sp, self.expr(cond), self.block(body), b);
                self.labeled(sp, label.as_deref(), w)
            }
            StmtKind::ForOf {
                label,
                pattern,
                mutable,
                iterable,
                body,
            } => {
                let id = self.pattern(pattern);
                let declarator = VariableDeclarator::new(SPAN, id, None, None, false, b);
                let kind = if *mutable {
                    VariableDeclarationKind::Let
                } else {
                    VariableDeclarationKind::Const
                };
                let left = ForStatementLeft::new_variable_declaration(
                    SPAN,
                    kind,
                    ArenaVec::from_iter_in([declarator], b),
                    false,
                    b,
                );
                let l = Statement::new_for_of_statement(sp, false, left, self.expr(iterable), self.block(body), b);
                self.labeled(sp, label.as_deref(), l)
            }
            StmtKind::For {
                label,
                name,
                start,
                test,
                body,
            } => {
                let id = BindingPattern::new_binding_identifier(SPAN, self.name(name), b);
                let declarator = VariableDeclarator::new(SPAN, id, None, Some(self.expr(start)), false, b);
                let init = ForStatementInit::new_variable_declaration(
                    SPAN,
                    VariableDeclarationKind::Let,
                    ArenaVec::from_iter_in([declarator], b),
                    false,
                    b,
                );
                let counter = SimpleAssignmentTarget::new_assignment_target_identifier(SPAN, self.name(name), b);
                let update = Expression::new_update_expression(SPAN, UpdateOperator::Increment, false, counter, b);
                let l = Statement::new_for_statement(
                    sp,
                    Some(init),
                    Some(self.expr(test)),
                    Some(update),
                    self.block(body),
                    b,
                );
                self.labeled(sp, label.as_deref(), l)
            }
            StmtKind::Break(label) => Statement::new_break_statement(sp, label.as_deref().map(|l| self.label(l)), b),
            StmtKind::Continue(label) => {
                Statement::new_continue_statement(sp, label.as_deref().map(|l| self.label(l)), b)
            }
            StmtKind::Throw(value) => Statement::new_throw_statement(sp, self.expr(value), b),
            StmtKind::Return(value) => Statement::new_return_statement(sp, value.as_ref().map(|v| self.expr(v)), b),
        }
    }

    fn pattern(&self, pattern: &js::Pattern) -> BindingPattern<'a> {
        let b = &self.b;
        let name = |name: &str| BindingPattern::new_binding_identifier(SPAN, self.name(name), b);
        match pattern {
            js::Pattern::Name(n) => name(n),
            js::Pattern::Rest(_) | js::Pattern::Defaulted(..) => {
                unreachable!("a rest or a default is a function's parameter, `params`'")
            }
            js::Pattern::Array(items) => {
                let items = items.iter().map(|item| item.as_ref().map(|item| self.pattern(item)));
                BindingPattern::new_array_pattern(SPAN, ArenaVec::from_iter_in(items, b), None, b)
            }
            js::Pattern::Object(fields, rest) => {
                let fields = fields.iter().map(|(field, var, default)| {
                    // A field that isn't a JS name, `data-platform`, is quoted.
                    let key = if crate::names::is_identifier(field) {
                        PropertyKey::new_static_identifier(SPAN, self.name(field), b)
                    } else {
                        PropertyKey::new_string_literal(SPAN, self.name(field), None, b)
                    };
                    // `{ size = "md" }`, where it's missing (ADR 0212).
                    let value = match default {
                        Some(default) => {
                            BindingPattern::new_assignment_pattern(SPAN, self.pattern(var), self.expr(default), b)
                        }
                        None => self.pattern(var),
                    };
                    // `{ x }` for `{ x: x }`.
                    BindingProperty::new(SPAN, key, value, var.name() == Some(field.as_str()), false, b)
                });
                let rest = rest
                    .as_deref()
                    .map(|rest| ArenaBox::new_in(BindingRestElement::new(SPAN, name(rest), b), b));
                BindingPattern::new_object_pattern(SPAN, ArenaVec::from_iter_in(fields, b), rest, b)
            }
        }
    }

    fn assignment_target(&self, e: &js::Expr) -> AssignmentTarget<'a> {
        let b = &self.b;
        let sp = span(e.span);
        match &e.kind {
            ExprKind::Var(name) => AssignmentTarget::new_assignment_target_identifier(sp, self.name(name), b),
            // A property that isn't a JS name, `files["2d"]`, by its key.
            ExprKind::Member(object, property) if !member_name(property) => {
                let key = Expression::new_string_literal(SPAN, self.name(property), None, b);
                AssignmentTarget::new_computed_member_expression(sp, self.expr(object), key, false, b)
            }
            ExprKind::Member(object, property) => AssignmentTarget::new_static_member_expression(
                sp,
                self.expr(object),
                IdentifierName::new(SPAN, self.name(property), b),
                false,
                b,
            ),
            ExprKind::Index(object, index) => {
                AssignmentTarget::new_computed_member_expression(sp, self.expr(object), self.expr(index), false, b)
            }
            _ => unreachable!("lowering only assigns to variables and fields"),
        }
    }

    fn labeled(&self, sp: Span, label: Option<&str>, s: Statement<'a>) -> Statement<'a> {
        match label {
            Some(l) => Statement::new_labeled_statement(sp, self.label(l), s, &self.b),
            None => s,
        }
    }

    fn declare(&self, sp: Span, kind: VariableDeclarationKind, name: &str, init: Option<&js::Expr>) -> Statement<'a> {
        let b = &self.b;
        let id = BindingPattern::new_binding_identifier(SPAN, self.name(name), b);
        let declarator = VariableDeclarator::new(sp, id, None, init.map(|e| self.expr(e)), false, b);
        Statement::new_variable_declaration(sp, kind, ArenaVec::from_iter_in([declarator], b), false, b)
    }

    /// A call's argument, `...items` among them (ADR 0221).
    fn argument(&self, a: &js::Expr) -> Argument<'a> {
        match &a.kind {
            ExprKind::Spread(all) => Argument::new_spread_element(span(a.span), self.expr(all), &self.b),
            _ => Argument::from(self.expr(a)),
        }
    }

    /// A link of a chain with a `?.` in it, `o?.inner` of `o?.inner.v`: its
    /// object a link too, inside the one chain expression, so each `?.`
    /// ends all of it where its object is `undefined`.
    fn chain_element(&self, e: &js::Expr) -> ChainElement<'a> {
        let b = &self.b;
        let sp = span(e.span);
        match &e.kind {
            ExprKind::Member(object, property) | ExprKind::OptionalMember(object, property) => {
                let optional = matches!(e.kind, ExprKind::OptionalMember(..));
                let object = self.chain_object(object);
                if member_name(property) {
                    let name = IdentifierName::new(SPAN, self.name(property), b);
                    ChainElement::StaticMemberExpression(StaticMemberExpression::boxed(sp, object, name, optional, b))
                } else {
                    // A property that isn't a JS name, by its key.
                    let key = Expression::new_string_literal(SPAN, self.name(property), None, b);
                    ChainElement::ComputedMemberExpression(ComputedMemberExpression::boxed(
                        sp, object, key, optional, b,
                    ))
                }
            }
            ExprKind::Index(object, index) | ExprKind::OptionalIndex(object, index) => {
                ChainElement::ComputedMemberExpression(ComputedMemberExpression::boxed(
                    sp,
                    self.chain_object(object),
                    self.expr(index),
                    matches!(e.kind, ExprKind::OptionalIndex(..)),
                    b,
                ))
            }
            ExprKind::Call(callee, args) | ExprKind::OptionalCall(callee, args) => {
                let optional = matches!(e.kind, ExprKind::OptionalCall(..));
                let args = ArenaVec::from_iter_in(args.iter().map(|a| self.argument(a)), b);
                ChainElement::CallExpression(CallExpression::boxed(
                    sp,
                    self.chain_object(callee),
                    None,
                    args,
                    optional,
                    b,
                ))
            }
            _ => unreachable!("a chain is of members, indexes and calls"),
        }
    }

    /// The object of a chain's link: a link itself, where the chain goes on
    /// through it, else the expression it is.
    fn chain_object(&self, object: &js::Expr) -> Expression<'a> {
        if !in_chain(object) {
            return self.expr(object);
        }
        match self.chain_element(object) {
            ChainElement::StaticMemberExpression(member) => Expression::StaticMemberExpression(member),
            ChainElement::ComputedMemberExpression(member) => Expression::ComputedMemberExpression(member),
            ChainElement::CallExpression(call) => Expression::CallExpression(call),
            _ => unreachable!("a chain's links are members and calls"),
        }
    }

    fn expr(&self, e: &js::Expr) -> Expression<'a> {
        let b = &self.b;
        let sp = span(e.span);
        let argument = |a: &js::Expr| self.argument(a);
        // A chain with a `?.` in it, `o?.inner.v` or `a?.m(x)`, is one chain
        // expression: `(o?.inner).v` would read `.v` of `undefined`.
        if in_chain(e) {
            return Expression::new_chain_expression(sp, self.chain_element(e), b);
        }
        match &e.kind {
            ExprKind::Handle(_) | ExprKind::Pair(..) => unreachable!("written by preparation"),
            ExprKind::Num(n) => self.number(sp, *n),
            // `5n`, and `-5n` as `-` of it, as a number is written.
            ExprKind::BigInt(n) => {
                let digits = Expression::new_identifier(sp, self.name(&format!("{}n", n.unsigned_abs())), b);
                if *n < 0 {
                    Expression::new_unary_expression(sp, UnaryOperator::UnaryNegation, digits, b)
                } else {
                    digits
                }
            }
            ExprKind::BigUint(n) => Expression::new_identifier(sp, self.name(&format!("{n}n")), b),
            ExprKind::Bool(v) => Expression::new_boolean_literal(sp, *v, b),
            ExprKind::Str(s) => Expression::new_string_literal(sp, self.allocator.alloc_str(s), None, b),
            ExprKind::Undefined | ExprKind::GivenUndefined => Expression::new_identifier(sp, "undefined", b),
            ExprKind::Null => Expression::new_null_literal(sp, b),
            ExprKind::Symbol(_) => unreachable!("linking resolves every module symbol before emission"),
            ExprKind::OptionalMember(..) | ExprKind::OptionalIndex(..) | ExprKind::OptionalCall(..) => {
                unreachable!("a chain is printed whole, above")
            }
            ExprKind::Var(name) => Expression::new_identifier(sp, self.name(name), b),
            // A property that isn't a JS name, `files["worker-bundle"]`, by its
            // key: `files.worker-bundle` would be a subtraction.
            ExprKind::Member(object, property) if !member_name(property) => {
                let key = Expression::new_string_literal(SPAN, self.name(property), None, b);
                Expression::new_computed_member_expression(sp, self.expr(object), key, false, b)
            }
            ExprKind::Member(object, property) => {
                // `5.toString()` would read `5.` as a number: `(5).toString()`.
                // A number is printed as its digits (`number`), so oxc can't
                // tell it needs them.
                let object = match object.kind {
                    ExprKind::Num(n) if n.is_finite() && n >= 0.0 && !js_number(n).contains(['.', 'e']) => {
                        Expression::new_identifier(sp, self.name(&format!("({})", js_number(n))), b)
                    }
                    _ => self.expr(object),
                };
                Expression::new_static_member_expression(
                    sp,
                    object,
                    IdentifierName::new(SPAN, self.name(property), b),
                    false,
                    b,
                )
            }
            ExprKind::Index(object, index) => {
                Expression::new_computed_member_expression(sp, self.expr(object), self.expr(index), false, b)
            }
            // `String(n)` where it isn't in a template (ADR 0066).
            ExprKind::Stringed(value) => self.expr(&js::Expr::call(js::Expr::var("String"), vec![(**value).clone()])),
            ExprKind::Array(items) => {
                let items = items.iter().map(|item| match &item.kind {
                    ExprKind::Spread(all) => {
                        ArrayExpressionElement::new_spread_element(span(item.span), self.expr(all), b)
                    }
                    _ => ArrayExpressionElement::from(self.expr(item)),
                });
                Expression::new_array_expression(sp, ArenaVec::from_iter_in(items, b), b)
            }
            ExprKind::Object(props) => {
                let props = props.iter().map(|prop| self.property(prop));
                Expression::new_object_expression(sp, ArenaVec::from_iter_in(props, b), b)
            }
            ExprKind::Jsx(jsx) => self.jsx(sp, jsx),
            // Printed as written, as `number` prints an integer.
            ExprKind::Regex(literal) => Expression::new_identifier(sp, self.name(literal), b),
            ExprKind::Lines(text) => {
                let value = TemplateElementValue {
                    raw: self.name(&template_raw(text, true)).into(),
                    cooked: Some(self.name(text).into()),
                };
                let quasis = ArenaVec::from_iter_in([TemplateElement::new(SPAN, value, true, b)], b);
                Expression::new_template_literal(sp, quasis, ArenaVec::new_in(b), b)
            }
            ExprKind::Template(texts, values, lines) => {
                let quasis = texts.iter().enumerate().map(|(i, text)| {
                    let value = TemplateElementValue {
                        raw: self.name(&template_raw(text, *lines)).into(),
                        cooked: Some(self.name(text).into()),
                    };
                    TemplateElement::new(SPAN, value, i + 1 == texts.len(), b)
                });
                let quasis = ArenaVec::from_iter_in(quasis, b);
                let values = ArenaVec::from_iter_in(values.iter().map(|v| self.expr(v)), b);
                Expression::new_template_literal(sp, quasis, values, b)
            }
            ExprKind::Unary(op, arg) => {
                let op = match op {
                    UnaryOp::Neg => UnaryOperator::UnaryNegation,
                    UnaryOp::Not => UnaryOperator::LogicalNot,
                    UnaryOp::BitNot => UnaryOperator::BitwiseNot,
                    UnaryOp::Typeof => UnaryOperator::Typeof,
                };
                Expression::new_unary_expression(sp, op, self.expr(arg), b)
            }
            ExprKind::Binary(op, l, r) => {
                let (l, r) = (self.expr(l), self.expr(r));
                match binary_op(*op) {
                    Ok(op) => Expression::new_binary_expression(sp, l, op, r, b),
                    Err(op) => Expression::new_logical_expression(sp, l, op, r, b),
                }
            }
            ExprKind::Cond(test, then, els) => {
                Expression::new_conditional_expression(sp, self.expr(test), self.expr(then), self.expr(els), b)
            }
            ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => {
                let is_async = matches!(e.kind, ExprKind::AsyncArrow(..));
                let params = ArenaBox::new_in(self.params(FormalParameterKind::ArrowFormalParameters, params), b);
                // `() => x` when the body only returns a value, and no
                // comment is printed before it: a statement's.
                let body = match body.as_slice() {
                    [
                        js::Stmt {
                            kind: StmtKind::Return(Some(value)),
                            span,
                        },
                    ] if span.is_none() || !self.commented.contains(&span.lo) => {
                        ArrowFunctionBody::from(self.expr(value))
                    }
                    _ => ArrowFunctionBody::FunctionBody(ArenaBox::new_in(self.function_body(body), b)),
                };
                Expression::new_arrow_function_expression(sp, is_async, None, params, None, body, b)
            }
            // `function Label(props) { .. }`, named (ADR 0296).
            ExprKind::Function(f) => {
                let params = self.params(FormalParameterKind::FormalParameter, &f.params);
                let body = self.function_body(&f.body);
                Expression::new_function_expression(
                    span(f.span),
                    FunctionType::FunctionExpression,
                    Some(BindingIdentifier::new(span(f.name_span), self.name(&f.name), b)),
                    false, // generator
                    f.is_async,
                    false, // declare
                    None,  // type parameters
                    None,  // this param
                    ArenaBox::new_in(params, b),
                    None, // return type
                    Some(ArenaBox::new_in(body, b)),
                    b,
                )
            }
            ExprKind::FunctionHole(_) => unreachable!("the pipeline puts each function where its hole is"),
            ExprKind::Import(js::ImportFrom::Specifier(from)) => {
                let source = Expression::new_string_literal(SPAN, self.allocator.alloc_str(from), None, b);
                Expression::new_import_expression(sp, source, None, None, b)
            }
            ExprKind::Import(js::ImportFrom::Module(_)) => {
                unreachable!("its output plan gives a module of the crate's its specifier")
            }
            ExprKind::DropArgument(..) => unreachable!("the pipeline keeps each drop argument or leaves it out"),
            ExprKind::Await(promise) => Expression::new_await_expression(sp, self.expr(promise), b),
            ExprKind::Spread(_) => unreachable!("`...items` is an array's item or a call's argument, which they write"),
            ExprKind::Call(callee, args) => {
                let args = args.iter().map(argument);
                Expression::new_call_expression(sp, self.expr(callee), None, ArenaVec::from_iter_in(args, b), false, b)
            }
            ExprKind::New(callee, args) if let Some(literal) = regex_literal(callee, args) => {
                Expression::new_identifier(sp, self.name(&literal), b)
            }
            ExprKind::New(callee, args) => {
                let args = args.iter().map(argument);
                Expression::new_new_expression(sp, self.expr(callee), None, ArenaVec::from_iter_in(args, b), b)
            }
        }
    }

    /// A JSX element (ADR 0040), laid out by the formatter.
    fn jsx(&self, sp: Span, jsx: &js::Jsx) -> Expression<'a> {
        let b = &self.b;
        let mut children = ArenaVec::new_in(b);
        // Which children are JSX text: text that JSX can write, but not
        // right before text, which JSX would read as one with it, and React
        // render as one text node, not two. So the earlier is in braces,
        // `{" of"} {total}`: a formatter makes a later `{" "}` a space, of
        // the text before it.
        let mut text = vec![false; jsx.children.len()];
        for i in (0..jsx.children.len()).rev() {
            let writable = matches!(&jsx.children[i].kind, ExprKind::Str(s) if jsx_text_safe(s) && !s.is_empty());
            text[i] = writable && !text.get(i + 1).is_some_and(|&next| next);
        }
        // Of two, the later in braces where it has a space at an edge and
        // the earlier none, as a person writes `If this is a mistake{", "}`:
        // never a blank one, which a formatter makes a space of the text.
        let plain = |s: &str| jsx_text_safe(s) && !s.is_empty() && s.trim() == s;
        let spaced = |s: &str| !s.trim().is_empty() && s.trim() != s;
        for i in 0..jsx.children.len().saturating_sub(1) {
            if let (ExprKind::Str(earlier), ExprKind::Str(later)) = (&jsx.children[i].kind, &jsx.children[i + 1].kind)
                && !text[i]
                && text[i + 1]
                && plain(earlier)
                && spaced(later)
                && (i == 0 || !text[i - 1])
            {
                text[i] = true;
                text[i + 1] = false;
            }
        }
        for (child, &text) in jsx.children.iter().zip(&text) {
            children.push(self.jsx_child(child, text));
        }
        if let JsxTag::Fragment = jsx.tag {
            let (open, close) = (JSXOpeningFragment::new(sp, b), JSXClosingFragment::new(SPAN, b));
            return Expression::new_jsx_fragment(sp, open, children, close, b);
        }
        let attrs = jsx.props.iter().filter_map(|prop| self.jsx_attribute(prop));
        let opening = JSXOpeningElement::boxed(
            sp,
            self.jsx_name(&jsx.tag, sp),
            None,
            ArenaVec::from_iter_in(attrs, b),
            b,
        );
        // `<img />` has nothing to close.
        let closing = (!children.is_empty()).then(|| JSXClosingElement::boxed(SPAN, self.jsx_name(&jsx.tag, SPAN), b));
        Expression::new_jsx_element(sp, opening, children, closing, b)
    }

    /// `div`, `Counter`, or `stats.Chart` from another module.
    fn jsx_name(&self, tag: &JsxTag, sp: Span) -> JSXElementName<'a> {
        let b = &self.b;
        let component = match tag {
            JsxTag::Intrinsic(tag) => return JSXElementName::new_identifier(sp, self.name(tag), b),
            JsxTag::Component(component) => component,
            JsxTag::Fragment => unreachable!("a fragment has no name"),
        };
        match &component.kind {
            ExprKind::Var(name) => JSXElementName::new_identifier_reference(sp, self.name(name), b),
            ExprKind::Member(object, property) => {
                let property = JSXIdentifier::new(SPAN, self.name(property), b);
                JSXElementName::new_member_expression(SPAN, self.jsx_object(object), property, b)
            }
            _ => unreachable!("lowering only makes components of names and paths"),
        }
    }

    fn jsx_object(&self, object: &js::Expr) -> JSXMemberExpressionObject<'a> {
        let b = &self.b;
        match &object.kind {
            ExprKind::Var(name) => JSXMemberExpressionObject::new_identifier_reference(SPAN, self.name(name), b),
            ExprKind::Member(inner, property) => {
                let property = JSXIdentifier::new(SPAN, self.name(property), b);
                JSXMemberExpressionObject::new_member_expression(SPAN, self.jsx_object(inner), property, b)
            }
            _ => unreachable!("lowering only makes components of names and paths"),
        }
    }

    /// `className="hero"`, `disabled` for `true`, `onClick={f}` or `{...props}`.
    /// Preparation has left out an attribute that's `undefined` (a `None`).
    fn jsx_attribute(&self, prop: &Prop) -> Option<JSXAttributeItem<'a>> {
        let b = &self.b;
        let (name, value) = match prop {
            Prop::Spread(value) => return Some(JSXAttributeItem::new_spread_attribute(SPAN, self.expr(value), b)),
            Prop::Field(name, value) | Prop::Getter(name, value) | Prop::Setter(name, value) => (name, value),
        };
        let sp = span(value.span);
        let value = match &value.kind {
            ExprKind::Bool(true) => None,
            ExprKind::Str(s) if jsx_text_safe(s) && !s.contains('"') => {
                Some(JSXAttributeValue::new_string_literal(sp, self.name(s), None, b))
            }
            _ => Some(JSXAttributeValue::new_expression_container(
                sp,
                JSXExpression::from(self.expr(value)),
                b,
            )),
        };
        // An inline callback may have no mapping before its first argument.
        // Anchor the attribute to its value so a newly wrapped line still maps.
        let name = JSXAttributeName::new_identifier(sp, self.name(name), b);
        Some(JSXAttributeItem::new_attribute(sp, name, value, b))
    }

    /// Text as text, `Count is `, when it's `text`; anything else in braces,
    /// `{count}`.
    fn jsx_child(&self, child: &js::Expr, text: bool) -> JSXChild<'a> {
        let b = &self.b;
        let sp = span(child.span);
        match &child.kind {
            ExprKind::Str(s) if text => JSXChild::new_text(sp, self.name(s), None, b),
            ExprKind::Jsx(jsx) => match self.jsx(sp, jsx) {
                Expression::JSXElement(e) => JSXChild::Element(e),
                Expression::JSXFragment(f) => JSXChild::Fragment(f),
                _ => unreachable!("JSX converts to JSX"),
            },
            _ => JSXChild::new_expression_container(sp, JSXExpression::from(self.expr(child)), b),
        }
    }

    /// `get name() { .. }` or `set name(value) { .. }`.
    fn accessor(
        &self,
        kind: PropertyKind,
        name: &str,
        params: &[js::Pattern],
        body: &[js::Stmt],
    ) -> ObjectPropertyKind<'a> {
        let b = &self.b;
        let params = self.params(FormalParameterKind::FormalParameter, params);
        let body = self.function_body(body);
        let value = Expression::new_function_expression(
            SPAN,
            FunctionType::FunctionExpression,
            None,
            false, // generator
            false, // async
            false, // declare
            None,  // type parameters
            None,  // this param
            ArenaBox::new_in(params, b),
            None, // return type
            Some(ArenaBox::new_in(body, b)),
            b,
        );
        let key = PropertyKey::new_static_identifier(SPAN, self.name(name), b);
        ObjectPropertyKind::new_object_property(SPAN, kind, key, value, false, false, false, b)
    }

    fn property(&self, prop: &Prop) -> ObjectPropertyKind<'a> {
        let b = &self.b;
        match prop {
            Prop::Setter(name, value) => match &value.kind {
                ExprKind::Arrow(params, body) => self.accessor(PropertyKind::Set, name, params, body),
                _ => unreachable!("a setter is an arrow of one parameter"),
            },
            Prop::Getter(name, value) => match &value.kind {
                ExprKind::Arrow(params, body) => self.accessor(PropertyKind::Get, name, params, body),
                _ => self.accessor(
                    PropertyKind::Get,
                    name,
                    &[],
                    &[js::StmtKind::Return(Some(value.clone())).at(value.span)],
                ),
            },
            Prop::Field(name, value) => {
                let value = self.expr(value);
                // `{ x: x }` reads better as `{ x }`.
                let shorthand = matches!(&value, Expression::Identifier(id) if id.name == name.as_str());
                // In a literal, a plain `__proto__:` key sets the prototype. Quoted
                // and computed, it's an ordinary field, like any other Rust field.
                let computed = name == "__proto__";
                // A key that isn't a JS name, like a CSS custom property's
                // `--gap`, is quoted.
                let key = if computed || !crate::names::is_identifier(name) {
                    PropertyKey::new_string_literal(SPAN, self.name(name), None, b)
                } else {
                    PropertyKey::new_static_identifier(SPAN, self.name(name), b)
                };
                ObjectPropertyKind::new_object_property(
                    SPAN,
                    PropertyKind::Init,
                    key,
                    value,
                    false,
                    shorthand,
                    computed,
                    b,
                )
            }
            Prop::Spread(value) => ObjectPropertyKind::new_spread_property(SPAN, self.expr(value), b),
        }
    }

    /// oxc prints numbers in their shortest form, like a minifier: `1000`
    /// becomes `1e3`. Whole numbers should read as written, so print their
    /// decimal digits verbatim (as an identifier, which oxc copies as-is); a
    /// negative one gets a real unary minus, so oxc still handles spacing and
    /// parentheses. Anything else (`0.1`) is already shortest.
    fn number(&self, sp: Span, n: f64) -> Expression<'a> {
        if !n.is_finite() || (n == 0.0 && n.is_sign_negative()) {
            return Expression::new_numeric_literal(sp, n, None, NumberBase::Decimal, &self.b);
        }
        // oxc would print the shortest text, `.25`: the digits are JS's own,
        // `0.25`, as `String(n)` writes them.
        let digits = Expression::new_identifier(sp, self.name(&js_number(n.abs())), &self.b);
        if n < 0.0 {
            Expression::new_unary_expression(sp, UnaryOperator::UnaryNegation, digits, &self.b)
        } else {
            digits
        }
    }

    /// Copy a name into oxc's arena.
    fn name(&self, name: &str) -> &'a str {
        self.allocator.alloc_str(name)
    }

    fn label(&self, name: &str) -> LabelIdentifier<'a> {
        LabelIdentifier::new(SPAN, self.name(name), &self.b)
    }
}

/// `new RegExp("%s", "g")` as JS writes it, `/%s/g` (ADR 0243): its pattern
/// and flags as they're written, a `/` escaped where it would end it, when
/// JS parses them as a literal. One it doesn't is made as it runs, to throw
/// then, not as the module is read. A global is never a local's name, so
/// `RegExp` is JS's.
fn regex_literal(callee: &js::Expr, args: &[js::Expr]) -> Option<String> {
    if !matches!(&callee.kind, ExprKind::Var(name) if name == "RegExp") {
        return None;
    }
    let [pattern, rest @ ..] = args else {
        return None;
    };
    let ExprKind::Str(pattern) = &pattern.kind else {
        return None;
    };
    let flags = match rest {
        [] => "",
        [flags] => match &flags.kind {
            ExprKind::Str(flags) => flags.as_str(),
            _ => return None,
        },
        _ => return None,
    };
    if pattern.contains(['\n', '\r', '\u{2028}', '\u{2029}']) {
        return None;
    }
    let mut body = String::new();
    let (mut escaped, mut class) = (false, false);
    for c in pattern.chars() {
        match c {
            '/' if !escaped && !class => body.push('\\'),
            '[' if !escaped => class = true,
            ']' if !escaped => class = false,
            _ => {}
        }
        escaped = c == '\\' && !escaped;
        body.push(c);
    }
    // `//` would be a comment: an empty pattern's literal is JS's own source of it.
    if body.is_empty() {
        body.push_str("(?:)");
    }
    let allocator = Allocator::default();
    LiteralParser::new(&allocator, &body, Some(flags), Options::default())
        .parse()
        .ok()?;
    Some(format!("/{body}/{flags}"))
}

/// Can `s` be JSX text as it is? Braces and angle brackets start JSX, `&` an
/// entity, and JSX drops whitespace at the start or end of a line.
fn jsx_text_safe(s: &str) -> bool {
    !s.contains(['{', '}', '<', '>', '&', '\n', '\r'])
}

fn span(s: js::Span) -> Span {
    Span::new(s.lo, s.hi)
}

/// JS splits binary operators in two: `&&`/`||` are "logical", the rest "binary".
fn binary_op(op: Op) -> Result<BinaryOperator, LogicalOperator> {
    Ok(match op {
        Op::And => return Err(LogicalOperator::And),
        Op::Or => return Err(LogicalOperator::Or),
        Op::Coalesce => return Err(LogicalOperator::Coalesce),
        Op::BitOr => BinaryOperator::BitwiseOR,
        Op::BitXor => BinaryOperator::BitwiseXOR,
        Op::BitAnd => BinaryOperator::BitwiseAnd,
        Op::Eq => BinaryOperator::StrictEquality,
        Op::Ne => BinaryOperator::StrictInequality,
        Op::LooseEq => BinaryOperator::Equality,
        Op::LooseNe => BinaryOperator::Inequality,
        Op::InstanceOf => BinaryOperator::Instanceof,
        Op::In => BinaryOperator::In,
        Op::Lt => BinaryOperator::LessThan,
        Op::Le => BinaryOperator::LessEqualThan,
        Op::Gt => BinaryOperator::GreaterThan,
        Op::Ge => BinaryOperator::GreaterEqualThan,
        Op::Shl => BinaryOperator::ShiftLeft,
        Op::Shr => BinaryOperator::ShiftRight,
        Op::UShr => BinaryOperator::ShiftRightZeroFill,
        Op::Add => BinaryOperator::Addition,
        Op::Sub => BinaryOperator::Subtraction,
        Op::Mul => BinaryOperator::Multiplication,
        Op::Div => BinaryOperator::Division,
        Op::Rem => BinaryOperator::Remainder,
        Op::Pow => BinaryOperator::Exponential,
    })
}

/// A positive finite number as JS's `Number.prototype.toString` writes it:
/// its shortest digits, with an exponent only past 1e21 or below 1e-6.
fn js_number(n: f64) -> String {
    // `{:e}` has the shortest digits that read back as `n`: `2.5e-1`.
    let text = format!("{n:e}");
    let (mantissa, exponent) = text.split_once('e').expect("`{:e}` has an exponent");
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let k = digits.len() as i32;
    // Where the point goes: after `point` digits.
    let point = exponent.parse::<i32>().expect("an integer exponent") + 1;
    if k <= point && point <= 21 {
        format!("{digits}{}", "0".repeat((point - k) as usize))
    } else if 0 < point && point <= 21 {
        format!("{}.{}", &digits[..point as usize], &digits[point as usize..])
    } else if -6 < point && point <= 0 {
        format!("0.{}{digits}", "0".repeat((-point) as usize))
    } else {
        let sign = if point - 1 < 0 { "-" } else { "+" };
        let rest = if k > 1 {
            format!(".{}", &digits[1..])
        } else {
            String::new()
        };
        format!("{}{rest}e{sign}{}", &digits[..1], (point - 1).abs())
    }
}

/// `text` as a template literal writes it: what would end the text or start
/// a value is escaped, and so are controls, as in a string, but a line
/// break where it keeps its `lines`.
fn template_raw(text: &str, lines: bool) -> String {
    let mut raw = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '`' => raw.push_str("\\`"),
            '\\' => raw.push_str("\\\\"),
            '$' if chars.peek() == Some(&'{') => raw.push_str("\\$"),
            '\n' if lines => raw.push('\n'),
            '\n' => raw.push_str("\\n"),
            '\r' => raw.push_str("\\r"),
            '\t' => raw.push_str("\\t"),
            '\u{2028}' => raw.push_str("\\u2028"),
            '\u{2029}' => raw.push_str("\\u2029"),
            c if c.is_control() => raw.push_str(&format!("\\u{:04x}", c as u32)),
            c => raw.push(c),
        }
    }
    raw
}

/// Replace the temporary concatenated source with the actual source files.
fn restore_sources<'a>(
    map: &'a SourceMap<'a>,
    sources: &'a crate::program::Sources,
    paths: &'a std::collections::HashMap<usize, String>,
    primary: Option<usize>,
) -> SourceMap<'a> {
    let mut out = SourceMapBuilder::default();
    let mut ids = std::collections::HashMap::new();
    if let Some(i) = primary {
        ids.insert(i, out.set_source_and_content(&paths[&i], &sources.files[i].text));
    }
    let names: Vec<_> = map.get_names().map(|name| out.add_name(name)).collect();
    for token in map.get_tokens() {
        if token.get_source_id().is_none() {
            out.add_token(token.get_dst_line(), token.get_dst_col(), 0, 0, None, None);
            continue;
        }
        let Some(i) = sources.file_at(token.get_src_line()) else {
            continue;
        };
        let id = *ids
            .entry(i)
            .or_insert_with(|| out.set_source_and_content(&paths[&i], &sources.files[i].text));
        out.add_token(
            token.get_dst_line(),
            token.get_dst_col(),
            token.get_src_line() - sources.files[i].line,
            token.get_src_col(),
            Some(id),
            token.get_name_id().map(|id| names[id as usize]),
        );
    }
    out.into_sourcemap()
}

/// Whether `.property` can be written as it is: a JS name, or a path of
/// them, `target.value`, which lowering writes as one.
fn member_name(property: &str) -> bool {
    property.split('.').all(crate::names::is_identifier)
}

/// Where `code`, a module rust-js wrote, or its `declarations`, names another
/// by its path: each specifier of an `import` and an `export .. from`, what
/// rust-js prints of them, as the byte range of its text between its quotes,
/// and the text. A host that moves the files rewrites these and nothing
/// else (ADR 0101): found by a parse, whatever comes before them.
pub fn module_specifiers(code: &str, declarations: bool) -> Result<Vec<(usize, usize, String)>, String> {
    struct Specifiers(Vec<(usize, usize, String)>);
    impl Specifiers {
        fn add(&mut self, source: &StringLiteral) {
            let (start, end) = (source.span.start as usize + 1, source.span.end as usize - 1);
            self.0.push((start, end, source.value.to_string()));
        }
    }
    impl<'a> Visit<'a> for Specifiers {
        fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
            self.add(&it.source);
        }
        fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
            self.add(&it.source);
        }
    }
    let allocator = Allocator::default();
    let source_type = if declarations {
        SourceType::d_ts()
    } else {
        SourceType::mjs().with_jsx(true)
    };
    let parsed = Parser::new(&allocator, code, source_type).parse();
    if let Some(error) = parsed.diagnostics.errors().next() {
        // Where, in what it wrote: the line the parse stopped at.
        let at = error.labels.iter().next().map_or(0, |label| label.offset() as usize);
        let line = code[..at.min(code.len())].matches('\n').count() + 1;
        let text = code.lines().nth(line - 1).unwrap_or("").trim();
        let what = if declarations { "declarations" } else { "module" };
        return Err(format!(
            "rust-js can't read the {what} it wrote: {error}, at line {line}: `{text}`"
        ));
    }
    let mut found = Specifiers(Vec::new());
    found.visit_program(&parsed.program);
    Ok(found.0)
}

/// Is `e` of a chain with a `?.` in it: a `?.` link, or a member, an index
/// or a call of one.
fn in_chain(e: &js::Expr) -> bool {
    match &e.kind {
        ExprKind::OptionalMember(..) | ExprKind::OptionalIndex(..) | ExprKind::OptionalCall(..) => true,
        ExprKind::Member(object, _) | ExprKind::Index(object, _) | ExprKind::Call(object, _) => in_chain(object),
        _ => false,
    }
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
