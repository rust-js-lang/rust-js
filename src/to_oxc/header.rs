//! What's above a module's code, and its imports and re-exports.
//!
//! The header is plain text the map doesn't place: rust-js's line, the
//! file's leading comments and its directives. Each import and re-export is
//! a statement of the module's program, at the `use` that places it, its
//! comments before it, so the one rule of every statement's blank lines is
//! theirs too (`to_oxc::apart`, ADR 0865, amended, rd637), and an item
//! written between two `use`s is between their imports (rd652). ES hoists
//! imports and re-exports, so where one is among the statements changes
//! nothing the module does (ADR 0262's case "None"); their order among
//! themselves is their load order (ADR 1063).

use std::collections::HashMap;

use oxc_allocator::ArenaVec;
use oxc_ast::ast::{
    BindingIdentifier, ExportSpecifier, ImportDeclarationSpecifier, ImportOrExportKind, ModuleExportName, Statement,
    StringLiteral,
};
use oxc_span::{GetSpan, SPAN};

use super::{Cx, span};
use crate::js::{self, Module};

/// What's above a module's code: its header, the module's own `//!`s,
/// `docs`, and its directives, then a blank line, but where the Rust has
/// none right after its `//!`s, or after leading comments right above the
/// code of a module whose JS doesn't start with an import, `imports`, as
/// the source has none (#419).
pub(super) fn header(module: &Module, imports: bool, docs: &Docs) -> String {
    let mut code = format!("{}\n", module.header);
    // The module's `//!`s, then what's next right after them where the
    // Rust has no blank line after them (#486).
    let mut tight = false;
    if !docs.lines.is_empty() {
        code.push('\n');
        code.push_str(&docs.lines.iter().map(|line| format!("{line}\n")).collect::<String>());
        tight = !docs.apart;
    }
    for directive in &module.directives {
        if !std::mem::take(&mut tight) {
            code.push('\n');
        }
        code.push_str(&format!("{directive:?};\n"));
    }
    let on_code = module.header_on_code && !imports && module.directives.is_empty() && docs.lines.is_empty();
    if !(on_code || tight) {
        code.push('\n');
    }
    code
}

/// A module's own inner doc comments, `//!` or `/*!`: its file's
/// description, one JSDoc, as upstream's files write theirs (rd644), above
/// its JS, below rust-js's line and the file's leading comments, and above
/// its directives and imports (#486, ADR 0444, amended). A blank line after
/// them, `apart`, where the Rust has one right after them.
#[derive(Default)]
pub(super) struct Docs {
    lines: Vec<String>,
    apart: bool,
}

/// `own`'s `Docs`, each where it is in `text`: a run of `//!` lines one
/// `/** */`, as a `///` run is its item's (`comments::jsdoc`), runs a blank
/// line apart its paragraphs, ` *` between; a `/*!` block `/**`. Where a
/// line holds the `*/` that would end it, each line as JS writes it, `//`
/// (`comments::in_js`), a blank line between two where the Rust has one.
pub(super) fn docs(own: &[crate::program::Comment], text: &str) -> Docs {
    let blank_after = |hi: u32| {
        let rest = &text[hi as usize..];
        rest[..rest.len() - rest.trim_start().len()].matches('\n').count() > 1
    };
    let written = |c: &crate::program::Comment| &text[c.lo as usize..c.hi as usize];
    let mut lines = Vec::new();
    let mut i = 0;
    while i < own.len() {
        if i > 0 && blank_after(own[i - 1].hi) {
            lines.push(String::new());
        }
        let run = (own[i..].iter()).take_while(|c| written(c).starts_with("//!")).count();
        let Some(block) = written(&own[i]).strip_prefix("/*!") else {
            let run = &own[i..i + run];
            let mut said = Vec::new();
            for (j, c) in run.iter().enumerate() {
                if j > 0 && blank_after(run[j - 1].hi) {
                    said.push("");
                }
                said.push(&written(c)["//!".len()..]);
            }
            match crate::comments::jsdoc(&said) {
                Some(jsdoc) => lines.push(jsdoc),
                None => {
                    for (j, c) in run.iter().enumerate() {
                        if j > 0 && blank_after(run[j - 1].hi) {
                            lines.push(String::new());
                        }
                        lines.push(crate::comments::in_js(written(c)));
                    }
                }
            }
            i += run.len();
            continue;
        };
        lines.push(format!("/**{block}"));
        i += 1;
    }
    Docs {
        apart: own.last().is_some_and(|last| blank_after(last.hi)),
        lines,
    }
}

