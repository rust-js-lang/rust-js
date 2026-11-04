//! How the crate is built, as its command asks.

/// How the crate is built, as its command asks.
pub struct Built<'a> {
    /// For others to use, a library (ADR 0100), or a `program` its tests use.
    pub library: bool,
    /// `--tested`: a program whose contract its tests are held to (#597).
    pub program: bool,
    /// Given std's bodies to lower (ADR 0561).
    pub std_bodies: bool,
    /// Asked to abort on a panic, `-C panic=abort`, which rust-js unwinds
    /// (ADR 0090): unwinding is refused of it (ADR 0660).
    pub panics_abort: bool,
    /// `--tidy-until <stage/pass>`: tidy stops after that pass (ADR 0659).
    pub tidy_until: Option<&'a str>,
    /// What the JS modules its bindings import load of the crate's modules
    /// (ADR 0031, amended).
    pub js_modules: &'a dyn crate::program::JsModules,
}
