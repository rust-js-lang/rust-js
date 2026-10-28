//! What a JS file loads, read by oxc: the modules it imports, which may
//! load the crate's modules where it loads (ADR 0031, amended); and what a
//! package's entry re-exports (ADR 1008).

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, CallExpression, ExportAllDeclaration, ExportFromDeclaration, Expression, ImportDeclaration,
    ImportExpression, Statement,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;

/// Each module `code`, a JS file of a program's, loads, as it names it,
/// `./priority.js`, `react`: by an `import`, an `export .. from`, an
/// `import()` or a `require()` of a string, wherever it's written, as one
/// a function makes may run as the file loads. Not one imported for its
/// types alone, `import type`, which loads nothing.
///
/// `None` where what it loads can't be said: a file that's neither JS nor
/// TypeScript, nor read as either, Flow's that TypeScript doesn't read, or
/// one that loads a module it doesn't name, `import(name)`.
pub fn loaded_modules(code: &str) -> Option<Vec<String>> {
    struct Loads {
        named: Vec<String>,
        unnamed: bool,
    }
    impl Loads {
        fn named(&mut self, source: Option<&Expression>) {
            match source {
                Some(Expression::StringLiteral(literal)) => self.named.push(literal.value.to_string()),
                Some(Expression::TemplateLiteral(template)) if template.expressions.is_empty() => {
                    match template.single_quasi() {
                        Some(quasi) => self.named.push(quasi.to_string()),
                        None => self.unnamed = true,
                    }
                }
                _ => self.unnamed = true,
            }
        }
    }
    impl<'a> Visit<'a> for Loads {
        fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
            if !it.import_kind.is_type() {
                self.named.push(it.source.value.to_string());
            }
        }
        fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
            if !it.export_kind.is_type() {
                self.named.push(it.source.value.to_string());
            }
        }
        fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
            if !it.export_kind.is_type() {
                self.named.push(it.source.value.to_string());
            }
        }
        fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
            self.named(Some(&it.source));
            walk::walk_import_expression(self, it);
        }
        fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
            if let Expression::Identifier(callee) = &it.callee
                && callee.name == "require"
            {
                self.named(it.arguments.first().and_then(Argument::as_expression));
            }
            walk::walk_call_expression(self, it);
        }
    }
    // As JS with JSX, then as TypeScript, with JSX and without, as each
    // reads `<T>` differently.
    [SourceType::mjs().with_jsx(true), SourceType::ts(), SourceType::tsx()]
        .into_iter()
        .find_map(|source_type| {
            let allocator = Allocator::default();
            let parsed = Parser::new(&allocator, code, source_type).parse();
            if parsed.diagnostics.errors().next().is_some() {
                return None;
            }
            let mut loads = Loads {
                named: Vec::new(),
                unnamed: false,
            };
            loads.visit_program(&parsed.program);
            Some((!loads.unnamed).then_some(loads.named))
        })
        .flatten()
}

/// What `code`, a package's entry, exports (`packages::Entry`), read as
/// JS, as a package's file is: its `export { .. } from`s and `export *
/// from`s, and whether it's those alone. An error, where it isn't JS,
/// Flow's types in it, saying where the parse stopped.
pub fn entry_exports(code: &str) -> Result<crate::packages::Entry, String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::mjs().with_jsx(true)).parse();
    if let Some(error) = parsed.diagnostics.errors().next() {
        let at = error.labels.iter().next().map_or(0, |label| label.offset() as usize);
        let line = code[..at.min(code.len())].matches('\n').count() + 1;
        let text = code.lines().nth(line - 1).unwrap_or("").trim();
        return Err(format!("{error}, at line {line}: `{text}`"));
    }
    let mut entry = crate::packages::Entry {
        reexports: Vec::new(),
        own: Vec::new(),
        plain: true,
    };
    for statement in &parsed.program.body {
        match statement {
            Statement::ExportFromDeclaration(it) => {
                for specifier in &it.specifiers {
                    entry.reexports.push(crate::packages::Reexport::Named {
                        from: it.source.value.to_string(),
                        name: specifier.local.name().to_string(),
                        exported: specifier.exported.name().to_string(),
                    });
                }
            }
            Statement::ExportAllDeclaration(it) => match &it.exported {
                Some(namespace) => entry.own.push(namespace.name().to_string()),
                None => entry.reexports.push(crate::packages::Reexport::All {
                    from: it.source.value.to_string(),
                }),
            },
            Statement::EmptyStatement(_) => {}
            _ => entry.plain = false,
        }
    }
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::loaded_modules;

    #[test]
    fn each_module_a_file_loads() {
        let code = "import type {Lane} from './ReactFiberLane';\n\
                    import {DidCapture} from './ReactFiberFlags';\n\
                    import './styles.css';\n\
                    export {a} from './a.js';\n\
                    export * from 'react';\n\
                    export type {B} from './b';\n\
                    function later() { return import('./later.js'), require(`./required.js`); }\n\
                    function f(x: Fiber): void {}\n";
        assert_eq!(
            loaded_modules(code).unwrap(),
            [
                "./ReactFiberFlags",
                "./styles.css",
                "./a.js",
                "react",
                "./later.js",
                "./required.js"
            ]
        );
        assert_eq!(loaded_modules("const m = import(name);\n"), None);
        assert_eq!(loaded_modules("const m = require(name);\n"), None);
        assert_eq!(loaded_modules("import {a} from './a.js'\nexport const = ;\n"), None);
    }
}
