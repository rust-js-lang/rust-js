//! What's above a module's code, plain text the map doesn't place: its
//! header, directives and imports, each import after the comments of the
//! `use` that makes it (rd92).

use crate::js::{self, Module};

/// `import clause from "from";`, an import's line, the one way it's written:
/// a module's, and the `#[test]` runner's (`output.rs`).
pub fn import_line(clause: &str, from: &str) -> String {
    format!("import {clause} from {from:?};\n")
}

/// What's above a module's code: its header, directives, imports and
/// re-exports, one block in the order its `use`s are written (ADR 1063,
/// amended, rd620), each after the comments before the `use` that makes it,
/// `comments`. Layout, as `format::spaced`'s blank lines are, of what the
/// module says.
pub(super) fn header(module: &Module, comments: &std::collections::HashMap<u32, Vec<String>>) -> String {
    let mut code = format!("{}\n", module.header);
    for directive in &module.directives {
        code.push_str(&format!("\n{directive:?};\n"));
    }
    let packages = &module.packages;
    // Sorted already, and each once, as the artifact planner chose them.
    let helpers = &module.helpers;
    // One block, in the order the module's source names them (ADR 1063):
    // each import where the first `use` that makes it is, or a binding the
    // module declares of it, as JS loads its modules in its imports' order;
    // one the source names neither way after, packages then relative ones,
    // each by its specifier; then what it imports for its effect, as
    // written, its CSS after what it styles; then the runtime's helpers.
    // Each with where it's named and the `use` its comments are before.
    let mut named: Vec<(bool, &str, String, js::Span, js::Span)> = Vec::new();

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
        let relative = crate::program::is_relative_specifier(&package.from);
        if !clause.is_empty() {
            let line = import_line(&clause.join(", "), &package.from);
            named.push((relative, &package.from, line, package.named_at, package.span));
        }
        if let Some(namespace) = &package.namespace {
            let line = import_line(&format!("* as {namespace}"), &package.from);
            named.push((relative, &package.from, line, package.named_at, package.span));
        }
        if clause.is_empty() && package.namespace.is_none() {
            effects.push(format!("import {:?};\n", package.from));
        }
    }
    for import in &module.imports {
        let relative = crate::program::is_relative_specifier(&import.from);
        // The module itself, its own line after what's named, as a
        // package's (rd114, ADR 1018).
        let namespace = (import.namespace.iter()).map(|namespace| {
            (
                relative,
                import.from.as_str(),
                import_line(&format!("* as {namespace}"), &import.from),
                import.span,
                import.span,
            )
        });
        if import.named.is_empty() {
            named.extend(namespace);
            continue;
        }
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
        named.push((relative, &import.from, line, import.span, import.span));
        named.extend(namespace);
    }
    // Each re-export where it's placed among them (`Reexport::at`), after
    // an import placed there too, JS's load order either way (rd620).
    for reexport in &module.reexports {
        let relative = crate::program::is_relative_specifier(&reexport.from);
        let clause = match reexport.named.as_slice() {
            [(namespace, alias)] if namespace == "*" => format!("* as {alias}"),
            named => format!("{{ {} }}", clause_of(&mut named.iter()).join(", ")),
        };
        let line = format!("export {clause} from {:?};\n", reexport.from);
        named.push((relative, &reexport.from, line, reexport.at, reexport.span));
    }
    // Of two one `use` makes, `use crate::{a::x, b::y};`, by specifier.
    let written = |span: js::Span| (span.is_none(), span.lo);
    named.sort_by(|a, b| (written(a.3), a.0, a.1).cmp(&(written(b.3), b.0, b.1)));
    // Each after the comments of its `use`, on lines of their own, once: of
    // two imports one `use` makes, above the first printed.
    let mut said: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let named = named.into_iter().map(|(_, _, line, _, span)| {
        let texts = (comments.get(&span.lo))
            .filter(|_| !span.is_none() && said.insert(span.lo))
            .into_iter()
            .flatten();
        texts.map(|text| format!("{text}\n")).chain([line]).collect::<String>()
    });
    let runtime =
        (!helpers.is_empty()).then(|| import_line(&format!("{{ {} }}", helpers.join(", ")), crate::runtime::PACKAGE));
    let lines: Vec<String> = named.chain(effects).chain(runtime).collect();
    if !lines.is_empty() {
        code.push('\n');
        code.push_str(&lines.concat());
    }
    // A blank line before the code, but after leading comments right above
    // it, of a module that imports nothing, as the source has none (#419).
    if !(module.header_on_code && lines.is_empty() && module.directives.is_empty()) {
        code.push('\n');
    }
    code
}

/// The comments `own`, before an import's `use` or a re-export's at `at`,
/// as the header prints them above its line (rd92, rd620): a run of `///`
/// lines one JSDoc, as an item's is (`jsdoc`), each other as JS writes it;
/// and a blank line after them where the Rust has one between them and
/// what they're above, a module's `//!` doc above its first `use`.
pub(super) fn above(own: &[crate::program::Comment], at: u32, text: &str) -> Vec<String> {
    let written = |c: &crate::program::Comment| &text[c.lo as usize..c.hi as usize];
    let mut texts = Vec::new();
    let mut i = 0;
    while i < own.len() {
        let (run, lines) = crate::comments::doc_lines(&own[i..], text);
        if run > 0
            && let Some(jsdoc) = crate::comments::jsdoc(&lines)
        {
            texts.push(jsdoc);
            i += run;
            continue;
        }
        texts.push(crate::comments::in_js(written(&own[i])));
        i += 1;
    }
    if let Some(last) = own.last()
        && let between = &text[last.hi as usize..at as usize]
        && between.trim().is_empty()
        && between.matches('\n').count() > 1
    {
        texts.push(String::new());
    }
    texts
}
