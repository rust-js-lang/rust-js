//! The program's own files lowering reads (ADR 0031, amended): the JS
//! files its bindings import, and what each imports in turn, found where
//! the crate's JS is written and where it goes in source, each what the
//! build read.

use super::{absolute, in_source, js_path, located};
use crate::collections::{IndexMap, IndexSet};
use crate::paths::{parent_dir, specifier_file};
use crate::{program, to_oxc};
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

impl program::JsModules for LocalJs {
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
    if !crate::program::is_relative_specifier(specifier) {
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
