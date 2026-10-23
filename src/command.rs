//! rust-js's command line: what it's asked to do, and the arguments rustc is
//! run with to do it. `main` runs it; nothing here runs rustc.

use std::path::PathBuf;

/// What a call of rust-js asks for.
pub(crate) enum Command {
    /// `--runtime-module`: `@rust-js/runtime`'s module (ADR 0103).
    RuntimeModule,
    /// `--version-json`: the compiler's identity, as a manifest records it.
    VersionJson,
    /// `--version`.
    Version,
    /// `--rustc <rustc flags>`: rustc itself, with rust-js's tool and cfg
    /// (ADR 0113).
    Rustc(Vec<String>),
    /// `--format-jsx [input.rs]`: the crate's `rsx!`, formatted, on stdout.
    FormatJsx(String),
    /// `--checks <input.rs> <file.js>..`: the crate's
    /// checks of its JS as a host has it, which Cargo's build runs none of
    /// (ADR 0117).
    Checks(Checks),
    /// A crate, compiled.
    Build(Build),
    /// `--tidy-passes`: each of tidy's passes, a line each, in the order
    /// they run, what `--tidy-until` may name (ADR 0659).
    TidyPasses,
}

/// A crate's checks: its root, whose `Cargo.toml` names them, and the JS
/// they read.
pub(crate) struct Checks {
    pub input: PathBuf,
    pub files: Vec<PathBuf>,
}

/// A crate's build: its input and output, and what's asked of it.
pub(crate) struct Build {
    pub input: PathBuf,
    pub output: PathBuf,
    /// `--test`: the crate's `#[test]` functions too (ADR 0026).
    pub test: bool,
    /// `--cargo`: Cargo's build (ADR 0101).
    pub cargo: bool,
    /// `--library`: the crate is published as a library.
    pub library: bool,
    /// `--dependency <manifest.json>`, each a library the crate uses.
    pub dependencies: Vec<PathBuf>,
    /// `--declarations`: a `.d.ts` beside each module's JS, which Cargo's
    /// build, `rust-js build`'s, always asks for (ADR 0412).
    pub declarations: bool,
    /// `--manifest <file.json>` (ADR 0042).
    pub manifest: Option<PathBuf>,
    /// `--std-coverage <file>` (ADR 0314).
    pub std_coverage: Option<PathBuf>,
    /// `--std-bodies`: the crate is `@rust-js/std`, given std's bodies to
    /// lower, each from the MIR the toolchain ships (ADR 0561).
    pub std_bodies: bool,
    /// `--tidy-until <stage/pass>`: each body's JS as tidy leaves it after
    /// that pass, to see what each pass does (ADR 0659).
    pub tidy_until: Option<String>,
    /// What's after `--`, rustc's as-is, e.g. `-- --sysroot /sysroot`.
    pub rustc: Vec<String>,
}

const USAGE: &str = "usage: rust-js [--test] <input.rs> [-o <output.js>] [--manifest <file.json>] [--library] [--std-bodies] [--dependency <manifest.json>] [--declarations] [-- <rustc flags>]";

