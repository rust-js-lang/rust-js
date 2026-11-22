//! `rust-js fmt`, the one way to format a crate's Rust (#1901): the
//! toolchain's rustfmt, as `cargo fmt` runs it, then the `jsx!` pass, which
//! lays out what rustfmt leaves as it's written (`rust_js_jsx::formatting`).
//!
//! ```text
//!   rust-js fmt [--check] [--stdin] [paths..]
//! ```
//!
//! - No paths: each package of the Cargo workspace it's run in, each
//!   target's module tree, as `cargo fmt` finds it.
//! - Paths: those files alone, not their modules, as rustfmt formats a file
//!   with `skip_children`.
//! - `--stdin`: what stdin gives, on stdout, as an editor asks:
//!   rust-analyzer's `rust-analyzer.rustfmt.overrideCommand: ["rust-js",
//!   "fmt", "--stdin"]`.
//! - `--check`: nothing written; each file that would change is a diff on
//!   stdout, and the run exits 1.
//!
//! rustfmt runs where each file is, so it reads the project's toolchain and
//! its `rustfmt.toml`, given the edition of the package the file is of.

mod wrap;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Stdio};

/// What `rust-js fmt` is asked: what to format, and what to do of it.
pub(crate) struct Fmt {
    pub mode: Mode,
    pub sources: Sources,
}

/// What `rust-js fmt` does of what it formats.
pub(crate) enum Mode {
    /// Writes each file that changes.
    Write,
    /// `--check`: writes nothing, prints each change's diff, exits 1.
    Check,
}

/// What `rust-js fmt` formats.
pub(crate) enum Sources {
    /// `--stdin`, to stdout.
    Stdin,
    /// No paths: the Cargo workspace's module trees.
    Workspace,
    /// The files named, those alone.
    Paths(Vec<PathBuf>),
}

/// A file to format, of the edition its package is.
struct Source {
    path: PathBuf,
    edition: String,
}

/// A file being formatted: where rustfmt runs, its edition and name,
/// rustfmt's width and indent for it, and the notes of what it couldn't
/// lay out, which `--check` says.
pub(super) struct Job {
    dir: PathBuf,
    edition: String,
    name: PathBuf,
    max_width: usize,
    tab: usize,
    notes: std::rc::Rc<std::cell::RefCell<Vec<String>>>,
}

impl Job {
    fn new(dir: &Path, edition: &str, name: &Path) -> Result<Job, String> {
        let (max_width, tab) = rustfmt_widths(dir)?;
        Ok(Job {
            dir: dir.to_path_buf(),
            edition: edition.to_string(),
            name: name.to_path_buf(),
            max_width,
            tab,
            notes: Default::default(),
        })
    }

    fn note(&self, note: String) {
        self.notes.borrow_mut().push(note);
    }

    /// This job at `max_width`, a piece of Rust's in a `jsx!`, its notes
    /// this one's.
    fn at_width(&self, max_width: usize) -> Job {
        Job {
            dir: self.dir.clone(),
            edition: self.edition.clone(),
            name: self.name.clone(),
            max_width,
            tab: self.tab,
            notes: std::rc::Rc::clone(&self.notes),
        }
    }
}

