//! A library's contract with the crates that use it (ADR 0100): what each
//! item another crate can reach is in JS, and what a consumer can't work out
//! from the types alone. Versioned on its own, beside the host manifest.

use crate::manifest::{Compiler, Manifest};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

/// 3: every `Rc` and `Weak` is counted, `{ value, strong, weak }`, in a
/// library and in a crate that uses one (ADR 0523). 4: each item says the
/// type parameters it's given an `Iterator` dictionary for, or an
/// extension's (ADR 0548). 5: what an item asks of each of its type
/// parameters is one record of it, `params` (ADR 0100, amended). 6: each
/// yes-or-no fact of an item is one of a set (ADR 0906). 7: each of its
/// structs' fields its consumers read a fact of is one of `fields`, its
/// facts a set too (ADRs 0863, amended, 0906). 8: a function takes the
/// copies and drops its own crate's calls give it, each type parameter's
/// copy a fact of it, `copy`, and a consumer compiles what more it needs
/// of it from its MIR (ADR 0914).
pub const VERSION: u32 = 8;

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
    /// What its file exports it as where that isn't `export`, which a
    /// consumer imports it as, named after `export`: `default`, of its
    /// module's default export, `export default function noop() {}` (ADRs
    /// 0611, 0773), or the name its module's `pub use` gives it, `export {
    /// getCurrentTime as unstable_now }` (ADR 0596, amended).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exported_as: Option<String>,
    /// What a consumer's call of it gives it and checks.
    #[serde(flatten)]
    pub calling: Calling,
}

/// What a consumer's call of a library's function gives it beside its
/// arguments, and checks of what it gives: rustc's metadata keeps none of
/// it (ADR 0100).
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Calling {
    /// What it asks of each of its type parameters that it asks anything
    /// of, by index, in order: what its consumers' calls give it beside
    /// their arguments, and check of what they give.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub params: Vec<TypeParam>,
    /// What else its calls give it or read of it, each a fact of a set, by
    /// its name (ADR 0906).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub facts: BTreeSet<ItemFact>,
}

/// A fact of a library's function its consumers' calls read (ADR 0906):
/// each one of a set, so a new one is a variant here, written and read as
/// the set is, and a manifest names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemFact {
    /// It may return `Err(fmt::Error)` (ADR 0187).
    Fails,
    /// It takes its caller's location, `#[track_caller]` (ADR 0427), which
    /// its consumers' calls give it.
    TakesLocation,
    /// Its last parameter, a slice, is JS's rest parameter,
    /// `#[rust_js::rest]` (ADR 0664), which its consumers' calls spread.
    /// rustc's metadata keeps no parameter's attributes.
    Rest,
    /// It's a thread-local made where it's first used, read by calling it,
    /// `START()` (ADR 0037, amended).
    Lazy,
    /// It's its module's variable, read by its name, which only its module
    /// sets: a thread-local of what its cell holds, `export let
    /// hasBadMapPolyfill` (ADR 0270, amended), or a `static mut`, `export
    /// let LINES` (ADR 0096, amended).
    Variable,
}

impl Calling {
    /// Whether it's `fact`.
    pub fn has(&self, fact: ItemFact) -> bool {
        self.facts.contains(&fact)
    }

    /// The type parameters it takes a `dropT` of, by index.
    pub fn drop_params(&self) -> Vec<u32> {
        self.params
            .iter()
            .filter(|param| param.has(ParamFact::Drop))
            .map(|param| param.index)
            .collect()
    }

    /// What it asks of its type parameter `index`, if anything.
    pub fn param(&self, index: u32) -> Option<&TypeParam> {
        self.params.iter().find(|param| param.index == index)
    }
}