/// What one import or re-export statement says.
enum Says<'m> {
    /// `import d, { a, b as c } from`, each `(export, local)`.
    Named(Option<&'m str>, Vec<(&'m str, &'m str)>),
    /// `import * as ns from`, the module itself (rd114, ADR 1018).
    Namespace(&'m str),
    /// `import "x";`, for its effect.
    Effect,
    /// `export { a, b as c } from`, each `(export, alias)`, and where its
    /// `use` writes it (`Reexport::written`).
    Reexport(Vec<(&'m str, &'m str, (js::Span, js::Span))>),
    /// `export * as ns from` (ADR 0240, amended); `export * from`, of
    /// none, a glob's of every name (#593).
    ReexportAll(Option<&'m str>),
}

/// One of a module's imports and re-exports, in their order (`entries`):
/// what it says, from where, where the source places it, and the `use` its
/// comments are before.
pub(super) struct Entry<'m> {
    says: Says<'m>,
    from: &'m str,
    /// Where it's placed (ADR 1063, amended): the first `use` that makes it,
    /// or a binding the module declares of it (`Package::named_at`), or
    /// where a re-export is placed (`Reexport::at`); `NONE` of none.
    pub placed: js::Span,
    /// The `use` whose comments are above it (rd92); `NONE` of none.
    commented: js::Span,
    /// Whether it's placed at a binding the module declares, not a `use`.
    declared: bool,
}

impl Entry<'_> {
    /// Its statement's span: where it's placed, or else where its `use` is;
    /// of a binding's declaration, which is no `use`, where it starts, of no
    /// width, so the map places nothing there and no blank line is told
    /// after it, a function item's, as none is between `use`s that have
    /// none (rd637).
    fn at(&self) -> js::Span {
        match self.placed.is_none() {
            true => self.commented,
            false if self.declared => js::Span {
                lo: self.placed.lo,
                hi: self.placed.lo,
            },
            false => self.placed,
        }
    }
}