/// rustfmt's `max_width` and `tab_spaces` of a file in `dir`, as the
/// `rustfmt.toml` nearest above it says, else its defaults, 100 and 4.
fn rustfmt_widths(dir: &Path) -> Result<(usize, usize), String> {
    let dir = std::fs::canonicalize(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for at in dir.ancestors() {
        for name in ["rustfmt.toml", ".rustfmt.toml"] {
            let path = at.join(name);
            if let Ok(text) = std::fs::read_to_string(&path) {
                let value: toml::Table = text.parse().map_err(|e| format!("{}: {e}", path.display()))?;
                let get = |key: &str, default: usize| {
                    value
                        .get(key)
                        .and_then(|v| v.as_integer())
                        .map_or(default, |v| v as usize)
                };
                return Ok((get("max_width", 100), get("tab_spaces", 4)));
            }
        }
    }
    Ok((100, 4))
}

/// Runs `fmt`: 0 where all's formatted, 1 where `--check` found a file that
/// isn't or something failed, each failure said on stderr.
pub(crate) fn run(fmt: Fmt) -> ExitCode {
    match formatted(fmt) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: rust-js fmt: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Whether everything is formatted, or was.
fn formatted(fmt: Fmt) -> Result<bool, String> {
    let check = matches!(fmt.mode, Mode::Check);
    if let Sources::Stdin = fmt.sources {
        let mut source = String::new();
        std::io::stdin()
            .read_to_string(&mut source)
            .map_err(|e| format!("reading stdin: {e}"))?;
        let here = std::env::current_dir().map_err(|e| e.to_string())?;
        let edition = edition_of(&here)?;
        let job = Job::new(&here, &edition, Path::new("<stdin>"))?;
        let out = formatted_rust(&job, &source, None)?;
        notes(&job, check);
        if check {
            print!("{}", diff(Path::new("<stdin>"), &source, &out));
            return Ok(out == source);
        }
        std::io::stdout().write_all(out.as_bytes()).map_err(|e| e.to_string())?;
        return Ok(true);
    }
    let sources = match &fmt.sources {
        Sources::Stdin => unreachable!("stdin is formatted above"),
        Sources::Workspace => workspace_sources()?,
        Sources::Paths(paths) => (paths.iter())
            .map(|path| {
                let dir = path
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or(Path::new("."));
                Ok(Source {
                    path: path.clone(),
                    edition: edition_of(dir)?,
                })
            })
            .collect::<Result<_, String>>()?,
    };
    // rustfmt of each, on all the machine's cores; then the JSX pass, of
    // each that has one, in this process.
    let rustfmt: Vec<Result<(String, String), String>> = parallel(&sources, |source| {
        let text = std::fs::read_to_string(&source.path).map_err(|e| format!("{}: {e}", source.path.display()))?;
        let dir = source
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let job = Job::new(dir, &source.edition, &source.path)?;
        let rust = rustfmt(&job, &text, None)?;
        let rust = kept_jsx_bodies(&text, &rust, &source.path)?;
        Ok((text, rust))
    });
    // Each file's, every one read before any is written: one that can't
    // be leaves them all as they were.
    let mut changed = Vec::new();
    for (source, result) in sources.iter().zip(rustfmt) {
        let (text, rust) = result?;
        let dir = (source.path.parent())
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let job = Job::new(dir, &source.edition, &source.path)?;
        let out = match rust.contains("jsx!") {
            true => wrap::laid_out(&job, &jsx(&job, &rust)?)?,
            false => rust,
        };
        notes(&job, check);
        if out != text {
            changed.push((&source.path, text, out));
        }
    }
    for (path, text, out) in &changed {
        match fmt.mode {
            Mode::Check => print!("{}", diff(path, text, out)),
            Mode::Write => std::fs::write(path, out).map_err(|e| format!("{}: {e}", path.display()))?,
        }
    }
    Ok(!check || changed.is_empty())
}

/// `source`, formatted: rustfmt's, at `width` where it's given, else its
/// config's, each `jsx!` body as written, then laid out. A piece of Rust in
/// a `jsx!` is formatted so, its own `jsx!`s first (`wrap.rs`).
fn formatted_rust(job: &Job, source: &str, width: Option<usize>) -> Result<String, String> {
    let rust = rustfmt(job, source, width)?;
    let rust = kept_jsx_bodies(source, &rust, &job.name)?;
    // Its `jsx!`s at the width it's given, a piece's.
    let narrower = width.map(|width| job.at_width(width));
    let job = narrower.as_ref().unwrap_or(job);
    match rust.contains("jsx!") {
        true => wrap::laid_out(job, &jsx(job, &rust)?),
        false => Ok(rust),
    }
}

/// What `job` couldn't lay out, said on stderr where `--check` asks.
fn notes(job: &Job, check: bool) {
    if check {
        for note in job.notes.borrow().iter() {
            eprintln!("note: rust-js fmt: {note}");
        }
    }
}

/// The toolchain's rustfmt of `source`, run in `dir`: rustup's proxy picks
/// the project's toolchain there, and rustfmt its `rustfmt.toml`.
fn rustfmt(job: &Job, source: &str, width: Option<usize>) -> Result<String, String> {
    let (edition, name) = (&job.edition, &job.name);
    let mut command = std::process::Command::new("rustfmt");
    command.args(["--edition", edition, "--emit", "stdout"]);
    if let Some(width) = width {
        command.args(["--config", &format!("max_width={width}")]);
    }
    let mut child = command
        .current_dir(&job.dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("running rustfmt: {e}"))?;
    let mut stdin = child.stdin.take().expect("a piped stdin");
    let text = source.to_string();
    let writer = std::thread::spawn(move || stdin.write_all(text.as_bytes()));
    let output = child.wait_with_output().map_err(|e| format!("running rustfmt: {e}"))?;
    let _ = writer.join();
    if !output.status.success() {
        return Err(format!(
            "rustfmt of {}:\n{}",
            name.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|_| format!("rustfmt of {} gave what isn't UTF-8", name.display()))
}

/// `formatted`, rustfmt's of `original`, with each `jsx!` body as the
/// original wrote it: rustfmt, which can't read one, moves each of its
/// lines with the call, a multi-line string's too, `"first\n second"`'s
/// second line, which changes the program. The JSX pass lays the body out,
/// leaving a literal's inside as it is. Each body, in order, by its
/// tokens; where rustfmt's has another number of them, an error.
fn kept_jsx_bodies(original: &str, formatted: &str, name: &Path) -> Result<String, String> {
    let (before, after) = (jsx_bodies(original), jsx_bodies(formatted));
    if before.len() != after.len() {
        return Err(format!(
            "{}: rustfmt gave {} `jsx!` bodies where the file has {}",
            name.display(),
            after.len(),
            before.len()
        ));
    }
    let mut out = String::with_capacity(formatted.len());
    let mut at = 0;
    for (written, laid) in before.iter().zip(&after) {
        out.push_str(&formatted[at..laid.start]);
        out.push_str(&original[written.clone()]);
        at = laid.end;
    }
    out.push_str(&formatted[at..]);
    Ok(out)
}

/// Where each outermost `jsx!` call's delimited body is in `text`, by its
/// tokens, so a comment's or a string's `jsx!` is none; one inside another
/// is the outer's.
fn jsx_bodies(text: &str) -> Vec<std::ops::Range<usize>> {
    use rustc_lexer::TokenKind as T;
    let mut tokens = Vec::new();
    let mut at = 0;
    for token in rustc_lexer::tokenize(text, rustc_lexer::FrontmatterAllowed::Yes) {
        let len = token.len as usize;
        if !matches!(
            token.kind,
            T::Whitespace | T::LineComment { .. } | T::BlockComment { .. }
        ) {
            tokens.push((token.kind, at, at + len));
        }
        at += len;
    }
    let mut bodies = Vec::new();
    let mut i = 0;
    while i + 2 < tokens.len() {
        let (kind, start, end) = tokens[i];
        let open = matches!(tokens[i + 2].0, T::OpenBrace | T::OpenParen | T::OpenBracket);
        // Not another crate's, `foreign::jsx!`, as the compiler reads it.
        let pathed = i >= 2 && tokens[i - 1].0 == T::Colon && tokens[i - 2].0 == T::Colon;
        if kind == T::Ident && &text[start..end] == "jsx" && tokens[i + 1].0 == T::Bang && open && !pathed {
            let body_start = tokens[i + 2].1;
            let mut depth = 0;
            let mut j = i + 2;
            while j < tokens.len() {
                match tokens[j].0 {
                    T::OpenBrace | T::OpenParen | T::OpenBracket => depth += 1,
                    T::CloseBrace | T::CloseParen | T::CloseBracket => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            let Some(&(_, _, body_end)) = tokens.get(j) else { break };
            bodies.push(body_start..body_end);
            i = j + 1;
            continue;
        }
        i += 1;
    }
    bodies
}

/// The JSX pass of `source`, a file rustfmt formatted: its `jsx!` bodies
/// laid out (`rust_js_jsx::formatting`).
fn jsx(job: &Job, source: &str) -> Result<String, String> {
    let formatter = pass(job, source, false)?;
    formatter.output.ok_or_else(|| {
        format!(
            "{}: its `jsx!` couldn't be read, as rustc said above",
            job.name.display()
        )
    })
}

/// Each `jsx!` body of `source`, a file the JSX pass laid out, with its
/// shape.
fn shapes(job: &Job, source: &str) -> Result<Vec<rust_js_jsx::formatting::Body>, String> {
    let formatter = pass(job, source, true)?;
    match formatter.output {
        Some(_) => Ok(formatter.bodies.unwrap_or_default()),
        None => Err(format!(
            "{}: its `jsx!` couldn't be read, as rustc said above",
            job.name.display()
        )),
    }
}

/// The JSX pass of `source`, with its bodies' shapes where `bodies` asks.
fn pass(job: &Job, source: &str, bodies: bool) -> Result<rust_js_jsx::formatting::Formatter, String> {
    let (edition, name) = (&job.edition, &job.name);
    let mut formatter = rust_js_jsx::formatting::Formatter {
        input: Some((name.display().to_string(), source.to_string())),
        bodies: bodies.then(Vec::new),
        ..Default::default()
    };
    let args = vec![
        "rust-js".to_string(),
        // A name only: `config` gives rustc the text, where `-` would have
        // it read stdin first.
        name.display().to_string(),
        "--crate-type=lib".to_string(),
        format!("--edition={edition}"),
    ];
    // Its output is none where rustc couldn't read it, as it says.
    let _ = rustc_driver::catch_with_exit_code(|| rustc_driver::run_compiler(&args, &mut formatter));
    Ok(formatter)
}

/// `each` of every item, on as many threads as the machine has cores, in
/// the items' order.
fn parallel<T: Sync, R: Send>(items: &[T], each: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(items.len().max(1));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Vec<std::sync::Mutex<Option<R>>> = items.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(item) = items.get(i) else { break };
                    *results[i].lock().expect("a result's slot") = Some(each(item));
                }
            });
        }
    });
    (results.into_iter())
        .map(|slot| slot.into_inner().expect("a result's slot").expect("each item run"))
        .collect()
}

