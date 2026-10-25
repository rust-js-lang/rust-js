//! Format the JS as oxfmt does, and carry the source map over to it.
//!
//! ```text
//!   printed JS ──oxc_formatter──► formatted JS
//!       │                              │
//!     parse                          parse
//!       ▼                              ▼
//!   nodes in order  ◄──── paired ────► nodes in order
//!
//!   a mapping at a node's start in the printed JS  ──►  the same node's
//!   start in the formatted JS
//! ```
//!
//! The formatter changes where things are, never what they are, so the two
//! programs have the same nodes in the same order. It can add a few (JSX's
//! `{" "}`), which the pairing steps over.

use std::collections::HashMap;

use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast::ast::{Declaration, Statement};
use oxc_ast::ast_kind::AstType;
use oxc_ast_visit::Visit;
use oxc_formatter::{JsFormatOptions, format, parse_for_format};
use oxc_parser::Parser;
use oxc_sourcemap::{OwnedSourceMap, SourceMap, SourceMapBuilder};
use oxc_span::{GetSpan, SourceType};

/// How a JS file of rust-js's is read, by its name: a `.jsx` one as JSX
/// (ADR 0040), as it was written.
fn source_type(js_file_name: &str) -> SourceType {
    if js_file_name.ends_with(".jsx") {
        SourceType::jsx()
    } else {
        SourceType::mjs()
    }
}

/// What the source writes across lines, of a module's JS, in the order a
/// visit of its parse meets each: its objects (ADR 0835), and each value of
/// its template literals (ADR 0641, amended).
#[derive(Default)]
pub(crate) struct Written {
    pub objects: Vec<bool>,
    pub interpolations: Vec<bool>,
}

/// `code` as oxfmt formats it, each of its objects, in the order a visit
/// meets them, on lines of its own where `across_lines` says its source
/// writes it so (ADR 0835), and `map`'s mappings moved to match. `None` if
/// the formatter fails, and the caller keeps what it has.
fn formatted(code: &str, map: &SourceMap<'_>, js_file_name: &str, across_lines: &Written) -> Option<(String, String)> {
    let source_type = source_type(js_file_name);
    let allocator = Allocator::default();
    let written = as_written(&allocator, code, source_type, across_lines)?;
    let text = format(&allocator, &written, source_type, options())
        .ok()?
        .print()
        .ok()?
        .into_code();
    let before = node_starts(&allocator, code, source_type).ok()?;
    let after = node_starts(&allocator, &text, source_type).ok()?;
    let map = moved(code, &text, map, &before, &after, js_file_name);
    Some((text, map))
}

/// `text`, a hook's layout of `code` (ADR 0117), with `map`'s mappings
/// moved to it, if it's the same program: the same nodes, in the same order,
/// but for the ones a formatter adds or takes out, JSX's text and `{" "}`.
/// `map` is JSON. Otherwise, why not.
pub fn transformed(code: &str, text: String, map: &str, js_file_name: &str) -> Result<(String, String), String> {
    let map = SourceMap::from_json_string(map).map_err(|e| format!("was given a map it can't read: {e}"))?;
    let map = &map;
    let source_type = source_type(js_file_name);
    let allocator = Allocator::default();
    let (before, meant) =
        nodes(&allocator, code, source_type).map_err(|e| format!("was given JS it can't read: {e}"))?;
    let (after, means) = nodes(&allocator, &text, source_type).map_err(|e| format!("gave what isn't JS: {e}"))?;
    let kinds = |nodes: Vec<(AstType, u32)>| nodes.into_iter().map(|(kind, _)| kind).collect::<Vec<_>>();
    let (was, is) = (kinds(layout_free(&before, code)), kinds(layout_free(&after, &text)));
    if let Some(at) = (0..was.len().max(is.len())).find(|&i| was.get(i) != is.get(i)) {
        // Taken out, with what's in it, if the rest is what the rest was;
        // added, if what was there is the rest; or else made another.
        let change = match (was.get(at), is.get(at)) {
            (Some(node), _) if was.ends_with(&is[at..]) => format!("took out a `{node:?}`"),
            (_, Some(node)) if is.ends_with(&was[at..]) => format!("added a `{node:?}`"),
            (Some(was), Some(is)) => format!("made a `{was:?}` a `{is:?}`"),
            (Some(node), None) => format!("took out a `{node:?}`"),
            (None, Some(node)) => format!("added a `{node:?}`"),
            (None, None) => unreachable!("the kinds differ at {at}"),
        };
        return Err(format!("changed the program: it {change}"));
    }
    // Of the same nodes, what one means: a name, a value, an operator.
    for (was, is) in layout_free(&before, code).into_iter().zip(layout_free(&after, &text)) {
        if let (Some(a), Some(b)) = (meant.get(&was), means.get(&is))
            && a != b
        {
            return Err(format!("changed the program: it made `{a}` `{b}`, a `{:?}`", was.0));
        }
    }
    let map = moved(code, &text, map, &before, &after, js_file_name);
    Ok((text, map))
}

