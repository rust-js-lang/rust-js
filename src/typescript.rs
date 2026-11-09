//! A crate's `.d.ts`, printed by TypeScript (ADR 0207): each module's
//! declarations, @rust-js/typescript's model (ADR 0206), which builds them
//! as TypeScript's syntax tree and prints them with TypeScript's printer,
//! all in one session of TypeScript's, run where the crate is.

use std::path::{Path, PathBuf};

use crate::node_session::{self, Failure};
use crate::settings::Settings;

/// Where a build's `@rust-js/typescript` is, its module's file: `rust-js
/// build`'s own, which it gives Cargo (ADR 0412).
const PRINTER: &str = "RUST_JS_TYPESCRIPT";

/// The printer `PRINTER` names, its module and its package's manifest, or
/// `None` where no build names one: what both of what follow are of.
fn named_printer() -> Result<Option<(PathBuf, PathBuf)>, String> {
    let Some(module) = std::env::var_os(PRINTER) else {
        return Ok(None);
    };
    let module = crate::paths::absolute(Path::new(&module))?;
    let package = (module.ancestors().skip(1))
        .map(|dir| dir.join("package.json"))
        .find(|manifest| manifest.is_file())
        .ok_or_else(|| format!("`{PRINTER}`, {}, is in no package", module.display()))?;
    Ok(Some((module, package)))
}

/// Which `@rust-js/typescript` prints a build's `.d.ts`, `PRINTER`'s, as
/// Cargo is told it beside the compiler's build (ADR 1003, amended): a hash
/// of its module and of its package's manifest, which pins the TypeScript it
/// runs. What it prints is what rust-js writes, but it isn't in the binary,
/// so a change to it, a checkout's `typescript/index.js`, is a build again,
/// as a change to the compiler's sources is (#675). `None` where no build
/// names one, and a crate's own is found from where it is.
pub fn printer() -> Result<Option<String>, String> {
    let Some((module, package)) = named_printer()? else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    for file in [module.as_path(), package.as_path()] {
        let read = std::fs::read(file).map_err(|e| format!("`{PRINTER}`'s {}: {e}", file.display()))?;
        bytes.extend((read.len() as u64).to_le_bytes());
        bytes.extend(read);
    }
    Ok(Some(crate::manifest::fingerprint(&bytes)))
}

/// What a build that prints declarations reads of TypeScript, where
/// `PRINTER` names the printer (ADR 1099, amended): its module, its
/// package's manifest, and the manifest of the TypeScript its module finds
/// as Node does, the nearest `node_modules/typescript` up from it, which
/// pins the release it runs, its platform's binary of the same. Given the
/// models, what it prints is of those alone. `None` where no build names
/// one: the crate's own, found from where it is, isn't known here.
pub fn printer_reads() -> Result<Option<Vec<PathBuf>>, String> {
    let Some((module, package)) = named_printer()? else {
        return Ok(None);
    };
    let typescript = (module.ancestors().skip(1))
        .map(|dir| dir.join("node_modules").join("typescript").join("package.json"))
        .find(|manifest| manifest.is_file())
        .ok_or_else(|| format!("`{PRINTER}`, {}, finds no TypeScript", module.display()))?;
    Ok(Some(vec![module, package, typescript]))
}

/// `@rust-js/typescript`, where `RUST_JS_TYPESCRIPT` says, `rust-js
/// build`'s own (ADR 0412), or else the crate's, found from where it is,
/// given the models, writing their texts (`node_session`'s).
const PRINT: &str = r#"import { pathToFileURL } from "node:url";
const at = process.env.RUST_JS_TYPESCRIPT;
const { printModules } = await import(at ? pathToFileURL(at).href : "@rust-js/typescript");
process.stdout.write(JSON.stringify(await printModules(await given())));
"#;

/// The text of each of `modules`' `.d.ts`, as TypeScript prints them, or
/// why there's none.
pub fn print(settings: &Settings, modules: &[serde_json::Value]) -> Result<Vec<String>, String> {
    let node = node_session::node();
    let needs = format!(
        "a crate's declarations are printed by TypeScript, through @rust-js/typescript, which `{node}` runs where the crate is, `{}`: install `@rust-js/build`, which has it",
        settings.dir.display()
    );
    let texts: Vec<String> = node_session::run(&settings.dir, PRINT, modules).map_err(|failure| match failure {
        Failure::CantRun(e) => format!("{needs}; `{node}` can't be run: {e}"),
        Failure::Failed(e) => format!("{needs}; `{node}` failed: {e}"),
        Failure::Exited(stderr) => format!("{needs}:\n{stderr}"),
        Failure::Unreadable(e) => format!("{needs}; it gave what isn't its texts: {e}"),
    })?;
    if texts.len() != modules.len() {
        return Err(format!(
            "{needs}; it gave {} texts of {} modules",
            texts.len(),
            modules.len()
        ));
    }
    Ok(texts)
}
