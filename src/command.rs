//! rust-js's command line: what it's asked to do, and the arguments rustc is
//! run with to do it. `main` runs it; nothing here runs rustc.

use std::path::PathBuf;

/// What a call of rust-js asks for.
pub(crate) enum Command {
    /// `--runtime-module`: `@rust-js/std`'s helpers' module, `runtime.js` (ADR 0103).
    RuntimeModule,
    /// `--std-declarations`: `@rust-js/std`'s types, `index.d.ts` (ADR 1149).
    StdDeclarations,
    /// `--version-json`: the compiler's identity, as a manifest records it.
    VersionJson,
    /// `--version`.
    Version,
    /// `--rustc <rustc flags>`: rustc itself, with rust-js's tool and cfg
    /// (ADR 0113).
    Rustc(Vec<String>),
    /// `fmt [--check] [--stdin] [paths..]`: the crate's Rust formatted,
    /// rustfmt's and its `jsx!`'s (#1901): natively, where rustfmt is.
    #[cfg(not(target_os = "wasi"))]
    Fmt(crate::source_format::Fmt),
    /// `--checks <input.rs> <file.js>..`: the crate's
    /// checks of its JS as a host has it, which Cargo's build runs none of
    /// (ADR 0117).
    Checks(Checks),
    /// A crate, compiled: boxed, as it's many times any other's size.
    Build(Box<Build>),
    /// `--layout-identity`: which Prettier lays out the JS of the crates
    /// the tooling lists, `RUST_JS_CRATES` and `RUST_JS_DEPENDENCIES`, a
    /// hash, or an empty line where none has one: what the tooling gives
    /// Cargo as `RUST_JS_LAYOUT`, once a build (ADR 1003, amended; #892).
    LayoutIdentity,
    /// `--worker`: jobs on stdin, each a case's builds compiled in one rustc
    /// session, answered on stdout: the tests' harness's way, dev-only (ADR
    /// 1182, #2301).
    #[cfg(not(target_os = "wasi"))]
    Worker,
    /// `--tidy-passes`: each of tidy's passes, a line each, in the order
    /// they run, what `--tidy-until` may name (ADR 0659).
    TidyPasses,
    /// `--prepare-passes`: each of preparation's rewrites, a line each, what
    /// `--prepare-without` may name (ADR 1087).
    PreparePasses,
    /// `--knowns`: each fact an expression may carry and its role, a line
    /// each, what `--without-known` may name (ADR 1127).
    Knowns,
    /// `--css-module-binding <./x.module.css>`: the binding of a CSS
    /// Module's classes, the form its check reads, on stdout, which `rust-js
    /// bindings` prints (ADR 1133).
    CssModuleBinding(String),
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
    /// `--tested`: the crate is a program, compiled as `rust-js build`
    /// compiles it, whose contract its integration tests read, each held to
    /// what it decided (ADR 1076, amended, #597).
    pub tested: bool,
    /// `--dependency <manifest.json>`, each a library the crate uses.
    pub dependencies: Vec<PathBuf>,
    /// `--declarations`: a `.d.ts` beside each module's JS, which Cargo's
    /// build, `rust-js build`'s, always asks for (ADR 0412).
    pub declarations: bool,
    /// `--manifest <file.json>` (ADR 0042).
    pub manifest: Option<PathBuf>,
    /// `--std-coverage <file>` (ADR 0314).
    pub std_coverage: Option<PathBuf>,
    /// `--std-catalogue <file>`: where to write the catalogue of std's
    /// bodies `@rust-js/std` compiles, walked, which `bun run std` keeps
    /// and every compile reads embedded (#2305): dev-only, std's build's,
    /// as `--std-coverage` is.
    pub std_catalogue: Option<PathBuf>,
    /// `--std-bodies`: the crate is `@rust-js/std`, given std's bodies to
    /// lower, each from the MIR of a `core` built to unwind (ADR 0561, #424).
    pub std_bodies: bool,
    /// `--tidy-until <stage/pass>`: each body's JS as tidy leaves it after
    /// that pass, to see what each pass does (ADR 0659): no pass after it
    /// runs, `finish`'s naming and taking apart its parameters among them,
    /// so what's printed is what that pass made.
    pub tidy_until: Option<String>,
    /// `--tidy-fired <file>`: where to write which of tidy's passes changed
    /// a body, and how many, a line each (#1461).
    pub tidy_fired: Option<PathBuf>,
    /// `--label-census <file>`: where to write each compiler-made label a
    /// body keeps after tidy, by the MIR shape that left it (#1461).
    pub label_census: Option<PathBuf>,
    /// `--prepare-without <pass>`, any number: each module prepared with
    /// every rewrite but these, to see that each changes nothing a program
    /// does (ADR 1087).
    pub prepare_without: Vec<crate::prepare::Pass>,
    /// `--prepare-fired <file>`: where to write which of preparation's
    /// rewrites changed something, a line each (ADR 1087).
    pub prepare_fired: Option<PathBuf>,
    /// `--without-known <fact>`, any number: the crate compiled with these
    /// facts forgotten, no reader finding one, to see that the JS is right
    /// without each that only permits (ADR 1127).
    pub without_known: Vec<crate::js::Known>,
    /// `--knowns-found <file>`: where to write which facts a reader found,
    /// a line each (ADR 1127).
    pub knowns_found: Option<PathBuf>,
    /// `--without-recognizer <std path>`, any number: the crate compiled
    /// with no recogniser taking these std functions, to see that what's
    /// left of each is refused or right (ADR 1128).
    pub without_recognizers: Vec<String>,
    /// `--without-mutated <type>`, any number: the crate compiled with these
    /// types, as `{ty}` prints them, forgotten from the set of what's
    /// changed in place, to see that a write in place refuses where its
    /// copies are left out (#1665).
    pub without_mutated: Vec<String>,
    /// `--without-intervals`: the crate compiled with nothing MIR's
    /// intervals prove, every bounds check, wrap and cast they'd leave out
    /// kept, to see that each case runs the same: a run that differs is a
    /// fact the intervals kept past a write (#2112 B). A test hook, not a
    /// way to build: `test/tidy-stages.test.ts`' variant.
    pub without_intervals: bool,
    /// `--without-moves`: rust-js's drop elaboration blind to moves, each
    /// drop dropping all of what it drops, a moved part too, to see that
    /// the comparison with rustc's refuses it (#2112's change D). A test
    /// hook, not a way to build: `test/drops-shadow.test.ts`' refusal.
    pub without_moves: bool,
    /// `--intervals-by-effects`: what MIR's intervals keep through a
    /// statement asked of the effects query too, each local it says may be
    /// written forgotten (#2112 B step 3a). A test hook till the query is
    /// the transfer's own: `test/intervals-shadow.test.ts`' rows.
    pub intervals_by_effects: bool,
    /// `--intervals-shadow <file>`: where to write each check THIR's ranges
    /// leave out that MIR's intervals don't prove, a line each (#1672).
    pub intervals_shadow: Option<PathBuf>,
    /// `--drops-shadow <file>`: where to write, of each `Drop` of the
    /// crate's bodies, whether rust-js's elaboration drops what rustc's
    /// does, a line each (#2112's change D).
    pub drops_shadow: Option<PathBuf>,
    /// `--recognizers-found <file>`: where to write which std functions a
    /// recogniser took, a line each (ADR 1128).
    pub recognizers_found: Option<PathBuf>,
    /// `--props-replayed <file>`: where to write the components whose props
    /// a replay built by their companion, rustc's names having proved the
    /// syntax pass's prediction wrong, a line each (ADR 0213, amended).
    pub props_replayed: Option<PathBuf>,
    /// `--std-iterators`: every iterator adapter std compiles is its struct,
    /// none rewritten to the JS chain the recognizers make in place. A test
    /// hook, not a way to build: the differential job compares it with the
    /// rewritten build and native Rust (ADR 0908).
    pub std_iterators: bool,
    /// `--js-made-all`: every struct the crate doesn't build itself, a
    /// library's too, is one JS may make (ADR 1022), whose fields a getter
    /// may answer. A test hook, not a way to build: the object model's net
    /// reads a library's values through counting proxies (ADR 1105).
    pub js_made_all: bool,
    /// `--no-thir-hints`: tidy is told nothing of where the program writes
    /// its constructs, so its JS reads worse and must run the same. A test
    /// hook, not a way to build: `test/tidy-stages.test.ts`' variant of
    /// each case (ADR 1144, amended).
    pub no_thir_hints: bool,
    /// `--std-paths-first`: std's paths are resolved before any body is
    /// lowered, not where a body first asks, so rustc reads std's spans
    /// first; the JS is the same. A test hook, not a way to build:
    /// `test/std-package.test.ts`' compile of std's bodies both ways (ADR
    /// 0879, amended).
    pub std_paths_first: bool,
    /// What's after `--`, rustc's as-is, e.g. `-- --sysroot /sysroot`.
    pub rustc: Vec<String>,
}

