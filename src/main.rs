//! rust-js: compile Rust to readable JavaScript, in the spirit of ReScript.
//!
//! ReScript reuses OCaml's type checker and swaps in a JS backend. We do
//! the same with rustc:
//!
//! ```text
//!   .rs ─► rustc: parse, resolve, type check ─► THIR ──┐ (copied: shape, facts)
//!                                                │      │
//!                                               MIR ────┤ (copied: what runs)
//!                                                │      ▼
//!                                       borrowck │   lower.rs ─► js.rs ─► prepare.rs ─► to_oxc.rs ─► .js + .js.map
//!                          errors? stop here ◄───┘
//! ```
//!
//! Usage: `rust-js [--test] <input.rs> [-o <output.js>] [--manifest <file.json>]
//! [--library] [--dependency <manifest.json>] [-- <rustc flags>]`. Also
//! writes `<output.js>.map`. Flags after `--` go to rustc unchanged. With
//! `--test`, the crate's `#[test]` functions are compiled too, and
//! `<output>.test.js` runs them with `bun test` (ADR 0026).

#![feature(rustc_private)]

extern crate rustc_abi;
extern crate rustc_arena;
extern crate rustc_ast;
extern crate rustc_const_eval;
extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_expand;
extern crate rustc_hir;
extern crate rustc_hir_analysis;
extern crate rustc_index;
extern crate rustc_interface;
extern crate rustc_lexer;
extern crate rustc_middle;
extern crate rustc_mir_dataflow;
extern crate rustc_parse;
extern crate rustc_session;
extern crate rustc_span;

mod cargo;
mod command;
mod format;
mod hooks;
mod js;
mod jsx_syntax;
mod library;
mod link;
mod lower;
mod manifest;
mod names;
mod output;
mod paths;
mod prepare;
mod program;
mod publish;
mod reachability;
mod runtime;
mod settings;
mod to_oxc;
mod typescript;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::OnceLock;

use rustc_driver::{Callbacks, Compilation};
use rustc_interface::interface::Compiler;
use rustc_middle::ty::RegisteredTools;
use rustc_middle::ty::TyCtxt;
use rustc_session::config::CrateType;
use rustc_span::def_id::{LOCAL_CRATE, StableCrateId};
use rustc_span::{Ident, Symbol};

/// The crate's `StableCrateId`, as rustc computes it once it has the crate's
/// context, which is after JSX's expansions need it (ADR 0110). rustc's own
/// helpers, in its order; `after_expansion` checks it's rustc's.
fn stable_crate_id(compiler: &Compiler, krate: &rustc_ast::Crate) -> StableCrateId {
    let sess = &compiler.sess;
    let attrs = rustc_expand::config::pre_configure_attrs(sess, &krate.attrs);
    let name = rustc_interface::passes::get_crate_name(sess, &attrs);
    let backend = &compiler.codegen_backend;
    let types = rustc_interface::passes::collect_crate_types(
        sess,
        &backend.supported_crate_types(sess),
        backend.name(),
        &attrs,
        krate.spans.inner_span,
    );
    StableCrateId::new(
        name,
        types.contains(&CrateType::Executable),
        sess.opts.cg.metadata.clone(),
        sess.cfg_version,
    )
}

/// rustc's own tools, `rustfmt` and the rest, which `rust_js` joins.
static RUSTC_TOOLS: OnceLock<for<'tcx> fn(TyCtxt<'tcx>, ()) -> RegisteredTools> = OnceLock::new();

struct RustJs {
    dependencies: library::Dependencies,
    export_library: bool,
    /// `--emit=metadata` among rustc's flags: rustc goes on to write the
    /// crate's metadata, for the crates that use it (ADR 0100).
    metadata: bool,
    /// What's lowered, and the sources it was of, planned and published once
    /// rustc is done, and has written the metadata too: a library's JS and
    /// its metadata are one build's, or neither is.
    lowered: Option<(link::Linked, Vec<PathBuf>)>,
    /// `--std-coverage <file>`: where to write how much of std's data
    /// structures rust-js lowers (ADR 0314).
    std_coverage: Option<PathBuf>,
    /// Cargo's build (ADR 0101), which is told what `RUST_JS_APP` was.
    cargo: bool,
}

/// `rust_js` is a tool rustc knows, as it knows `rustfmt`: a program
/// writes `#[rust_js::link_name = ".."]` as stable Rust (ADR 0110).
fn register_tool(config: &mut rustc_interface::interface::Config) {
    config.override_queries = Some(|_, providers| {
        RUSTC_TOOLS.get_or_init(|| providers.queries.registered_attr_tools);
        providers.queries.registered_attr_tools = |tcx, ()| {
            let mut tools = RUSTC_TOOLS.get().expect("rustc's tools, kept first")(tcx, ());
            tools.insert(Ident::with_dummy_span(Symbol::intern("rust_js")));
            tools
        };
    });
}