/// `nodes`, of `code`, but for what a formatter lays out otherwise: JSX's
/// text, and the `{" "}` that stands for its spaces.
fn layout_free(nodes: &[(AstType, u32)], code: &str) -> Vec<(AstType, u32)> {
    let mut kinds = Vec::with_capacity(nodes.len());
    let mut space = false;
    for &(kind, at) in nodes {
        let spaces = space && kind == AstType::StringLiteral;
        space = kind == AstType::JSXExpressionContainer
            && ["{\" \"}", "{' '}"].iter().any(|s| code[at as usize..].starts_with(s));
        if !(space || spaces || kind == AstType::JSXText) {
            kinds.push((kind, at));
        }
    }
    kinds
}

/// `map`, of `code`, moved to `text`, the same program laid out otherwise,
/// by pairing their nodes, `before` and `after`.
fn moved(
    code: &str,
    text: &str,
    map: &SourceMap<'_>,
    before: &[(AstType, u32)],
    after: &[(AstType, u32)],
    js_file_name: &str,
) -> String {
    // A `{" "}` the formatter made text, `{a}{" "}— {b}` to `{a} — {b}`,
    // pairs with nothing: paired, it would pair each node after it with a
    // later one.
    let pairs = pair(&layout_free(before, code), &layout_free(after, text));
    let (old_lines, new_lines) = (Lines::new(code), Lines::new(text));

    let mut out = SourceMapBuilder::default();
    out.set_file(js_file_name);
    for (source, content) in map.get_sources().zip(map.get_source_contents()) {
        out.set_source_and_content(source, content.unwrap_or_default());
    }
    let name_ids: Vec<u32> = map.get_names().map(|name| out.add_name(name)).collect();
    let mut tokens: Vec<_> = map
        .get_tokens()
        .filter_map(|t| {
            old_lines
                .offset(t.get_dst_line(), t.get_dst_col())
                .map(|offset| (offset, t))
        })
        .collect();
    // Where `offset`, of the code, is in the text: as far into the node that
    // starts there, or else the last one before it on the same line.
    let moved_to = |offset: u32| {
        let at = pairs.partition_point(|&(old, _)| old <= offset);
        let &(old, new) = at.checked_sub(1).map(|i| &pairs[i])?;
        if old != offset && code[old as usize..offset as usize].contains('\n') {
            return None;
        }
        Some(new_lines.position(new + (offset - old)))
    };
    // Codegen coalesces identical mappings on one line: `return <button>`,
    // or `() => setCount((count) => ..)`, may have only the first's. A node
    // the formatter moves onto another line, where that mapping no longer
    // applies, is given it at its new place as well.
    let mut carried = Vec::new();
    for &(_, offset) in before {
        let at = tokens.partition_point(|&(old, _)| old <= offset);
        if let Some(&(old, token)) = at.checked_sub(1).map(|i| &tokens[i])
            && old != offset
            && !code[old as usize..offset as usize].contains('\n')
            && let (Some((from, _)), Some((to, _))) = (moved_to(old), moved_to(offset))
            && from != to
        {
            carried.push((offset, token));
        }
    }
    tokens.extend(carried);
    tokens.sort_by_key(|&(offset, _)| offset);
    // In the order of where they are in the text, as a map's tokens are: a
    // node paired with one the formatter moved before its neighbour's would
    // otherwise come after it, and the map be one the encoder can't write.
    let mut placed: Vec<_> = tokens
        .into_iter()
        .filter_map(|(offset, t)| moved_to(offset).map(|at| (at, t)))
        .collect();
    placed.sort_by_key(|&(at, _)| at);
    for ((line, col), t) in placed {
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

/// oxfmt's defaults, Prettier's `objectWrap: "preserve"` among them: an
/// object a line ends in after its `{` stays on lines of its own, as a
/// person wrote it, which `as_written` makes of the source's (ADR 0835).
fn options() -> JsFormatOptions {
    JsFormatOptions::default()
}

/// `code`, the printer's, with a line ending after each of its objects' `{`
/// where `across_lines` says its source writes it across lines, and none
/// there otherwise, which the printer writes of every object of two
/// fields: what oxfmt, and the app's Prettier after it, keep or join
/// (ADR 0835); and a line ending before each template literal's value the
/// source writes across lines, after its `${`, so the formatter may break
/// the line there where it's too long, as Prettier does of a value written
/// so, and keeps one on one line otherwise (ADR 0641, amended).
/// `across_lines` is of each in the order a visit meets them; one of another
/// count is none's. `None` if `code` doesn't parse.
fn as_written(allocator: &Allocator, code: &str, source_type: SourceType, across_lines: &Written) -> Option<String> {
    #[derive(Default)]
    struct Found {
        // Where each object starts, and its first property.
        objects: Vec<(u32, Option<u32>)>,
        // Where each template literal's value starts.
        values: Vec<u32>,
    }
    impl<'a> Visit<'a> for Found {
        fn visit_object_expression(&mut self, it: &oxc_ast::ast::ObjectExpression<'a>) {
            self.objects
                .push((it.span.start, it.properties.first().map(|p| p.span().start)));
            oxc_ast_visit::walk::walk_object_expression(self, it);
        }
        fn visit_template_literal(&mut self, it: &oxc_ast::ast::TemplateLiteral<'a>) {
            self.values.extend(it.expressions.iter().map(|e| e.span().start));
            oxc_ast_visit::walk::walk_template_literal(self, it);
        }
    }
    let parsed = parse_for_format(allocator, allocator.alloc_str(code), source_type);
    if parsed.diagnostics.errors().next().is_some() {
        return None;
    }
    let mut found = Found::default();
    found.visit_program(&parsed.program);
    let of = |across: &[bool], count: usize| match across.len() == count {
        true => across.to_vec(),
        false => vec![false; count],
    };
    // Each edit, where it starts and ends in `code`, and what's written there.
    let mut edits: Vec<(usize, usize, &str)> = Vec::new();
    for (&(start, first), lines) in found.objects.iter().zip(of(&across_lines.objects, found.objects.len())) {
        let Some(first) = first else { continue };
        let (open, first) = (start as usize + 1, first as usize);
        let between = &code[open..first];
        let replaced = match (lines, between.contains('\n')) {
            (true, false) => "\n",
            // A comment there is kept, and where it is.
            (false, true) if between.trim().is_empty() => " ",
            _ => continue,
        };
        edits.push((open, first, replaced));
    }
    for (&start, lines) in found
        .values
        .iter()
        .zip(of(&across_lines.interpolations, found.values.len()))
    {
        let start = start as usize;
        if lines && !code[code[..start].trim_end().len()..start].contains('\n') {
            edits.push((start, start, "\n"));
        }
    }
    edits.sort_by_key(|&(start, ..)| start);
    let mut written = String::with_capacity(code.len() + edits.len());
    let mut past = 0;
    for (start, end, replaced) in edits {
        written.push_str(&code[past..start]);
        written.push_str(replaced);
        past = end;
    }
    written.push_str(&code[past..]);
    Some(written)
}

/// A written file's source map, for finding where its JS came from.
pub struct Map(OwnedSourceMap);

impl Map {
    pub fn read(json: &str) -> Option<Map> {
        OwnedSourceMap::from_json_string(json).ok().map(Map)
    }

    /// The source, as the map names it, and the line and column there, all
    /// 0-based, that the JS at `line` and `col` came from.
    pub fn original(&self, line: u32, col: u32) -> Option<(String, u32, u32)> {
        let table = self.0.generate_lookup_table();
        let token = self.0.lookup_token(&table, line, col)?;
        let source = self.0.get_source(token.get_source_id()?)?;
        Some((source.to_string(), token.get_src_line(), token.get_src_col()))
    }
}

/// Each node's kind and where it starts, in the order a visit meets them,
/// or the first error parsing `code`.
fn node_starts(allocator: &Allocator, code: &str, source_type: SourceType) -> Result<Vec<(AstType, u32)>, String> {
    Ok(nodes(allocator, code, source_type)?.0)
}

/// What each node of `code` means beyond its kind, by its kind and where it
/// starts: a name, a literal's value, a template's text, an operator.
type Meanings = HashMap<(AstType, u32), String>;

/// The nodes of `code`, each its kind and where it starts, in order, and
/// what each means beyond its kind.
fn nodes(
    allocator: &Allocator,
    code: &str,
    source_type: SourceType,
) -> Result<(Vec<(AstType, u32)>, Meanings), String> {
    struct Nodes<'c> {
        code: &'c str,
        starts: Vec<(AstType, u32)>,
        meanings: Meanings,
    }
    impl<'a> Visit<'a> for Nodes<'_> {
        fn enter_node(&mut self, kind: AstKind<'a>) {
            let at = (kind.ty(), kind.span().start);
            self.starts.push(at);
            if let Some(meaning) = meaning(&kind, self.code) {
                self.meanings.insert(at, meaning);
            }
        }
    }
    let parsed = parse_for_format(allocator, allocator.alloc_str(code), source_type);
    if let Some(error) = parsed.diagnostics.errors().next() {
        return Err(error.to_string());
    }
    let mut nodes = Nodes {
        code,
        starts: Vec::new(),
        meanings: HashMap::new(),
    };
    nodes.visit_program(&parsed.program);
    Ok((nodes.starts, nodes.meanings))
}