const USAGE: &str = "usage: rust-js [--test] <input.rs> [-o <output.js>] [--manifest <file.json>] [--library] [--std-bodies] [--dependency <manifest.json>] [--declarations] [-- <rustc flags>]";

/// The command `args` asks for, or what's wrong with them.
pub(crate) fn parse(args: Vec<String>) -> Result<Command, String> {
    match args.first().map(String::as_str) {
        Some("--runtime-module") if args.len() == 1 => return Ok(Command::RuntimeModule),
        Some("--std-declarations") if args.len() == 1 => return Ok(Command::StdDeclarations),
        Some("--version-json") if args.len() == 1 => return Ok(Command::VersionJson),
        Some("--version") if args.len() == 1 => return Ok(Command::Version),
        Some("--layout-identity") if args.len() == 1 => return Ok(Command::LayoutIdentity),
        Some("--tidy-passes") if args.len() == 1 => return Ok(Command::TidyPasses),
        #[cfg(not(target_os = "wasi"))]
        Some("--worker") if args.len() == 1 => return Ok(Command::Worker),
        Some("--prepare-passes") if args.len() == 1 => return Ok(Command::PreparePasses),
        Some("--knowns") if args.len() == 1 => return Ok(Command::Knowns),
        Some("--rustc") => return Ok(Command::Rustc(args[1..].to_vec())),
        Some("--css-module-binding") => {
            return match args.as_slice() {
                [_, specifier] => Ok(Command::CssModuleBinding(specifier.clone())),
                _ => Err("usage: rust-js --css-module-binding <./x.module.css>".into()),
            };
        }
        Some("--checks") => {
            let [input, files @ ..] = &args[1..] else {
                return Err("usage: rust-js --checks <input.rs> <file.js>..".into());
            };
            return Ok(Command::Checks(Checks {
                input: PathBuf::from(input),
                files: files.iter().map(PathBuf::from).collect(),
            }));
        }
        // The one way to format, `rust-js fmt` (#1901).
        Some("--format-jsx") => {
            return Err("rust-js: `--format-jsx` is `rust-js fmt` now, rustfmt's and the `jsx!`'s: `rust-js fmt --stdin` formats stdin to stdout".into());
        }
        #[cfg(not(target_os = "wasi"))]
        Some("fmt") => {
            use crate::source_format::{Fmt, Mode, Sources};
            let usage = "usage: rust-js fmt [--check] [--stdin] [paths..]";
            let mut mode = Mode::Write;
            let mut paths = Vec::new();
            let mut stdin = None;
            for arg in &args[1..] {
                match arg.as_str() {
                    "--check" => mode = Mode::Check,
                    "--stdin" => stdin = Some(Sources::Stdin),
                    flag if flag.starts_with('-') => {
                        return Err(format!("rust-js fmt: unknown flag `{flag}`\n{usage}"));
                    }
                    path => paths.push(PathBuf::from(path)),
                }
            }
            let sources = match (stdin, paths.is_empty()) {
                (Some(_), false) => {
                    return Err(format!(
                        "rust-js fmt: `--stdin` formats stdin alone; give it no paths\n{usage}"
                    ));
                }
                (Some(stdin), true) => stdin,
                (None, true) => Sources::Workspace,
                (None, false) => Sources::Paths(paths),
            };
            return Ok(Command::Fmt(Fmt { mode, sources }));
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
    let tested = flag(&mut ours, "--tested");
    let declarations = flag(&mut ours, "--declarations");
    let std_bodies = flag(&mut ours, "--std-bodies");
    let std_iterators = flag(&mut ours, "--std-iterators");
    let js_made_all = flag(&mut ours, "--js-made-all");
    let no_thir_hints = flag(&mut ours, "--no-thir-hints");
    let without_intervals = flag(&mut ours, "--without-intervals");
    let without_moves = flag(&mut ours, "--without-moves");
    let intervals_by_effects = flag(&mut ours, "--intervals-by-effects");
    let std_paths_first = flag(&mut ours, "--std-paths-first");
    let mut dependencies = Vec::new();
    while let Some(path) = valued(&mut ours, "--dependency", "a manifest path")? {
        dependencies.push(PathBuf::from(path));
    }
    let manifest = valued(&mut ours, "--manifest", "a path")?.map(PathBuf::from);
    let std_coverage = valued(&mut ours, "--std-coverage", "a path")?.map(PathBuf::from);
    let std_catalogue = valued(&mut ours, "--std-catalogue", "a path")?.map(PathBuf::from);
    let tidy_until = valued(&mut ours, "--tidy-until", "a pass, `stage/pass`")?;
    if let Some(pass) = &tidy_until
        && !crate::lower::tidy_pass_names().contains(pass)
    {
        return Err(format!(
            "--tidy-until: no pass `{pass}`; `rust-js --tidy-passes` lists them"
        ));
    }
    let mut prepare_without = Vec::new();
    while let Some(name) = valued(&mut ours, "--prepare-without", "a rewrite of preparation's")? {
        let Some(pass) = crate::prepare::Pass::named(&name) else {
            return Err(format!(
                "--prepare-without: no rewrite `{name}`; `rust-js --prepare-passes` lists them"
            ));
        };
        prepare_without.push(pass);
    }
    let prepare_fired = valued(&mut ours, "--prepare-fired", "a path")?.map(PathBuf::from);
    let tidy_fired = valued(&mut ours, "--tidy-fired", "a path")?.map(PathBuf::from);
    let label_census = valued(&mut ours, "--label-census", "a path")?.map(PathBuf::from);
    let mut without_known = Vec::new();
    while let Some(name) = valued(&mut ours, "--without-known", "a fact an expression may carry")? {
        let Some(known) = crate::js::Known::named(&name) else {
            return Err(format!(
                "--without-known: no fact `{name}`; `rust-js --knowns` lists them"
            ));
        };
        without_known.push(known);
    }
    let knowns_found = valued(&mut ours, "--knowns-found", "a path")?.map(PathBuf::from);
    let mut without_recognizers = Vec::new();
    // A std function's path, as `--recognizers-found` writes it: `std::..`,
    // or of a trait's impl, `<core::io::Cursor<T> as core::io::Seek>::seek`.
    while let Some(path) = valued(&mut ours, "--without-recognizer", "a std function's path")? {
        without_recognizers.push(path);
    }
    let recognizers_found = valued(&mut ours, "--recognizers-found", "a path")?.map(PathBuf::from);
    let props_replayed = valued(&mut ours, "--props-replayed", "a path")?.map(PathBuf::from);
    let intervals_shadow = valued(&mut ours, "--intervals-shadow", "a path")?.map(PathBuf::from);
    let drops_shadow = valued(&mut ours, "--drops-shadow", "a path")?.map(PathBuf::from);
    let mut without_mutated = Vec::new();
    while let Some(ty) = valued(&mut ours, "--without-mutated", "a type, as rustc prints it")? {
        without_mutated.push(ty);
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
    if tested && (library || manifest.is_none()) {
        return Err("--tested requires --manifest, and is a program's, not a --library's".into());
    }
    if std_bodies && !library {
        return Err("--std-bodies requires --library".into());
    }
    Ok(Command::Build(Box::new(Build {
        input,
        output,
        test,
        cargo,
        library,
        tested,
        dependencies,
        declarations,
        manifest,
        std_coverage,
        std_catalogue,
        std_bodies,
        tidy_until,
        tidy_fired,
        label_census,
        prepare_without,
        prepare_fired,
        without_known,
        knowns_found,
        without_recognizers,
        without_mutated,
        without_intervals,
        without_moves,
        intervals_by_effects,
        intervals_shadow,
        drops_shadow,
        recognizers_found,
        props_replayed,
        std_iterators,
        js_made_all,
        no_thir_hints,
        std_paths_first,
        rustc: rustc.to_vec(),
    })))
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
}

impl Staged {
    pub fn any(&self) -> bool {
        self.metadata.is_some() || self.dep_info.is_some()
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
    /// with it, checked for collisions with the rest, or not at all.
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
        };
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
                    // The build's record of what it read, Cargo's (ADR 0101)
                    // or a file's, of what it wrote too (ADR 1099), staged as
                    // the metadata is, and published with it.
                    None if let Some(path) = kind.strip_prefix("dep-info=").map(PathBuf::from) => {
                        let to = dir().join(path.file_name().unwrap_or_default());
                        rewritten.push(format!("dep-info={}", to.display()));
                        staged.dep_info = Some((to, path));
                    }
                    None => {
                        return Err(format!(
                            "rustc's `--emit={kind}` isn't something rust-js writes; only a library's --emit=metadata=<path>, and the build's record, --emit=dep-info=<path>"
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
            Command::Build(build) => Ok(*build),
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
        // The build's record, of a file compiled alone too (ADR 1099),
        // staged as the metadata is; but where it goes is said.
        let (args, staged) = build(&["lib.rs", "-o", "out/lib.js", "--", "--emit=dep-info=out/lib.d"])
            .unwrap()
            .rustc_args()
            .unwrap();
        let (to, published) = staged.dep_info.unwrap();
        assert_eq!(published, PathBuf::from("out/lib.d"));
        assert!(to.starts_with(&staged.dir));
        assert!(args.contains(&format!("--emit=dep-info={}", to.display())));
        let bare = build(&["lib.rs", "--", "--emit=dep-info"])
            .unwrap()
            .rustc_args()
            .err()
            .unwrap();
        assert!(bare.contains("`--emit=dep-info` isn't something rust-js writes"));
    }
}
