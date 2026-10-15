//! rust-js as Cargo's `RUSTC_WRAPPER` (ADRs 0101, 0414). Cargo runs it with
//! rustc's path and the flags it worked out, for each crate it builds, and
//! for its probes of rustc. A library of the app's own, one `RUST_JS_CRATES`
//! names, checked for rust-js's target, is compiled by rust-js: its metadata
//! where Cargo expects it (ADR 0100), and its JS and manifest beside it, in
//! `rust-js/<crate>-<hash>/`, one for each build of it Cargo keeps. Anything
//! else, a probe, a build script, a procedural macro, a native build, a
//! registry's crate, is rustc's, as Cargo asked.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::command::{TARGET, flag_values};
use crate::manifest::Compiler;

/// What running rust-js as it was run comes to.
pub enum Invocation {
    /// rust-js's own arguments: as given, or as Cargo's flags make them.
    RustJs(Vec<String>),
    /// rustc was run as Cargo asked, and ended so.
    Rustc(ExitCode),
}

/// The package the tooling builds as the app, a program no crate uses
/// (ADR 0360).
pub const APP: &str = "RUST_JS_APP";

/// The crates rust-js compiles, the app's own, by their directories, as the
/// tooling says: each of Cargo's packages of a path, its workspace's, a path
/// dependency, or a crate an npm package has, but a binding crate (ADR 0414).
pub const CRATES: &str = "RUST_JS_CRATES";

pub fn translate(args: Vec<String>) -> Invocation {
    let Some(rustc) = args
        .first()
        .filter(|arg| Path::new(arg).file_stem().is_some_and(|stem| stem == "rustc"))
        .cloned()
    else {
        return Invocation::RustJs(args);
    };
    let given = &args[1..];
    if let [flag] = given
        && flag == "-vV"
    {
        return verbose_version(&rustc);
    }
    let flags = match expand(given) {
        Ok(flags) => flags,
        Err(error) => {
            eprintln!("rust-js: {error}");
            return Invocation::Rustc(ExitCode::FAILURE);
        }
    };
    let library = flag_values(&flags, "--crate-type").any(|kind| kind == "lib" || kind == "rlib");
    let probe = flags
        .iter()
        .any(|flag| flag == "-" || flag == "-vV" || flag.starts_with("--print"));
    // rustc is rust-js's `--rustc`, with its tool known, as the tooling runs
    // Cargo (ADR 0112): a binding crate's, which is metadata only (ADR 0024),
    // and a registry's, which `RUST_JS_CRATES` leaves out, are checked by it.
    if probe || !library || flag_values(&flags, "--target").next().as_deref() != Some(TARGET) {
        return rustc_itself(&rustc, given);
    }
    match is_own() {
        Ok(true) => {}
        Ok(false) => return rustc_itself(&rustc, given),
        Err(error) => {
            eprintln!("rust-js: {error}");
            return Invocation::Rustc(ExitCode::FAILURE);
        }
    }
    match compile(&flags) {
        Ok(args) => Invocation::RustJs(args),
        Err(error) => {
            eprintln!("rust-js: {error}");
            Invocation::Rustc(ExitCode::FAILURE)
        }
    }
}

/// Whether the crate Cargo builds is one of the app's own, which
/// `RUST_JS_CRATES` names, by where Cargo has it.
fn is_own() -> Result<bool, String> {
    let crates = std::env::var(CRATES).map_err(|_| {
        format!(
            "Cargo ran rust-js without `{CRATES}`, the crates it compiles, which `rust-js build` gives it: run that"
        )
    })?;
    let crates: Vec<PathBuf> =
        serde_json::from_str(&crates).map_err(|e| format!("`{CRATES}` isn't a list of directories: {e}"))?;
    let dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| "Cargo gave no CARGO_MANIFEST_DIR")?;
    let dir = std::fs::canonicalize(&dir).map_err(|e| format!("{dir}: {e}"))?;
    Ok(crates.contains(&dir))
}