/// What `kind` means that a layout keeps: a name, a literal's value, a
/// template's text, an operator; `None` of a node that's its kind alone.
fn meaning(kind: &AstKind<'_>, code: &str) -> Option<String> {
    Some(match kind {
        AstKind::IdentifierReference(i) => i.name.to_string(),
        AstKind::BindingIdentifier(i) => i.name.to_string(),
        AstKind::IdentifierName(i) => i.name.to_string(),
        AstKind::LabelIdentifier(i) => i.name.to_string(),
        AstKind::JSXIdentifier(i) => i.name.to_string(),
        AstKind::StringLiteral(s) => s.value.to_string(),
        AstKind::NumericLiteral(n) => n.value.to_string(),
        AstKind::BigIntLiteral(n) => n.value.to_string(),
        AstKind::BooleanLiteral(b) => b.value.to_string(),
        AstKind::RegExpLiteral(r) => code[r.span.start as usize..r.span.end as usize].to_string(),
        AstKind::TemplateElement(t) => t.value.cooked.as_ref().unwrap_or(&t.value.raw).to_string(),
        AstKind::BinaryExpression(e) => e.operator.as_str().to_string(),
        AstKind::LogicalExpression(e) => e.operator.as_str().to_string(),
        AstKind::UnaryExpression(e) => e.operator.as_str().to_string(),
        AstKind::AssignmentExpression(e) => e.operator.as_str().to_string(),
        AstKind::UpdateExpression(e) => format!("{}{}", e.operator.as_str(), e.prefix),
        _ => return None,
    })
}