/// The edition of the package whose `Cargo.toml` is nearest above `dir`,
/// as Cargo gives rustfmt it; 2024 where none says.
fn edition_of(dir: &Path) -> Result<String, String> {
    let dir = std::fs::canonicalize(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    // A package's own, or its workspace's, `edition.workspace = true`.
    let mut inherited = false;
    for at in dir.ancestors() {
        let manifest = at.join("Cargo.toml");
        if let Ok(text) = std::fs::read_to_string(&manifest) {
            let value: toml::Table = text.parse().map_err(|e| format!("{}: {e}", manifest.display()))?;
            let workspace = (value.get("workspace").and_then(|w| w.get("package")))
                .and_then(|p| p.get("edition"))
                .and_then(|e| e.as_str());
            if inherited && let Some(edition) = workspace {
                return Ok(edition.to_string());
            }
            if inherited {
                continue;
            }
            let package = value.get("package").and_then(|p| p.as_table());
            match package.and_then(|p| p.get("edition")) {
                Some(toml::Value::String(edition)) => return Ok(edition.clone()),
                Some(toml::Value::Table(edition))
                    if edition.get("workspace").and_then(|w| w.as_bool()) == Some(true) =>
                {
                    if let Some(edition) = workspace {
                        return Ok(edition.to_string());
                    }
                    inherited = true;
                }
                _ if package.is_some() => return Ok("2015".into()),
                _ => {}
            }
        }
    }
    Ok("2024".into())
}

/// Each file of each package of the Cargo workspace it's run in: each
/// target's root and the modules it declares, as `cargo fmt` formats them.
fn workspace_sources() -> Result<Vec<Source>, String> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1", "--offline"])
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("running cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(
            "cargo metadata failed: run `rust-js fmt` where the workspace's Cargo.toml is, or name the files".into(),
        );
    }
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("cargo metadata: {e}"))?;
    let members: Vec<&str> = (metadata["workspace_members"].as_array().into_iter().flatten())
        .filter_map(|m| m.as_str())
        .collect();
    let mut seen = rust_js_program::collections::IndexSet::default();
    let mut sources = Vec::new();
    for package in metadata["packages"].as_array().into_iter().flatten() {
        if !package["id"].as_str().is_some_and(|id| members.contains(&id)) {
            continue;
        }
        let edition = package["edition"].as_str().unwrap_or("2015").to_string();
        for target in package["targets"].as_array().into_iter().flatten() {
            let Some(root) = target["src_path"].as_str() else {
                continue;
            };
            for path in module_tree(Path::new(root)) {
                if seen.insert(path.clone()) {
                    sources.push(Source {
                        path,
                        edition: edition.clone(),
                    });
                }
            }
        }
    }
    Ok(sources)
}

