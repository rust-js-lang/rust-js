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
/// its JS is, where it's written and where it goes in source, and the names
/// it exports.
pub struct Module<'a> {
    pub path: &'a [String],
    pub files: Vec<PathBuf>,
    pub exports: &'a [String],
}

/// What the crate's package is: the files read to say, `package.json` and
/// each entry of its root, which the build read; and its name and what its
/// root re-exports of the crate's modules, where it has a name and one.
#[derive(Default)]
pub struct Read {
    pub files: Vec<PathBuf>,
    pub package: Option<Package>,
}

/// The package of the crate in `dir`, of `modules`, where a `package.json`
/// is there: what its root re-exports under every condition, each of its
/// entries by the same name from the same module (ADR 1008). None of a
/// root that's one of the crate's own modules' JS (ADR 0414's), nor of
/// one Node can't import, a file not there or of none of the crate's. An
/// entry that isn't JS, Flow's types in it, is refused, naming it.
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
    };
    let Some(name) = json["name"].as_str() else {
        return Ok(read);
    };
    let Some(entries) = root_entries(dir, &json) else {
        return Ok(read);
    };
    let own: BTreeSet<PathBuf> = modules.iter().flat_map(|module| module.files.iter().cloned()).collect();
    let mut agreed: Option<BTreeMap<(Vec<String>, String), String>> = None;
    for entry in entries {
        let Ok(file) = crate::paths::absolute(&entry) else {
            return Ok(read);
        };
        let js = file.extension().is_some_and(|e| e == "js" || e == "mjs" || e == "jsx");
        if !file.is_file() || !js || own.contains(&file) {
            return Ok(read);
        }
        read.files.push(file.clone());
        let code = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        let parsed = parse(&code).map_err(|error| {
            format!(
                "rust-js can't read `{}`, package `{name}`'s entry, as JavaScript: {error}. rust-js reads what a \
                 package's entry re-exports, to import it by the package's name: keep only JS in it, its types in \
                 a `.d.ts` beside it, `{}`",
                file.display(),
                file.with_extension("d.ts")
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            )
        })?;
        let exports = reexported(crate::paths::parent_dir(&file), &parsed, modules);
        agreed = Some(match agreed {
            None => exports,
            Some(mut agreed) => {
                agreed.retain(|key, exported| exports.get(key) == Some(exported));
                agreed
            }
        });
    }
    read.package = Some(Package {
        name: name.to_string(),
        dir: crate::paths::absolute(dir)?,
        exports: (agreed.unwrap_or_default().into_iter())
            .map(|((module, export), entry)| PackageExport { module, export, entry })
            .collect(),
    });
    Ok(read)
}

/// The files the package's root may be, `"."`, under each of its
/// conditions, of `exports`, else its `main`, else `index.js`, as Node
/// resolves its name: None where it may be none, `null`'s, or exports no
/// root.
fn root_entries(dir: &Path, json: &serde_json::Value) -> Option<Vec<PathBuf>> {
    use serde_json::Value;
    fn targets(value: &Value, found: &mut Vec<String>) -> bool {
        match value {
            Value::String(target) => {
                found.push(target.clone());
                true
            }
            Value::Array(each) => each.iter().all(|value| targets(value, found)),
            Value::Object(conditions) => conditions.values().all(|value| targets(value, found)),
            _ => false,
        }
    }
    let mut found = Vec::new();
    match &json["exports"] {
        Value::Null => {
            let main = json["main"].as_str().unwrap_or("index.js");
            let file = dir.join(main);
            return [file.clone(), file.with_extension("js"), file.join("index.js")]
                .into_iter()
                .find(|file| file.is_file())
                .map(|file| vec![file]);
        }
        Value::Object(map) if map.keys().any(|key| key.starts_with('.')) => {
            if !targets(map.get(".")?, &mut found) {
                return None;
            }
        }
        exports => {
            if !targets(exports, &mut found) {
                return None;
            }
        }
    }
    (found.iter())
        .map(|target| target.starts_with("./").then(|| dir.join(&target[2..])))
        .collect()
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