/// The command `args` asks for, or what's wrong with them.
pub(crate) fn parse(args: Vec<String>) -> Result<Command, String> {
    match args.first().map(String::as_str) {
        Some("--runtime-module") if args.len() == 1 => return Ok(Command::RuntimeModule),
        Some("--version-json") if args.len() == 1 => return Ok(Command::VersionJson),
        Some("--version") if args.len() == 1 => return Ok(Command::Version),
        Some("--tidy-passes") if args.len() == 1 => return Ok(Command::TidyPasses),
        Some("--rustc") => return Ok(Command::Rustc(args[1..].to_vec())),
        Some("--checks") => {
            let [input, files @ ..] = &args[1..] else {
                return Err("usage: rust-js --checks <input.rs> <file.js>..".into());
            };
            return Ok(Command::Checks(Checks {
                input: PathBuf::from(input),
                files: files.iter().map(PathBuf::from).collect(),
            }));
        }
        Some("--format-jsx") => {
            return match args.as_slice() {
                [_] => Ok(Command::FormatJsx("-".into())),
                [_, input] => Ok(Command::FormatJsx(input.clone())),
                _ => Err("usage: rust-js --format-jsx [input.rs] (defaults to stdin; writes stdout)".into()),
            };
        }
        _ => {}
    }
    let (ours, rustc) = match args.iter().position(|a| a == "--") {
        Some(i) => (&args[..i], &args[i + 1..]),
        None => (&args[..], &[][..]),
    };
    let mut ours = ours.to_vec();
    let cargo = flag(&mut ours, "--cargo");
    let library = flag(&mut ours, "--library");
    let declarations = flag(&mut ours, "--declarations");
    let std_bodies = flag(&mut ours, "--std-bodies");
    let mut dependencies = Vec::new();
    while let Some(path) = valued(&mut ours, "--dependency", "a manifest path")? {
        dependencies.push(PathBuf::from(path));
    }
    let manifest = valued(&mut ours, "--manifest", "a path")?.map(PathBuf::from);
    let std_coverage = valued(&mut ours, "--std-coverage", "a path")?.map(PathBuf::from);
    let tidy_until = valued(&mut ours, "--tidy-until", "a pass, `stage/pass`")?;
    if let Some(pass) = &tidy_until
        && !crate::lower::tidy_pass_names().contains(pass)
    {
        return Err(format!(
            "--tidy-until: no pass `{pass}`; `rust-js --tidy-passes` lists them"
        ));
    }
    let test = ours.first().is_some_and(|a| a == "--test");
    let (input, output) = match &ours[usize::from(test)..] {
        [input] => (PathBuf::from(input), PathBuf::from(input).with_extension("js")),
        [input, flag, output] if flag == "-o" => (PathBuf::from(input), PathBuf::from(output)),
        _ => return Err(USAGE.into()),
    };
    if library && manifest.is_none() {
        return Err("--library requires --manifest".into());
    }
    if std_bodies && !library {
        return Err("--std-bodies requires --library".into());
    }
    Ok(Command::Build(Build {
        input,
        output,
        test,
        cargo,
        library,
        dependencies,
        declarations,
        manifest,
        std_coverage,
        std_bodies,
        tidy_until,
        rustc: rustc.to_vec(),
    }))
}

/// Whether `name` is among `args`, taken out of them.
fn flag(args: &mut Vec<String>, name: &str) -> bool {
    let Some(i) = args.iter().position(|arg| arg == name) else {
        return false;
    };
    args.remove(i);
    true
}

/// The value after `name` in `args`, both taken out of them.
fn valued(args: &mut Vec<String>, name: &str, what: &str) -> Result<Option<String>, String> {
    let Some(i) = args.iter().position(|arg| arg == name) else {
        return Ok(None);
    };
    if i + 1 >= args.len() {
        return Err(format!("{name} requires {what}"));
    }
    let value = args.remove(i + 1);
    args.remove(i);
    Ok(Some(value))
}

/// The target rustc checks programs for (ADR 0090): WebAssembly's, whose
/// `usize` is 32 bits, as a JS one is here (ADR 0025). Nothing is made for
/// it. A crate Cargo checks for it is one rust-js compiles (`cargo.rs`).
pub(crate) const TARGET: &str = "wasm32-unknown-unknown";

/// What rustc writes beside the JS, staged in a directory of its own and
/// published with it, as artifacts of the same plan (ADR 0100): each is
/// `(staged, published)`.
pub(crate) struct Staged {
    pub dir: PathBuf,
    /// `--emit=metadata=<path>`: a library's metadata.
    pub metadata: Option<(PathBuf, PathBuf)>,
    /// `--emit=dep-info=<path>`: Cargo's record of the sources (ADR 0101).
    pub dep_info: Option<(PathBuf, PathBuf)>,
    /// The panic runtime that unwinds a session that builds more than a
    /// library is given, made here (`panic_runtime`, ADR 0090).
    pub panic_runtime: Option<PathBuf>,
}

impl Staged {
    pub fn any(&self) -> bool {
        self.metadata.is_some() || self.dep_info.is_some() || self.panic_runtime.is_some()
    }
}

/// Each value of `flag` in `flags`, a rustc command's or Cargo's, given as
/// `--flag value` or `--flag=value`: the one reading of either spelling.
pub fn flag_values<'a>(flags: &'a [String], flag: &'a str) -> impl Iterator<Item = String> + 'a {
    flags.iter().enumerate().filter_map(move |(i, given)| {
        if given == flag {
            flags.get(i + 1).cloned()
        } else {
            given.strip_prefix(flag)?.strip_prefix('=').map(str::to_string)
        }
    })
}