/// `root` and each file of the modules it declares, `mod name;`, found as
/// rustc finds them: `name.rs` or `name/mod.rs` in the directory its file
/// owns, or a `#[path]`'s. A module whose file isn't there is left out, as
/// a `#[cfg]`'d one may be.
fn module_tree(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![(root.to_path_buf(), true)];
    while let Some((file, owns)) = pending.pop() {
        if found.contains(&file) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let parent = file.parent().unwrap_or(Path::new(".")).to_path_buf();
        // The directory its modules are in: its own, of a crate's root or a
        // `mod.rs`; else one named for it, `a/b.rs`'s `a/b/`.
        let dir = match owns || file.file_name().is_some_and(|n| n == "mod.rs") {
            true => parent.clone(),
            false => parent.join(file.file_stem().unwrap_or_default()),
        };
        for module in declared_modules(&text) {
            let base = module.inline.iter().fold(dir.clone(), |dir, inner| dir.join(inner));
            match module.path {
                Some(path) => {
                    let at = match module.inline.is_empty() {
                        true => parent.join(path),
                        false => base.join(path),
                    };
                    pending.push((at, true));
                }
                None => {
                    let flat = base.join(format!("{}.rs", module.name));
                    let nested = base.join(&module.name).join("mod.rs");
                    if flat.exists() {
                        pending.push((flat, false));
                    } else if nested.exists() {
                        pending.push((nested, true));
                    }
                }
            }
        }
        found.push(file);
    }
    found.sort();
    found
}

