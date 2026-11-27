//! The worker, `rust-js --worker` (#2301, ADR 1182): the tests' harness's
//! way to compile a case's builds, its program's and its library's, in one
//! rustc session, its front end and the process's start paid once. Dev-only:
//! every other host compiles a build a process, as `rust-js <input.rs> ..`.
//!
//! A job is a line of JSON on stdin, `{"builds": [[..], [..]]}`, each build
//! the arguments its own compile is given, read by `command::parse` as that
//! compile's are; its answer a line on stdout, each build's exit and output,
//! `{"code", "stdout", "stderr"}`, what its own process would have given, or
//! `{"apart": ".."}`, a build the harness compiles in a process of its own.
//!
//! A build's output is its own process's, byte for byte, or it runs apart:
//! - the session's builds differ only in what they write, `-o`, `--library`,
//!   `--manifest` and the cache's record (`--emit=dep-info`), or the job is
//!   refused;
//! - each build lowers in a lowering session of its own (ADR 1111), of the
//!   crate's THIR and MIR copied once, before borrowck steals them;
//! - the session goes on to a build only where nothing before it printed:
//!   rustc gives a query it has answered again without its diagnostics, so
//!   a build after one that printed could print less than its own process;
//! - each is published once rustc is done, in order, its output captured
//!   apart from the others' (`Capture`);
//! - the statics a build leaves are each what `test/architecture.test.ts`'s
//!   catalogue says, none another build reads (ADR 1182).

// POSIX's, which the worker's capture of a build's output redirects with.
unsafe extern "C" {
    fn dup(fd: i32) -> i32;
    fn dup2(from: i32, to: i32) -> i32;
}