/// What a function asks of one of its type parameters, beside a value of
/// it: each given at its call, or checked of what's given.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct TypeParam {
    /// Which, by its index among the function's type parameters.
    pub index: u32,
    /// What's given of it or checked of what's given, each a fact of a set,
    /// by its name (ADR 0906).
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub marks: BTreeSet<ParamFact>,
    /// What it asks of it, `Size` and the like (ADR 0145), which its
    /// consumers' calls give it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<String>,
    /// The traits it's stepped by as it is, each given a dictionary of, by
    /// the trait's key: `Iterator`'s or an extension's (ADRs 0411, 0548).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stepped: Vec<String>,
}

/// A fact of one of a library's function's type parameters its consumers'
/// calls give or check (ADR 0906), one of a set, as `ItemFact`s are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamFact {
    /// It's given a `dropT`, what drops a value of it (ADR 0098).
    Drop,
    /// It's given a `TCopy`, what copies a value of it where one may not be
    /// its own copy (ADRs 0289, 0914).
    Copy,
    /// It takes no destructor, which its consumers' calls are checked for
    /// (ADR 0190).
    NoDrop,
}

impl TypeParam {
    /// Whether it's `fact`.
    pub fn has(&self, fact: ParamFact) -> bool {
        self.marks.contains(&fact)
    }
}

impl Item {
    /// What its file exports it as, which a consumer imports: `default`,
    /// the name a `pub use` gives it, or its name.
    fn exported_as(&self) -> String {
        self.exported_as.clone().unwrap_or_else(|| self.export.clone())
    }
}

/// A binding a module of the library holds as a value of its own, its
/// `const` (ADR 0633), which a consumer naming the binding through that
/// module imports.
#[derive(Clone, Serialize, Deserialize)]
pub struct Held {
    /// rustc's `DefPathHash` of the binding.
    pub binding: String,
    /// rustc's `DefPathHash` of the module that holds it, which a
    /// consumer's path names.
    pub holder: String,
    /// Its name in that module in Rust, which the path names it by.
    pub rust_name: String,
    /// The module whose file it's in.
    pub module: Vec<String>,
    /// What that file exports it by: `default`, or its name.
    pub export: String,
    /// Its `const`'s name there, which an import of it is named after.
    pub name: String,
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
    /// Whether it makes a `dyn Any` of a value that drops, or of a type
    /// parameter's (ADR 0691, amended): a consumer's `dyn Any`, what a
    /// `catch_unwind` caught among it, may then hold one. A library a
    /// compiler wrote before is taken to.
    #[serde(default = "made_before")]
    pub makes_dropping_anys: bool,
    /// The bindings its modules hold (ADR 0633). A library a compiler
    /// wrote before held none a consumer read through it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub held: Vec<Held>,
    /// What the `Rc`s it counts point at, each `{ value, strong, weak }`
    /// (ADR 0320), and every one that may be one of them; each other the
    /// value itself (ADRs 0844, 0852): a crate it hands one counts as it
    /// does. None where it counts none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub counted_rcs: Vec<RcShape>,
    /// What it does with its `RefCell`s, which its consumers agree with
    /// (ADR 0863). None where it has none.
    #[serde(default, skip_serializing_if = "RefCellFacts::is_empty")]
    pub ref_cells: RefCellFacts,
    /// The keys of its types equal only to themselves (ADR 0864): a
    /// consumer keys a `Set` or a `Map` by each, and compares two by `===`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identities: Vec<String>,
    /// Each of its structs' fields its consumers read a fact of, by key,
    /// and its facts (ADRs 0863, amended, 0906).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
}

/// One of a library's structs' fields, by rustc's key for it, and what
/// its consumers read of it, each a fact of a set, by its name (ADR 0906).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub key: String,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub facts: BTreeSet<FieldFact>,
}

/// A fact of a library's struct's field its consumers read (ADR 0906), one
/// of a set, as an item's `ItemFact`s are.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldFact {
    /// A `RefCell` none of whose borrows its code checks, as it only ever
    /// borrows it for a moment (ADR 0863, amended): a consumer lends none,
    /// and holds no borrow of one while code that may borrow it runs. What
    /// it holds, the value or the object, both read of its declaration.
    Quiet,
}

