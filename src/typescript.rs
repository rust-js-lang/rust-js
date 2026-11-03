//! A crate's `.d.ts`, printed by TypeScript (ADR 0207): each module's
//! declarations, @rust-js/typescript's model (ADR 0206), which builds them
//! as TypeScript's syntax tree and prints them with TypeScript's printer,
//! all in one session of TypeScript's, run where the crate is.

use crate::node_session::{self, Failure};
use crate::settings::Settings;

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