/// A module's imports and re-exports, in the order JS loads them (ADR
/// 1063): each where the first `use` that makes it is, or a binding the
/// module declares of it, as JS loads its modules in its imports' order;
/// one the source names neither way after, packages then relative ones,
/// each by its specifier; then what it imports for its effect, as written,
/// its CSS after what it styles; then the runtime's helpers. Each
/// re-export where it's placed among them (`Reexport::at`), after an import
/// placed there too, JS's load order either way (rd620).
pub(super) fn entries(module: &Module) -> Vec<Entry<'_>> {
    let mut named: Vec<(bool, Entry<'_>)> = Vec::new();
    let mut effects = Vec::new();
    fn pairs<'m>(pairs: impl Iterator<Item = &'m (String, String)>) -> Vec<(&'m str, &'m str)> {
        pairs.map(|(export, local)| (export.as_str(), local.as_str())).collect()
    }
    for package in &module.packages {
        let relative = crate::program::is_relative_specifier(&package.from);
        let entry = |says| Entry {
            says,
            from: &package.from,
            placed: package.named_at,
            commented: package.span,
            declared: package.named_at != package.span,
        };
        let names = pairs(package.named.iter());
        let clause = package.default.is_some() || !names.is_empty();
        if clause {
            named.push((relative, entry(Says::Named(package.default.as_deref(), names))));
        }
        if let Some(namespace) = &package.namespace {
            named.push((relative, entry(Says::Namespace(namespace))));
        }
        if !clause && package.namespace.is_none() {
            effects.push(entry(Says::Effect));
        }
    }
    for import in &module.imports {
        let relative = crate::program::is_relative_specifier(&import.from);
        let entry = |says| Entry {
            says,
            from: &import.from,
            placed: import.span,
            commented: import.span,
            declared: false,
        };
        if !import.named.is_empty() {
            let default = (import.named.iter())
                .find(|(export, _)| export == "default")
                .map(|(_, local)| local.as_str());
            let names = pairs(import.named.iter().filter(|(export, _)| export != "default"));
            named.push((relative, entry(Says::Named(default, names))));
        }
        // The module itself, its own statement after what's named, as a
        // package's (rd114, ADR 1018).
        if let Some(namespace) = &import.namespace {
            named.push((relative, entry(Says::Namespace(namespace))));
        }
    }
    for reexport in &module.reexports {
        let relative = crate::program::is_relative_specifier(&reexport.from);
        let says = match reexport.named.as_slice() {
            _ if reexport.every => Says::ReexportAll(None),
            [(namespace, alias)] if namespace == "*" => Says::ReexportAll(Some(alias)),
            named => Says::Reexport(
                (named.iter().enumerate())
                    .map(|(i, (export, alias))| {
                        let written = (reexport.written.get(i).copied()).unwrap_or((js::Span::NONE, js::Span::NONE));
                        (export.as_str(), alias.as_str(), written)
                    })
                    .collect(),
            ),
        };
        let entry = Entry {
            says,
            from: &reexport.from,
            placed: reexport.at,
            commented: reexport.span,
            declared: false,
        };
        named.push((relative, entry));
    }
    // Of two one `use` makes, `use crate::{a::x, b::y};`, by specifier.
    let written = |span: js::Span| (span.is_none(), span.lo);
    named.sort_by(|(a_relative, a), (b_relative, b)| {
        (written(a.placed), a_relative, a.from).cmp(&(written(b.placed), b_relative, b.from))
    });
    let runtime = (!module.helpers.is_empty()).then(|| Entry {
        says: Says::Named(None, module.helpers.iter().map(|&helper| (helper, helper)).collect()),
        from: crate::runtime::PACKAGE,
        placed: js::Span::NONE,
        commented: js::Span::NONE,
        declared: false,
    });
    (named.into_iter().map(|(_, entry)| entry))
        .chain(effects)
        .chain(runtime)
        .collect()
}

