//! Versioned build-tool contract. Paths are resolved by artifact planning.

use std::path::{Path, PathBuf};

use crate::offered::OfferedPath;
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;
pub const ABI_VERSION: u32 = 1;
/// This build of the compiler, a hash of what it's made of: the binary's
/// `build.rs` makes it (ADR 1004), and gives it here as it starts
/// (`set_build`), as this crate is built apart from it (ADR 1181).
static BUILD: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

/// The compiler's build, `RUST_JS_BUILD`, given once as it starts.
pub fn set_build(build: &'static str) {
    BUILD
        .set(build)
        .expect("internal error: the compiler's build given twice (`manifest::set_build`)");
}

/// This build of the compiler, as it was given (`set_build`).
pub fn build() -> &'static str {
    BUILD
        .get()
        .expect("internal error: the compiler's build asked before `main` gave it (`manifest::set_build`)")
}

#[derive(PartialEq, Serialize, Deserialize)]
pub struct Compiler {
    pub version: String,
    pub toolchain: String,
    pub abi: u32,
    /// Which build of the compiler it is, a hash of what it's made of
    /// (`build.rs`): two builds of one version decide otherwise, so what one
    /// made is another's to refuse, and Cargo's to build again (ADR 1003).
    /// None of `@rust-js/std`'s manifest, built into the compiler that reads
    /// it, which it doesn't check; and none of a compiler before it, whose
    /// libraries are then refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<String>,
}

