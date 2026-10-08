//! Versioned build-tool contract. Paths are resolved by artifact planning.

use std::path::PathBuf;

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
        let value = |text: &str, key: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(key))
                .expect("version is declared in the checked-in configuration")
                .trim()
                .trim_matches('"')
                .to_owned()
        };
        Self {
            version: value(include_str!("../Cargo.toml"), "version = "),
            toolchain: value(include_str!("../rust-toolchain.toml"), "channel = "),
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
    /// Whether its `source` is where a `#[path]` says (ADR 0273), where its
    /// JS goes in source too.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub located: bool,
    pub imports: Vec<PathBuf>,
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

#[derive(Serialize, Deserialize)]
pub struct Artifact {
    pub file: PathBuf,
    pub hash: String,
}

impl Manifest {
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
