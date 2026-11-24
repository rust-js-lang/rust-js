//! The program's own files lowering reads (ADR 0031, amended): the JS
//! files its bindings import, and what each imports in turn, found where
//! the crate's JS is written and where it goes in source, each what the
//! build read.

use super::{absolute, in_source, js_path, located};
use crate::to_oxc;
use rust_js_program::collections::{IndexMap, IndexSet};
use rust_js_program::paths::{parent_dir, specifier_file};
use rust_js_program::program;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// What a JS file named without its extension is, `./ReactEventPriorities`,
/// as Node and bundlers find it: the file with one of these.
const JS_EXTENSIONS: [&str; 8] = ["js", "jsx", "mjs", "cjs", "ts", "tsx", "mts", "cts"];

/// What a JS file imports that loads no module: a style, data, an image.
const ASSET_EXTENSIONS: [&str; 15] = [
    "css", "scss", "sass", "less", "json", "svg", "png", "jpg", "jpeg", "gif", "webp", "avif", "ico", "wasm", "txt",
];

/// The JS files of the program's own a crate's bindings import, `#[link_name
/// = "./src/hook.js#isPresent"]` (ADR 0028), and the files each imports in
/// turn: which of the crate's modules each may load where it loads (ADR
/// 0031, amended). Each is read where the crate's JS is, beside the output
/// and in source (ADR 0041), as the root's file names it; each once a
/// build, of what the build read (`read`).
pub struct LocalJs {
    input: PathBuf,
    output: PathBuf,
    /// The crate's package, its `Cargo.toml`'s directory, of a crate Cargo
    /// builds: where it and its workspace hold the program's hand-written
    /// JS (`hand_imported`).
    package: Option<PathBuf>,
    /// Each file read, by its path, and what it loads, or `None` where
    /// that can't be said.
    loads: std::cell::RefCell<BTreeMap<PathBuf, Option<Vec<String>>>>,
}

/// What a JS file names where it imports a file.
enum Imported<'s> {
    /// A module of the crate's, by its place in what `LocalJs::loaded` was
    /// given: one, or each whose JS may be there.
    Modules(&'s IndexSet<usize>),
    /// A file of the program's own.
    File(PathBuf),
    /// A package's, `react`, or a runtime's, `node:path`: none of the crate's.
    Package,
    /// One it may be any file: not found, or of a package's `imports`, `#internal`.
    Unknown,
}

impl LocalJs {
    pub fn new(plan: &super::OutputPlan) -> Self {
        Self {
            input: plan.input.clone(),
            output: plan.output.clone(),
            package: plan.settings.cargo.then(|| plan.settings.dir.clone()),
            loads: Default::default(),
        }
    }

    /// Each module's JS, where it's written and where it goes in source,
    /// as a file named without its extension, case-folded as macOS's and
    /// Windows's file systems find it: the modules', by their places in
    /// `modules`.
    fn stems(&self, modules: &[program::ModuleAt]) -> Option<IndexMap<String, IndexSet<usize>>> {
        let mut stems: IndexMap<String, IndexSet<usize>> = IndexMap::default();
        for (at, &(path, file, placed)) in modules.iter().enumerate() {
            let located = file
                .filter(|_| placed)
                .and_then(|file| located(&self.input, &self.output, file));
            let written = absolute(&js_path(&self.output, path, located.as_ref(), false)).ok()?;
            let in_source = in_source(&self.input, (path, file, placed), "js").ok()?;
            for file in [written, in_source] {
                stems.entry(stem(&file)).or_default().insert(at);
            }
        }
        Some(stems)
    }

    /// What `specifier`, of `LocalJs::loaded`, may load of the modules whose
    /// JS is at `stems`.
    fn reached(&self, specifier: &str, stems: &IndexMap<String, IndexSet<usize>>) -> Option<IndexSet<usize>> {
        let mut loaded = IndexSet::default();
        let mut pending = Vec::new();
        for root in [&self.output, &self.input] {
            match imported(parent_dir(&absolute(root).ok()?), specifier, stems) {
                Imported::Modules(modules) => loaded.extend(modules),
                Imported::File(file) => pending.push(file),
                Imported::Package => return Some(loaded),
                Imported::Unknown => {}
            }
        }
        if loaded.is_empty() && pending.is_empty() {
            return None;
        }
        let mut seen = IndexSet::default();
        while let Some(file) = pending.pop() {
            if !seen.insert(file.clone()) {
                continue;
            }
            for specifier in self.loads(&file)? {
                match imported(parent_dir(&file), &specifier, stems) {
                    Imported::Modules(modules) => loaded.extend(modules),
                    Imported::File(next) => pending.push(next),
                    Imported::Package => {}
                    Imported::Unknown => return None,
                }
            }
        }
        Some(loaded)
    }

