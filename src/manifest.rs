//! Versioned build-tool contract. Paths are resolved by artifact planning.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;
pub const ABI_VERSION: u32 = 1;

#[derive(PartialEq, Serialize, Deserialize)]
pub struct Compiler {
    pub version: String,
    pub toolchain: String,
    pub abi: u32,
}

impl Compiler {
    pub fn current() -> Self {
        let toolchain = include_str!("../rust-toolchain.toml")
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
    /// Its `#[test]` functions' runner, `<output>.test.js`, of a build with
    /// `--test` (ADR 0026): what a host runs them by.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tests: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
pub struct Module {
    pub module: Vec<String>,
    pub file: PathBuf,
    pub map: PathBuf,
    /// Its `.d.ts`, where the crate asks for one (ADR 0196).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub types: Option<PathBuf>,
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
        }
        self.artifacts.iter_mut().for_each(|a| rel(&mut a.file));
        self.stale.iter_mut().for_each(|a| rel(&mut a.file));
        if let Some(library) = &mut self.library {
            library.inputs.iter_mut().for_each(|a| rel(&mut a.file));
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

/// An artifact's `hash`: what a build wrote, so a later one removes only
/// what's still as written, and a consumer sees a library's file changed.
pub(crate) fn fingerprint(bytes: &[u8]) -> String {
    // Stable across compiler releases; an ownership check, not a security hash.
    let hash = bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}")
}