/// Whether a session of `args` builds more than a library, which rustc
/// gives a panic runtime: a test harness, or a crate type that's linked.
fn builds_more_than_a_library(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--test")
        || flag_values(args, "--crate-type")
            .flat_map(|types| types.split(',').map(str::to_string).collect::<Vec<_>>())
            .any(|kind| kind != "lib" && kind != "rlib")
}

/// What tells a program it's compiled by rust-js, `cfg(rust_js)`, declared
/// so rustc checks it: the crates' attributes of its tool are
/// `cfg_attr(rust_js, ..)`, which a plain rustc, a user's own `cargo check`,
/// leaves out (ADR 0113). The one list, of a file's compilation and of a
/// Cargo crate's alike.
pub const RUST_JS_CFG: [&str; 2] = ["--cfg=rust_js", "--check-cfg=cfg(rust_js)"];

impl Build {
    /// The arguments rustc is run with, and what of its outputs is staged.
    pub fn rustc_args(&self) -> Result<(Vec<String>, Staged), String> {
        let mut args = vec![
            "rust-js".to_string(), // argv[0], ignored by rustc
            self.input.display().to_string(),
            "--crate-type=lib".to_string(),
            "--edition=2024".to_string(),
            // What rustc works out for the program, `usize::MAX`, `size_of` and
            // `cfg(target_pointer_width)`, is for a 32-bit `usize`, as rust-js's is
            // (ADR 0090).
            format!("--target={TARGET}"),
            // `#[cfg(browser)]` marks tests that need a real browser (ADR 0027):
            // `-- --cfg browser` turns it on. Declaring any cfg makes rustc check them
            // all, so `test` is declared too, as Cargo does.
            "--check-cfg=cfg(browser, test)".to_string(),
        ];
        args.extend(RUST_JS_CFG.map(String::from));
        if self.test {
            args.push("--test".to_string());
        }
        // An edition or a target after `--` is the one: rustc takes only one.
        if flag_values(&self.rustc, "--edition").next().is_some() {
            args.retain(|arg| arg != "--edition=2024");
        }
        if flag_values(&self.rustc, "--target").next().is_some() {
            args.retain(|arg| !arg.starts_with("--target="));
        }
        args.extend(self.rustc.iter().cloned());
        let staged = self.stage(&mut args)?;
        Ok((args, staged))
    }

