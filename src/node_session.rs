//! A session of Node's that a printing adapter runs where the crate is,
//! TypeScript's (ADR 0207) and the app's Prettier's (ADR 0381): its script
//! given JSON on its stdin and writing JSON on its stdout. How it reads
//! what it's given is this module's, one way for each session.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::de::DeserializeOwned;

/// What a session's script reads its input by, `await given()`: its
/// stdin's JSON, decoded as one UTF-8 text, so a character a pipe's chunks
/// split, React's `⚛`, is read whole, not each chunk's bytes alone as two
/// `\u{FFFD}` (rd716, ADR 1079).
const GIVEN: &str = r#"const given = async () => {
  process.stdin.setEncoding("utf8");
  let input = "";
  for await (const chunk of process.stdin) input += chunk;
  return JSON.parse(input);
};
"#;

/// Why a session gave no answer.
pub enum Failure {
    /// Node can't be started.
    CantRun(std::io::Error),
    /// Its pipes failed.
    Failed(std::io::Error),
    /// It exited unsuccessfully, saying this on its stderr.
    Exited(String),
    /// What it wrote isn't the answer's JSON.
    Unreadable(serde_json::Error),
}

/// The Node that runs a session, as it runs a crate's JS: `RUST_JS_NODE`
/// names another.
pub fn node() -> String {
    std::env::var("RUST_JS_NODE").unwrap_or_else(|_| "node".to_string())
}

/// What `script`, an ES module that reads `input` by `await given()`,
/// writes on its stdout, run by `node()` in `dir`.
pub fn run<T: DeserializeOwned>(dir: &Path, script: &str, input: &[serde_json::Value]) -> Result<T, Failure> {
    let mut child = Command::new(node())
        .args(["--input-type=module", "--eval", &format!("{GIVEN}{script}")])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(Failure::CantRun)?;
    let input = serde_json::to_string(input).expect("JSON values are JSON");
    let mut stdin = child.stdin.take().expect("a piped stdin");
    // Written as it's read, so neither side waits on a full pipe.
    let output = std::thread::scope(|scope| {
        scope.spawn(move || stdin.write_all(input.as_bytes()));
        child.wait_with_output()
    })
    .map_err(Failure::Failed)?;
    if !output.status.success() {
        return Err(Failure::Exited(
            String::from_utf8_lossy(&output.stderr).trim_end().to_string(),
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(Failure::Unreadable)
}