/// A module a file declares, `mod name;`, inside the inline modules
/// `inline` names, with its `#[path]`.
struct Declared {
    name: String,
    path: Option<String>,
    inline: Vec<String>,
}

/// Each `mod name;` of `text`, by its tokens, so a comment's or a string's
/// is none.
fn declared_modules(text: &str) -> Vec<Declared> {
    use rustc_lexer::TokenKind as T;
    let mut tokens = Vec::new();
    let mut at = 0;
    for token in rustc_lexer::tokenize(text, rustc_lexer::FrontmatterAllowed::Yes) {
        let len = token.len as usize;
        if !matches!(
            token.kind,
            T::Whitespace | T::LineComment { .. } | T::BlockComment { .. }
        ) {
            tokens.push((token.kind, &text[at..at + len]));
        }
        at += len;
    }
    let mut found = Vec::new();
    // The inline modules open, each with the depth of braces it opened at.
    let mut inline: Vec<(String, usize)> = Vec::new();
    let mut depth = 0;
    // A `#[path = "..."]` just read, for the item after it.
    let mut path: Option<String> = None;
    let mut i = 0;
    while i < tokens.len() {
        let (kind, text) = tokens[i];
        match kind {
            T::Pound if matches!(tokens.get(i + 1), Some((T::OpenBracket, _))) => {
                // An attribute: `path = "..."`'s value, else skipped whole.
                let mut j = i + 2;
                let mut nesting = 1;
                let is_path =
                    matches!(tokens.get(j), Some((T::Ident, "path"))) && matches!(tokens.get(j + 1), Some((T::Eq, _)));
                if is_path && let Some((T::Literal { .. }, literal)) = tokens.get(j + 2) {
                    path = Some(literal.trim_matches('"').to_string());
                }
                while j < tokens.len() && nesting > 0 {
                    match tokens[j].0 {
                        T::OpenBracket => nesting += 1,
                        T::CloseBracket => nesting -= 1,
                        _ => {}
                    }
                    j += 1;
                }
                i = j;
                continue;
            }
            T::Ident if text == "mod" => {
                if let Some((T::Ident, name)) = tokens.get(i + 1) {
                    let name = name.trim_start_matches("r#").to_string();
                    match tokens.get(i + 2) {
                        Some((T::Semi, _)) => found.push(Declared {
                            name,
                            path: path.take(),
                            inline: inline.iter().map(|(n, _)| n.clone()).collect(),
                        }),
                        Some((T::OpenBrace, _)) => {
                            inline.push((path.take().unwrap_or(name), depth));
                            depth += 1;
                            i += 3;
                            continue;
                        }
                        _ => {}
                    }
                }
            }
            T::OpenBrace => depth += 1,
            T::CloseBrace => {
                depth = depth.saturating_sub(1);
                if inline.last().is_some_and(|&(_, opened)| opened == depth) {
                    inline.pop();
                }
            }
            T::Semi => {
                path = None;
            }
            _ => {}
        }
        i += 1;
    }
    found
}