/// What a library does with its `RefCell`s (ADR 0863), each type by its
/// shape, as an `Rc`'s pointee is named (ADR 0852): a consumer represents,
/// checks and borrows each as the library does, or is refused.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefCellFacts {
    /// Those of an object it has as `{ value }`, not the object: one of a
    /// type parameter's value, `_`, all of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boxed: Vec<RcShape>,
    /// Those some borrow of which it counts, held while code that may
    /// borrow it runs: a consumer checks each borrow of one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub counted: Vec<RcShape>,
    /// Those it borrows unchecked, none of whose borrows can meet another:
    /// a consumer holds no borrow of one while the library's code runs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unchecked: Vec<RcShape>,
    /// Those its types' fields hold, and its exported functions take and
    /// give, which a consumer shares with it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handed: Vec<RcShape>,
}

impl RefCellFacts {
    pub fn is_empty(&self) -> bool {
        *self == RefCellFacts::default()
    }
}

/// What a library a compiler wrote before a fact was in its contract is
/// taken to have done, where not doing it would be wrong.
fn made_before() -> bool {
    true
}

/// A type an `Rc` points at, as a library's contract names it (ADR 0852):
/// what makes it, an item by its key or a primitive's name, and the types
/// it's made of; `_`, no `of`, where it may be any type, a type
/// parameter's value.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RcShape {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<RcShape>,
}

/// An item as a consumer imports it: from a JS file, relative to its own
/// output, by what that file exports it as, its default or its name, and a
/// method of what it names.
#[derive(Clone)]
pub struct Imported {
    pub from: String,
    /// `default`, or its name there.
    pub export: String,
    /// Its name there, which an import of it is named after.
    pub name: String,
    pub member: Option<String>,
    pub calling: Calling,
}

/// A binding a library's module holds as a consumer imports it: from a JS
/// file, relative to its own output, by its export (ADR 0633).
#[derive(Clone)]
pub struct HeldImport {
    pub binding: String,
    pub from: String,
    pub export: String,
    pub name: String,
}