    /// `--emit=metadata=<path>` (ADR 0100), and Cargo's `dep-info`, written
    /// by rustc to a directory of its own beside the JS, to be published
    /// with it, checked for collisions with the rest, or not at all; and the
    /// panic runtime a test harness is given (ADR 0090).
    fn stage(&self, args: &mut Vec<String>) -> Result<Staged, String> {
        // Named after the JS it's beside, which no other build writes at
        // once: WASI, the playground's, has no process id to name it by.
        let beside = crate::paths::parent_dir(&self.output);
        let name = self.output.file_name().unwrap_or_default().to_string_lossy();
        let dir = || beside.join(format!(".rust-js-{name}"));
        let mut staged = Staged {
            dir: PathBuf::new(),
            metadata: None,
            dep_info: None,
            panic_runtime: None,
        };
        if builds_more_than_a_library(args) {
            let runtime = dir().join("libpanic_unwind.rmeta");
            args.push(format!("--extern=panic_unwind={}", runtime.display()));
            staged.panic_runtime = Some(runtime);
        }
        // `--emit mir`, rustc's other spelling, as `--emit=mir`.
        while let Some(i) = args.iter().position(|arg| arg == "--emit") {
            let kinds = if i + 1 < args.len() {
                args.remove(i + 1)
            } else {
                String::new()
            };
            args[i] = format!("--emit={kinds}");
        }
        for arg in args.iter_mut() {
            let Some(kinds) = arg.strip_prefix("--emit=") else {
                continue;
            };
            let mut rewritten = Vec::new();
            for kind in kinds.split(',') {
                match kind.strip_prefix("metadata") {
                    Some(rest) => {
                        let Some(path) = rest.strip_prefix('=').map(PathBuf::from) else {
                            return Err("say where a library's metadata goes: --emit=metadata=<path>".into());
                        };
                        let to = dir().join(path.file_name().unwrap_or_default());
                        rewritten.push(format!("metadata={}", to.display()));
                        staged.metadata = Some((to, path));
                    }
                    // rustc's other outputs are written after rust-js has published,
                    // past its checks, and are no part of what rust-js makes.
                    // Cargo's record of the sources (ADR 0101), staged as the
                    // metadata is, and published with it.
                    None if self.cargo
                        && let Some(path) = kind.strip_prefix("dep-info=").map(PathBuf::from) =>
                    {
                        let to = dir().join(path.file_name().unwrap_or_default());
                        rewritten.push(format!("dep-info={}", to.display()));
                        staged.dep_info = Some((to, path));
                    }
                    None => {
                        return Err(format!(
                            "rustc's `--emit={kind}` isn't something rust-js writes; only a library's --emit=metadata=<path>"
                        ));
                    }
                }
            }
            *arg = format!("--emit={}", rewritten.join(","));
        }
        if staged.any() {
            staged.dir = dir();
        }
        Ok(staged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(args: &[&str]) -> Result<Build, String> {
        match parse(args.iter().map(|arg| arg.to_string()).collect())? {
            Command::Build(build) => Ok(build),
            _ => Err("not a build".into()),
        }
    }

    #[test]
    fn checks_are_a_crate_and_its_files() {
        let checks = |args: &[&str]| match parse(args.iter().map(|arg| arg.to_string()).collect()) {
            Ok(Command::Checks(checks)) => Ok((checks.input, checks.files)),
            Ok(_) => Err("not checks".to_string()),
            Err(error) => Err(error),
        };
        assert_eq!(
            checks(&["--checks", "src/lib.rs", "src/lib.js", "src/api.js"]),
            Ok((
                PathBuf::from("src/lib.rs"),
                vec![PathBuf::from("src/lib.js"), PathBuf::from("src/api.js")]
            ))
        );
        assert!(checks(&["--checks"]).is_err());
    }

    #[test]
    fn a_build_is_its_flags() {
        let b = build(&[
            "--test",
            "lib.rs",
            "-o",
            "out/lib.js",
            "--library",
            "--manifest",
            "m.json",
            "--dependency",
            "a.json",
            "--dependency",
            "b.json",
            "--",
            "--cfg",
            "browser",
        ])
        .unwrap();
        assert!(b.test && b.library && !b.cargo);
        assert_eq!((b.input, b.output), ("lib.rs".into(), "out/lib.js".into()));
        assert_eq!(b.manifest, Some("m.json".into()));
        assert_eq!(b.dependencies, [PathBuf::from("a.json"), "b.json".into()]);
        assert_eq!(b.rustc, ["--cfg", "browser"]);
        assert_eq!(build(&["lib.rs"]).unwrap().output, PathBuf::from("lib.js"));
    }

    #[test]
    fn a_build_missing_a_value_is_refused() {
        assert_eq!(
            build(&["lib.rs", "--library"]).err().unwrap(),
            "--library requires --manifest"
        );
        assert_eq!(
            build(&["lib.rs", "--dependency"]).err().unwrap(),
            "--dependency requires a manifest path"
        );
        assert_eq!(build(&["lib.rs", "lib.js"]).err().unwrap(), USAGE);
    }

    #[test]
    fn rustcs_edition_and_target_are_the_ones_after_the_separator() {
        let (args, staged) = build(&["lib.rs", "--", "--edition", "2021"])
            .unwrap()
            .rustc_args()
            .unwrap();
        assert!(!args.iter().any(|arg| arg == "--edition=2024"));
        assert!(args.iter().any(|arg| arg == &format!("--target={TARGET}")));
        assert!(!staged.any());
        let (args, _) = build(&["lib.rs", "--", "--target=wasm32-wasip1"])
            .unwrap()
            .rustc_args()
            .unwrap();
        assert!(!args.iter().any(|arg| arg == &format!("--target={TARGET}")));
    }

    #[test]
    fn rustcs_outputs_are_staged_or_refused() {
        let b = build(&["lib.rs", "-o", "out/lib.js", "--", "--emit", "metadata=out/lib.rmeta"]).unwrap();
        let (args, staged) = b.rustc_args().unwrap();
        let (to, published) = staged.metadata.unwrap();
        assert_eq!(published, PathBuf::from("out/lib.rmeta"));
        assert!(to.starts_with(&staged.dir) && staged.dir.starts_with("out"));
        assert!(args.contains(&format!("--emit=metadata={}", to.display())));
        let mir = build(&["lib.rs", "--", "--emit=mir"])
            .unwrap()
            .rustc_args()
            .err()
            .unwrap();
        assert!(mir.contains("`--emit=mir` isn't something rust-js writes"));
        let dep_info = build(&["lib.rs", "--", "--emit=dep-info=lib.d"]).unwrap();
        assert!(dep_info.rustc_args().is_err());
    }
}
