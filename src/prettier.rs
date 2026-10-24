//! The app's own layout of what rust-js writes (ADR 0381): where Prettier
//! finds a config for a file, as the app's `prettier --check` does, the
//! app's Prettier lays it out, by that config, all of a build's files in
//! one session of it, run where the crate is. Elsewhere rust-js's own
//! layout stands: an app without Prettier, a file Prettier has no config
//! for, or a file compiled alone, which is no app's.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::settings::Settings;

/// The app's Prettier, found from where it is, given the files on its
/// stdin, writing what it lays out on its stdout: for each file, its text,
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
let input = "";
for await (const chunk of process.stdin) input += chunk;
const texts = [];
const configs = new Set();
for (const { file, code } of JSON.parse(input)) {
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
    // Node runs it, as it does a crate's JS; `RUST_JS_NODE` names another.
    let node = std::env::var("RUST_JS_NODE").unwrap_or_else(|_| "node".to_string());
    let failed = |why: String| {
        format!(
            "the app's Prettier, which `{node}` runs where the crate is, `{}`, {why}",
            settings.dir.display()
        )
    };
    let mut child = Command::new(&node)
        .args(["--input-type=module", "--eval", LAY_OUT])
        .current_dir(&settings.dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| failed(format!("can't be run: {e}")))?;
    let given: Vec<serde_json::Value> = files
        .iter()
        .map(|(file, code)| serde_json::json!({ "file": file, "code": code }))
        .collect();
    let input = serde_json::to_string(&given).map_err(|e| e.to_string())?;
    let mut stdin = child.stdin.take().expect("a piped stdin");
    // Written as it's read, so neither side waits on a full pipe.
    let output = std::thread::scope(|scope| {
        scope.spawn(move || stdin.write_all(input.as_bytes()));
        child.wait_with_output()
    })
    .map_err(|e| failed(format!("failed: {e}")))?;
    if !output.status.success() {
        return Err(failed(format!(
            "failed:\n{}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        )));
    }
    let laid_out: LaidOut =
        serde_json::from_slice(&output.stdout).map_err(|e| failed(format!("gave what isn't its layout: {e}")))?;
    if laid_out.texts.len() != files.len() {
        return Err(failed(format!(
            "gave {} texts of {} files",
            laid_out.texts.len(),
            files.len()
        )));
    }
    Ok(laid_out)
}
