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
//!              +  code, blank line between functions
//!                                             (map shifted to match)
//!   ──format.rs──► as oxfmt formats it, the map moved to match
//!              +  //# sourceMappingURL=...
//! ```
//!
//! The trick that makes the map point at Rust: oxc computes line/column for
//! each node from `program.source_text` and the node's span. We hand it the
//! *Rust* file as `source_text`, and spans that are byte offsets into it.

use std::cell::Cell;
use std::path::PathBuf;

use oxc_allocator::{Allocator, ArenaBox, ArenaVec};
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, ArrowFunctionBody, AssignmentTarget, BindingIdentifier, BindingPattern,
    BindingProperty, BindingRestElement, BlockStatement, CallExpression, CatchClause, CatchParameter, ChainElement,
    ComputedMemberExpression, Declaration, ExportFromDeclaration, Expression, ForStatementInit, ForStatementLeft,
    FormalParameter, FormalParameterKind, FormalParameters, FunctionBody, FunctionType, IdentifierName,
    ImportDeclaration, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXChild, JSXClosingElement,
    JSXClosingFragment, JSXElementName, JSXExpression, JSXIdentifier, JSXMemberExpressionObject, JSXOpeningElement,
    JSXOpeningFragment, LabelIdentifier, ObjectPropertyKind, Program, PropertyKey, PropertyKind,
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