/// Pair the nodes of the two programs: where the kinds differ, the
/// formatted one has a node the other hasn't, which is stepped over. The
/// pairs are sorted by where they start before, first one kept.
fn pair(before: &[(AstType, u32)], after: &[(AstType, u32)]) -> Vec<(u32, u32)> {
    let mut pairs = Vec::with_capacity(before.len());
    let mut j = 0;
    for &(ty, old) in before {
        // Look a little way ahead for the same kind; past that, this node is
        // one the formatter dropped.
        let Some(k) = after[j..].iter().take(8).position(|&(t, _)| t == ty) else {
            continue;
        };
        pairs.push((old, after[j + k].1));
        j += k + 1;
    }
    pairs.sort_by_key(|&(old, _)| old);
    pairs.dedup_by_key(|&mut (old, _)| old);
    pairs
}

/// Byte offsets and a source map's positions (line, UTF-16 column) in a text.
struct Lines<'t> {
    text: &'t str,
    starts: Vec<u32>,
}

impl<'t> Lines<'t> {
    fn new(text: &'t str) -> Self {
        let starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i as u32 + 1))
            .collect();
        Lines { text, starts }
    }

    fn offset(&self, line: u32, col: u32) -> Option<u32> {
        let start = *self.starts.get(line as usize)?;
        let mut units = 0;
        for (i, c) in self.text[start as usize..].char_indices() {
            if units >= col || c == '\n' {
                return Some(start + i as u32);
            }
            units += c.len_utf16() as u32;
        }
        Some(self.text.len() as u32)
    }

    fn position(&self, offset: u32) -> (u32, u32) {
        let line = self.starts.partition_point(|&s| s <= offset) - 1;
        let start = self.starts[line] as usize;
        let col = self.text[start..offset as usize].encode_utf16().count() as u32;
        (line as u32, col)
    }
}

