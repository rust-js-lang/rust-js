//! A crate's npm package, its `package.json` beside its `Cargo.toml` (ADR
//! 0414): its name, the files its root, `"."`, is under each of its
//! conditions, and what each re-exports of the crate's modules, which a
//! consumer imports by the package's name (ADR 1008). And where Node finds
//! a package from a directory, as it resolves a bare specifier. The one
//! owner of what the compiler reads of a package: artifact planning asks it
//! of the crate it builds, a consumer of the libraries it's given.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A library's npm package: its name, its directory, and what its root,
/// `"."`, re-exports of the library's modules under each of its
/// conditions (ADR 1008), which a consumer that Node finds the package
/// from imports by the package's name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub dir: std::path::PathBuf,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exports: Vec<PackageExport>,
}

/// One of the library's modules' exports, by the module's path and its
/// name there, which its package's root re-exports as `entry`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageExport {
    pub module: Vec<String>,
    pub export: String,
    pub entry: String,
}

/// What a package's entry, a JS file, exports, as oxc reads it
/// (`to_oxc::entry_exports`).
pub struct Entry {
    /// Each `export { .. } from` and `export * from`, in order.
    pub reexports: Vec<Reexport>,
    /// What else it exports by name: `export * as ns from`'s namespace,
    /// and anything of its own.
    pub own: Vec<String>,
    /// Whether it's re-exports alone: none of its statements runs anything
    /// as it loads, but loading what it re-exports.
    pub plain: bool,
}

/// One of an entry's re-exports.
pub enum Reexport {
    /// `export { name as exported } from 'from'`.
    Named {
        from: String,
        name: String,
        exported: String,
    },
    /// `export * from 'from'`: each name `from` exports, but `default`,
    /// that nothing else in the entry exports.
    All { from: String },
}

/// One of the crate's modules, as an entry names it: its path, the files
/// its JS is, where it's written and where it goes in source, the names
/// it exports, and what it re-exports of the crate's modules, each by the
/// module, its name there and the name it gives.
pub struct Module<'a> {
    pub path: &'a [String],
    pub files: Vec<PathBuf>,
    pub exports: &'a [String],
    pub reexports: Vec<(Vec<String>, String, String)>,
}

/// What the crate's package is: the files read to say, `package.json` and
/// its root's entry, which the build read; and its name and what its root
/// re-exports of the crate's modules, where it has a name and one.
#[derive(Default)]
pub struct Read {
    pub files: Vec<PathBuf>,
    pub package: Option<Package>,
    /// Whether its entry is a JS file of its own, not the crate's root
    /// module's, which a root re-exporting it alone needn't repeat.
    pub hand_written: bool,
}