/// rustc itself, with rust-js's tool and syntax and nothing else, `--rustc`:
/// what the crates a program uses, js, webapi and react, are compiled with,
/// for their metadata (ADR 0112).
struct Syntax;

impl Callbacks for Syntax {
    fn config(&mut self, config: &mut rustc_interface::interface::Config) {
        register_tool(config);
    }

    fn after_crate_root_parsing(&mut self, compiler: &Compiler, krate: &mut rustc_ast::Crate) -> Compilation {
        jsx_syntax::expand(&compiler.sess, krate, stable_crate_id(compiler, krate));
        Compilation::Continue
    }

    fn after_expansion<'tcx>(&mut self, _compiler: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
        jsx_syntax::check_crate_id(tcx.stable_crate_id(LOCAL_CRATE));
        Compilation::Continue
    }
}

impl Callbacks for RustJs {
    fn config(&mut self, config: &mut rustc_interface::interface::Config) {
        register_tool(config);
        // Integers wrap, as the release profile has them: MIR read for what
        // runs says so, where it would check each operation (ADR 0364).
        config.opts.cg.overflow_checks = Some(false);
    }

    fn after_crate_root_parsing(&mut self, compiler: &Compiler, krate: &mut rustc_ast::Crate) -> Compilation {
        // Whether the crate is the app is `RUST_JS_APP`'s, which Cargo
        // reads of the dep-info, as of an `env!`: another value is another
        // build (ADR 0360).
        if self.cargo {
            let value = std::env::var(cargo::APP).ok().map(|value| Symbol::intern(&value));
            compiler
                .sess
                .env_depinfo
                .lock()
                .insert((Symbol::intern(cargo::APP), value));
        }
        Syntax.after_crate_root_parsing(compiler, krate)
    }

    fn after_expansion<'tcx>(&mut self, _compiler: &Compiler, tcx: TyCtxt<'tcx>) -> Compilation {
        jsx_syntax::check_crate_id(tcx.stable_crate_id(LOCAL_CRATE));
        // 0. `#[serde(..)]`, which only the expanded crate still has (ADR 0077).
        let serde_attrs = lower::serde_attributes(tcx);
        // 1. Copy each function's THIR. MIR building (for borrowck) steals it.
        let mut bodies = lower::collect_bodies(tcx);
        let mut initializers = lower::collect_initializers(tcx);
        // Then, read from their MIR, what runs (ADR 0364): the initializers
        // lowered as code too.
        lower::collect_mir(tcx, &mut bodies);
        lower::collect_mir(tcx, &mut initializers);

        // 2. Run rustc's full analysis: type check, borrow check, lints.
        tcx.ensure_ok().analysis(());
        if let Some(path) = &self.std_coverage
            && let Err(error) = std::fs::write(path, lower::std_coverage::report(tcx, &self.dependencies))
        {
            tcx.dcx().err(format!("rust-js: writing {}: {error}", path.display()));
        }

        // 3. Only a program rustc accepts becomes JavaScript.
        if tcx.dcx().has_errors().is_none()
            && let Some(unlinked) = lower::lower_crate(
                tcx,
                &bodies,
                &initializers,
                &serde_attrs,
                &self.dependencies,
                self.export_library,
            )
            && tcx.dcx().has_errors().is_none()
        {
            let linked = link::link(unlinked);
            let sources = tcx
                .sess
                .source_map()
                .files()
                .iter()
                .filter(|file| file.src.is_some())
                .filter_map(|file| file.name.clone().into_local_path())
                .chain(self.dependencies.inputs.iter().cloned())
                .collect();
            self.lowered = Some((linked, sources));
        }

        // We never want rustc's own codegen, only, for a library, the metadata
        // its consumers' rustc reads: that of the crate rust-js compiled, as it
        // configured it, `cfg(rust_js)` and all.
        if self.metadata && tcx.dcx().has_errors().is_none() {
            Compilation::Continue
        } else {
            Compilation::Stop
        }
    }
}