/// rust-js's arguments for a library Cargo checks for its target.
fn compile(flags: &[String]) -> Result<Vec<String>, String> {
    let name = flag_values(flags, "--crate-name")
        .next()
        .ok_or("Cargo gave no --crate-name")?;
    let source = flags
        .iter()
        .find(|flag| flag.ends_with(".rs"))
        .ok_or("Cargo gave no source file")?
        .clone();
    let out_dir = PathBuf::from(
        flag_values(flags, "--out-dir")
            .next()
            .ok_or("Cargo gave no --out-dir")?,
    );
    let extra = flag_values(flags, "-C")
        .find_map(|option| option.strip_prefix("extra-filename=").map(str::to_string))
        .unwrap_or_default();
    let emitted: Vec<String> = flag_values(flags, "--emit")
        .flat_map(|kinds| kinds.split(',').map(str::to_string).collect::<Vec<_>>())
        .collect();
    // What `cargo build` asks for, rustc links. Only a check's crates are
    // metadata alone, which Cargo never starts a dependent before rustc ends:
    // rust-js publishes a crate's with its JS, after.
    if emitted.iter().any(|kind| kind == "link") {
        return Err("rust-js writes JS for `cargo check`; `cargo build` asks for what rustc links".into());
    }
    // Each build of the crate Cargo keeps, of a feature set say, has JS of its
    // own, as it has metadata, so a build Cargo has as done is still its JS.
    let dir = out_dir.join("rust-js").join(format!("{name}{extra}"));
    let mut ours = vec![
        source.clone(),
        "-o".into(),
        dir.join("lib.js").display().to_string(),
        "--manifest".into(),
        dir.join("lib.manifest.json").display().to_string(),
        "--cargo".into(),
        // Each module's declarations, `rust-js build`'s always (ADR 0412).
        "--declarations".into(),
    ];
    // Each is a library, but the app, the package `RUST_JS_APP` names, which
    // the tooling builds and no crate of its build uses (ADR 0360). What
    // `RUST_JS_APP` is, rust-js records for Cargo, which builds it again as
    // a library once another build uses it.
    if std::env::var(APP)
        .ok()
        .is_none_or(|app| std::env::var("CARGO_PKG_NAME").ok() != Some(app))
    {
        ours.push("--library".into());
    }
    for manifest in dependencies(flags)? {
        ours.push("--dependency".into());
        ours.push(manifest.display().to_string());
    }
    ours.push("--".into());
    // What rustc writes is what Cargo expects, where it expects it: the
    // metadata, published with the JS, and Cargo's own record of the sources.
    let mut emit = vec![format!(
        "metadata={}",
        out_dir.join(format!("lib{name}{extra}.rmeta")).display()
    )];
    if emitted.iter().any(|kind| kind == "dep-info") {
        emit.push(format!(
            "dep-info={}",
            out_dir.join(format!("{name}{extra}.d")).display()
        ));
    }
    ours.push(format!("--emit={}", emit.join(",")));
    let mut rest = flags.iter();
    while let Some(flag) = rest.next() {
        if *flag == source {
            continue;
        }
        if flag == "--emit" {
            rest.next();
            continue;
        }
        if flag.starts_with("--emit=") {
            continue;
        }
        ours.push(flag.clone());
    }
    Ok(ours)
}

/// The manifests of the libraries of the workspace this crate uses, directly
/// or through one of them. Beside the metadata of each is its manifest, and
/// the manifests it was compiled with (`record`), of the builds Cargo gave
/// it; a dependency without that, serde say, is a crate rustc built. One with
/// it and a manifest gone is refused: a crate that uses it would be compiled
/// as if it were rustc's.
fn dependencies(flags: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut found: Vec<PathBuf> = flag_values(flags, "--extern")
        .filter_map(|given| Some(PathBuf::from(given.split_once('=')?.1)))
        .filter_map(|metadata| std::fs::read_to_string(marker(&metadata)).ok())
        .flat_map(|recorded| recorded.lines().map(PathBuf::from).collect::<Vec<_>>())
        .collect();
    found.sort();
    found.dedup();
    if let Some(gone) = found.iter().find(|manifest| !manifest.is_file()) {
        return Err(format!(
            "{} is gone; rust-js wrote it for Cargo's build, which has it as done: `cargo clean` to build it again",
            gone.display()
        ));
    }
    Ok(found)
}