/// The package of the crate in `dir`, of `modules`, where a `package.json`
/// is there: what its root's entry, the one an `import` of it resolves to
/// by default (ADR 1008, amended), re-exports. The crate's root module's
/// own JS, ADR 0414's `src/lib.js`, exports what that module does; another
/// module's, or a file Node can't import, nothing. An entry that isn't JS,
/// Flow's types in it, is refused, naming it.
pub fn read(dir: &Path, modules: &[Module<'_>], parse: impl Fn(&str) -> Result<Entry, String>) -> Result<Read, String> {
    let manifest = dir.join("package.json");
    if !manifest.is_file() {
        return Ok(Read::default());
    }
    let text = std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", manifest.display()))?;
    let mut read = Read {
        files: vec![crate::paths::absolute(&manifest)?],
        package: None,
        hand_written: false,
    };
    let Some(name) = json["name"].as_str() else {
        return Ok(read);
    };
    // The first of the files it may be that's one, or one of the crate's
    // modules' JS, which may not be written yet.
    let own = |file: &PathBuf| modules.iter().any(|module| module.files.contains(file));
    let candidates = root_entry(dir, &json).into_iter().flatten();
    let candidates = candidates.filter_map(|entry| crate::paths::absolute(&entry).ok());
    let Some(file) = candidates.into_iter().find(|file| own(file) || file.is_file()) else {
        return Ok(read);
    };
    let exports = match modules.iter().find(|module| module.files.contains(&file)) {
        // The crate's root module's JS, which exports its own and what it
        // re-exports, as its module says.
        Some(root) if root.path.is_empty() => {
            let own = (root.exports.iter()).map(|export| ((Vec::new(), export.clone()), export.clone()));
            let reexported = (root.reexports.iter())
                .map(|(module, export, alias)| ((module.clone(), export.clone()), alias.clone()));
            own.chain(reexported).collect()
        }
        Some(_) => return Ok(read),
        None => {
            let js = file.extension().is_some_and(|e| e == "js" || e == "mjs" || e == "jsx");
            if !file.is_file() || !js {
                return Ok(read);
            }
            read.files.push(file.clone());
            read.hand_written = true;
            let code = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let parsed = parse(&code).map_err(|error| {
                format!(
                    "rust-js can't read `{}`, package `{name}`'s entry, as JavaScript: {error}. rust-js reads what \
                     a package's entry re-exports, to import it by the package's name: keep only JS in it, its \
                     types in a `.d.ts` beside it, `{}`",
                    file.display(),
                    file.with_extension("d.ts")
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                )
            })?;
            reexported(crate::paths::parent_dir(&file), &parsed, modules)
        }
    };
    read.package = Some(Package {
        name: name.to_string(),
        dir: crate::paths::absolute(dir)?,
        exports: (exports.into_iter())
            .map(|((module, export), entry)| PackageExport { module, export, entry })
            .collect(),
    });
    Ok(read)
}

/// The file the package's root, `"."`, is to an `import` of its name by
/// default, of `exports`: a condition's `import` or `default`, one where
/// both are, as Node takes whichever an object names first, a fallback
/// array's first; the others, `react-server`'s,
/// `browser`'s, the host's to choose (the user's decision, ADR 1008,
/// amended, listed in ADR 0262). Else its `main`, else `index.js`, as
/// Node finds a file of it, each it may be. None where it's none, `null`'s,
/// or exports no root.
fn root_entry(dir: &Path, json: &serde_json::Value) -> Option<Vec<PathBuf>> {
    use serde_json::Value;
    fn target(value: &Value) -> Option<&str> {
        match value {
            Value::String(target) => Some(target),
            Value::Array(each) => each.first().and_then(target),
            // `import`'s or `default`'s; of both, one where they agree, as
            // which Node takes is whichever is written first, an order
            // `serde_json` doesn't keep.
            Value::Object(conditions) => {
                let import = conditions.get("import").map(target);
                let default = conditions.get("default").map(target);
                match (import, default) {
                    (Some(import), Some(default)) => (import == default).then_some(import).flatten(),
                    (Some(one), None) | (None, Some(one)) => one,
                    (None, None) => None,
                }
            }
            _ => None,
        }
    }
    let found = match &json["exports"] {
        Value::Null => {
            let main = json["main"].as_str().unwrap_or("index.js");
            let file = dir.join(main);
            return Some(vec![file.clone(), file.with_extension("js"), file.join("index.js")]);
        }
        Value::Object(map) if map.keys().any(|key| key.starts_with('.')) => target(map.get(".")?)?,
        exports => target(exports)?,
    };
    found.strip_prefix("./").map(|path| vec![dir.join(path)])
}

/// What `entry`, in `dir`, re-exports of `modules`: each module's export,
/// by its path and its name there, and the entry's name for it. A
/// re-export of what's none of the crate's modules says none; nor does an
/// `export *` where any other names what may not be one, or two modules
/// export one name, which JS then exports neither of.
fn reexported(dir: &Path, entry: &Entry, modules: &[Module<'_>]) -> BTreeMap<(Vec<String>, String), String> {
    let mut found = BTreeMap::new();
    if !entry.plain {
        return found;
    }
    let module_of = |specifier: &str| -> Option<&Module<'_>> {
        if !crate::program::is_relative_specifier(specifier) {
            return None;
        }
        let file = crate::paths::specifier_file(dir, specifier).ok()?;
        let bare = |file: &Path| match file.extension().and_then(|e| e.to_str()) {
            Some("js" | "jsx" | "mjs") => file.with_extension(""),
            _ => file.to_path_buf(),
        };
        let named = [bare(&file), file.join("index")];
        modules
            .iter()
            .find(|module| module.files.iter().any(|written| named.contains(&bare(written))))
    };
    let mut named: BTreeSet<&str> = entry.own.iter().map(String::as_str).collect();
    for reexport in &entry.reexports {
        if let Reexport::Named { from, name, exported } = reexport {
            named.insert(exported);
            if let Some(module) = module_of(from) {
                found.insert((module.path.to_vec(), name.clone()), exported.clone());
            }
        }
    }
    let mut starred: BTreeMap<&str, BTreeSet<&[String]>> = BTreeMap::new();
    for reexport in &entry.reexports {
        if let Reexport::All { from } = reexport {
            let Some(module) = module_of(from) else {
                return found;
            };
            for export in module.exports.iter().filter(|export| *export != "default") {
                starred.entry(export).or_default().insert(module.path);
            }
        }
    }
    for (export, from) in starred {
        if let ([path], false) = (Vec::from_iter(from).as_slice(), named.contains(export)) {
            found.insert((path.to_vec(), export.to_string()), export.to_string());
        }
    }
    found
}

/// Where Node finds package `name` from `dir`, as a bare specifier of a
/// file there imports it: the first `node_modules/<name>` with a
/// `package.json`, from `dir` up, through its links. As `tooling/patch.js`'s
/// `lookup` finds an app's packages, for its own question.
pub fn installed(dir: &Path, name: &str) -> Option<PathBuf> {
    let dir = crate::paths::absolute(dir).ok()?;
    (dir.ancestors())
        .filter(|dir| dir.file_name().is_none_or(|base| base != "node_modules"))
        .map(|dir| dir.join("node_modules").join(name))
        .find(|candidate| candidate.join("package.json").is_file())
        .and_then(|found| crate::paths::absolute(&found).ok())
}

/// Whether `package`'s entry re-exports each of `reexports`, a crate's
/// root's `pub use`s, each by the module it's of, its name there and the
/// name the root gives it: by that name, from the same module. A root of
/// those alone writes no JS of its own then (`output::entry_is_root`, the
/// user's decision, ADR 1008, amended).
pub fn reexports_each(package: &Package, reexports: &[(Vec<String>, String, String)]) -> bool {
    (reexports.iter()).all(|(from, name, alias)| {
        (package.exports.iter())
            .any(|export| &export.module == from && &export.export == name && &export.entry == alias)
    })
}