fn main() -> ExitCode {
    // A body's MIR is lowered block by block, as deep as the function is
    // long (ADR 0364): rustc's thread has the stack for it, unless
    // `RUST_MIN_STACK` says otherwise, as it does to rustc.
    if std::env::var_os("RUST_MIN_STACK").is_none() {
        let _ = rustc_interface::util::STACK_SIZE.set(256 * 1024 * 1024);
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Cargo's workspace wrapper (ADR 0101): its call, as rust-js's, or rustc's.
    let args = match cargo::translate(args) {
        cargo::Invocation::RustJs(args) => args,
        cargo::Invocation::Rustc(exit) => return exit,
    };
    let build = match command::parse(args) {
        Ok(command::Command::Build(build)) => build,
        Ok(command::Command::RuntimeModule) => {
            print!("{}", runtime::package_module());
            return ExitCode::SUCCESS;
        }
        Ok(command::Command::VersionJson) => {
            println!(
                "{}",
                serde_json::to_string(&manifest::Compiler::current()).expect("serialize compiler identity")
            );
            return ExitCode::SUCCESS;
        }
        Ok(command::Command::Version) => {
            let compiler = manifest::Compiler::current();
            println!(
                "rust-js {} ({}; ABI {})",
                compiler.version, compiler.toolchain, compiler.abi
            );
            return ExitCode::SUCCESS;
        }
        Ok(command::Command::Rustc(args)) => {
            // `cfg(rust_js)` says it's rust-js, as it says to a program: the crates'
            // attributes of its tool are `cfg_attr(rust_js, ..)`, which a plain
            // rustc, a user's own `cargo check`, leaves out (ADR 0113).
            let ours = ["rust-js", "--cfg=rust_js", "--check-cfg=cfg(rust_js)"].map(String::from);
            let rustc_args: Vec<String> = ours.into_iter().chain(args).collect();
            return rustc_driver::catch_with_exit_code(|| rustc_driver::run_compiler(&rustc_args, &mut Syntax));
        }
        Ok(command::Command::FormatJsx(input)) => {
            let mut formatter = jsx_syntax::formatting::Formatter::default();
            let args = vec![
                "rust-js".into(),
                input,
                "--crate-type=lib".into(),
                "--edition=2024".into(),
            ];
            let result = rustc_driver::catch_with_exit_code(|| rustc_driver::run_compiler(&args, &mut formatter));
            if let Some(output) = formatter.output {
                print!("{output}");
            }
            return result;
        }
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    match compile(build) {
        Ok(exit) => exit,
        // As rustc says its errors, which a build's are among.
        Err(error) => {
            eprintln!("error: rust-js: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The directory rustc's outputs are staged in (`command::Staged`), made
/// where something is, and removed when it's dropped, however the build ends.
struct Stage(Option<std::path::PathBuf>);

impl Stage {
    fn create(staged: &command::Staged) -> Result<Self, String> {
        if !staged.any() {
            return Ok(Stage(None));
        }
        std::fs::create_dir_all(&staged.dir).map_err(|error| format!("cannot stage the metadata: {error}"))?;
        Ok(Stage(Some(staged.dir.clone())))
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        if let Some(dir) = &self.0 {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// A build: rustc run with rust-js's callbacks, then what it made published
/// and checked. Its error, whichever step's, `main` says.
fn compile(build: command::Build) -> Result<ExitCode, String> {
    let dependencies = library::Dependencies::load(&build.dependencies, &build.output)?;
    let (rustc_args, staged) = build.rustc_args()?;
    let stage = Stage::create(&staged)?;
    let command::Build {
        input,
        output,
        test,
        cargo,
        library: export_library,
        dependencies: dependency_paths,
        save,
        manifest,
        std_coverage,
        ..
    } = build;
    let command::Staged { metadata, dep_info, .. } = staged;
    let recorded = cargo.then(|| (manifest.clone(), metadata.clone()));
    let settings = settings::read(&input, cargo)?;
    // A width oxfmt can't take fails here, once, not each module after: the
    // printer's to say, which settings, read before it, doesn't ask.
    format::options_of(&settings.format)
        .map_err(|error| format!("`{}`: {error}", settings.dir.join("Cargo.toml").display()))?;
    let mut plan = output::OutputPlan::new(input, output, test, manifest);
    plan.metadata = metadata.clone();
    plan.settings = settings;
    let mut callbacks = RustJs {
        dependencies,
        export_library,
        metadata: metadata.is_some(),
        lowered: None,
        std_coverage,
        cargo,
    };
    let exit = rustc_driver::catch_with_exit_code(|| rustc_driver::run_compiler(&rustc_args, &mut callbacks));
    // Once rustc has written the metadata: if it couldn't, nothing is published.
    if let Some((linked, sources)) = callbacks.lowered.take()
        && exit == ExitCode::SUCCESS
    {
        // What Cargo is told of the build, published with it (ADR 0101).
        plan.extra = match &recorded {
            Some((Some(manifest), Some((_, metadata)))) => cargo::record(
                manifest,
                &dependency_paths,
                metadata,
                dep_info
                    .as_ref()
                    .map(|(staged, path)| (staged.as_path(), path.as_path())),
            )?,
            _ => Vec::new(),
        };
        plan.plan(linked, sources)?.publish()?;
    }
    drop(stage);
    // Once it's all written, the crate's checks of it. Cargo's build runs
    // rust-js for each crate, so its driver is to run them, once (ADR 0117).
    if exit == ExitCode::SUCCESS && !cargo {
        hooks::check(&plan.settings, &plan.written, save)?;
    }
    Ok(exit)
}