/// What Cargo is told of a crate rust-js compiled (ADR 0101), each file as
/// it's published with the JS, or not at all: beside its metadata, where its
/// manifest is, and those of the libraries it was compiled with, `used`, for
/// the crates that use it. Its record of the sources is `dep_info`'s.
pub fn record(manifest: &Path, used: &[PathBuf], metadata: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>, String> {
    let mut manifests = Vec::new();
    for manifest in std::iter::once(manifest).chain(used.iter().map(PathBuf::as_path)) {
        let path = std::path::absolute(manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
        manifests.push(format!("{}\n", path.display()));
    }
    Ok(vec![(marker(metadata), manifests.concat().into_bytes())])
}

/// Cargo's record of a crate's sources, as rustc wrote it to `written`,
/// with what else the build read added to its files: rust-js
/// itself, or Cargo would have the crate as done when rust-js had changed,
/// and `read`, the app's Prettier configs (ADR 0381), which lay its JS out.
pub fn dep_info(written: &Path, read: &[PathBuf]) -> Result<Vec<u8>, String> {
    let exe = std::env::current_exe().map_err(|e| format!("rust-js's own path: {e}"))?;
    let text = std::fs::read_to_string(written).map_err(|e| format!("cannot read the dep-info rustc wrote: {e}"))?;
    // Cargo reads the files of the first `<output>: <files>` line only.
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let first = lines
        .iter_mut()
        .find(|line| !line.starts_with('#') && line.contains(": "))
        .ok_or("rustc's dep-info lists no sources")?;
    for file in std::iter::once(&exe).chain(read) {
        first.push(' ');
        first.push_str(&file.display().to_string().replace(' ', "\\ "));
    }
    Ok((lines.join("\n") + "\n").into_bytes())
}

/// Where the manifest of the crate whose metadata is `metadata` is recorded.
fn marker(metadata: &Path) -> PathBuf {
    metadata.with_extension("rust-js")
}

/// Cargo's flags, with each `@file` it wrote for a long command read in.
fn expand(given: &[String]) -> Result<Vec<String>, String> {
    let mut flags = Vec::new();
    for flag in given {
        match flag.strip_prefix('@') {
            Some(file) => {
                let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
                flags.extend(text.lines().map(str::to_string));
            }
            None => flags.push(flag.clone()),
        }
    }
    Ok(flags)
}

/// `rustc -vV`, and rust-js's own identity after it. Cargo hashes what it
/// says into each crate's fingerprint and file names, so a build by another
/// rust-js, an app's upgrade or its rollback, is another build: an installed
/// compiler's file is as old as its package says, too old for Cargo to see
/// by its date that it changed.
fn verbose_version(rustc: &str) -> Invocation {
    let output = match std::process::Command::new(rustc).arg("-vV").output() {
        Ok(output) => output,
        Err(error) => {
            eprintln!("rust-js: cannot run {rustc}: {error}");
            return Invocation::Rustc(ExitCode::FAILURE);
        }
    };
    if !output.status.success() {
        let _ = std::io::stderr().write_all(&output.stderr);
        return Invocation::Rustc(ExitCode::FAILURE);
    }
    let Compiler {
        version,
        toolchain,
        abi,
    } = Compiler::current();
    let mut out = std::io::stdout().lock();
    match out
        .write_all(&output.stdout)
        .and_then(|()| writeln!(out, "rust-js: {version}, Rust {toolchain}, ABI {abi}"))
    {
        Ok(()) => Invocation::Rustc(ExitCode::SUCCESS),
        Err(error) => {
            eprintln!("rust-js: cannot write rustc's version: {error}");
            Invocation::Rustc(ExitCode::FAILURE)
        }
    }
}

/// rustc, run as Cargo asked, with what it prints passed through.
fn rustc_itself(rustc: &str, given: &[String]) -> Invocation {
    match std::process::Command::new(rustc).args(given).status() {
        Ok(status) => Invocation::Rustc(match status.code() {
            Some(0) => ExitCode::SUCCESS,
            Some(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
            None => ExitCode::FAILURE,
        }),
        Err(error) => {
            eprintln!("rust-js: cannot run {rustc}: {error}");
            Invocation::Rustc(ExitCode::FAILURE)
        }
    }
}