impl Compiler {
    pub fn current() -> Self {
        let toolchain = include_str!("../../../rust-toolchain.toml")
            .lines()
            .find_map(|line| line.strip_prefix("channel = "))
            .expect("the toolchain is pinned in the checked-in configuration")
            .trim()
            .trim_matches('"')
            .to_owned();
        Self {
            // Cargo's, as the runtime's module says it.
            version: env!("CARGO_PKG_VERSION").to_owned(),
            toolchain,
            abi: ABI_VERSION,
            build: Some(build().to_owned()),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    // Version-1 manifests written before identity was added remain readable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiler: Option<Compiler>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library: Option<crate::library::Library>,
    pub input: PathBuf,
    pub output: PathBuf,
    pub sources: Vec<PathBuf>,
    pub modules: Vec<Module>,
    pub artifacts: Vec<Artifact>,
    /// What an older build wrote, by its manifest, that this one may remove
    /// (ADR 0091), each as it was written: a generated file in the output's
    /// directory that this build neither writes nor reads. A WASI compiler's
    /// host publishes, and is told this, to remove each still as written and
    /// not, on the disk it has, a file this build reads or writes by another
    /// name, which the compiler can't see (ADR 0375). What's published has
    /// none: a build over an older one leaves what a build from nothing does.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stale: Vec<Artifact>,
    /// Its `#[test]`s, of a build with `--test` (ADR 1076): what a host
    /// runs them by.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tests: Option<Tests>,
    /// The paths the crate offers JS, from its root: each `pub` item of the
    /// root and each `pub use` of it, and through each module it re-exports,
    /// the crate's or a library's, what that module offers in turn, by its
    /// Rust path and the path JS reads it by (#2481). Found of rustc's
    /// `module_children`, apart from what writes the JS and its `.d.ts`:
    /// what a host, and the export net, may hold them to. A module's own
    /// `pub mod` isn't one, as only Rust names it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub offered_paths: Vec<OfferedPath>,
}

/// A test build's test files, one beside each module's JS that has tests,
/// and the runner they're written for, which runs them (ADR 1076).
#[derive(Serialize, Deserialize)]
pub struct Tests {
    /// `[package.metadata.rust-js.test]`'s `runner`: `node`, `bun`,
    /// `vitest` or `jest`.
    pub runner: String,
    /// The module each test file imports `test` from, the runner's,
    /// `node:test`: what a host that runs them in a page of its own gives
    /// them in its place, as the playground does.
    pub module: String,
    pub files: Vec<PathBuf>,
}

#[derive(Serialize, Deserialize)]
pub struct Module {
    pub module: Vec<String>,
    pub file: PathBuf,
    pub map: PathBuf,
    /// Its `.d.ts`, where the crate asks for one (ADR 0196).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub types: Option<PathBuf>,
    /// Whether it's of types alone: its `.d.ts` is written, and no `file`
    /// nor `map`, which are where its JS would be, as a module imports its
    /// types from (the user's decision, ADR 1005).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub types_only: bool,
    pub source: Option<PathBuf>,
    /// Where its JS goes in source, beside its Rust, as a project commits it
    /// (ADRs 0041, 0273): what a host that writes it there writes. Absent of
    /// an older compiler's.
    #[serde(default)]
    pub in_source: Option<PathBuf>,
    pub imports: Vec<PathBuf>,
    /// What it exports, by name, `default` its default export's: what a
    /// host may call of it, a program's `main`, without reading its JS.
    #[serde(default)]
    pub exports: Vec<String>,
    /// Where its JS names another module's file, `"./util.js"`, by the
    /// compiler's parse of it: what a host that copies the files elsewhere
    /// rewrites, and nothing else (ADR 0101). Written even where there's
    /// none, so a host tells a manifest of an older compiler, which had
    /// none, from one of a module that names no other.
    #[serde(default)]
    pub links: Vec<Link>,
    /// Where its `.d.ts` does.
    #[serde(default)]
    pub type_links: Vec<Link>,
}

/// A specifier in a file: the byte range of its text, between its quotes,
/// and the file it names.
#[derive(Serialize, Deserialize)]
pub struct Link {
    pub start: usize,
    pub end: usize,
    pub file: PathBuf,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Artifact {
    pub file: PathBuf,
    pub hash: String,
}

impl Manifest {
    /// The manifest with each path relative to `dir`, its own directory:
    /// `@rust-js/std`'s, committed and built into the compiler, which reads
    /// it wherever the checkout is (ADR 0561).
    pub fn relative_to(mut self, dir: &Path) -> Self {
        let rel = |path: &mut PathBuf| *path = PathBuf::from(crate::paths::relative(dir, path));
        rel(&mut self.input);
        rel(&mut self.output);
        self.sources.iter_mut().for_each(rel);
        for module in &mut self.modules {
            rel(&mut module.file);
            rel(&mut module.map);
            module.types.iter_mut().for_each(rel);
            module.source.iter_mut().for_each(rel);
            module.in_source.iter_mut().for_each(rel);
            module.imports.iter_mut().for_each(rel);
            // Its links' files too, `@rust-js/std`'s `runtime.js` (#1095).
            module.links.iter_mut().for_each(|link| rel(&mut link.file));
            module.type_links.iter_mut().for_each(|link| rel(&mut link.file));
        }
        self.artifacts.iter_mut().for_each(|a| rel(&mut a.file));
        self.stale.iter_mut().for_each(|a| rel(&mut a.file));
        if let Some(library) = &mut self.library {
            library.inputs.iter_mut().for_each(|a| rel(&mut a.file));
        }
        self
    }

    /// The manifest, its compiler's build left out: `@rust-js/std`'s,
    /// committed, and built into each build of the compiler (ADR 1003).
    pub fn of_no_build(mut self) -> Self {
        if let Some(compiler) = &mut self.compiler {
            compiler.build = None;
        }
        self
    }

    /// Its library's crate hash of what it's made of alone, `@rust-js/std`'s:
    /// its inputs, by their relative paths and their contents' hashes, and
    /// the toolchain's `release`. rustc's, of a `core` built from where the
    /// toolchain is on the machine and of where its own crate was, differed
    /// from one checkout and machine to the next, and std's is a builtin's,
    /// which nothing checks against metadata (`Foreign::check`, #1094).
    pub fn of_its_inputs(mut self, release: &str) -> Self {
        if let Some(library) = &mut self.library {
            let mut made_of = format!("{release}\n");
            for input in &library.inputs {
                made_of.push_str(&format!("{} {}\n", input.file.display(), input.hash));
            }
            library.crate_hash = fingerprint(made_of.as_bytes());
        }
        self
    }

    /// The manifest at `path`, each relative path in it, `@rust-js/std`'s,
    /// resolved against its directory: the one way a manifest file is read.
    pub fn read_from(path: &Path) -> Result<Self, String> {
        let manifest = Self::read(&std::fs::read(path).map_err(|e| e.to_string())?)?;
        let dir = path.parent().ok_or("a manifest has a directory")?;
        Ok(manifest.resolved_from(dir))
    }

    /// Each relative path, resolved against `dir`.
    fn resolved_from(mut self, dir: &Path) -> Self {
        let abs = |path: &mut PathBuf| {
            if path.is_relative() {
                *path = dir.join(&*path);
            }
        };
        abs(&mut self.input);
        abs(&mut self.output);
        self.sources.iter_mut().for_each(abs);
        for module in &mut self.modules {
            abs(&mut module.file);
            abs(&mut module.map);
            module.types.iter_mut().for_each(abs);
            module.source.iter_mut().for_each(abs);
            module.in_source.iter_mut().for_each(abs);
            module.imports.iter_mut().for_each(abs);
            module.links.iter_mut().for_each(|link| abs(&mut link.file));
            module.type_links.iter_mut().for_each(|link| abs(&mut link.file));
        }
        self.artifacts.iter_mut().for_each(|a| abs(&mut a.file));
        self.stale.iter_mut().for_each(|a| abs(&mut a.file));
        if let Some(library) = &mut self.library {
            library.inputs.iter_mut().for_each(|a| abs(&mut a.file));
        }
        self
    }

    pub fn read(bytes: &[u8]) -> Result<Self, String> {
        let manifest: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if manifest.version != VERSION {
            return Err(format!(
                "unsupported manifest version {}; expected {VERSION}",
                manifest.version
            ));
        }
        Ok(manifest)
    }
}

/// What a build over an earlier one reads of the manifest that one wrote
/// at its path (ADR 0091): the input and output it was of, and the files it
/// wrote, by their bytes, which this build may remove. Of another build of
/// the compiler too, whose manifest holds other fields: Cargo builds a crate
/// again in place where rust-js's build changed (ADR 1003, amended, #2473),
/// so a build reads the last one's, and its contract, written by another
/// compiler's schema, is none of this build's to read. Read whole, as
/// `Manifest::read_from` reads it, a field the other compiler's library
/// hadn't refused the build: "missing field `crate`" (2026-11-25, a kept
/// app of the guide's).
#[derive(Deserialize)]
pub struct Previous {
    /// The manifest's own version, which a host reads it by: another is
    /// refused, as `Manifest::read` refuses it.
    pub version: u32,
    pub input: PathBuf,
    pub output: PathBuf,
    pub artifacts: Vec<Artifact>,
}

/// Which compiler wrote the manifest at `path`, read alone: what a
/// consumer refuses another build's manifest by, before it reads the rest
/// by its own schema, which another build's may not be (#2473).
pub fn compiler_of(path: &Path) -> Result<Option<Compiler>, String> {
    #[derive(Deserialize)]
    struct Written {
        #[serde(default)]
        compiler: Option<Compiler>,
    }
    let written: Written =
        serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok(written.compiler)
}

impl Previous {
    /// The manifest at `path` as an earlier build's, its paths resolved
    /// against its directory, as `Manifest::read_from` resolves them.
    pub fn read_from(path: &Path) -> Result<Self, String> {
        let mut previous: Self =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        if previous.version != VERSION {
            return Err(format!(
                "unsupported manifest version {}; expected {VERSION}",
                previous.version
            ));
        }
        let dir = path.parent().ok_or("a manifest has a directory")?;
        let abs = |path: &mut PathBuf| {
            if path.is_relative() {
                *path = dir.join(&*path);
            }
        };
        abs(&mut previous.input);
        abs(&mut previous.output);
        previous.artifacts.iter_mut().for_each(|a| abs(&mut a.file));
        Ok(previous)
    }
}

/// An artifact's `hash`: what a build wrote, so a later one removes only
/// what's still as written, and a consumer sees a library's file changed.
pub fn fingerprint(bytes: &[u8]) -> String {
    // Stable across compiler releases; an ownership check, not a security hash.
    let hash = bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}")
}