    /// What `file`, of the program's own, loads (`to_oxc::loaded_modules`),
    /// read once: none of a style's or an image's; `None` of a file that's
    /// not JS, or not read.
    fn loads(&self, file: &Path) -> Option<Vec<String>> {
        let extension = file.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
        if extension.as_deref().is_some_and(|e| ASSET_EXTENSIONS.contains(&e)) {
            return Some(Vec::new());
        }
        if let Some(known) = self.loads.borrow().get(file) {
            return known.clone();
        }
        let loads = match extension.as_deref() {
            Some(e) if JS_EXTENSIONS.contains(&e) => {
                (std::fs::read_to_string(file).ok()).and_then(|code| to_oxc::loaded_modules(&code))
            }
            _ => None,
        };
        self.loads.borrow_mut().insert(file.to_path_buf(), loads.clone());
        loads
    }

    /// The files read, which the build read: Cargo's to rebuild of when
    /// one changes (`OutputPlan::bound`).
    pub fn read(&self) -> Vec<PathBuf> {
        self.loads.borrow().keys().cloned().collect()
    }
}

/// A file rust-js wrote, which starts so (`output::header`): no
/// hand-written JS.
const GENERATED: &str = "// Generated by rust-js from ";

/// The directories a scan of a program's JS never enters: what's installed,
/// built, or a tool's own.
const UNSCANNED: [&str; 2] = ["node_modules", "target"];

impl LocalJs {
    /// Where the program's hand-written JS is: its Cargo workspace, where
    /// the crate's package is a member of one, else its package, else the
    /// directory of the crate's root file.
    fn scope(&self) -> Option<PathBuf> {
        let package = match &self.package {
            Some(package) => package.clone(),
            None => parent_dir(&absolute(&self.input).ok()?).to_path_buf(),
        };
        let package = absolute(&package).ok()?;
        Some(rust_js_program::packages::workspace_of(&package).unwrap_or(package))
    }
}

/// Each JS file under `dir`, but a declaration's, `.d.ts`, and what's in an
/// `UNSCANNED` directory or a hidden one.
fn js_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_dir() {
            if !name.starts_with('.') && !UNSCANNED.contains(&name.as_ref()) {
                js_files(&entry.path(), found);
            }
        } else if kind.is_file()
            && !name.contains(".d.")
            && (name.rsplit_once('.')).is_some_and(|(_, extension)| JS_EXTENSIONS.contains(&extension))
        {
            found.push(entry.path());
        }
    }
}

/// Each module specifier `code` names after `from`, `import` or `require`,
/// a string's: `import x from "./a"`, `export * from "./b"`, `import "./c"`,
/// `import("./d")`, `require("./e")`. Read as words, not parsed: one in a
/// comment or a string is named too, which only checks more.
fn specifiers(code: &str) -> Vec<&str> {
    let bytes = code.as_bytes();
    let word =
        |at: usize| at < bytes.len() && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b'_' || bytes[at] == b'$');
    let mut found = Vec::new();
    for keyword in ["from", "import", "require"] {
        for (at, _) in code.match_indices(keyword) {
            // Not of a word that ends with it, `reimport(..)`: one that goes on,
            // `fromage`, meets no string after it.
            if at > 0 && word(at - 1) {
                continue;
            }
            let mut i = at + keyword.len();
            while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'(') {
                i += 1;
            }
            let Some(&quote) = bytes.get(i).filter(|&&b| b == b'"' || b == b'\'') else {
                continue;
            };
            let start = i + 1;
            let Some(length) = code[start..].find(quote as char) else {
                continue;
            };
            let specifier = &code[start..start + length];
            if !specifier.contains('\n') {
                found.push(specifier);
            }
        }
    }
    found
}

/// A bare specifier's package and the subpath it names in it,
/// `react-reconciler/src/ReactFiber` as `react-reconciler` and
/// `./src/ReactFiber`, `@scope/name/x` as `@scope/name` and `./x`; none
/// of a package's root alone.
fn package_subpath(specifier: &str) -> Option<(&str, String)> {
    let split = match specifier.starts_with('@') {
        true => specifier.match_indices('/').nth(1)?.0,
        false => specifier.find('/')?,
    };
    let (name, rest) = specifier.split_at(split);
    Some((name, format!(".{rest}")))
}

impl program::JsModules for LocalJs {
    /// Of `modules`, each a hand-written JS file of the program or its
    /// workspace imports (`scope`), by its file, `./src/ReactFiber`, or a
    /// package's subpath, `react-reconciler/src/ReactFiber`: what JS may
    /// call of the crate's besides what its root gives (#2403 lever (3),
    /// Tu's decision). A file rust-js wrote isn't one.
    fn hand_imported(&self, modules: &[program::ModuleAt]) -> IndexSet<usize> {
        let mut found = IndexSet::default();
        let (Some(stems), Some(scope)) = (self.stems(modules), self.scope()) else {
            return found;
        };
        let mut files = Vec::new();
        js_files(&scope, &mut files);
        for file in files {
            let Ok(code) = std::fs::read_to_string(&file) else {
                continue;
            };
            if code.starts_with(GENERATED) {
                continue;
            }
            let dir = parent_dir(&file);
            for specifier in specifiers(&code) {
                let reached = match rust_js_program::program::is_relative_specifier(specifier) {
                    true => imported(dir, specifier, &stems),
                    false => match package_subpath(specifier)
                        .and_then(|(name, subpath)| Some((rust_js_program::packages::installed(dir, name)?, subpath)))
                    {
                        Some((package, subpath)) => imported(&package, &subpath, &stems),
                        None => continue,
                    },
                };
                if let Imported::Modules(reached) = reached {
                    found.extend(reached.iter().copied());
                }
            }
        }
        found
    }