/// A unified diff of `before` and `after`, `path`'s: each run of lines
/// that changed, by their longest common lines, with three lines of
/// context. Of files too long to compare so, one hunk from the first line
/// that differs to the last.
fn diff(path: &Path, before: &str, after: &str) -> String {
    #[derive(Clone, Copy, PartialEq)]
    enum Op {
        Same,
        Gone,
        Added,
    }
    if before == after {
        return String::new();
    }
    let old: Vec<&str> = before.split_inclusive('\n').collect();
    let new: Vec<&str> = after.split_inclusive('\n').collect();
    // The lines alike at each end, then the longest common lines of what's
    // between (`lcs[i][j]`, of `old[i..]` and `new[j..]`).
    let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let suffix = (old[prefix..].iter().rev())
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let (a, b) = (&old[prefix..old.len() - suffix], &new[prefix..new.len() - suffix]);
    let mut ops: Vec<(Op, &str)> = old[..prefix].iter().map(|&l| (Op::Same, l)).collect();
    if a.len().saturating_mul(b.len()) <= 16_000_000 {
        let width = b.len() + 1;
        let mut lcs = vec![0u32; (a.len() + 1) * width];
        for i in (0..a.len()).rev() {
            for j in (0..b.len()).rev() {
                lcs[i * width + j] = match a[i] == b[j] {
                    true => lcs[(i + 1) * width + j + 1] + 1,
                    false => lcs[(i + 1) * width + j].max(lcs[i * width + j + 1]),
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < a.len() || j < b.len() {
            if i < a.len() && j < b.len() && a[i] == b[j] {
                ops.push((Op::Same, a[i]));
                (i, j) = (i + 1, j + 1);
            } else if i < a.len() && (j == b.len() || lcs[(i + 1) * width + j] >= lcs[i * width + j + 1]) {
                ops.push((Op::Gone, a[i]));
                i += 1;
            } else {
                ops.push((Op::Added, b[j]));
                j += 1;
            }
        }
    } else {
        ops.extend(a.iter().map(|&l| (Op::Gone, l)));
        ops.extend(b.iter().map(|&l| (Op::Added, l)));
    }
    ops.extend(old[old.len() - suffix..].iter().map(|&l| (Op::Same, l)));
    let mut out = format!("--- {0}\n+++ {0}\n", path.display());
    let changed: Vec<usize> = (0..ops.len()).filter(|&k| ops[k].0 != Op::Same).collect();
    let mut k = 0;
    while k < changed.len() {
        // A hunk: changes no more than six lines apart, three of context
        // each side.
        let first = changed[k];
        let mut last = first;
        while k + 1 < changed.len() && changed[k + 1] - last <= 7 {
            k += 1;
            last = changed[k];
        }
        k += 1;
        let (start, end) = (first.saturating_sub(3), (last + 4).min(ops.len()));
        let line_at = |op: Op| 1 + ops[..start].iter().filter(|(o, _)| *o == Op::Same || *o == op).count();
        let count = |op: Op| {
            ops[start..end]
                .iter()
                .filter(|(o, _)| *o == Op::Same || *o == op)
                .count()
        };
        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            line_at(Op::Gone),
            count(Op::Gone),
            line_at(Op::Added),
            count(Op::Added)
        ));
        for &(op, text) in &ops[start..end] {
            out.push(match op {
                Op::Same => ' ',
                Op::Gone => '-',
                Op::Added => '+',
            });
            out.push_str(text);
            if !text.ends_with('\n') {
                out.push_str("\n\\ No newline at end of file\n");
            }
        }
    }
    out
}
