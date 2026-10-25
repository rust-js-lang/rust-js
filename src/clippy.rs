//! `rust-js clippy` (ADR 0888): clippy's lints, which rust-js links, given
//! rustc as clippy-driver gives them. `main.rs`'s `Clippy` checks a crate of
//! the app's own with them, as `--rustc` checks a binding crate: rust-js's
//! tool, its JSX and its liveness (ADR 0440), so what JS uses is used.
//! Nothing is lowered and no JS is written: rustc writes the metadata Cargo
//! asked for, and stops.
//!
//! Its lint flags, `-D warnings` say, are `cargo::CLIPPY`'s, which the tooling
//! sets, as `cargo clippy` sets clippy-driver's `CLIPPY_ARGS`; Cargo checks
//! the crate again where they change, as it's told they're read.

use crate::command::RUST_JS_CFG;

/// rustc's arguments for the crate Cargo's `flags` check, linted with
/// `lints`: rust-js's cfg, and clippy's, `cfg(clippy)`, as clippy-driver's.
pub fn args(flags: Vec<String>, lints: Vec<String>) -> Vec<String> {
    (std::iter::once("rust-js").chain(RUST_JS_CFG).chain(["--cfg=clippy"]))
        .map(String::from)
        .chain(flags)
        .chain(lints)
        .collect()
}

/// Clippy's lints registered, `clippy.toml` read, and rustc's options for
/// them, as clippy-driver's; `value`, `cargo::CLIPPY`'s, is what Cargo is
/// told the check read.
#[cfg(not(target_os = "wasi"))]
pub fn configure(config: &mut rustc_interface::interface::Config, value: Option<String>) {
    use rustc_span::Symbol;
    config.track_state = Some(Box::new(move |sess| {
        let mut read = sess.env_depinfo.borrow_mut();
        read.insert((
            Symbol::intern(crate::cargo::CLIPPY),
            value.as_deref().map(Symbol::intern),
        ));
        // Where `clippy.toml` is, as clippy-driver has Cargo read it.
        let dir = std::env::var("CLIPPY_CONF_DIR").ok();
        read.insert((Symbol::intern("CLIPPY_CONF_DIR"), dir.as_deref().map(Symbol::intern)));
    }));
    let previous = config.register_lints.take();
    config.register_lints = Some(Box::new(move |sess, store| {
        if let Some(previous) = &previous {
            previous(sess, store);
        }
        let mut lints = declare_clippy_lint::LintListBuilder::default();
        lints.insert(clippy_lints::declared_lints::LINTS);
        lints.register(store);
        clippy_lints::register_lint_passes(store, clippy_config::Conf::load(sess));
    }));
    config.extra_symbols = clippy_utils::sym::EXTRA_SYMBOLS.into();
    // clippy-driver's: its MIR lints read MIR unoptimized, and its
    // `format!` lints a HIR that matches the AST. Nothing is lowered of
    // either, which rust-js would read otherwise.
    config.opts.unstable_opts.mir_opt_level = Some(0);
    config.opts.unstable_opts.mir_enable_passes =
        vec![("CheckNull".to_owned(), false), ("CheckAlignment".to_owned(), false)];
    config.opts.unstable_opts.flatten_format_args = false;
}