/// Print `module` as JS, with a source map pointing into `rust_source`.
///
/// `source_path` is how the map names the Rust file (relative to the map),
/// and `js_file_name` is the output's file name, for `sourceMappingURL`.
/// `path_of` names a source the map points into, asked only for those it
/// does; `primary` is the module's own, which the map names first.
/// `format` is the formatter's options, and `layout` what the crate's
/// transforms make of the formatted text and its map (ADR 0117).
#[allow(clippy::too_many_arguments)]
pub fn emit(
    module: &Module,
    sources: &crate::program::Sources,
    path_of: &dyn Fn(usize) -> String,
    primary: Option<usize>,
    source_path: &str,
    js_file_name: &str,
    format: &crate::settings::Format,
    layout: &dyn Fn(String, String) -> (String, String),
) -> Output {
    let rust_source = sources.text.as_str();
    let allocator = Allocator::default();
    let cx = Cx {
        b: AstBuilder::new(&allocator),
        allocator: &allocator,
        depth: Cell::new(0),
        inline: Cell::new(false),
    };
    let b = &cx.b;

    let namespaces = module.namespaces.iter().map(|n| cx.namespace(n));
    let consts = module.consts.iter().map(|c| cx.constant(c));
    let body = ArenaVec::from_iter_in(
        namespaces
            .chain(consts)
            .chain(cx.stmts(&module.statements))
            .chain(module.functions.iter().map(|f| cx.function(f))),
        b,
    );
    let program = Program::new(
        Span::new(0, rust_source.len() as u32),
        SourceType::mjs(),
        rust_source,
        ArenaVec::new_in(b), // comments
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

    // The header, imports and runtime helpers are plain text above the
    // generated code. None of them map to Rust.
    let mut code = format!("{}\n", module.header);
    for directive in &module.directives {
        code.push_str(&format!("\n{directive:?};\n"));
    }
    let packages = &module.packages;
    let mut helpers: Vec<&str> = module.helpers.clone();
    helpers.sort_unstable();
    helpers.dedup();
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
        let relative = package.from.starts_with('.');
        if !clause.is_empty() {
            let line = format!("import {} from {:?};\n", clause.join(", "), package.from);
            named.push((relative, &package.from, line));
        }
        if let Some(namespace) = &package.namespace {
            let line = format!("import * as {namespace} from {:?};\n", package.from);
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
        let line = format!("import {clause} from {:?};\n", import.from);
        named.push((import.from.starts_with('.'), &import.from, line));
    }
    named.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    let runtime = (!helpers.is_empty()).then(|| {
        format!(
            "import {{ {} }} from {:?};\n",
            helpers.join(", "),
            crate::runtime::PACKAGE
        )
    });
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

    // oxc prints functions back to back; put a blank line between them.
    // oxc also puts an object of one property on one line, so a type with one
    // method comes out `const Tally = { doubled(tally) {`: lay that out as an
    // object of several, the method on its own lines. Record where each part
    // of each generated line ends up, to fix the map.
    let one_method: Vec<String> = module
        .namespaces
        .iter()
        .filter(|n| n.methods.len() == 1)
        .map(|n| format!("{}const {} = {{ ", if n.export { "export " } else { "" }, n.name))
        .collect();
    let mut out_line = code.matches('\n').count() as u32;
    let mut places: Vec<Vec<Place>> = Vec::new();
    let mut previous = None;
    let mut in_object = false;
    for (i, line) in generated.code.lines().enumerate() {
        // Only functions start at column 0, so this can't match nested code.
        let top_level = [
            "function ",
            "async function ",
            "export function ",
            "export async function ",
        ]
        .iter()
        .any(|p| line.starts_with(p));
        // And after a type's methods, which end the object that holds them.
        if i > 0 && (top_level || matches!(previous, Some("};" | "} };"))) {
            code.push('\n');
            out_line += 1;
        }
        let mut put = |text: &str, from_col: u32, delta: i64, parts: &mut Vec<Place>| {
            code.push_str(text);
            code.push('\n');
            parts.push(Place {
                from_col,
                line: out_line,
                delta,
            });
            out_line += 1;
        };
        let mut parts = Vec::new();
        let opening = one_method
            .iter()
            .find(|prefix| line.starts_with(prefix.as_str()) && line.ends_with('{'));
        match opening {
            Some(prefix) if !in_object => {
                let at = prefix.len() as u32;
                put(prefix.trim_end(), 0, 0, &mut parts);
                put(
                    &format!("  {}", &line[prefix.len()..]),
                    at,
                    2 - i64::from(at),
                    &mut parts,
                );
                in_object = true;
            }
            _ if in_object && line == "} };" => {
                put("  }", 0, 2, &mut parts);
                put("};", 1, -2, &mut parts);
                in_object = false;
            }
            _ if in_object => put(&format!("  {line}"), 0, 2, &mut parts),
            _ => put(line, 0, 0, &mut parts),
        }
        places.push(parts);
        previous = Some(line);
    }
    // After everything it maps, so the map's lines stay where they are.
    if let Some(function) = &module.default_export {
        code.push_str(&format!("\nexport default {function};\n"));
    }
    let map = generated.map.expect("a source map, since source_map_path is set");
    // The sources the map points into, and only those, named from here.
    let source_of = |line: u32| sources.files.partition_point(|file| file.line <= line).checked_sub(1);
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
    let mut map = shift_lines(&map, &places, js_file_name);
    // Laid out as oxfmt lays it out, with the crate's options (ADR 0117), the
    // map moved to match. They were checked as they were read.
    let shifted = SourceMap::from_json_string(&map).expect("the map just built");
    let options = crate::format::options_of(format).expect("options checked as they were read");
    if let Some((formatted, formatted_map)) =
        crate::format::formatted(&code, &shifted, js_file_name.ends_with(".jsx"), js_file_name, &options)
    {
        code = formatted;
        map = formatted_map;
    }
    (code, map) = layout(code, map);
    code.push_str(&format!("//# sourceMappingURL={js_file_name}.map\n"));
    Output { code, map }
}

/// Where the part of a generated line from `from_col` on ends up: on output
/// line `line`, `delta` columns over.
struct Place {
    from_col: u32,
    line: u32,
    delta: i64,
}

/// Rebuild `map` with each part of each generated line where `places` says.
fn shift_lines(map: &SourceMap<'_>, places: &[Vec<Place>], js_file_name: &str) -> String {
    let mut out = SourceMapBuilder::default();
    out.set_file(js_file_name);
    for (source, content) in map.get_sources().zip(map.get_source_contents()) {
        out.set_source_and_content(source, content.unwrap_or_default());
    }
    // `add_name` deduplicates, so ids can change: translate them.
    let name_ids: Vec<u32> = map.get_names().map(|name| out.add_name(name)).collect();
    // Past the last line, lines keep the last one's shift.
    let last_shift = places
        .last()
        .and_then(|parts| parts.last())
        .map_or(0, |p| p.line + 1 - places.len() as u32);
    for t in map.get_tokens() {
        let (line, col) = (t.get_dst_line(), t.get_dst_col());
        let (line, col) = match places
            .get(line as usize)
            .and_then(|parts| parts.iter().rev().find(|p| p.from_col <= col))
        {
            Some(place) => (place.line, (i64::from(col) + place.delta).max(0) as u32),
            None => (line + last_shift, col),
        };
        out.add_token(
            line,
            col,
            t.get_src_line(),
            t.get_src_col(),
            t.get_source_id(),
            t.get_name_id().map(|id| name_ids[id as usize]),
        );
    }
    out.into_sourcemap().to_json_string()
}

struct Cx<'a> {
    b: AstBuilder<'a>,
    allocator: &'a Allocator,
    /// How many levels oxc indents what's being converted: one per block,
    /// and per array of 3 or more items or object of 2 or more fields, which
    /// oxc puts on several lines. JSX laid out on several lines indents to match.
    depth: Cell<u32>,
    /// Inside JSX that's on one line, where everything stays on it.
    inline: Cell<bool>,
}