use std::io::{BufRead, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::path::PathBuf;
use std::process::ExitCode;

use rustc_driver::{Callbacks, Compilation};
use rustc_interface::interface::Compiler;
use rustc_middle::ty::TyCtxt;

use crate::{RustJs, command, finish_build, front, phase_times, pipeline, prepare_build, run_resolving_props};

/// A job: each build's arguments.
#[derive(serde::Deserialize)]
struct Job {
    builds: Vec<Vec<String>>,
}

/// A build's answer.
#[derive(serde::Serialize)]
#[serde(untagged)]
enum Answer {
    Ran { code: u8, stdout: String, stderr: String },
    Apart { apart: String },
}

#[derive(serde::Serialize)]
#[serde(untagged)]
enum Answered {
    Builds { builds: Vec<Answer> },
    Refused { error: String },
}

/// Each job on stdin, answered on stdout, until stdin ends.
pub(crate) fn run() -> ExitCode {
    let mut capture = match Capture::new() {
        Ok(capture) => capture,
        Err(error) => {
            eprintln!("error: rust-js: the worker can't capture a build's output: {error}");
            return ExitCode::FAILURE;
        }
    };
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let answered = match serde_json::from_str::<Job>(&line) {
            Ok(job) => match job_builds(job, &mut capture) {
                Ok(builds) => Answered::Builds { builds },
                Err(error) => Answered::Refused { error },
            },
            Err(error) => Answered::Refused {
                error: format!("a job is `{{\"builds\": [[<a build's arguments>], ..]}}`: {error}"),
            },
        };
        let text = serde_json::to_string(&answered).expect("an answer serializes");
        if capture.answer(&text).is_err() {
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// What a build's arguments are but what it writes: `-o`, `--library`,
/// `--manifest` and the cache's record, whose path is left out. Builds a
/// session compiles have the same, their input alone of rust-js's own.
fn shared_part(args: &[String]) -> Result<(Vec<String>, Vec<String>), String> {
    let at = args.iter().position(|arg| arg == "--").unwrap_or(args.len());
    let (ours, rustc) = (&args[..at], args.get(at + 1..).unwrap_or_default());
    let mut kept = Vec::new();
    let mut given = ours.iter();
    while let Some(arg) = given.next() {
        match arg.as_str() {
            "-o" | "--manifest" => {
                given.next();
            }
            "--library" => {}
            _ => kept.push(arg.clone()),
        }
    }
    if kept.len() != 1 || kept[0].starts_with('-') {
        return Err(format!(
            "a worker's builds take no flag of rust-js's but -o, --library and --manifest: {}",
            ours.join(" ")
        ));
    }
    let rustc = (rustc.iter())
        .map(|arg| match arg.starts_with("--emit=dep-info=") {
            true => "--emit=dep-info=<record>".to_string(),
            false => arg.clone(),
        })
        .collect();
    Ok((kept, rustc))
}

/// `job`'s builds compiled in one rustc session, each answered.
fn job_builds(job: Job, capture: &mut Capture) -> Result<Vec<Answer>, String> {
    let Some(first) = job.builds.first() else {
        return Err("a job has a build at least".into());
    };
    let shared = shared_part(first)?;
    for build in &job.builds[1..] {
        if shared_part(build)? != shared {
            return Err("a job's builds are of one input, with the same rustc flags".into());
        }
    }
    phase_times::start();
    let mut prepared = Vec::new();
    for args in job.builds {
        match command::parse(args)? {
            command::Command::Build(build) if !build.cargo => prepared.push(prepare_build(*build)?),
            _ => return Err("a job's builds are each a crate's compile".into()),
        }
    }
    if prepared.iter().any(|build| build.callbacks.metadata) {
        return Err("a worker's builds write no metadata".into());
    }
    let rustc_args = prepared[0].rustc_args.clone();
    let input = shared.0[0].clone();
    let mut session = Shared {
        builds: prepared.iter_mut().map(|build| &mut build.callbacks).collect(),
        capture,
        ran: 0,
        expansions: 0,
        apart: None,
    };
    session.capture.begin()?;
    let exit = run_resolving_props(&rustc_args, &mut session);
    phase_times::mark("rustc: the rest");
    let Shared {
        builds,
        capture,
        ran,
        expansions,
        apart,
    } = session;
    drop(builds);
    let (ran, apart) = match expansions {
        1 => (ran, apart),
        _ => (0, Some("its props were replayed (ADR 0213)".to_string())),
    };
    let lowered = capture.end()?;
    // The record rustc wrote of the session, each build's own: what it read
    // is what every build read, as the last of them is lowered.
    let record = prepared[0].dep_info.as_ref().map(|(staged, _)| staged.clone());
    if let Some(record) = record.filter(|record| record.exists()) {
        for build in prepared.iter().take(ran).skip(1) {
            if let Some((staged, _)) = &build.dep_info {
                std::fs::copy(&record, staged).map_err(|error| format!("{}: {error}", staged.display()))?;
            }
        }
    }
    let mut answers = Vec::new();
    for (i, build) in prepared.into_iter().enumerate() {
        if i >= ran {
            let why = apart.clone().unwrap_or_else(|| "an earlier build printed".into());
            answers.push(Answer::Apart { apart: why });
            continue;
        }
        // Each build before the last the session ran printed nothing, and
        // so ended as rustc does of a build with no error.
        let exit = if i + 1 == ran { exit } else { ExitCode::SUCCESS };
        capture.begin()?;
        let code = match finish_build(build, exit) {
            Ok(exit) if exit == ExitCode::SUCCESS => 0,
            Ok(_) => 1,
            Err(error) => {
                eprintln!("error: rust-js: {error}");
                1
            }
        };
        let published = capture.end()?;
        let (out, err) = (&lowered[i], &published[0]);
        answers.push(Answer::Ran {
            code,
            stdout: String::from_utf8_lossy(&[out.0.as_slice(), err.0.as_slice()].concat()).into_owned(),
            stderr: String::from_utf8_lossy(&[out.1.as_slice(), err.1.as_slice()].concat()).into_owned(),
        });
    }
    crate::append_phase_times(Some(&input), false);
    Ok(answers)
}

/// rustc's callbacks of a job's builds, in one session.
struct Shared<'b, 'c> {
    builds: Vec<&'b mut RustJs>,
    capture: &'c mut Capture,
    /// How many of the builds, in order, the session lowered; the rest run
    /// apart.
    ran: usize,
    /// How many times rustc expanded the crate: twice where props' names
    /// were replayed (ADR 0213).
    expansions: usize,
    /// Why the builds after `ran` run apart.
    apart: Option<String>,
}

impl Callbacks for Shared<'_, '_> {
    fn config(&mut self, config: &mut rustc_interface::interface::Config) {
        // Each build's, which agree but where a library's metadata keeps its
        // MIR, which only its contract reads of, by the crate's hash.
        let panic = config.opts.cg.panic;
        for build in &mut self.builds {
            config.opts.cg.panic = panic;
            build.config(config);
        }
    }

    fn after_crate_root_parsing(&mut self, compiler: &Compiler, krate: &mut rustc_ast::Crate) -> Compilation {
        self.builds[0].after_crate_root_parsing(compiler, krate)
    }

    fn after_expansion<'tcx>(&mut self, compiler: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
        self.expansions += 1;
        self.ran = 1;
        phase_times::mark("rustc: parse and expand");
        let Some(serde_attrs) = front(compiler, tcx) else {
            return Compilation::Stop;
        };
        // Each build's bodies copied once, before borrowck steals them,
        // and their MIR read once: in the first build's lowering session,
        // as its own compile reads them in it, and filtered by each.
        let session = pipeline::start_session(tcx, self.builds[0].session_flags(tcx));
        let mut copied = rust_js_lower::copied_bodies(tcx);
        let mut initializers = rust_js_lower::collect_initializers(tcx);
        rust_js_lower::collect_mir(tcx, &mut copied.bodies);
        rust_js_lower::collect_mir(tcx, &mut initializers);
        let mut bodies = rust_js_lower::bodies_of_build(tcx, copied.clone(), self.builds[0].library_filter());
        tcx.ensure_ok().analysis(());
        phase_times::mark("rustc: type check, MIR and borrow check");
        let mut given = initializers.clone();
        self.builds[0].lower_build(tcx, (&mut bodies, &mut given), &serde_attrs);
        drop(session);
        for i in 1..self.builds.len() {
            if self.capture.printed() {
                self.apart = Some("an earlier build printed".into());
                break;
            }
            if let Err(error) = self.capture.next() {
                self.apart = Some(error);
                break;
            }
            self.ran = i + 1;
            let build = &mut self.builds[i];
            let _session = pipeline::start_session(tcx, build.session_flags(tcx));
            let mut bodies = rust_js_lower::bodies_of_build(tcx, copied.clone(), build.library_filter());
            let mut given = initializers.clone();
            build.lower_build(tcx, (&mut bodies, &mut given), &serde_attrs);
        }
        let lowered = self.builds.iter().any(|build| build.lowered.is_some());
        self.builds[0].after_builds(tcx, lowered)
    }
}

/// What a segment printed: its stdout's bytes and its stderr's.
type Printed = (Vec<u8>, Vec<u8>);

/// Where a build's output goes while it runs: a file a segment, of stdout
/// and of stderr, the process's own kept for the answers.
struct Capture {
    dir: PathBuf,
    stdout: OwnedFd,
    stderr: OwnedFd,
    /// The segments since `begin`, each its stdout's and stderr's files.
    segments: Vec<(PathBuf, PathBuf)>,
    made: usize,
}

impl Capture {
    fn new() -> Result<Capture, String> {
        let dir = std::env::temp_dir().join(format!("rust-js-worker-{}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|error| format!("{}: {error}", dir.display()))?;
        Ok(Capture {
            dir,
            stdout: duplicate(1)?,
            stderr: duplicate(2)?,
            segments: Vec::new(),
            made: 0,
        })
    }

    /// The first segment's output goes to its files.
    fn begin(&mut self) -> Result<(), String> {
        self.segments.clear();
        self.next()
    }

    /// The next segment's output goes to its own files.
    fn next(&mut self) -> Result<(), String> {
        flush();
        self.made += 1;
        let files = (
            self.dir.join(format!("{}.out", self.made)),
            self.dir.join(format!("{}.err", self.made)),
        );
        for (path, fd) in [(&files.0, 1), (&files.1, 2)] {
            let file = std::fs::File::create(path).map_err(|error| format!("{}: {error}", path.display()))?;
            redirect(file.as_raw_fd(), fd)?;
        }
        self.segments.push(files);
        Ok(())
    }

    /// Whether any segment since `begin` printed.
    fn printed(&self) -> bool {
        flush();
        (self.segments.iter()).any(|(out, err)| {
            [out, err]
                .iter()
                .any(|path| std::fs::metadata(path).is_ok_and(|file| file.len() > 0))
        })
    }

    /// The process's own output back, and each segment's since `begin`.
    fn end(&mut self) -> Result<Vec<Printed>, String> {
        flush();
        redirect(self.stdout.as_raw_fd(), 1)?;
        redirect(self.stderr.as_raw_fd(), 2)?;
        let read = |path: &PathBuf| {
            let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()));
            let _ = std::fs::remove_file(path);
            bytes
        };
        (self.segments.drain(..))
            .map(|(out, err)| Ok((read(&out)?, read(&err)?)))
            .collect()
    }

    /// `line`, an answer, on the process's own stdout.
    fn answer(&self, line: &str) -> std::io::Result<()> {
        let mut out = std::fs::File::from(self.stdout.try_clone()?);
        out.write_all(format!("{line}\n").as_bytes())?;
        out.flush()
    }
}

impl Drop for Capture {
    /// Its files removed; but where a panic unwinds past it, what each
    /// segment since `begin` printed, the panic's message among it, said
    /// on the process's own stderr first: rustc's `catch_with_exit_code`
    /// unwinds a panic that isn't a fatal error past the session, and the
    /// harness saw only the worker's exit, 101, of a job it died on (#2448).
    fn drop(&mut self) {
        if std::thread::panicking() {
            flush();
            let _ = redirect(self.stdout.as_raw_fd(), 1);
            let _ = redirect(self.stderr.as_raw_fd(), 2);
            let mut stderr = std::io::stderr();
            for (out, err) in &self.segments {
                for path in [out, err] {
                    if let Ok(bytes) = std::fs::read(path) {
                        let _ = stderr.write_all(&bytes);
                    }
                }
            }
            let _ = stderr.flush();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn flush() {
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
}

/// A copy of the process's `fd`.
fn duplicate(fd: i32) -> Result<OwnedFd, String> {
    // SAFETY: `dup` of a descriptor the process has; the copy is owned here.
    let copy = unsafe { dup(fd) };
    match copy {
        -1 => Err(format!("dup({fd}): {}", std::io::Error::last_os_error())),
        // SAFETY: `copy` is a descriptor `dup` just made, owned by none else.
        copy => Ok(unsafe { OwnedFd::from_raw_fd(copy) }),
    }
}

/// `to`, the process's stdout or stderr, made `from`.
fn redirect(from: i32, to: i32) -> Result<(), String> {
    // SAFETY: both are descriptors the process has open.
    match unsafe { dup2(from, to) } {
        -1 => Err(format!("dup2({from}, {to}): {}", std::io::Error::last_os_error())),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Capture, duplicate, redirect, shared_part};
    use std::io::Write;
    use std::os::fd::{AsRawFd, FromRawFd};

    /// A panic past a build's capture says what the build printed, its
    /// panic's message among it, on the process's own stderr, not in a file
    /// it removes (#2448).
    #[test]
    fn a_panic_past_a_capture_says_what_its_build_printed() {
        let dir = std::env::temp_dir().join(format!("rust-js-worker-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let own = dir.join("stderr");
        // The process's own, put back whatever the capture left: the test's
        // runner reports on them.
        let saved = (duplicate(1).unwrap(), duplicate(2).unwrap());
        let file = std::fs::File::create(&own).unwrap();
        redirect(file.as_raw_fd(), 2).unwrap();
        let died = std::panic::catch_unwind(|| {
            let mut capture = Capture::new().unwrap();
            capture.begin().unwrap();
            // SAFETY: the process's stderr, written and not closed.
            let mut stderr = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(2) });
            stderr.write_all(b"the build's own line\n").unwrap();
            panic!("a build dies");
        });
        redirect(saved.0.as_raw_fd(), 1).unwrap();
        redirect(saved.1.as_raw_fd(), 2).unwrap();
        let said = std::fs::read_to_string(&own).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert!(died.is_err());
        assert!(said.contains("the build's own line"), "{said}");
    }

    fn args(text: &str) -> Vec<String> {
        text.split(' ').map(String::from).collect()
    }

    #[test]
    fn builds_share_a_session_where_they_differ_only_in_what_they_write() {
        let program = shared_part(&args("lib.rs -o case.js -- --edition=2024 --emit=dep-info=a.d")).unwrap();
        let library = shared_part(&args(
            "lib.rs -o library/case.js --library --manifest library/case.manifest.json -- --edition=2024 --emit=dep-info=b.d",
        ))
        .unwrap();
        assert_eq!(program, library);
        let other = shared_part(&args("lib.rs -o case.js -- --edition=2021")).unwrap();
        assert_ne!(program, other);
        let flagged = shared_part(&args("lib.rs -o case.js --tidy-until x -- --edition=2024")).unwrap_err();
        assert!(flagged.contains("no flag of rust-js's but -o, --library and --manifest"));
    }
}
