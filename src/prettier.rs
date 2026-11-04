//! The app's own layout of what rust-js writes (ADR 0381): where Prettier
//! finds a config for a file, as the app's `prettier --check` does, the
//! app's Prettier lays it out, by that config, all of a build's files in
//! one session of it, run where the crate is. Elsewhere rust-js's own
//! layout stands: an app without Prettier, a file Prettier has no config
//! for, or a file compiled alone, which is no app's.

use std::path::PathBuf;

use serde::Deserialize;

use crate::node_session::{self, Failure};
use crate::settings::Settings;

/// The app's Prettier, found from where it is, given the files, writing
/// what it lays out (`node_session`'s): for each file, its text,
/// or `null` where Prettier finds no config for it, or where the app has no
/// Prettier; and the configs it read.
const LAY_OUT: &str = r#"// A session that doesn't finish says where it was, not hangs its build.
let at = "importing prettier";
setTimeout(() => {
  process.stderr.write(`no answer in 60 s, ${at}\n`);
  process.exit(3);
}, 60_000).unref();
let prettier;
try {
  // Prettier 3's API is its module's; Prettier 2's, CommonJS, its `default`.
  const module = await import("prettier");
  prettier = typeof module.format === "function" ? module : module.default;
} catch (error) {
  if (error?.code !== "ERR_MODULE_NOT_FOUND" || !String(error.message).includes("'prettier'")) throw error;
}
at = "reading the files";
const texts = [];
const configs = new Set();
for (const { file, code } of await given()) {
  at = `laying out ${file}`;
  const config = prettier && (await prettier.resolveConfigFile(file));
  if (!config) {
    texts.push(null);
    continue;
  }
  configs.add(config);
  const options = await prettier.resolveConfig(file, { editorconfig: true });
  texts.push(await prettier.format(code, { ...options, filepath: file }));
}
process.stdout.write(JSON.stringify({ texts, configs: [...configs] }));
"#;

/// Each directory's Prettier, as `LAY_OUT` imports it where the crate is:
/// the manifest of the package it resolves to, or `null` where none is.
const FIND: &str = r#"import { findPackageJSON } from "node:module";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
const found = [];
for (const dir of await given()) {
  try {
    found.push(findPackageJSON("prettier", pathToFileURL(join(dir, "lib.js"))) ?? null);
  } catch (error) {
    if (error?.code !== "ERR_MODULE_NOT_FOUND") throw error;
    found.push(null);
  }
}
process.stdout.write(JSON.stringify(found));
"#;

/// Which Prettier lays out the JS of the crates in `dirs`, as Cargo is told
/// it beside the compiler's build (ADR 1003, amended; #684): a hash of each
/// one's installed package manifest, its version with it, found as the
/// layout's session finds it. Its configs are in each crate's dep-info; an
/// upgrade of it at the same path, as old as the cache, is no change to
/// any file Cargo dates, so it's said by what it is. `None` where no crate
/// has one, whose JS is rust-js's own layout.
pub fn identity(dirs: &[PathBuf]) -> Result<Option<String>, String> {
    let failed = |why: String| format!("the app's Prettier, which `{}` looks for, {why}", node_session::node());
    let given: Vec<serde_json::Value> = dirs.iter().map(|dir| serde_json::json!(dir)).collect();
    let here = std::env::current_dir().map_err(|e| failed(format!("from no directory: {e}")))?;
    let found: Vec<Option<PathBuf>> = node_session::run(&here, FIND, &given).map_err(|failure| {
        failed(match failure {
            Failure::CantRun(e) => format!("can't be run: {e}"),
            Failure::Failed(e) => format!("failed: {e}"),
            Failure::Exited(stderr) => format!("failed:\n{stderr}"),
            Failure::Unreadable(e) => format!("gave what isn't its packages: {e}"),
        })
    })?;
    if found.len() != dirs.len() {
        return Err(failed(format!(
            "gave {} packages of {} crates",
            found.len(),
            dirs.len()
        )));
    }
    if found.iter().all(Option::is_none) {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    for (dir, manifest) in dirs.iter().zip(&found) {
        let read = match manifest {
            Some(manifest) => std::fs::read(manifest).map_err(|e| failed(format!("{}: {e}", manifest.display())))?,
            None => Vec::new(),
        };
        for part in [dir.as_os_str().as_encoded_bytes(), &read] {
            bytes.extend((part.len() as u64).to_le_bytes());
            bytes.extend(part);
        }
    }
    Ok(Some(crate::manifest::fingerprint(&bytes)))
}

/// What the app's Prettier made of a build's files.
#[derive(Default, Deserialize)]
pub struct LaidOut {
    /// Each file's text, in the order given; `None` where it's rust-js's.
    pub texts: Vec<Option<String>>,
    /// The configs it read, which another build reads again if they change.
    pub configs: Vec<PathBuf>,
}

/// `files`, each where it goes, beside its Rust, whose config Prettier
/// reads, as the app's `prettier --check` does, and its text, as the app's
/// Prettier lays them out, or why it couldn't.
pub fn laid_out(settings: &Settings, files: &[(PathBuf, &str)]) -> Result<LaidOut, String> {
    // The playground has no app; nor does a file compiled alone, a test's
    // (ADR 0414): the `Cargo.toml` above it is maybe another's, its JS is
    // nobody's to check, and asking is Node started and Prettier imported,
    // 30 ms of a small file's 80.
    if files.is_empty() || !settings.cargo || cfg!(target_os = "wasi") {
        return Ok(LaidOut {
            texts: vec![None; files.len()],
            configs: Vec::new(),
        });
    }
    let failed = |why: String| {
        format!(
            "the app's Prettier, which `{}` runs where the crate is, `{}`, {why}",
            node_session::node(),
            settings.dir.display()
        )
    };
    let given: Vec<serde_json::Value> = files
        .iter()
        .map(|(file, code)| serde_json::json!({ "file": file, "code": code }))
        .collect();
    let laid_out: LaidOut = node_session::run(&settings.dir, LAY_OUT, &given).map_err(|failure| {
        failed(match failure {
            Failure::CantRun(e) => format!("can't be run: {e}"),
            Failure::Failed(e) => format!("failed: {e}"),
            Failure::Exited(stderr) => format!("failed:\n{stderr}"),
            Failure::Unreadable(e) => format!("gave what isn't its layout: {e}"),
        })
    })?;
    if laid_out.texts.len() != files.len() {
        return Err(failed(format!(
            "gave {} texts of {} files",
            laid_out.texts.len(),
            files.len()
        )));
    }
    Ok(laid_out)
}