impl<'a> Cx<'a> {
    fn params(&self, kind: FormalParameterKind, patterns: &[js::Pattern]) -> FormalParameters<'a> {
        let b = &self.b;
        let params = patterns.iter().map(|pattern| {
            FormalParameter::new(
                SPAN,
                ArenaVec::new_in(b),
                self.pattern(pattern),
                None,
                None,
                false,
                None,
                false,
                false,
                b,
            )
        });
        FormalParameters::new(SPAN, kind, ArenaVec::from_iter_in(params, b), None, b)
    }

    fn function(&self, f: &js::Function) -> Statement<'a> {
        let b = &self.b;
        let params = self.params(FormalParameterKind::FormalParameter, &f.params);
        let body = FunctionBody::new(SPAN, ArenaVec::new_in(b), self.stmts(&f.body), b);
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
        let methods = self.nested(true, || {
            ArenaVec::from_iter_in(
                n.methods.iter().map(|f| {
                    let params = self.params(FormalParameterKind::FormalParameter, &f.params);
                    let body = FunctionBody::new(SPAN, ArenaVec::new_in(b), self.stmts(&f.body), b);
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
                }),
                b,
            )
        });
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
        self.nested(true, || {
            let mut out = ArenaVec::new_in(&self.b);
            self.push_stmts(stmts, &mut out);
            out
        })
    }

    /// A test, which JS makes a `bool` of itself: `!!a` is `a` there, and
    /// in what `&&` and `||` test of it.
    fn test(&self, e: &js::Expr) -> Expression<'a> {
        self.expr(&tested(e))
    }

    /// `stmts`, each pushed to `out`. An `if` whose branch leaves has no
    /// `else`: what was in it follows, as JS writes it, `if (c) { return a; }
    /// return b;` (ADR 0237). Its locals stay apart, as every local of a
    /// function has a name of its own.
    fn push_stmts(&self, stmts: &[js::Stmt], out: &mut ArenaVec<'a, Statement<'a>>) {
        for s in stmts {
            match &s.kind {
                StmtKind::If(cond, then, Some(els)) if leaves(then) => {
                    out.push(Statement::new_if_statement(
                        span(s.span),
                        self.test(cond),
                        self.block(then),
                        None,
                        &self.b,
                    ));
                    self.push_stmts(els, out);
                }
                _ => out.push(self.stmt(s)),
            }
        }
    }

    /// Run `f` one level deeper, if `deeper`.
    fn nested<T>(&self, deeper: bool, f: impl FnOnce() -> T) -> T {
        let depth = self.depth.get();
        self.depth.set(depth + u32::from(deeper));
        let result = f();
        self.depth.set(depth);
        result
    }

    fn block(&self, stmts: &[js::Stmt]) -> Statement<'a> {
        Statement::new_block_statement(SPAN, self.stmts(stmts), &self.b)
    }

    fn stmt(&self, s: &js::Stmt) -> Statement<'a> {
        let b = &self.b;
        let sp = span(s.span);
        match &s.kind {
            StmtKind::Const(name, init) => self.declare(sp, VariableDeclarationKind::Const, name, Some(init)),
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
                // `s = s + t` is `s += t`, as JS writes a string built up, and
                // `r = r * 2` is `r *= 2`.
                let compound = |op: &Op| match op {
                    Op::Add => Some(AssignmentOperator::Addition),
                    Op::Sub => Some(AssignmentOperator::Subtraction),
                    Op::Mul => Some(AssignmentOperator::Multiplication),
                    Op::Div => Some(AssignmentOperator::Division),
                    Op::Rem => Some(AssignmentOperator::Remainder),
                    Op::Coalesce => Some(AssignmentOperator::LogicalNullish),
                    _ => None,
                };
                let (operator, value) = match &value.kind {
                    ExprKind::Binary(op, lhs, rhs) if same_place(lhs, target) && compound(op).is_some() => {
                        (compound(op).unwrap_or(AssignmentOperator::Assign), &**rhs)
                    }
                    _ => (AssignmentOperator::Assign, value),
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
                Statement::new_if_statement(sp, self.test(cond), self.block(then), els, b)
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
            // `return;` of `undefined`, as JS ends a function with nothing to give
            // (ADR 0299).
            StmtKind::Return(value) => Statement::new_return_statement(
                sp,
                value
                    .as_ref()
                    .filter(|v| !matches!(v.kind, ExprKind::Undefined))
                    .map(|v| self.expr(v)),
                b,
            ),
        }
    }

    fn pattern(&self, pattern: &js::Pattern) -> BindingPattern<'a> {
        let b = &self.b;
        let name = |name: &str| BindingPattern::new_binding_identifier(SPAN, self.name(name), b);
        match pattern {
            js::Pattern::Name(n) => name(n),
            js::Pattern::Array(items) => {
                let items = items.iter().map(|item| item.as_deref().map(name));
                BindingPattern::new_array_pattern(SPAN, ArenaVec::from_iter_in(items, b), None, b)
            }
            js::Pattern::Object(fields, rest) => {
                let fields = fields.iter().map(|(field, var, default)| {
                    // A field that isn't a JS name, `data-platform`, is quoted.
                    let key = if js_identifier(field) {
                        PropertyKey::new_static_identifier(SPAN, self.name(field), b)
                    } else {
                        PropertyKey::new_string_literal(SPAN, self.name(field), None, b)
                    };
                    // `{ size = "md" }`, where it's missing (ADR 0212).
                    let value = match default {
                        Some(default) => BindingPattern::new_assignment_pattern(SPAN, name(var), self.expr(default), b),
                        None => name(var),
                    };
                    // `{ x }` for `{ x: x }`.
                    BindingProperty::new(SPAN, key, value, field == var, false, b)
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
            ExprKind::Index(object, index) => ChainElement::ComputedMemberExpression(ComputedMemberExpression::boxed(
                sp,
                self.chain_object(object),
                self.expr(index),
                false,
                b,
            )),
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
            ExprKind::Handle(place) => self.handle(sp, place, None),
            ExprKind::Pair(place, dictionary) => self.handle(sp, place, Some(dictionary)),
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
            ExprKind::Undefined => Expression::new_identifier(sp, "undefined", b),
            ExprKind::Null => Expression::new_null_literal(sp, b),
            ExprKind::Symbol(_) => unreachable!("linking resolves every module symbol before emission"),
            ExprKind::OptionalMember(..) | ExprKind::OptionalCall(..) => {
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
            ExprKind::Array(items) => self.nested(items.len() > 2, || {
                let items = items.iter().map(|item| match &item.kind {
                    ExprKind::Spread(all) => {
                        ArrayExpressionElement::new_spread_element(span(item.span), self.expr(all), b)
                    }
                    _ => ArrayExpressionElement::from(self.expr(item)),
                });
                Expression::new_array_expression(sp, ArenaVec::from_iter_in(items, b), b)
            }),
            ExprKind::Object(props) => self.nested(props.len() > 1, || {
                let props = props.iter().map(|prop| self.property(prop));
                Expression::new_object_expression(sp, ArenaVec::from_iter_in(props, b), b)
            }),
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
            // `a ? a : b` is `a || b`: `a` where it's truthy, else `b`.
            ExprKind::Cond(test, then, els)
                if js::same_path(&tested(test), then) && !matches!(els.kind, ExprKind::Undefined) =>
            {
                Expression::new_logical_expression(sp, self.expr(then), LogicalOperator::Or, self.expr(els), b)
            }
            ExprKind::Cond(test, then, els) => {
                Expression::new_conditional_expression(sp, self.test(test), self.expr(then), self.expr(els), b)
            }
            ExprKind::Arrow(params, body) | ExprKind::AsyncArrow(params, body) => {
                let is_async = matches!(e.kind, ExprKind::AsyncArrow(..));
                let params = ArenaBox::new_in(self.params(FormalParameterKind::ArrowFormalParameters, params), b);
                // `() => x` when the body only returns a value.
                let body = match body.as_slice() {
                    [
                        js::Stmt {
                            kind: StmtKind::Return(Some(value)),
                            ..
                        },
                    ] => ArrowFunctionBody::from(self.expr(value)),
                    _ => ArrowFunctionBody::new_function_body(SPAN, ArenaVec::new_in(b), self.stmts(body), b),
                };
                Expression::new_arrow_function_expression(sp, is_async, None, params, None, body, b)
            }
            // `function Label(props) { .. }`, named (ADR 0296).
            ExprKind::Function(f) => {
                let params = self.params(FormalParameterKind::FormalParameter, &f.params);
                let body = FunctionBody::new(SPAN, ArenaVec::new_in(b), self.stmts(&f.body), b);
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

    /// A JSX element (ADR 0040), laid out as by hand: children that are all
    /// elements or expressions go on their own lines, one level deeper. With
    /// text among them they stay on one line, where JSX keeps every space.
    fn jsx(&self, sp: Span, jsx: &js::Jsx) -> Expression<'a> {
        let b = &self.b;
        let has_text = jsx.children.iter().any(|c| matches!(c.kind, ExprKind::Str(_)));
        let lines = !has_text && !self.inline.get() && jsx.children.iter().any(js::Expr::contains_jsx);
        let depth = self.depth.get();
        let mut children = ArenaVec::new_in(b);
        let inline = self.inline.replace(self.inline.get() || has_text);
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
        self.nested(lines, || {
            for (child, &text) in jsx.children.iter().zip(&text) {
                if lines {
                    children.push(self.jsx_newline(depth + 1));
                }
                children.push(self.jsx_child(child, text));
            }
        });
        self.inline.set(inline);
        if lines {
            children.push(self.jsx_newline(depth));
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
    /// An attribute that's `undefined` (a `None`) is left out, as React would.
    fn jsx_attribute(&self, prop: &Prop) -> Option<JSXAttributeItem<'a>> {
        let b = &self.b;
        let (name, value) = match prop {
            Prop::Spread(value) => return Some(JSXAttributeItem::new_spread_attribute(SPAN, self.expr(value), b)),
            Prop::Field(name, value) | Prop::Getter(name, value) => (name, value),
        };
        let sp = span(value.span);
        let value = match &value.kind {
            ExprKind::Undefined => return None,
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

    /// A line break and indentation between children, which JSX ignores.
    fn jsx_newline(&self, depth: u32) -> JSXChild<'a> {
        let text = format!("\n{}", "  ".repeat(depth as usize));
        JSXChild::new_text(SPAN, self.name(&text), None, &self.b)
    }

    /// A handle (ADR 0099): `{ get value() { return x; }, set value(value) { x = value; } }`,
    /// the setter's parameter named as the place doesn't name anything.
    /// A handle on `place`, and a `dyn`'s pair with `dictionary` its `impl`.
    fn handle(&self, sp: Span, place: &js::Expr, dictionary: Option<&js::Expr>) -> Expression<'a> {
        let b = &self.b;
        let mut param = "value".to_string();
        if place.mentions_var(&param) {
            param = "newValue".to_string();
        }
        while place.mentions_var(&param) {
            param.push('_');
        }
        let get = self.accessor(
            PropertyKind::Get,
            "value",
            &[],
            &[js::StmtKind::Return(Some(place.clone())).at(place.span)],
        );
        let set = self.accessor(
            PropertyKind::Set,
            "value",
            &[js::Pattern::Name(param.clone())],
            &[js::StmtKind::Assign(place.clone(), js::Expr::var(&param)).at(place.span)],
        );
        let dictionary = dictionary.map(|d| self.property(&Prop::Field("impl".into(), d.clone())));
        Expression::new_object_expression(
            sp,
            ArenaVec::from_iter_in(dictionary.into_iter().chain([get, set]), b),
            b,
        )
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
        let body = FunctionBody::new(SPAN, ArenaVec::new_in(b), self.stmts(body), b);
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
                let key = if computed || !js_identifier(name) {
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

/// Are `a` and `b` the same variable or property path: `s`, `a.b`?
fn same_place(a: &js::Expr, b: &js::Expr) -> bool {
    match (&a.kind, &b.kind) {
        (ExprKind::Var(a), ExprKind::Var(b)) => a == b,
        (ExprKind::Member(a, x), ExprKind::Member(b, y)) => x == y && same_place(a, b),
        // `$index(v, i).hits`: an item, checked the same each time (ADR 0056).
        (ExprKind::Call(f, xs), ExprKind::Call(g, ys)) => {
            matches!((&f.kind, &g.kind), (ExprKind::Var(f), ExprKind::Var(g)) if f == "$index" && g == "$index")
                && xs.len() == ys.len()
                && xs.iter().zip(ys).all(|(x, y)| {
                    same_place(x, y) || matches!((&x.kind, &y.kind), (ExprKind::Num(x), ExprKind::Num(y)) if x == y)
                })
        }
        _ => false,
    }
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
        let position = sources.files.partition_point(|file| file.line <= token.get_src_line());
        let Some(i) = position.checked_sub(1) else { continue };
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

/// Does running `stmts` always leave, by a `return` or a `throw` last, or an
/// `if` both of whose branches do?
fn leaves(stmts: &[js::Stmt]) -> bool {
    match stmts.last().map(|s| &s.kind) {
        Some(StmtKind::Return(_) | StmtKind::Throw(_)) => true,
        Some(StmtKind::If(_, then, Some(els))) => leaves(then) && leaves(els),
        _ => false,
    }
}

/// Is `name` a JS name, which a key may be as it is?
/// Whether `.property` can be written as it is: a JS name, or a path of
/// them, `target.value`, which lowering writes as one.
fn member_name(property: &str) -> bool {
    property.split('.').all(js_identifier)
}

fn js_identifier(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

/// `e` as a test: `!!a` is `a`, in the parts of `&&` and `||` too, whose
/// value the test only asks the truth of.
fn tested(e: &js::Expr) -> js::Expr {
    match &e.kind {
        ExprKind::Unary(UnaryOp::Not, inner) if let ExprKind::Unary(UnaryOp::Not, value) = &inner.kind => tested(value),
        ExprKind::Binary(op @ (Op::And | Op::Or), a, b) => js::Expr {
            kind: ExprKind::Binary(*op, Box::new(tested(a)), Box::new(tested(b))),
            span: e.span,
        },
        _ => e.clone(),
    }
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
        return Err(format!("rust-js can't read the module it wrote: {error}"));
    }
    let mut found = Specifiers(Vec::new());
    found.visit_program(&parsed.program);
    Ok(found.0)
}

/// Is `e` of a chain with a `?.` in it: a `?.` link, or a member, an index
/// or a call of one.
fn in_chain(e: &js::Expr) -> bool {
    match &e.kind {
        ExprKind::OptionalMember(..) | ExprKind::OptionalCall(..) => true,
        ExprKind::Member(object, _) | ExprKind::Index(object, _) | ExprKind::Call(object, _) => in_chain(object),
        _ => false,
    }
}
