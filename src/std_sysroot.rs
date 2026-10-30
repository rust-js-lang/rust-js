//! The standard library `--std-bodies` lowers std's bodies of, built to
//! unwind (#424): wasm32's, as the toolchain ships it, is built to abort, so
//! each call of its MIR unwinds nowhere, and a compiled body would drop
//! nothing a panic unwinds past. `@rust-js/std` is `core`'s bodies alone
//! (`std/lib.rs` is `#![no_std]`), so what's built is `core`, of the
//! toolchain's own source, its metadata alone with each body's MIR, and a
//! `compiler_builtins` that's a crate and nothing more, as the panic runtime
//! a test harness is given is (`panic_runtime`). Nothing is linked. It's
//! built once for a toolchain and a target, into the compiler's own
//! directory, and given to the session as its `--sysroot`. Std's items are
//! keyed by their paths, which this `core` shares with the one a program's
//! session reads (`lower::library::key`).

use rustc_driver::Callbacks;
use std::path::{Path, PathBuf};

/// The `compiler_builtins` a `#![no_std]` crate's session loads: a crate
/// that says it's that, and nothing more, as what it links is never asked
/// for.
const BUILTINS: &str = "#![no_std]\n#![feature(compiler_builtins)]\n#![compiler_builtins]\n";

/// What each crate's session is set to beside its arguments: each body's
/// MIR kept in its metadata, which `--std-bodies` reads, and MIR inlined,
/// as std's own build has it.
#[derive(Hash)]
struct Settings {
    always_encode_mir: bool,
    inline_mir: bool,
}

const SETTINGS: Settings = Settings {
    always_encode_mir: true,
    inline_mir: true,
};

/// The sysroot whose `core`, for `target`, is built to unwind, of the
/// toolchain at `toolchain`, the session's own sysroot: made where it
/// isn't yet, beside the compiler.
pub fn sysroot(toolchain: &Path, target: &str) -> Result<PathBuf, String> {
    let home = match std::env::var_os("RUST_JS_STD_SYSROOT") {
        Some(dir) => PathBuf::from(dir),
        None => std::env::current_exe()
            .map_err(|error| format!("cannot find the compiler's directory: {error}"))?
            .with_file_name("std-sysroot"),
    };
    let core = toolchain.join("lib/rustlib/src/rust/library/core/src/lib.rs");
    if !core.exists() {
        return Err(format!(
            "std's source isn't there, `{}`: `rustup component add rust-src`",
            core.display()
        ));
    }
    // Kept by what it's built of: the toolchain, which its path names, the
    // target, and how each crate is built, its arguments and its settings,
    // so one built otherwise is never taken for it.
    let args = |building: &Path| {
        let lib = building.join("lib/rustlib").join(target).join("lib");
        [
            crate_args("core", &core.display().to_string(), building, &lib, target),
            crate_args("compiler_builtins", "compiler_builtins.rs", building, &lib, target),
        ]
    };
    let mut key = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(&(toolchain, BUILTINS, &SETTINGS, args(Path::new(""))), &mut key);
    let name = format!("{target}-{:016x}", std::hash::Hasher::finish(&key));
    let made = home.join(&name);
    if made.join("done").exists() {
        return Ok(made);
    }
    let building = home.join(format!(".{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&building);
    let lib = building.join("lib/rustlib").join(target).join("lib");
    std::fs::create_dir_all(&lib).map_err(|error| format!("cannot make the unwinding std's sysroot: {error}"))?;
    let [core_args, builtins_args] = args(&building);
    build(core_args, None)?;
    build(builtins_args, Some(BUILTINS))?;
    std::fs::write(building.join("done"), "").map_err(|error| format!("cannot finish the unwinding std: {error}"))?;
    // Another build's that finished first is as good.
    if std::fs::rename(&building, &made).is_err() {
        let _ = std::fs::remove_dir_all(&building);
    }
    match made.join("done").exists() {
        true => Ok(made),
        false => Err("cannot keep the unwinding std's sysroot".into()),
    }
}

/// rustc's arguments for `name`, of `input`, its metadata written into
/// `lib`, the sysroot `building`'s: built to unwind, its bodies' MIR kept.
fn crate_args(name: &str, input: &str, building: &Path, lib: &Path, target: &str) -> Vec<String> {
    vec![
        "rust-js".to_string(),
        input.to_string(),
        format!("--crate-name={name}"),
        "--crate-type=rlib".to_string(),
        "--edition=2024".to_string(),
        "-Cpanic=unwind".to_string(),
        "-Copt-level=3".to_string(),
        format!("-Cmetadata=rust-js-unwind-{name}"),
        "--cap-lints=allow".to_string(),
        format!("--target={target}"),
        format!("--sysroot={}", building.display()),
        format!("--emit=metadata={}", lib.join(format!("lib{name}.rmeta")).display()),
    ]
}

/// rustc run on `args`, its input `source` where it's given one.
fn build(args: Vec<String>, source: Option<&'static str>) -> Result<(), String> {
    let mut crate_build = Crate(source);
    let exit = rustc_driver::catch_with_exit_code(|| rustc_driver::run_compiler(&args, &mut crate_build));
    match exit == std::process::ExitCode::SUCCESS {
        true => Ok(()),
        false => Err(format!(
            "cannot build the unwinding std's `{}`",
            args[2].trim_start_matches("--crate-name=")
        )),
    }
}

/// rustc, allowed std's unstable features, of its source where it has one.
struct Crate(Option<&'static str>);

impl Callbacks for Crate {
    fn config(&mut self, config: &mut rustc_interface::interface::Config) {
        if let Some(source) = self.0 {
            config.input = rustc_session::config::Input::Str {
                name: rustc_span::FileName::Custom("compiler_builtins".into()),
                input: source.into(),
            };
        }
        config.opts.unstable_features = rustc_feature::UnstableFeatures::Allow;
        config.opts.unstable_opts.always_encode_mir = SETTINGS.always_encode_mir;
        config.opts.unstable_opts.inline_mir = Some(SETTINGS.inline_mir);
    }
}
