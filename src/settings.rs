//! A crate's settings for the JS rust-js writes (ADR 0117), in its
//! `Cargo.toml`'s `[package.metadata.rust-js]`: the hooks it runs, and
//! whether it has declarations. How the JS is laid out isn't one: the
//! app's Prettier config says (ADR 0381).

use std::path::PathBuf;

use serde::Deserialize;

#[cfg(not(target_os = "wasi"))]
pub use cargo_toml::read;

/// What a crate's settings are: none, but for a crate that says.
#[derive(Default)]
pub struct Settings {
    pub hooks: Hooks,
    /// `declarations = true`: a `.d.ts` beside each module's JS, for
    /// TypeScript that imports it (ADR 0196).
    pub declarations: bool,
    /// Where its `Cargo.toml` is, which its hooks are run in, and what's
    /// said of a file is said from.
    pub dir: PathBuf,
}

/// `[package.metadata.rust-js.hooks]`.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hooks {
    /// Commands a module's text goes through, in order, each one's stdin
    /// the text, its stdout the text it gives.
    #[serde(default)]
    pub transform: Vec<Vec<String>>,
    /// Commands run on what a build wrote.
    #[serde(default)]
    pub check: Vec<Check>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    pub run: Vec<String>,
    #[serde(default)]
    pub fail: Fail,
    #[serde(default)]
    pub when: When,
}

/// What a check's failure, an exit otherwise than 0, is.
#[derive(Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Fail {
    /// The build's.
    #[default]
    Build,
    /// Its output, as warnings.
    Never,
}

/// Which builds run a check.
#[derive(Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum When {
    /// Each one, a save in the Vite server too.
    #[default]
    Always,
    /// A build, not a save.
    Build,
}

/// Reading them from a `Cargo.toml`, natively: the playground has none.
#[cfg(not(target_os = "wasi"))]
mod cargo_toml {
    use std::path::{Path, PathBuf};

    use serde::Deserialize;

    use super::{Hooks, Settings};

    /// `[package.metadata.rust-js]`, of which these are two tables: ADR
    /// 0116's are others.
    #[derive(Default, Deserialize)]
    struct RustJs {
        /// Gone, the formatter's options: the app's Prettier config's.
        format: Option<toml::Value>,
        #[serde(default)]
        hooks: Hooks,
        #[serde(default)]
        declarations: bool,
    }

    /// Whether `command`'s program is Prettier's, by its file's name.
    fn runs_prettier(command: &[String]) -> bool {
        command
            .first()
            .and_then(|program| Path::new(program).file_stem())
            .is_some_and(|name| name == "prettier")
    }

    #[derive(Deserialize)]
    struct Manifest {
        package: Option<Package>,
    }

    #[derive(Deserialize)]
    struct Package {
        metadata: Option<Metadata>,
    }

    #[derive(Deserialize)]
    struct Metadata {
        #[serde(rename = "rust-js")]
        rust_js: Option<RustJs>,
    }

    /// The settings of the crate whose root is `input`: its `Cargo.toml`'s,
    /// Cargo's for a crate it builds, `CARGO_MANIFEST_DIR`, or else the
    /// nearest above it. None without one.
    pub fn read(input: &Path, cargo: bool) -> Result<Settings, String> {
        let dir = if cargo {
            std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from)
        } else {
            let input = crate::paths::absolute(input)?;
            input
                .ancestors()
                .skip(1)
                .find(|dir| dir.join("Cargo.toml").is_file())
                .map(PathBuf::from)
        };
        let Some(dir) = dir else {
            return Ok(Settings::default());
        };
        let path = dir.join("Cargo.toml");
        let text = std::fs::read_to_string(&path).map_err(|e| format!("cannot read `{}`: {e}", path.display()))?;
        let manifest: Manifest = toml::from_str(&text).map_err(|e| format!("`{}`: {e}", path.display()))?;
        let settings = manifest
            .package
            .and_then(|package| package.metadata)
            .and_then(|metadata| metadata.rust_js)
            .unwrap_or_default();
        // How the JS is laid out is the app's Prettier config's, which
        // its `prettier --check` reads too: one place says (ADR 0381).
        if settings.format.is_some() {
            return Err(format!(
                "`{}`: `[package.metadata.rust-js.format]` is gone: rust-js lays its JS out with the app's Prettier, by its config, `.prettierrc`: move the options there",
                path.display()
            ));
        }
        if let Some(prettier) = settings.hooks.transform.iter().find(|command| runs_prettier(command)) {
            return Err(format!(
                "`{}`: the transform `{}` runs Prettier, which rust-js runs itself, by the app's config, `.prettierrc`, where Prettier finds one: remove it",
                path.display(),
                prettier.join(" ")
            ));
        }
        Ok(Settings {
            hooks: settings.hooks,
            declarations: settings.declarations,
            dir,
        })
    }
}

#[cfg(target_os = "wasi")]
pub fn read(_: &std::path::Path, _: bool) -> Result<Settings, String> {
    Ok(Settings::default())
}