/// What one library gives its consumers.
#[derive(Default, Clone)]
pub struct Exports {
    pub crate_hash: String,
    /// Built into the compiler, `@rust-js/std`'s (ADR 0561):
    /// its bodies are the toolchain's std's, which the compiler links, so
    /// no metadata is checked against its hash.
    pub builtin: bool,
    pub keeps_capacity: bool,
    pub writers_take_options: bool,
    /// Whether it makes a `dyn Any` of what drops (ADR 0691, amended).
    pub makes_dropping_anys: bool,
    /// What the `Rc`s it counts point at (ADR 0852).
    pub counted_rcs: Vec<RcShape>,
    /// What it does with its `RefCell`s (ADR 0863).
    pub ref_cells: RefCellFacts,
    /// Its types equal only to themselves, by their keys (ADR 0864).
    pub identities: HashSet<String>,
    /// What it says of each of its structs' fields, by key (ADR 0906).
    pub fields: HashMap<String, BTreeSet<FieldFact>>,
    pub items: HashMap<String, Imported>,
    pub impls: HashSet<String>,
    /// Each binding a module holds, by the module's key and the binding's
    /// name in it.
    pub held: HashMap<(String, String), HeldImport>,
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
            let manifest = Manifest::read_from(path)?;
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
                let bytes = std::fs::read(&artifact.file).map_err(|e| format!("{}: {e}", artifact.file.display()))?;
                if crate::manifest::fingerprint(&bytes) != artifact.hash {
                    return Err(format!("dependency artifact changed: {}", artifact.file.display()));
                }
            }
            let mut exports = Exports {
                crate_hash: library.crate_hash,
                keeps_capacity: library.keeps_capacity,
                makes_dropping_anys: library.makes_dropping_anys,
                writers_take_options: library.writers_take_options,
                identities: library.identities.into_iter().collect(),
                fields: library
                    .fields
                    .into_iter()
                    .map(|field| (field.key, field.facts))
                    .collect(),
                counted_rcs: library.counted_rcs,
                ref_cells: library.ref_cells,
                impls: library.impls.into_iter().collect(),
                ..Exports::default()
            };
            // Where a module's JS is, from the consumer's.
            let specifier = |path: &Vec<String>| -> Result<String, String> {
                let module = (manifest.modules.iter())
                    .find(|m| &m.module == path)
                    .ok_or("dependency export references a missing module")?;
                if !manifest.artifacts.iter().any(|a| a.file == module.file) {
                    return Err("dependency module is not a fingerprinted artifact".into());
                }
                Ok(crate::paths::specifier(parent, &module.file))
            };
            for item in library.items {
                let imported = Imported {
                    from: specifier(&item.module)?,
                    export: item.exported_as(),
                    name: item.export,
                    member: item.member,
                    calling: item.calling,
                };
                if exports.items.insert(item.key, imported).is_some() {
                    return Err("duplicate dependency export".into());
                }
            }
            for held in library.held {
                let imported = HeldImport {
                    binding: held.binding,
                    from: specifier(&held.module)?,
                    export: held.export,
                    name: held.name,
                };
                if exports.held.insert((held.holder, held.rust_name), imported).is_some() {
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

/// `@rust-js/std`: std's bodies compiled by this compiler (the std-bodies
/// decision), the package every crate's JS imports them from, as the
/// runtime's helpers are imported from `@rust-js/runtime` (ADR 0103).
pub const STD_PACKAGE: &str = "@rust-js/std";

/// `@rust-js/std`'s manifest, written by `bun run std` and committed beside
/// its JS, built into the compiler that reads it: the one std its JS calls.
const STD_MANIFEST: &str = include_str!("../std/manifest.json");

impl Dependencies {
    /// `@rust-js/std` added to what a crate is compiled against: each of
    /// its items, keyed as std's are, imported from its package by the
    /// module's path, `@rust-js/std/iter.js`.
    pub fn add_std(&mut self) -> Result<(), String> {
        let manifest = Manifest::read(STD_MANIFEST.as_bytes())?;
        let library = manifest.library.ok_or("the std manifest has no library contract")?;
        if library.version != VERSION {
            return Err(format!(
                "the std manifest's library ABI is {}; expected {VERSION}",
                library.version
            ));
        }
        let mut exports = Exports {
            crate_hash: library.crate_hash,
            builtin: true,
            keeps_capacity: library.keeps_capacity,
            makes_dropping_anys: library.makes_dropping_anys,
            writers_take_options: library.writers_take_options,
            identities: library.identities.into_iter().collect(),
            fields: library
                .fields
                .into_iter()
                .map(|field| (field.key, field.facts))
                .collect(),
            counted_rcs: library.counted_rcs,
            ref_cells: library.ref_cells,
            impls: library.impls.into_iter().collect(),
            ..Exports::default()
        };
        for item in library.items {
            let imported = Imported {
                from: format!("{STD_PACKAGE}/{}.js", item.module.join("/")),
                export: item.exported_as(),
                name: item.export,
                member: item.member,
                calling: item.calling,
            };
            if exports.items.insert(item.key, imported).is_some() {
                return Err("duplicate std export".into());
            }
        }
        // Its items are std's, found by the crate each is of: `core`'s,
        // `alloc`'s or `std`'s, not the std crate's own name.
        for krate in ["core", "alloc", "std"] {
            self.libraries.insert(krate.to_string(), exports.clone());
        }
        Ok(())
    }
}

/// Where a crate of crates.io's rust-js refused says why, beside the
/// metadata rustc wrote of it, its types alone (ADR 0415), for a call into
/// it to say.
pub fn refusal(metadata: &std::path::Path) -> std::path::PathBuf {
    metadata.with_extension("rust-js-refused")
}