    /// Of `modules`, which each of `specifiers` may load where it loads: a
    /// relative one, `./src/hook.js`, each it imports, or a file it imports
    /// does, of the program's own. `None` where that can't be said, so any
    /// may be: a file not found where the crate's JS is, or not read, or
    /// that imports a file not found.
    fn loaded<'s>(
        &self,
        specifiers: &[&'s str],
        modules: &[program::ModuleAt],
    ) -> IndexMap<&'s str, Option<IndexSet<usize>>> {
        let stems = self.stems(modules);
        (specifiers.iter())
            .map(|&specifier| {
                (
                    specifier,
                    stems.as_ref().and_then(|stems| self.reached(specifier, stems)),
                )
            })
            .collect()
    }

    /// Of the file where the crate's JS is written, or where it goes in
    /// source, as `reached` finds a binding's file: the first that's there,
    /// read, and so what the build read, which loads nothing of the crate's,
    /// as a style doesn't (`loads`).
    fn css_module_keys(&self, specifier: &str) -> Result<program::Exported, String> {
        let mut looked = Vec::new();
        for root in [&self.output, &self.input] {
            let file = specifier_file(parent_dir(&absolute(root)?), specifier)?;
            if !file.is_file() {
                looked.push(file);
                continue;
            }
            let source = std::fs::read_to_string(&file).map_err(|e| format!("reading {}: {e}", file.display()))?;
            self.loads.borrow_mut().insert(file.clone(), Some(Vec::new()));
            let name = file.file_name().map(|name| name.to_string_lossy().into_owned());
            return rust_js_program::css_modules::exported_keys(&source, &name.unwrap_or_default());
        }
        looked.dedup();
        let looked: Vec<String> = looked.iter().map(|file| file.display().to_string()).collect();
        Err(format!("there's no file {}", looked.join(", nor ")))
    }
}

/// A JS file's path without its extension, `src/ReactEventPriorities`,
/// case-folded: as another names it.
fn stem(file: &Path) -> String {
    let bare = match file.extension().and_then(|e| e.to_str()) {
        Some(extension) if JS_EXTENSIONS.contains(&extension) => file.with_extension(""),
        _ => file.to_path_buf(),
    };
    bare.to_string_lossy().to_lowercase()
}

/// What `specifier`, imported by a JS file in `dir`, names: of the crate's
/// modules, of `stems` (`LocalJs::loaded`), by its file or its directory's
/// `index`; else a file, as Node and bundlers find one.
fn imported<'s>(dir: &Path, specifier: &str, stems: &'s IndexMap<String, IndexSet<usize>>) -> Imported<'s> {
    if !rust_js_program::program::is_relative_specifier(specifier) {
        return match specifier.starts_with(['#', '/']) || specifier.starts_with("file:") {
            true => Imported::Unknown,
            false => Imported::Package,
        };
    }
    let Ok(path) = specifier_file(dir, specifier) else {
        return Imported::Unknown;
    };
    if let Some(modules) = [stem(&path), stem(&path.join("index"))]
        .iter()
        .find_map(|s| stems.get(s))
    {
        return Imported::Modules(modules);
    }
    let named = |extension: &&str| PathBuf::from(format!("{}.{extension}", path.display()));
    let index = |extension: &&str| path.join(format!("index.{extension}"));
    (std::iter::once(path.clone()))
        .chain(JS_EXTENSIONS.iter().map(named))
        .chain(JS_EXTENSIONS.iter().map(index))
        .find(|file| file.is_file())
        .map_or(Imported::Unknown, Imported::File)
}

#[cfg(test)]
mod tests {
    use super::{package_subpath, specifiers};

    #[test]
    fn specifiers_are_read_after_from_import_and_require() {
        let code = r#"import a from "./a";
export * from './b.js';
import "./c";
const d = await import("./d");
const e = require('./e');
const fromage = "./not";
// importing "./f" in prose too, which only checks more
reimport("./g");
"#;
        assert_eq!(specifiers(code), vec!["./a", "./b.js", "./c", "./d", "./e"]);
    }

    #[test]
    fn a_bare_specifier_is_its_package_and_the_subpath_in_it() {
        assert_eq!(
            package_subpath("react-reconciler/src/ReactFiber"),
            Some(("react-reconciler", "./src/ReactFiber".to_string()))
        );
        assert_eq!(
            package_subpath("@scope/name/x/y"),
            Some(("@scope/name", "./x/y".to_string()))
        );
        assert_eq!(package_subpath("react"), None);
        assert_eq!(package_subpath("@scope/name"), None);
    }
}
