//! What a module imports and re-exports: a JS module's names it binds
//! (`Package`), another module's of the crate (`Import`), what it exports
//! of another (`Reexport`), and where a dynamic `import()` loads from
//! (`ImportFrom`).

use super::Span;

#[derive(Clone, PartialEq)]
pub struct Import {
    /// Exports as `(export, local)`, aliased only for name collisions.
    pub named: Vec<(String, String)>,
    /// The module itself, `import * as Plugin` (rd114, ADR 1018), as
    /// `Package::namespace` is: its own `import`, beside a named one.
    pub namespace: Option<String>,
    /// A relative specifier, like `./math.js` or `../lib.js`, or another
    /// package's subpath, `shared/ReactDOMFragmentRefShared`, of a module
    /// whose file is in that package (ADR 0770, amended).
    pub from: String,
    /// The `use` that makes it, the first of the module's naming what it
    /// imports, which its comments are before; `Span::NONE` of none.
    pub span: Span,
}

impl Import {
    /// Each name it binds in its module, its namespace's too.
    pub fn locals(&self) -> impl Iterator<Item = &String> {
        self.named.iter().map(|(_, local)| local).chain(&self.namespace)
    }

    /// Each name it binds in its module, to rename.
    pub fn locals_mut(&mut self) -> impl Iterator<Item = &mut String> {
        self.named.iter_mut().map(|(_, local)| local).chain(&mut self.namespace)
    }
}

/// One `pub use`'s `export { a, b as c } from "./m.js";` (ADR 0240), its
/// comments before it: each `(export, alias)`, `*` the namespace, `export *
/// as ns from`; `export * from` where `every`, all `m`'s file exports but
/// `default` (#593). With the imports, in JS's load order (ADR 1063).
#[derive(Clone, PartialEq)]
pub struct Reexport {
    pub named: Vec<(String, String)>,
    /// Where each of `named` is written (`program::Reexport::written`).
    pub written: Vec<(Span, Span)>,
    pub from: String,
    pub span: Span,
    /// Where it's printed among the imports (`program::Reexport::at`).
    pub at: Span,
    pub every: bool,
}

/// Where a dynamic `import()` or a `require` loads from (ADRs 0304, 0305): a specifier, as a
/// `link_name`'s, or a module of the crate's, until its file's, from the importing file, is known.
#[derive(Clone, PartialEq)]
pub enum ImportFrom {
    Specifier(String),
    Module(Vec<String>),
}

/// What one file imports from one JS module: its default export, named
/// exports as `(export, local)`, and the module itself.
#[derive(Clone, PartialEq)]
pub struct Package {
    pub from: String,
    pub default: Option<String>,
    pub named: Vec<(String, String)>,
    pub namespace: Option<String>,
    /// The `use` that makes it, as `Import::span` is.
    pub span: Span,
    /// Where the source first names it, which orders it among the imports
    /// (ADR 1063, amended): its `use`, or a binding of its module's own
    /// declared of it, whichever is first; `Span::NONE` of neither.
    pub named_at: Span,
}
