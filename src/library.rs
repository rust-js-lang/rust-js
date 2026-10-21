//! A library's contract with the crates that use it (ADR 0100): what each
//! item another crate can reach is in JS, and what a consumer can't work out
//! from the types alone. Versioned on its own, beside the host manifest.

use crate::manifest::{Compiler, Manifest};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// 3: every `Rc` and `Weak` is counted, `{ value, strong, weak }`, in a
/// library and in a crate that uses one (ADR 0523). 4: each item says the
/// type parameters it's given an `Iterator` dictionary for, or an
/// extension's (ADR 0548).
pub const VERSION: u32 = 4;

/// A function, method or impl method another crate can reach.
#[derive(Clone, Serialize, Deserialize)]
pub struct Item {
    /// rustc's `DefPathHash` of it, which a consumer finds it by.
    pub key: String,
    /// Its path, for messages: `models::User::validate`.
    pub rust_path: String,
    /// The module whose file it's in.
    pub module: Vec<String>,
    /// Its name in that file: the function's, or its type's object of
    /// methods (ADR 0047), `User` for `User.validate`.
    pub export: String,
    /// A method's name in that object.
    pub member: Option<String>,
    /// The type parameters it's given a `dropT` for (ADR 0098).
    pub drops: Vec<u32>,
    /// Whether it may return `Err(fmt::Error)` (ADR 0187). A library a
    /// compiler wrote before had none, as it refused them.
    #[serde(default)]
    pub fails: bool,
    /// The type parameters it takes no destructor of (ADR 0190), which its
    /// consumers' calls are checked for. A library a compiler wrote before
    /// had none, as it refused what needs one.
    #[serde(default)]
    pub no_drops: Vec<u32>,
    /// What it asks of its type parameters, `Size` of the one at an index
    /// (ADR 0145), which its consumers' calls give it. A library a compiler
    /// wrote before had none, as it refused what asks one.
    #[serde(default)]
    pub type_facts: Vec<(u32, String)>,
    /// Whether it takes its caller's location, `#[track_caller]` (ADR
    /// 0427), which its consumers' calls give it. A library a compiler wrote
    /// before had none, as it refused what reads one.
    #[serde(default)]
    pub takes_location: bool,
    /// The type parameters it steps as they are, given a dictionary of the
    /// trait each is stepped by, `Iterator`'s or an extension's, by the
    /// trait's key (ADRs 0411, 0548), which its consumers' calls give it.
    #[serde(default)]
    pub stepped: Vec<(u32, String)>,
}

#[derive(Serialize, Deserialize)]
pub struct Library {
    pub version: u32,
    pub name: String,
    /// rustc's hash of the crate its JS was made from, which a consumer
    /// checks against the metadata its rustc loads (ADR 0100).
    pub crate_hash: String,
    pub items: Vec<Item>,
    /// The keys of the trait impls whose methods are items: a consumer calls
    /// these, not what a derive would be.
    pub impls: Vec<String>,
    /// The libraries it was compiled against, whose manifests its consumers
    /// must have too: which traits get dictionaries, and which types drop,
    /// is decided by which crates are rust-js's.
    pub libraries: Vec<String>,
    pub inputs: Vec<crate::manifest::Artifact>,
    /// Whether it keeps a `Vec`'s capacity, as it or a library it uses asks
    /// it (ADR 0419): its consumers keep theirs too. A library a compiler
    /// wrote before kept none.
    #[serde(default)]
    pub keeps_capacity: bool,
    /// Whether its writers take their `Formatter`'s options after their
    /// arguments (ADRs 0058, 0137): a consumer's call of one gives them
    /// only then.
    #[serde(default)]
    pub writers_take_options: bool,
}

/// An item as a consumer imports it: from a JS file, relative to its own
/// output, by name, and a method of what it names.
pub struct Imported {
    pub from: String,
    pub export: String,
    pub member: Option<String>,
    pub drops: Vec<u32>,
    pub fails: bool,
    pub no_drops: Vec<u32>,
    pub type_facts: Vec<(u32, String)>,
    pub takes_location: bool,
    pub stepped: Vec<(u32, String)>,
}