/// Where the comments before each `use` an import or re-export is of are
/// attached, by where the `use` starts: the start of the first statement
/// of it, so they're above it once, of two one `use` makes (rd92).
pub(super) fn commented_at(entries: &[Entry<'_>]) -> HashMap<u32, u32> {
    let mut at = HashMap::new();
    for entry in entries.iter().filter(|entry| !entry.commented.is_none()) {
        at.entry(entry.commented.lo).or_insert(entry.at().lo);
    }
    at
}

impl<'a> Cx<'a> {
    /// `entry`'s statement, `import { a, b as c } from "x";` or `export *
    /// as ns from "x";`, of the span of where it's placed, which the map
    /// places it at and its comments and blank lines are before.
    pub(super) fn entry(&self, entry: &Entry<'_>) -> Statement<'a> {
        let b = &self.b;
        let at = span(entry.at());
        let from = StringLiteral::new(SPAN, self.name(entry.from), None, b);
        let name = |name: &str| ModuleExportName::new_identifier_name(SPAN, self.name(name), b);
        let local = |name: &str| BindingIdentifier::new(SPAN, self.name(name), b);
        let import = |specifiers: Option<Vec<ImportDeclarationSpecifier<'a>>>| {
            let specifiers = specifiers.map(|specifiers| ArenaVec::from_iter_in(specifiers, b));
            Statement::new_import_declaration(at, specifiers, from.clone(), None, None, ImportOrExportKind::Value, b)
        };
        match &entry.says {
            Says::Named(default, named) => {
                let default = (default.iter())
                    .map(|&default| ImportDeclarationSpecifier::new_import_default_specifier(SPAN, local(default), b));
                let named = named.iter().map(|&(export, alias)| {
                    ImportDeclarationSpecifier::new_import_specifier(
                        SPAN,
                        name(export),
                        local(alias),
                        ImportOrExportKind::Value,
                        b,
                    )
                });
                import(Some(default.chain(named).collect()))
            }
            Says::Namespace(namespace) => {
                import(Some(vec![ImportDeclarationSpecifier::new_import_namespace_specifier(
                    SPAN,
                    local(namespace),
                    b,
                )]))
            }
            Says::Effect => import(None),
            // Each where its leaf is, which the map places it at, so the
            // comments its `use` writes after it on its line are after it
            // and those before it on lines of their own before it (rd338).
            Says::Reexport(named) => {
                let specifiers = named.iter().map(|&(export, alias, (local, exported))| {
                    let named =
                        |at: js::Span, name: &str| ModuleExportName::new_identifier_name(span(at), self.name(name), b);
                    let whole = js::Span {
                        lo: local.lo,
                        hi: exported.hi,
                    };
                    let at = if local.is_none() { js::Span::NONE } else { whole };
                    ExportSpecifier::new(
                        span(at),
                        named(local, export),
                        named(exported, alias),
                        ImportOrExportKind::Value,
                        b,
                    )
                });
                let specifiers = ArenaVec::from_iter_in(specifiers, b);
                Statement::new_export_from_declaration(at, specifiers, from, ImportOrExportKind::Value, None, b)
            }
            Says::ReexportAll(alias) => {
                Statement::new_export_all_declaration(at, alias.map(name), from, None, ImportOrExportKind::Value, b)
            }
        }
    }
}

/// `body`, the module's statements, with `made`, its imports' and
/// re-exports' in their order (`entries`), each with where it's placed,
/// among them: each after the statements before it that are written, all
/// their Rust, before where it's placed in its file, a `const` between two
/// `use`s between their imports (rd652), one of a `use` inside a function
/// right above the function; one placed nowhere right after the one before
/// it, so their order is what JS loads (ADR 1063). Whether one is first,
/// before the code.
pub(super) fn among<'a>(
    body: &mut ArenaVec<'a, Statement<'a>>,
    made: Vec<(Statement<'a>, js::Span)>,
    sources: &crate::program::Sources,
    b: &oxc_ast::builder::AstBuilder<'a>,
) -> bool {
    if made.is_empty() {
        return false;
    }
    // The file each place is in, by where each starts.
    let mut files = Vec::new();
    let mut start = 0;
    for file in &sources.files {
        files.push((start, start + file.text.len() as u32));
        start += file.text.len() as u32 + 1;
    }
    let file_of = |at: u32| files.iter().position(|&(lo, hi)| lo <= at && at <= hi);
    let before = |placed: js::Span| {
        (body.iter())
            .take_while(|statement| {
                let at = statement.span();
                at.start != 0 && at.end <= placed.lo && file_of(at.start) == file_of(placed.lo)
            })
            .count()
    };
    // In their order, so each is after the one before it.
    let mut after = 0;
    let mut placed: Vec<(usize, Statement<'a>)> = Vec::with_capacity(made.len());
    for (statement, at) in made {
        if !at.is_none() {
            after = before(at);
        }
        placed.push((after, statement));
    }
    let first = placed.first().is_some_and(|&(at, _)| at == 0);
    let code = std::mem::replace(body, ArenaVec::new_in(b));
    let mut placed = placed.into_iter().peekable();
    for (i, statement) in code.into_iter().enumerate() {
        while let Some((_, made)) = placed.next_if(|&(at, _)| at == i) {
            body.push(made);
        }
        body.push(statement);
    }
    body.extend(placed.map(|(_, made)| made));
    first
}