/// `code` laid out as oxfmt lays it out, rust-js's own layout, and its
/// top-level statements apart as a person separates them (`spaced`); `map`
/// moved to match. The app's Prettier lays it out after, where it has a
/// config (ADR 0381).
pub(crate) fn laid_out(code: String, map: String, js_file_name: &str, across_lines: &Written) -> (String, String) {
    let unformatted = SourceMap::from_json_string(&map).expect("the map just built");
    let (code, map) = formatted(&code, &unformatted, js_file_name, across_lines).unwrap_or((code, map));
    spaced(code, map, js_file_name)
}

/// `code`, laid out, with a blank line between two top-level statements
/// where either is a function or spans lines, but after a `let x;` of no
/// value, as a person separates them,
/// and `map` moved to match. Its imports stay one block (ADR 0295). Read
/// from its parse, so a line of a template literal is never taken for one.
fn spaced(code: String, map: String, js_file_name: &str) -> (String, String) {
    let blank = blank_lines_before(&code);
    if blank.is_empty() {
        return (code, map);
    }
    let mut spaced = String::with_capacity(code.len() + blank.len());
    let mut lines = Vec::new();
    let mut added = 0;
    for (i, line) in code.lines().enumerate() {
        if blank.contains(&i) {
            spaced.push('\n');
            added += 1;
        }
        spaced.push_str(line);
        spaced.push('\n');
        lines.push((i + added) as u32);
    }
    let map = SourceMap::from_json_string(&map).expect("the map just built");
    (spaced, moved_lines(&map, &lines, js_file_name))
}

/// The lines of `code` that start a top-level statement to put a blank
/// line before (`spaced`).
fn blank_lines_before(code: &str) -> std::collections::HashSet<usize> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::mjs().with_jsx(true)).parse();
    if parsed.diagnostics.errors().next().is_some() {
        return Default::default();
    }
    let line_of = |offset: u32| code[..offset as usize].matches('\n').count();
    let function = |statement: &Statement<'_>| match statement {
        Statement::FunctionDeclaration(_) => true,
        Statement::ExportDeclaration(export) => matches!(export.declaration, Declaration::FunctionDeclaration(_)),
        _ => false,
    };
    let lines = |statement: &Statement<'_>| {
        let span = oxc_span::GetSpan::span(statement);
        (line_of(span.start), line_of(span.end))
    };
    // A statement's first line, of the comments right above it if it has
    // them (ADR 0444), each on lines of its own, after the one before.
    let own_line = |at: u32| {
        code[..at as usize]
            .rsplit('\n')
            .next()
            .is_some_and(|before| before.trim().is_empty())
    };
    let comments: Vec<(usize, usize)> = (parsed.program.comments.iter())
        .filter(|c| own_line(c.span.start))
        .map(|c| (line_of(c.span.start), line_of(c.span.end)))
        .collect();
    let top = |mut line: usize| {
        while let Some(&(start, _)) = comments.iter().find(|&&(start, end)| end + 1 == line && start < line) {
            line = start;
        }
        line
    };
    parsed
        .program
        .body
        .windows(2)
        .filter_map(|pair| {
            let ((first, last), (next, next_last)) = (lines(&pair[0]), lines(&pair[1]));
            let imports = matches!(pair[0], Statement::ImportDeclaration(_))
                || matches!(pair[1], Statement::ImportDeclaration(_));
            // `let x;` and what sets it, `if (..) { x = .. }`, are one
            // thing, as a person writes them (ADR 0601).
            let declared_only = matches!(&pair[0], Statement::VariableDeclaration(d)
                if d.declarations.iter().all(|d| d.init.is_none()));
            let apart =
                function(&pair[0]) || function(&pair[1]) || !declared_only && (first < last || next < next_last);
            // So are a function and what sets a property of it,
            // `disabledLog.__reactDisabledLog = true;` (ADR 0673).
            let marked = marks(&pair[0], &pair[1]);
            let apart = apart && !marked;
            let next = top(next);
            (apart && !imports && last + 1 == next).then_some(next)
        })
        .collect()
}