/// What one library gives its consumers.
#[derive(Default)]
pub struct Exports {
    pub crate_hash: String,
    pub keeps_capacity: bool,
    pub writers_take_options: bool,
    pub items: HashMap<String, Imported>,
    pub impls: HashSet<String>,
}

#[derive(Default)]
pub struct Dependencies {
    pub libraries: HashMap<String, Exports>,
    pub inputs: Vec<std::path::PathBuf>,
}

impl Dependencies {
    /// Read and validate complete dependency artifacts before compilation starts.
    pub fn load(paths: &[std::path::PathBuf], output: &Path) -> Result<Self, String> {
        let mut result = Self::default();
        let mut needed: Vec<(String, String)> = Vec::new();
        let output = crate::paths::absolute(output)?;
        let parent = output.parent().ok_or("output has no parent")?;
        for path in paths {
            result.inputs.push(crate::paths::absolute(path)?);
            let manifest = Manifest::read(&std::fs::read(path).map_err(|e| e.to_string())?)?;
            result.inputs.extend(manifest.sources.iter().cloned());
            if manifest.compiler.as_ref() != Some(&Compiler::current()) {
                return Err(format!("incompatible dependency compiler identity: {}", path.display()));
            }
            let library = manifest.library.ok_or("dependency manifest has no library contract")?;
            if library.version != VERSION {
                return Err(format!(
                    "unsupported library ABI {}; expected {VERSION}",
                    library.version
                ));
            }
            if result.libraries.contains_key(&library.name) {
                return Err(format!("duplicate dependency crate name: {}", library.name));
            }
            needed.extend(
                library
                    .libraries
                    .iter()
                    .map(|used| (library.name.clone(), used.clone())),
            );
            for artifact in manifest.artifacts.iter().chain(&library.inputs) {
                result.inputs.push(artifact.file.clone());
                let bytes = std::fs::read(&artifact.file).map_err(|e| e.to_string())?;
                if crate::manifest::fingerprint(&bytes) != artifact.hash {
                    return Err(format!("dependency artifact changed: {}", artifact.file.display()));
                }
            }
            let mut exports = Exports {
                crate_hash: library.crate_hash,
                keeps_capacity: library.keeps_capacity,
                writers_take_options: library.writers_take_options,
                impls: library.impls.into_iter().collect(),
                ..Exports::default()
            };
            for item in library.items {
                let module = manifest
                    .modules
                    .iter()
                    .find(|m| m.module == item.module)
                    .ok_or("dependency export references a missing module")?;
                if !manifest.artifacts.iter().any(|a| a.file == module.file) {
                    return Err("dependency module is not a fingerprinted artifact".into());
                }
                let imported = Imported {
                    from: crate::paths::specifier(parent, &module.file),
                    export: item.export,
                    member: item.member,
                    drops: item.drops,
                    fails: item.fails,
                    no_drops: item.no_drops,
                    type_facts: item.type_facts,
                    takes_location: item.takes_location,
                    stepped: item.stepped,
                };
                if exports.items.insert(item.key, imported).is_some() {
                    return Err("duplicate dependency export".into());
                }
            }
            result.libraries.insert(library.name, exports);
        }
        if let Some((library, used)) = needed.iter().find(|(_, used)| !result.libraries.contains_key(used)) {
            return Err(format!(
                "`{library}` was compiled against `{used}`, a rust-js library: give its manifest too, with --dependency"
            ));
        }
        result.inputs.sort();
        result.inputs.dedup();
        Ok(result)
    }
}

/// Where a crate of crates.io's rust-js refused says why, beside the
/// metadata rustc wrote of it, its types alone (ADR 0415), for a call into
/// it to say.
pub fn refusal(metadata: &std::path::Path) -> std::path::PathBuf {
    metadata.with_extension("rust-js-refused")
}
