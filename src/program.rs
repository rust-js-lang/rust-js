//! Owned compiler output. No rustc or printer types cross this boundary.

use crate::js;
use crate::runtime::Helper;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// A module of the crate's, as its JS is laid out: its module path, its
/// Rust file, and whether a `#[path]` placed it (ADR 0273).
/// Whether a module specifier names a file, `./util.js`, `../lib.js`, and
/// not a package, `react`: what lowering asks of a binding's module too
/// (ADR 0914), and planning and printing of an import's.
pub(crate) fn is_relative_specifier(specifier: &str) -> bool {
    specifier.starts_with("./") || specifier.starts_with("../")
}

pub type ModuleAt<'a> = (&'a [String], Option<&'a Path>, bool);

/// What a JS module a crate's module imports, as its binding names it,
/// `./src/hook.js`, `react`, loads of the crate's modules where it loads
/// (ADR 0031, amended). Lowering asks it, for which module may load before
/// another; artifact planning answers it, which lays out the crate's JS and
/// reads the program's own files (`output::LocalJs`).
pub trait JsModules {
    /// Of `modules`, which each of `specifiers` may load, by their places
    /// in it: none, of a package's; `None` where that can't be said, so any
    /// may be.
    fn loaded<'s>(&self, specifiers: &[&'s str], modules: &[ModuleAt]) -> HashMap<&'s str, Option<HashSet<usize>>>;
}

/// Normalized Rust source retained independently of the frontend.
pub struct Source {
    pub path: Option<PathBuf>,
    pub text: String,
    pub line: u32,
    /// Its comments, in order (ADR 0444).
    pub comments: Vec<Comment>,
}

/// A comment of a source, `//` or `/* */`, where it is in the arena: its JS's,
/// where its code is (ADR 0444). A doc comment, `///` or `/** */`, its
/// item's, is a JSDoc one, `doc`.
#[derive(Clone, Copy)]
pub struct Comment {
    pub lo: u32,
    pub hi: u32,
    pub doc: bool,
}

/// One Rust module's functions: a future JS file (ADR 0019).
pub struct LoweredModule {
    /// The module's path below the crate root: `[]` for the root itself,
    /// `["math", "stats"]` for `crate::math::stats`.
    pub path: Vec<String>,
    /// The `.rs` file the module's code lives in.
    pub file: Option<PathBuf>,
    /// Where its code is in the arena: its file, or an inline `mod`'s item
    /// (ADR 0621).
    pub code: js::Span,
    /// Its directives, `"use client"` (ADR 0192).
    pub directives: Vec<String>,
    /// What it imports from JS modules (ADR 0028), with the modules' names
    /// as written in `#[link_name]`.
    pub packages: Vec<js::Package>,
    pub imports: Vec<LoweredImport>,
    /// What it exports of the crate's other modules and of JS modules: its
    /// `pub use` of their functions and of bindings (ADR 0240).
    pub reexports: Vec<Reexport>,
    /// Its items, where its Rust has them (ADR 0306).
    pub items: Vec<js::Item>,
    /// The function it exports as its default too (ADR 0192).
    pub default_export: Option<String>,
    /// Whether that's its function's declaration (ADR 0611).
    pub default_declared: bool,
    /// Its own items it exports by another name too (ADR 0596).
    pub aliases: Vec<(String, String)>,
    /// What its `.d.ts` says of what it exports, but the header (ADR 0196):
    /// @rust-js/typescript's model, which TypeScript prints (ADR 0207).
    pub declarations: Option<serde_json::Value>,
    /// Runtime helpers its functions use.
    pub runtime: Vec<Helper>,
    /// Whether it has JSX, so it's a `.jsx` file (ADR 0040).
    pub jsx: bool,
    /// Whether its file is where its `#[path]` says, not its path's: its JS
    /// goes beside that file, as a Next.js page's `[code].rs` (ADR 0273).
    pub located: bool,
}

/// Named exports used from one Rust module, before its JS path is resolved.
/// What a module re-exports from one place, each `(export, alias)` (ADR
/// 0240): a module of the crate's, by its path, or a JS module a binding
/// names, by its specifier, as its `#[link_name]` writes it.
pub struct Reexport {
    pub from: js::ImportFrom,
    pub named: Vec<(String, String)>,
    /// The `pub use` it's of, which its comments are before.
    pub span: js::Span,
}

pub struct LoweredImport {
    pub path: Vec<String>,
    pub named: Vec<(String, String)>,
}

/// A `#[test]` function (ADR 0026).
pub struct TestFn {
    /// The module it's in, and its JS name there.
    pub module: Vec<String>,
    pub name: String,
    /// What the runner calls it: `tests::adds`.
    pub label: String,
    /// `#[should_panic]`, with its `expected` substring if any.
    pub should_panic: Option<Option<String>>,
    pub ignore: bool,
    /// It returns a `Result`: an `Err` fails it, as libtest's does.
    pub returns_result: bool,
}

/// Source arena used by the owned JS spans, with original file boundaries.
pub struct Sources {
    pub text: String,
    pub files: Vec<Source>,
}

impl Sources {
    /// Which of `files` the arena's `line` is in: the last that starts at or
    /// before it.
    pub fn file_at(&self, line: u32) -> Option<usize> {
        self.files.partition_point(|file| file.line <= line).checked_sub(1)
    }
}

/// The linked crate as JS: one module per Rust module, and tests in test mode.
pub struct Lowered {
    pub library: Option<crate::library::Library>,
    pub sources: Sources,
    pub modules: Vec<LoweredModule>,
    pub tests: Vec<TestFn>,
}

/// A module dependency expressed without frontend identities.
pub struct ImportRequest {
    pub symbol: js::Symbol,
    pub export: String,
    /// The name it's imported by, as the importer's `use .. as` names it.
    pub local: String,
    pub path: Vec<String>,
}

/// Symbolic module output. Imports and the helpers it asks for are filled
/// by linking.
pub struct UnlinkedModule {
    pub module: LoweredModule,
    pub imports: Vec<ImportRequest>,
    pub reserved_names: HashSet<String>,
}

/// Complete lowering output, owned independently of rustc. Only linking turns
/// this into the Linked wrapper accepted by output planning.
pub struct Unlinked {
    pub library: Option<crate::library::Library>,
    pub sources: Sources,
    pub modules: Vec<UnlinkedModule>,
    pub tests: Vec<TestFn>,
}
