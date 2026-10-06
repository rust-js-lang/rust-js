//! A crate's `.d.ts`, printed by TypeScript (ADR 0207): each module's
//! declarations, @rust-js/typescript's model (ADR 0206), which builds them
//! as TypeScript's syntax tree and prints them with TypeScript's printer,
//! all in one session of TypeScript's, run where the crate is.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::settings::Settings;

/// The module the crate has, found from where it is, given the models on
/// its stdin, writing their texts on its stdout.
const PRINT: &str = r#"import { printModules } from "@rust-js/typescript";
let input = "";
for await (const chunk of process.stdin) input += chunk;
process.stdout.write(JSON.stringify(await printModules(JSON.parse(input))));
"#;

/// The text of each of `modules`' `.d.ts`, as TypeScript prints them, or
/// why there's none.
pub fn print(settings: &Settings, modules: &[serde_json::Value]) -> Result<Vec<String>, String> {
    // Node runs it, as it does a crate's JS; `RUST_JS_NODE` names another.
    let node = std::env::var("RUST_JS_NODE").unwrap_or_else(|_| "node".to_string());
    let needs = format!(
        "a crate's declarations (`declarations = true`) are printed by TypeScript, through @rust-js/typescript, which `{node}` runs where the crate is, `{}`: add it, `npm install --save-dev @rust-js/typescript`",
        settings.dir.display()
    );
    let mut child = Command::new(&node)
        .args(["--input-type=module", "--eval", PRINT])
        .current_dir(&settings.dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{needs}; `{node}` can't be run: {e}"))?;
    let input = serde_json::to_string(modules).map_err(|e| e.to_string())?;
    let mut stdin = child.stdin.take().expect("a piped stdin");
    // Written as it's read, so neither side waits on a full pipe.
    let output = std::thread::scope(|scope| {
        scope.spawn(move || stdin.write_all(input.as_bytes()));
        child.wait_with_output()
    })
    .map_err(|e| format!("{needs}; `{node}` failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{needs}:\n{}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        ));
    }
    let texts: Vec<String> =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("{needs}; it gave what isn't its texts: {e}"))?;
    if texts.len() != modules.len() {
        return Err(format!(
            "{needs}; it gave {} texts of {} modules",
            texts.len(),
            modules.len()
        ));
    }
    Ok(texts)
}
