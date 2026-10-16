//! The panic runtime rustc is given where it wants one (ADR 0090): a
//! program's panics unwind, as rust-js's do, and a session that builds more
//! than a library, a test harness's, is given a panic runtime that unwinds.
//! wasm32's own, `panic_unwind` as its standard library ships it, is built
//! to abort, so rustc would refuse it; nothing is linked, so what's given is
//! a runtime's metadata alone, which rustc writes to the session's stage.

use rustc_driver::Callbacks;
use std::path::Path;

/// The runtime's source: a crate that says it's a panic runtime, and nothing
/// more, as what's linked is never asked for.
const SOURCE: &str = "#![no_std]\n#![feature(panic_runtime)]\n#![panic_runtime]\n";

/// Write the runtime's metadata to `to`, `libpanic_unwind.rmeta`, for the
/// session's `--target` and `--sysroot`, as `(flag, value)`s: the
/// playground's standard library is its own.
pub fn make(to: &Path, session: &[(&str, String)]) -> Result<(), String> {
    let mut runtime_args = vec![
        "rust-js".to_string(),
        // Named for rustc's arguments, read as `SOURCE` (`Runtime`).
        "panic_unwind.rs".to_string(),
        "--crate-name=panic_unwind".to_string(),
        "--crate-type=rlib".to_string(),
        "--edition=2021".to_string(),
        "-Cpanic=unwind".to_string(),
        "-Awarnings".to_string(),
        format!("--emit=metadata={}", to.display()),
    ];
    runtime_args.extend(session.iter().map(|(flag, value)| format!("{flag}={value}")));
    let exit = rustc_driver::catch_with_exit_code(|| rustc_driver::run_compiler(&runtime_args, &mut Runtime));
    match exit == std::process::ExitCode::SUCCESS {
        true => Ok(()),
        false => Err("cannot make the panic runtime a test harness is given".into()),
    }
}

/// rustc, of `SOURCE`, allowed the one unstable feature a panic runtime is
/// declared with.
struct Runtime;

impl Callbacks for Runtime {
    fn config(&mut self, config: &mut rustc_interface::interface::Config) {
        config.input = rustc_session::config::Input::Str {
            name: rustc_span::FileName::Custom("panic_unwind".into()),
            input: SOURCE.into(),
        };
        config.opts.unstable_features = rustc_feature::UnstableFeatures::Allow;
    }
}