/// Whether `next` is a statement that sets a property of the function
/// `statement` declares, `f.marked = true;`, which a person writes right
/// after it (`blank_lines_before`).
fn marks(statement: &Statement<'_>, next: &Statement<'_>) -> bool {
    use oxc_ast::ast::{AssignmentTarget, Expression};
    let declared = match statement {
        Statement::FunctionDeclaration(f) => f.id.as_ref(),
        Statement::ExportDeclaration(export) => match &export.declaration {
            Declaration::FunctionDeclaration(f) => f.id.as_ref(),
            _ => None,
        },
        _ => None,
    };
    let Statement::ExpressionStatement(set) = next else {
        return false;
    };
    let Expression::AssignmentExpression(assign) = &set.expression else {
        return false;
    };
    let object = match &assign.left {
        AssignmentTarget::StaticMemberExpression(member) => &member.object,
        AssignmentTarget::ComputedMemberExpression(member) => &member.object,
        _ => return false,
    };
    matches!((declared, object), (Some(id), Expression::Identifier(of)) if of.name == id.name)
}

/// `map` of code each of whose lines `lines` moves, line `i` to `lines[i]`;
/// past the last, each moved as the last is.
pub(crate) fn moved_lines(map: &SourceMap<'_>, lines: &[u32], js_file_name: &str) -> String {
    let last_shift = lines.last().map_or(0, |&last| last + 1 - lines.len() as u32);
    moved_positions(
        map,
        |line, column| (lines.get(line as usize).copied().unwrap_or(line + last_shift), column),
        js_file_name,
    )
}

/// `map` of code each of whose places `to` moves, a line and a column to
/// another: a line split in two, its rest a line of its own (ADR 0658).
pub(crate) fn moved_positions(map: &SourceMap<'_>, to: impl Fn(u32, u32) -> (u32, u32), js_file_name: &str) -> String {
    let mut out = SourceMapBuilder::default();
    out.set_file(js_file_name);
    for (source, content) in map.get_sources().zip(map.get_source_contents()) {
        out.set_source_and_content(source, content.unwrap_or_default());
    }
    // `add_name` deduplicates, so ids can change: translate them.
    let name_ids: Vec<u32> = map.get_names().map(|name| out.add_name(name)).collect();
    for t in map.get_tokens() {
        let (line, column) = to(t.get_dst_line(), t.get_dst_col());
        out.add_token(
            line,
            column,
            t.get_src_line(),
            t.get_src_col(),
            t.get_source_id(),
            t.get_name_id().map(|id| name_ids[id as usize]),
        );
    }
    out.into_sourcemap().to_json_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Every snapshot of generated JS is as oxfmt would leave it: formatting
    /// it again changes nothing.
    #[test]
    fn snapshots_are_formatted() {
        let mut checked = 0;
        let mut dirs = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("test/snapshots")];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).expect("the snapshots") {
                let path = entry.expect("an entry").path();
                let jsx = path.extension().is_some_and(|e| e == "jsx");
                if path.is_dir() {
                    dirs.push(path);
                } else if jsx || path.extension().is_some_and(|e| e == "js") {
                    let text = std::fs::read_to_string(&path).expect("a snapshot");
                    // Up to the source map's comment, which comes after the formatting.
                    let code = text
                        .rsplit_once("//# sourceMappingURL=")
                        .map_or(text.as_str(), |(code, _)| code);
                    let allocator = Allocator::default();
                    let again = format(&allocator, code, source_type(&path.to_string_lossy()), options())
                        .expect("it parses")
                        .print()
                        .expect("it prints")
                        .into_code();
                    assert_eq!(again, code, "{} changes when formatted again", path.display());
                    checked += 1;
                }
            }
        }
        assert!(checked > 20, "only {checked} snapshots found");
    }
}
